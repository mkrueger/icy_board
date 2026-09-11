use crate::NetError;
pub mod channel;
pub mod modem;
pub mod proxy;
pub mod raw;
pub mod rlogin;
pub mod serial;
pub mod ssh;
pub mod telnet;
pub mod websocket;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub struct ConnectionData {
    pub address: String,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    Rlogin,
    RloginSwapped,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
    #[default]
    Disconnected,
    Connected,
}

#[async_trait]
pub trait Connection: Send + Unpin {
    fn get_connection_type(&self) -> ConnectionType;

    /// Consume the latest positive (columns, rows) report parsed during I/O.
    /// Reports may coalesce and never appear as payload bytes or EOF.
    fn take_terminal_size_change(&mut self) -> Option<(u16, u16)> {
        None
    }

    /// Wait for payload bytes, EOF, or an error. For a nonempty buffer, `Ok(0)`
    /// means EOF, never temporary inactivity or a protocol-only/empty record.
    /// An empty buffer may return `Ok(0)` without indicating EOF.
    async fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize>;

    /// Read currently available payload without waiting indefinitely for data.
    /// Unlike `read`, `Ok(0)` may mean no payload is available yet (including
    /// protocol-only activity); it is not sufficient to identify EOF.
    async fn try_read(&mut self, buf: &mut [u8]) -> crate::Result<usize>;

    async fn send(&mut self, buf: &[u8]) -> crate::Result<()>;

    async fn poll(&mut self) -> crate::Result<ConnectionState> {
        Ok(ConnectionState::Connected)
    }

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
        self.read_exact(&mut buf).await?;
        Ok(buf[0])
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

    async fn try_read(&mut self, _buf: &mut [u8]) -> crate::Result<usize> {
        Ok(0)
    }

    async fn send(&mut self, _buf: &[u8]) -> crate::Result<()> {
        Err(NetError::Unsupported.into())
    }

    async fn shutdown(&mut self) -> crate::Result<()> {
        Err(NetError::Unsupported.into())
    }
}
