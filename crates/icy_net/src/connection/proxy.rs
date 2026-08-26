//! Per-connection proxy support.
//!
//! Currently only SOCKS5 with *remote* DNS is supported. That is exactly what
//! is needed to reach BBSes over Tor (`.onion`) and I2P (`.i2p`): the target
//! hostname must be resolved by the proxy, not locally.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_socks::tcp::Socks5Stream;

use super::ssh::SecretString;

/// Kind of proxy. Only SOCKS5 is supported for now; kept as an enum so other
/// kinds can be added without changing the config shape.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProxyKind {
    #[default]
    Socks5,
}

/// Per-connection proxy configuration.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub kind: ProxyKind,
    pub host: String,
    pub port: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<SecretString>,
}

impl ProxyConfig {
    /// SOCKS5 proxy without authentication (e.g. Tor at 127.0.0.1:9050).
    pub fn socks5(host: impl Into<String>, port: u16) -> Self {
        Self {
            kind: ProxyKind::Socks5,
            host: host.into(),
            port,
            username: None,
            password: None,
        }
    }
}

fn err(msg: &str) -> Box<dyn std::error::Error + Send + Sync> {
    msg.into()
}

/// Split a `host:port` endpoint without resolving the host, so DNS can happen
/// at the proxy (required for `.onion` / `.i2p`). Handles bracketed IPv6.
fn split_host_port(endpoint: &str) -> crate::Result<(String, u16)> {
    let endpoint = endpoint.trim();
    if let Some(rest) = endpoint.strip_prefix('[') {
        let close = rest.find(']').ok_or_else(|| err("unterminated IPv6 literal in proxy target"))?;
        let host = &rest[..close];
        let port = rest[close + 1..]
            .strip_prefix(':')
            .and_then(|p| p.parse().ok())
            .ok_or_else(|| err("proxy target must be host:port"))?;
        return Ok((host.to_string(), port));
    }
    let (host, port) = endpoint.rsplit_once(':').ok_or_else(|| err("proxy target must be host:port"))?;
    let port: u16 = port.parse().map_err(|_| err("invalid proxy target port"))?;
    Ok((host.to_string(), port))
}

/// Open a TCP connection to `endpoint` (`host:port`), optionally through a
/// proxy. With a SOCKS5 proxy the hostname is resolved by the proxy (remote
/// DNS), so `.onion` and `.i2p` targets work.
pub async fn connect_tcp(endpoint: &str, proxy: Option<&ProxyConfig>, timeout: Duration) -> crate::Result<TcpStream> {
    match proxy {
        None => Ok(tokio::time::timeout(timeout, TcpStream::connect(endpoint)).await??),
        Some(proxy) => {
            let (host, port) = split_host_port(endpoint)?;
            let target = (host.as_str(), port);
            let authentication = match (proxy.username.as_deref(), proxy.password.as_ref()) {
                (Some(username), Some(password)) => Some((username, password.expose_secret())),
                (None, None) => None,
                _ => return Err(err("proxy username and password must be configured together")),
            };
            let connect = async {
                let proxy_addrs: Vec<_> = tokio::net::lookup_host((proxy.host.as_str(), proxy.port)).await?.collect();
                match authentication {
                    Some((username, password)) => Socks5Stream::connect_with_password(proxy_addrs.as_slice(), target, username, password).await,
                    None => Socks5Stream::connect(proxy_addrs.as_slice(), target).await,
                }
            };
            let stream = tokio::time::timeout(timeout, connect).await??;
            Ok(stream.into_inner())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv6Addr, SocketAddr};

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;

    #[test]
    fn splits_hostnames_and_bracketed_ipv6_targets() {
        assert_eq!(("example.com".to_string(), 23), split_host_port("example.com:23").unwrap());
        assert_eq!(("::1".to_string(), 22), split_host_port("[::1]:22").unwrap());
        assert!(split_host_port("example.com").is_err());
        assert!(split_host_port("[::1:22").is_err());
    }

    #[test]
    fn proxy_password_is_redacted_but_still_serializes() {
        let proxy = ProxyConfig {
            username: Some("user".to_string()),
            password: Some(SecretString::new("secret")),
            ..ProxyConfig::socks5("localhost", 1080)
        };

        assert!(!format!("{proxy:?}").contains("secret"));
        assert_eq!("secret", serde_json::to_value(proxy).unwrap()["password"]);
    }

    #[tokio::test]
    async fn incomplete_credentials_are_rejected_before_connecting() {
        let proxy = ProxyConfig {
            username: Some("user".to_string()),
            ..ProxyConfig::socks5("not-resolved.invalid", 1080)
        };

        let error = connect_tcp("example.com:23", Some(&proxy), Duration::from_secs(1)).await.unwrap_err();
        assert_eq!("proxy username and password must be configured together", error.to_string());
    }

    #[tokio::test]
    async fn sends_the_target_hostname_to_the_proxy() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let proxy_addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut greeting = [0; 3];
            stream.read_exact(&mut greeting).await.unwrap();
            assert_eq!([5, 1, 0], greeting);
            stream.write_all(&[5, 0]).await.unwrap();

            let mut request = [0; 4];
            stream.read_exact(&mut request).await.unwrap();
            assert_eq!([5, 1, 0, 3], request);
            let name_len = stream.read_u8().await.unwrap() as usize;
            let mut hostname = vec![0; name_len];
            stream.read_exact(&mut hostname).await.unwrap();
            assert_eq!(b"hidden-service.onion", hostname.as_slice());
            assert_eq!(23, stream.read_u16().await.unwrap());
            stream.write_all(&[5, 0, 0, 1, 127, 0, 0, 1, 0, 23]).await.unwrap();
        });

        let proxy = ProxyConfig::socks5(proxy_addr.ip().to_string(), proxy_addr.port());
        connect_tcp("hidden-service.onion:23", Some(&proxy), Duration::from_secs(1)).await.unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn accepts_an_ipv6_proxy_address() {
        let Ok(listener) = TcpListener::bind((Ipv6Addr::LOCALHOST, 0)).await else {
            return;
        };
        let proxy_addr = listener.local_addr().unwrap();
        let server = tokio::spawn(accept_unauthenticated_connection(listener));
        let proxy = ProxyConfig::socks5(proxy_addr.ip().to_string(), proxy_addr.port());

        connect_tcp("example.com:23", Some(&proxy), Duration::from_secs(1)).await.unwrap();
        server.await.unwrap();
    }

    async fn accept_unauthenticated_connection(listener: TcpListener) {
        let (mut stream, _): (_, SocketAddr) = listener.accept().await.unwrap();
        let mut greeting = [0; 3];
        stream.read_exact(&mut greeting).await.unwrap();
        stream.write_all(&[5, 0]).await.unwrap();
        let mut request = [0; 4];
        stream.read_exact(&mut request).await.unwrap();
        let name_len = stream.read_u8().await.unwrap() as usize;
        let mut remainder = vec![0; name_len + 2];
        stream.read_exact(&mut remainder).await.unwrap();
        stream.write_all(&[5, 0, 0, 1, 127, 0, 0, 1, 0, 23]).await.unwrap();
    }
}
