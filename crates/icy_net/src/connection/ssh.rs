#![allow(dead_code)]
use async_trait::async_trait;
use russh::keys::ssh_key;
use russh::{client::Msg, *};
use std::{borrow::Cow, collections::HashMap, sync::Arc, time::Duration};

use crate::ConnectionState;
use crate::{Connection, ConnectionType, telnet::TermCaps};
use tokio::{
    io::AsyncWriteExt,
    net::TcpStream,
    sync::Mutex,
};

pub struct SSHConnection {
    client: SshClient,
    channel: Channel<Msg>,
    read_buffer: Vec<u8>,  // Add internal buffer for non-blocking reads
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
        return Ok(Self { 
            client: ssh, 
            channel,
            read_buffer: Vec::new(),  // Initialize empty buffer
        });
    }

    fn default_port() -> u16 {
        22
    }

    // Helper method to fill buffer from channel messages without blocking
    async fn fill_buffer_nonblocking(&mut self) -> crate::Result<()> {
        // Use a very short timeout to make this non-blocking
        let timeout = Duration::from_millis(1);
        
        loop {
            match tokio::time::timeout(timeout, self.channel.wait()).await {
                Ok(Some(msg)) => {
                    match msg {
                        ChannelMsg::Data { data } => {
                            // Add data to our buffer
                            self.read_buffer.extend_from_slice(&data);
                        }
                        ChannelMsg::Eof => {
                            // Channel received EOF, connection is ending
                            return Ok(());
                        }
                        ChannelMsg::Close => {
                            // Channel is closing
                            return Ok(());
                        }
                        _ => {
                            // Other messages, continue
                        }
                    }
                }
                Ok(None) => {
                    // Channel closed
                    return Ok(());
                }
                Err(_) => {
                    // Timeout - no more messages available right now
                    return Ok(());
                }
            }
        }
    }
}

#[async_trait]
impl Connection for SSHConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::SSH
    }

    async fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        // First check if we have buffered data
        if !self.read_buffer.is_empty() {
            let to_read = buf.len().min(self.read_buffer.len());
            buf[..to_read].copy_from_slice(&self.read_buffer[..to_read]);
            self.read_buffer.drain(..to_read);
            return Ok(to_read);
        }

        // No buffered data, wait for new data from the channel
        loop {
            let Some(msg) = self.channel.wait().await else {
                // Channel closed
                return Ok(0);
            };
            
            match msg {
                ChannelMsg::Data { data } => {
                    // We got data, copy what we can to the buffer
                    let to_read = buf.len().min(data.len());
                    buf[..to_read].copy_from_slice(&data[..to_read]);
                    
                    // If there's leftover data, store it in our buffer
                    if data.len() > to_read {
                        self.read_buffer.extend_from_slice(&data[to_read..]);
                    }
                    
                    return Ok(to_read);
                }
                ChannelMsg::Eof | ChannelMsg::Close => {
                    // Connection is closing
                    return Ok(0);
                }
                _ => {
                    // Other messages, continue waiting
                    continue;
                }
            }
        }
    }

    async fn poll(&mut self) -> crate::Result<ConnectionState> {
        // Check if the session is closed
        if self.client.session.is_closed() {
            return Ok(ConnectionState::Disconnected);
        }

        // Try to fill buffer without blocking
        self.fill_buffer_nonblocking().await?;

        // Use timeout to check if channel is still responsive
        let timeout = Duration::from_millis(1);
        match tokio::time::timeout(timeout, self.channel.wait()).await {
            Ok(Some(msg)) => {
                match msg {
                    ChannelMsg::Data { data } => {
                        // We got data during poll, buffer it
                        self.read_buffer.extend_from_slice(&data);
                        Ok(ConnectionState::Connected)
                    }
                    ChannelMsg::Eof | ChannelMsg::Close => {
                        log::debug!("SSH channel received EOF/Close");
                        Ok(ConnectionState::Disconnected)
                    }
                    _ => Ok(ConnectionState::Connected)
                }
            }
            Ok(None) => {
                // Channel is closed
                Ok(ConnectionState::Disconnected)
            }
            Err(_) => {
                // Timeout - no messages pending, connection is still active
                Ok(ConnectionState::Connected)
            }
        }
    }

    async fn try_read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        // First check if we have buffered data
        if !self.read_buffer.is_empty() {
            let to_read = buf.len().min(self.read_buffer.len());
            buf[..to_read].copy_from_slice(&self.read_buffer[..to_read]);
            self.read_buffer.drain(..to_read);
            return Ok(to_read);
        }

        // Try to fill buffer without blocking
        self.fill_buffer_nonblocking().await?;

        // Check buffer again after attempting to fill
        if !self.read_buffer.is_empty() {
            let to_read = buf.len().min(self.read_buffer.len());
            buf[..to_read].copy_from_slice(&self.read_buffer[..to_read]);
            self.read_buffer.drain(..to_read);
            Ok(to_read)
        } else {
            // No data available
            Ok(0)
        }
    }

    async fn send(&mut self, buf: &[u8]) -> crate::Result<()> {
        self.channel.make_writer().write_all(buf).await?;
        Ok(())
    }

    async fn shutdown(&mut self) -> crate::Result<()> {
        self.channel.eof().await?;
        self.channel.close().await?;

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
