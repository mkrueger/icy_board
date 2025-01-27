use crate::NetError;
pub mod channel;
pub mod modem;
pub mod raw;
pub mod serial;
pub mod ssh;
pub mod telnet;
pub mod websocket;
use async_trait::async_trait;

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

#[async_trait]
pub trait Connection: Send + Unpin {
    fn get_connection_type(&self) -> ConnectionType;

    async fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize>;

    async fn send(&mut self, buf: &[u8]) -> crate::Result<()>;

    async fn read_exact(&mut self, buf: &mut [u8]) -> crate::Result<()> {
        let mut offset = 0;
        while offset < buf.len() {
            let size = self.read(&mut buf[offset..]).await?;
            if size == 0 {
                return Err(NetError::ConnectionClosed.into());
            }
            offset += size;
        }
        Ok(())
    }

    async fn read_u8(&mut self) -> crate::Result<u8> {
        let mut buf = [0u8; 1];
        loop {
            let size = self.read(&mut buf).await?;
            if size == 1 {
                return Ok(buf[0]);
            }
        }
    }

    async fn shutdown(&mut self) -> crate::Result<()> {
        Ok(())
    }
}

pub struct NullConnection {}

#[async_trait]
impl Connection for NullConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Raw
    }
    async fn read(&mut self, _buf: &mut [u8]) -> crate::Result<usize> {
        Ok(0)
    }

    async fn send(&mut self, _buf: &[u8]) -> crate::Result<()> {
        Err(NetError::Unsupported.into())
    }

    async fn shutdown(&mut self) -> crate::Result<()> {
        Err(NetError::Unsupported.into())
    }
}
