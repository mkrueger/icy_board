#![allow(dead_code)]
use async_trait::async_trait;
use russh::{client::Msg, *};
use russh_keys::*;
use std::{borrow::Cow, collections::HashMap, io::ErrorKind, sync::Arc, time::Duration};

use crate::{telnet::TermCaps, Connection, ConnectionType};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::Mutex,
};

pub struct SSHConnection {
    client: SshClient,
    channel: Channel<Msg>,
}

pub struct Credentials {
    pub user_name: String,
    pub password: String,
    pub proxy_command: Option<String>,
}

impl SSHConnection {
    pub async fn open(addr: impl Into<String>, caps: TermCaps, credentials: Credentials) -> crate::Result<Self> {
        let mut addr: String = addr.into();
        if !addr.contains(':') {
            addr.push_str(":22");
        }
        let ssh = SshClient::connect(addr, &credentials.user_name, credentials.password).await?;
        let channel = ssh.session.channel_open_session().await?;
        let terminal_type: String = format!("{:?}", caps.terminal).to_lowercase();
        channel
            .request_pty(false, &terminal_type, caps.window_size.0 as u32, caps.window_size.1 as u32, 1, 1, &[])
            .await?;
        channel.request_shell(false).await?;
        return Ok(Self { client: ssh, channel });
    }

    fn default_port() -> u16 {
        22
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
            Err(e) => match e.kind() {
                ErrorKind::ConnectionAborted | ErrorKind::NotConnected => {
                    return Err(std::io::Error::new(ErrorKind::ConnectionAborted, format!("Connection aborted: {e}")).into());
                }
                ErrorKind::WouldBlock => Ok(0),
                _ => {
                    log::error!("Error {:?} reading from SSH connection: {:?}", e.kind(), e);
                    Ok(0)
                }
            },
        }
    }

    async fn try_read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        match self.channel.make_reader().read(buf).await {
            Ok(size) => Ok(size),
            Err(e) => match e.kind() {
                ErrorKind::ConnectionAborted | ErrorKind::NotConnected => {
                    return Err(std::io::Error::new(ErrorKind::ConnectionAborted, format!("Connection aborted: {e}")).into());
                }
                ErrorKind::WouldBlock => Ok(0),
                _ => {
                    log::error!("Error {:?} reading from SSH connection: {:?}", e.kind(), e);
                    Ok(0)
                }
            },
        }
    }

    async fn send(&mut self, buf: &[u8]) -> crate::Result<()> {
        self.channel.make_writer().write_all(buf).await?;
        Ok(())
    }

    async fn shutdown(&mut self) -> crate::Result<()> {
        self.client.session.disconnect(Disconnect::ByApplication, "bye", "en").await?;
        Ok(())
    }
}

#[derive(Clone)]
struct Server {
    clients: Arc<Mutex<HashMap<usize, (ChannelId, russh::server::Handle)>>>,
    id: usize,
}

struct Client {}

impl russh::client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(&mut self, _: &ssh_key::PublicKey) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

pub struct SshClient {
    session: client::Handle<Client>,
}

impl SshClient {
    async fn connect(addr: impl Into<String>, user: impl Into<String>, password: impl Into<String>) -> crate::Result<Self> {
        let mut addr: String = addr.into();
        if !addr.contains(':') {
            addr.push_str(":22");
        }

        let mut preferred = Preferred::DEFAULT.clone();
        preferred.kex = Cow::Owned(kex::ALL_KEX_ALGORITHMS.iter().map(|k| **k).collect());
        preferred.cipher = Cow::Owned(cipher::ALL_CIPHERS.iter().map(|k| **k).collect());
        let config = client::Config {
            inactivity_timeout: None,
            preferred,
            keepalive_interval: Some(Duration::from_secs(30)),
            keepalive_max: 3,
            ..<_>::default()
        };
        let config = Arc::new(config);
        let sh = Client {};
        let timeout = Duration::from_secs(5);
        let result = tokio::time::timeout(timeout, TcpStream::connect(addr)).await;
        match result {
            Ok(tcp_stream) => match tcp_stream {
                Ok(tcp_stream) => {
                    tcp_stream.set_nodelay(true)?;
                    let mut session: client::Handle<Client> = russh::client::connect_stream(config, tcp_stream, sh).await?;

                    let auth_res = session.authenticate_password(user, password).await?;
                    if !auth_res.success() {
                        return Err("Authentication failed".into());
                    }

                    Ok(Self { session })
                }

                Err(err) => Err(Box::new(err)),
            },
            Err(err) => Err(Box::new(err)),
        }
    }

    async fn call(&mut self, command: &str) -> crate::Result<u32> {
        let mut channel = self.session.channel_open_session().await?;
        channel.exec(true, command).await?;

        let mut code = None;
        let mut stdout = tokio::io::stdout();

        loop {
            // There's an event available on the session channel
            let Some(msg) = channel.wait().await else {
                break;
            };
            match msg {
                // Write data to the terminal
                ChannelMsg::Data { ref data } => {
                    stdout.write_all(data).await?;
                    stdout.flush().await?;
                }
                // The command has returned an exit code
                ChannelMsg::ExitStatus { exit_status } => {
                    code = Some(exit_status);
                    // cannot leave the loop immediately, there might still be more data to receive
                }
                _ => {}
            }
        }
        Ok(code.expect("program did not exit cleanly"))
    }

    async fn close(&mut self) -> crate::Result<()> {
        self.session.disconnect(Disconnect::ByApplication, "", "English").await?;
        Ok(())
    }
}
