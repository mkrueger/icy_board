//! Per-connection proxy support.
//!
//! Currently only SOCKS5 with *remote* DNS is supported. That is exactly what
//! is needed to reach BBSes over Tor (`.onion`) and I2P (`.i2p`): the target
//! hostname must be resolved by the proxy, not locally.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_socks::tcp::Socks5Stream;

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
    pub password: Option<String>,
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
            let proxy_addr = format!("{}:{}", proxy.host, proxy.port);
            let target = (host.as_str(), port);
            let connect = async {
                match (proxy.username.as_deref(), proxy.password.as_deref()) {
                    (Some(user), Some(pass)) => Socks5Stream::connect_with_password(proxy_addr.as_str(), target, user, pass).await,
                    _ => Socks5Stream::connect(proxy_addr.as_str(), target).await,
                }
            };
            let stream = tokio::time::timeout(timeout, connect).await??;
            Ok(stream.into_inner())
        }
    }
}
