#![allow(dead_code)]
use std::{
    io::ErrorKind,
    net::{TcpStream, ToSocketAddrs},
};
use async_ssh2_tokio::client::{Client, AuthMethod, ServerCheckMethod};
use async_trait::async_trait;
use russh::{client::Msg, Channel};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::{telnet::TermCaps, Connection, ConnectionType, NetError};
pub struct SSHConnection {
    client: Client,
    channel: Channel<Msg>,
}

pub struct Credentials {
    pub user_name: String,
    pub password: String,
    pub proxy_command: Option<String>,
}

const SUPPORTED_CIPHERS: &str = "aes128-ctr,aes192-ctr,aes256-ctr,aes128-gcm,aes128-gcm@openssh.com,aes256-gcm,aes256-gcm@openssh.com,aes256-cbc,aes192-cbc,aes128-cbc,blowfish-cbc,3des-cbc,arcfour256,arcfour128,cast128-cbc,arcfour";
const SUPPORTED_KEY_EXCHANGES: &str = "ecdh-sha2-nistp256,ecdh-sha2-nistp384,ecdh-sha2-nistp521,diffie-hellman-group14-sha1,diffie-hellman-group1-sha1";

impl SSHConnection {
    pub async fn open<A: ToSocketAddrs>(addr: &A, caps: TermCaps, credentials: Credentials) -> crate::Result<Self> {
        for addr in addr.to_socket_addrs()? {
            let auth_method = AuthMethod::with_password(&credentials.password);
            let client = Client::connect(
                addr,
                &credentials.user_name,
                auth_method,
                ServerCheckMethod::NoCheck,
            ).await?;
            let channel = client.get_channel().await?;
            let terminal_type: String = format!("{:?}", caps.terminal).to_lowercase();
            channel.request_pty(false, &terminal_type, caps.window_size.0 as u32, caps.window_size.1 as u32, 1, 1,&[]).await?;
            channel.request_shell(false).await?;

            return Ok(Self {
                client,
                channel: channel,
            });
        }

        /* 

        for addr in addr.to_socket_addrs()? {
            let session = Session::new()?;
            let host = addr.ip().to_string();
            let mut port = addr.port();

            if port == 0 {
                port = Self::default_port();
            }
            session.set_option(SshOption::Hostname(host))?;
            session.set_option(SshOption::Port(port))?;
            session.set_option(SshOption::KeyExchange(SUPPORTED_KEY_EXCHANGES.to_string()))?;
            session.set_option(SshOption::CiphersCS(SUPPORTED_CIPHERS.to_string()))?;
            session.set_option(SshOption::CiphersSC(SUPPORTED_CIPHERS.to_string()))?;
            session.set_option(SshOption::Timeout(Duration::from_millis(5000)))?;
            session.set_option(SshOption::LogLevel(libssh_rs::LogLevel::Warning))?;
            session.set_option(SshOption::ProxyCommand(credentials.proxy_command))?;
            session.connect()?;

            //  :TODO: SECURITY: verify_known_hosts() implemented here -- ie: user must accept & we save somewhere
            session.userauth_password(Some(credentials.user_name.as_str()), Some(credentials.password.as_str()))?;

            let chan = session.new_channel()?;
            chan.open_session()?;
            let terminal_type = format!("{:?}", caps.terminal).to_lowercase();
            chan.request_pty(terminal_type.as_str(), caps.window_size.0 as u32, caps.window_size.1 as u32)?;
            chan.request_shell()?;
            session.set_blocking(false);
        }
        */

        Err(NetError::CouldNotConnect.into())

    }

    fn default_port() -> u16 {
        22
    }

    pub fn accept(_tcp_stream: TcpStream) -> crate::Result<Self> {
        todo!("SSHConnection::accept");
    }
}

#[async_trait]
impl Connection for SSHConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::SSH
    }

    async fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        match self.channel.make_reader().read(buf).await {
            Ok(size) => Ok(size),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    return Ok(0);
                }
                Err(std::io::Error::new(ErrorKind::ConnectionAborted, format!("Connection aborted: {e}")).into())
            }
        }
    }

    async fn send(&mut self, buf: &[u8]) -> crate::Result<()>
    {
        self.channel.make_writer().write_all(buf).await?;
        Ok(())
    }

    async fn shutdown(&mut self) -> crate::Result<()> {
        self.client.disconnect().await?;
        Ok(())
    }
}
