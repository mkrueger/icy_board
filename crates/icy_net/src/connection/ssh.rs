#![allow(dead_code)]
use async_trait::async_trait;
use russh::{client::Msg, *};
use russh_keys::*;
use std::{io::ErrorKind, net::TcpStream, sync::Arc, time::Duration};

use crate::{telnet::TermCaps, Connection, ConnectionType};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::ToSocketAddrs,
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

const SUPPORTED_CIPHERS: &str = "aes128-ctr,aes192-ctr,aes256-ctr,aes128-gcm,aes128-gcm@openssh.com,aes256-gcm,aes256-gcm@openssh.com,aes256-cbc,aes192-cbc,aes128-cbc,blowfish-cbc,3des-cbc,arcfour256,arcfour128,cast128-cbc,arcfour";
const SUPPORTED_KEY_EXCHANGES: &str = "ecdh-sha2-nistp256,ecdh-sha2-nistp384,ecdh-sha2-nistp521,diffie-hellman-group14-sha1,diffie-hellman-group1-sha1";

impl SSHConnection {
    pub async fn open<A: ToSocketAddrs>(addr: &A, caps: TermCaps, credentials: Credentials) -> crate::Result<Self> {
        let ssh = SshClient::connect(addr, &credentials.user_name, credentials.password).await?;
        println!("Connected to SSH server   ");
        let channel = ssh.session.channel_open_session().await?;
        let terminal_type: String = format!("{:?}", caps.terminal).to_lowercase();
        channel
            .request_pty(false, &terminal_type, caps.window_size.0 as u32, caps.window_size.1 as u32, 1, 1, &[])
            .await?;
        channel.request_shell(false).await?;

        return Ok(Self { client: ssh, channel });

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
        Err(NetError::CouldNotConnect.into())
        */
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
            Ok(size) => {
                /*    if size == 0 {
                    return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "").into());
                }*/
                Ok(size)
            }
            Err(e) => Err(std::io::Error::new(ErrorKind::ConnectionAborted, format!("Connection aborted: {e}")).into()),
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

struct Client {}

// More SSH event handlers
// can be defined in this trait
// In this example, we're only using Channel, so these aren't needed.
#[async_trait]
impl client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(&mut self, _server_public_key: &ssh_key::PublicKey) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

pub struct SshClient {
    session: client::Handle<Client>,
}

impl SshClient {
    async fn connect<A: ToSocketAddrs>(addrs: A, user: impl Into<String>, password: impl Into<String>) -> crate::Result<Self> {
        // let key_pair = load_secret_key(key_path, None)?;
        let config = client::Config {
            inactivity_timeout: Some(Duration::from_secs(5)),
            ..<_>::default()
        };

        let config = Arc::new(config);
        let sh = Client {};

        let mut session = client::connect(config, addrs, sh).await?;
        let auth_res = session.authenticate_password(user, password).await?;

        if !auth_res {
            return Err("Authentication failed".into());
        }

        Ok(Self { session })
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
