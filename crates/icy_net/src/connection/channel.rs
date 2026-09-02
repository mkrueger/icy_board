use std::collections::VecDeque;

use async_trait::async_trait;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::{Connection, ConnectionType};

pub struct ChannelConnection {
    buffer: VecDeque<u8>,
    rx: UnboundedReceiver<Vec<u8>>,
    tx: Option<UnboundedSender<Vec<u8>>>,
}
unsafe impl Send for ChannelConnection {}
unsafe impl Sync for ChannelConnection {}

impl ChannelConnection {
    pub fn new(rx: UnboundedReceiver<Vec<u8>>, tx: UnboundedSender<Vec<u8>>) -> Self {
        Self {
            buffer: VecDeque::new(),
            rx,
            tx: Some(tx),
        }
    }

    pub fn create_pair() -> (ChannelConnection, ChannelConnection) {
        let (tx1, rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, rx2) = tokio::sync::mpsc::unbounded_channel();
        (ChannelConnection::new(rx1, tx2), ChannelConnection::new(rx2, tx1))
    }
}

#[async_trait]
impl Connection for ChannelConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Channel
    }

    async fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        if !self.buffer.is_empty() {
            let len = self.buffer.len().min(buf.len());
            buf[..len].copy_from_slice(&self.buffer.drain(..len).collect::<Vec<u8>>());
            return Ok(len);
        }
        match self.rx.recv().await {
            Some(data) => {
                if data.is_empty() {
                    return Ok(0);
                }
                let len = data.len().min(buf.len());
                buf[..len].copy_from_slice(&data[..len]);
                self.buffer.extend(data.into_iter().skip(len));
                Ok(len)
            }
            None => Ok(0),
        }
    }

    async fn try_read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        if !self.buffer.is_empty() {
            let len = self.buffer.len().min(buf.len());
            buf[..len].copy_from_slice(&self.buffer.drain(..len).collect::<Vec<u8>>());
            return Ok(len);
        }
        match self.rx.try_recv() {
            Ok(data) => {
                if data.is_empty() {
                    return Ok(0);
                }
                let len = data.len().min(buf.len());
                buf[..len].copy_from_slice(&data[..len]);
                self.buffer.extend(data.into_iter().skip(len));
                Ok(len)
            }
            _ => Ok(0),
        }
    }

    async fn send(&mut self, buf: &[u8]) -> crate::Result<()> {
        self.tx.as_ref().ok_or(crate::NetError::ConnectionClosed)?.send(buf.to_vec())?;
        Ok(())
    }

    async fn shutdown(&mut self) -> crate::Result<()> {
        self.tx = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn shutdown_makes_the_peer_read_return_eof() {
        let (mut local, mut peer) = ChannelConnection::create_pair();
        local.shutdown().await.unwrap();
        let mut buf = [0; 8];
        assert_eq!(peer.read(&mut buf).await.unwrap(), 0);
    }
}
