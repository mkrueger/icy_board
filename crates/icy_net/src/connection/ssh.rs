#![allow(dead_code)]
use libssh_rs::{Channel, Session, SshOption};
use std::{
    io::{self, ErrorKind, Read, Write},
    net::ToSocketAddrs,
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
}

const SUPPORTED_CIPHERS: &str = "aes128-ctr,aes192-ctr,aes256-ctr,aes128-gcm,aes128-gcm@openssh.com,aes256-gcm,aes256-gcm@openssh.com,aes256-cbc,aes192-cbc,aes128-cbc,blowfish-cbc,3des-cbc,arcfour256,arcfour128,cast128-cbc,arcfour";
const SUPPORTED_KEY_EXCHANGES: &str = "ecdh-sha2-nistp256,ecdh-sha2-nistp384,ecdh-sha2-nistp521,diffie-hellman-group14-sha1,diffie-hellman-group1-sha1";

impl SSHConnection {
    pub fn open<A: ToSocketAddrs>(addr: &A, caps: TermCaps, credentials: Credentials) -> crate::Result<Self> {
        for addr in addr.to_socket_addrs()? {
            let session = Session::new()?;
            session.set_option(SshOption::Hostname(addr.ip().to_string()))?;
            session.set_option(SshOption::Port(addr.port()))?;
            session.set_option(SshOption::KeyExchange(SUPPORTED_KEY_EXCHANGES.to_string()))?;
            session.set_option(SshOption::CiphersCS(SUPPORTED_CIPHERS.to_string()))?;
            session.set_option(SshOption::CiphersSC(SUPPORTED_CIPHERS.to_string()))?;
            session.set_option(SshOption::Timeout(Duration::from_millis(5000)))?;
            session.set_option(SshOption::LogLevel(libssh_rs::LogLevel::Warning))?;
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
    }

    fn default_port() -> u16 {
        22
    }
    /*
    fn parse_address(addr: &str) -> TermComResult<(String, u16)> {
        let components: Vec<&str> = addr.split(':').collect();
        match components.first() {
            Some(host) => match components.get(1) {
                Some(port_str) => {
                    let port = port_str.parse()?;
                    Ok(((*host).to_string(), port))
                }
                _ => Ok(((*host).to_string(), Self::default_port())),
            },
            _ => Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid address"))),
        }
    }*/
}

impl Connection for SSHConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::SSH
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
                stdout.read(buf)
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
