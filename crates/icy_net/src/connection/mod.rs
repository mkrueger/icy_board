use std::io::{self, Read, Write};

use crate::NetError;
pub mod channel;
pub mod modem;
pub mod raw;
pub mod serial;
pub mod ssh;
pub mod telnet;
pub mod websocket;

pub struct ConnectionData {
    pub address: String,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    Channel,
    Raw,
    #[default]
    Telnet,
    SSH,
    Modem,
    Serial,
    Websocket,
    SecureWebsocket,
}

impl ConnectionType {
    pub fn get_default_port(self) -> u16 {
        match self {
            ConnectionType::Telnet => 23,
            ConnectionType::SSH => 22,
            ConnectionType::Websocket | ConnectionType::SecureWebsocket => 443,
            _ => 0,
        }
    }
}

pub trait Connection: Read + Write {
    fn get_connection_type(&self) -> ConnectionType;

    fn shutdown(&mut self) -> crate::Result<()> {
        Ok(())
    }

    fn read_u8(&mut self) -> crate::Result<u8> {
        let mut buf = [0; 1];
        loop { 
            match self.read(&mut buf) {
                Ok(size) =>  {
                    if size == 0 {
                        continue;
                    }
                    break;
                }
                Err(e) => {
                    println!("Error: {:?}", e);
                    if e.kind() != io::ErrorKind::WouldBlock {
                        return Err(e.into());
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            
            }
        }
        Ok(buf[0])
    }
}

pub struct NullConnection {}

impl Connection for NullConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Raw
    }

    fn shutdown(&mut self) -> crate::Result<()> {
        Err(NetError::Unsupported.into())
    }

    fn read_u8(&mut self) -> crate::Result<u8> {
        Err(NetError::Unsupported.into())
    }
}

impl Read for NullConnection {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::Other, "Cannot read from NullConnection"))
    }
}

impl Write for NullConnection {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::Other, "Cannot write to NullConnection"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
