#![allow(dead_code)]
use libssh_rs::{Channel, Session, SshOption};
use std::{
    io::{self, ErrorKind, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{telnet::TermCaps, Connection, ConnectionType, NetError};
pub struct SSHConnection {
    session: Session,
    channel: Arc<Mutex<Channel>>,
}

pub struct Credentials {
    pub user_name: String,
    pub password: String,
    pub proxy_command: Option<String>,
}

const SUPPORTED_CIPHERS: &str = "aes128-ctr,aes192-ctr,aes256-ctr,aes128-gcm,aes128-gcm@openssh.com,aes256-gcm,aes256-gcm@openssh.com,aes256-cbc,aes192-cbc,aes128-cbc,blowfish-cbc,3des-cbc,arcfour256,arcfour128,cast128-cbc,arcfour";
const SUPPORTED_KEY_EXCHANGES: &str = "ecdh-sha2-nistp256,ecdh-sha2-nistp384,ecdh-sha2-nistp521,diffie-hellman-group14-sha1,diffie-hellman-group1-sha1";

impl SSHConnection {
    pub fn open<A: ToSocketAddrs>(addr: &A, caps: TermCaps, credentials: Credentials) -> crate::Result<Self> {

        let config = thrussh::client::Config::default();
        let config = Arc::new(config);
        let sh = Client {};
    
        let mut agent = thrussh_keys::agent::client::AgentClient::connect_env()
            .await
            .unwrap();
        let mut identities = agent.request_identities().await.unwrap();
        let mut session = thrussh::client::connect(config, "127.0.0.1:2200", sh)
            .await
            .unwrap();
        let (_, auth_res) = session
            .authenticate_future("pe", identities.pop().unwrap(), agent)
            .await;
        let auth_res = auth_res.unwrap();
        println!("=== auth: {}", auth_res);
        let mut channel = session
            .channel_open_direct_tcpip("localhost", 8000, "localhost", 3333)
            .await
            .unwrap();

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
            return Ok(Self {
                session,
                channel: Arc::new(Mutex::new(chan)),
            });
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

impl Connection for SSHConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::SSH
    }

    async fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize>
    {

    }

    async fn send(&mut self, buf: &[u8]) -> crate::Result<()>
    {

    }


    fn shutdown(&mut self) -> crate::Result<()> {
        self.session.disconnect();
        Ok(())
    }
}

impl Read for SSHConnection {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.channel.lock() {
            Ok(locked) => {
                let mut stdout = locked.stdout();
                match stdout.read(buf) {
                    Ok(size) => Ok(size),
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::WouldBlock {
                            return Ok(0);
                        }
                        Err(std::io::Error::new(ErrorKind::ConnectionAborted, format!("Connection aborted: {e}")))
                    }
                }
            }
            Err(err) => Err(std::io::Error::new(ErrorKind::ConnectionAborted, format!("Can't lock channel: {err}"))),
        }
    }
}

impl Write for SSHConnection {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.channel.lock() {
            Ok(locked) => {
                locked.stdin().write_all(buf)?;
                Ok(buf.len())
            }
            Err(err) => Err(std::io::Error::new(ErrorKind::ConnectionAborted, format!("Can't lock channel: {err}"))),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.channel.lock() {
            Ok(locked) => locked.stdin().flush(),
            Err(err) => Err(std::io::Error::new(ErrorKind::ConnectionAborted, format!("Can't lock channel: {err}"))),
        }
    }
}
