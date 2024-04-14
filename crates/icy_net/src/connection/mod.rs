use std::io::{Read, Write};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    Channel,
    Raw,
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
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }
}
