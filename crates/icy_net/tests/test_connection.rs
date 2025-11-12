use async_trait::async_trait;
use std::collections::VecDeque;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use icy_net::{
    Connection, ConnectionType,
    protocol::{Protocol, TransferState},
};

pub struct TestConnection {
    buffer: VecDeque<u8>,                 // leftover bytes from a partially consumed message
    rx: UnboundedReceiver<Vec<u8>>,       // inbound messages from peer
    tx: Option<UnboundedSender<Vec<u8>>>, // outbound sender to peer (Option so we can drop it)
    closed: bool,                         // peer closed (rx exhausted)
}

impl TestConnection {
    pub fn new(rx: UnboundedReceiver<Vec<u8>>, tx: UnboundedSender<Vec<u8>>) -> Self {
        Self {
            buffer: VecDeque::new(),
            rx,
            tx: Some(tx),
            closed: false,
        }
    }

    /// Create a bidirectional pair of channel connections.
    pub fn create_pair() -> (TestConnection, TestConnection) {
        let (a_tx, a_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
        let (b_tx, b_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
        let a = TestConnection::new(a_rx, b_tx);
        let b = TestConnection::new(b_rx, a_tx);
        (a, b)
    }

    /// Gracefully close the outbound direction.
    pub fn shutdown_tx(&mut self) {
        // Dropping sender signals peer receiver end-of-stream.
        self.tx = None;
    }

    fn drain_into(&mut self, buf: &mut [u8]) -> usize {
        let n = buf.len().min(self.buffer.len());
        for i in 0..n {
            buf[i] = self.buffer.pop_front().unwrap();
        }
        n
    }
    /*
    pub async fn write_u8(&mut self, byte: u8) -> icy_net::Result<()> {
        self.send(&[byte]).await
    }*/
}

#[async_trait]
impl Connection for TestConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Raw // Use Raw if Channel doesn't exist yet
    }

    async fn read(&mut self, buf: &mut [u8]) -> icy_net::Result<usize> {
        // Serve leftover first
        if !self.buffer.is_empty() {
            return Ok(self.drain_into(buf));
        }

        if self.closed {
            return Ok(0); // EOF
        }

        match self.rx.recv().await {
            Some(mut data) => {
                if data.is_empty() {
                    // Ignore empty frames
                    return self.read(buf).await; // Recursively try again
                }
                let take = buf.len().min(data.len());
                buf[..take].copy_from_slice(&data[..take]);
                if take < data.len() {
                    // Store leftovers
                    for b in data.drain(take..) {
                        self.buffer.push_back(b);
                    }
                }
                Ok(take)
            }
            _ => {
                self.closed = true;
                Ok(0)
            }
        }
    }

    async fn try_read(&mut self, buf: &mut [u8]) -> icy_net::Result<usize> {
        if !self.buffer.is_empty() {
            return Ok(self.drain_into(buf));
        }
        if self.closed {
            return Ok(0);
        }

        match self.rx.try_recv() {
            Ok(mut data) => {
                if data.is_empty() {
                    return Ok(0);
                }
                let take = buf.len().min(data.len());
                buf[..take].copy_from_slice(&data[..take]);
                if take < data.len() {
                    for b in data.drain(take..) {
                        self.buffer.push_back(b);
                    }
                }
                Ok(take)
            }
            Err(_) => {
                // Either no data available or channel closed
                Ok(0)
            }
        }
    }

    async fn send(&mut self, buf: &[u8]) -> icy_net::Result<()> {
        if let Some(tx) = &self.tx {
            if tx.send(buf.to_vec()).is_err() {
                // peer closed; treat as success or map to your error type
            }
        }
        Ok(())
    }

    async fn shutdown(&mut self) -> icy_net::Result<()> {
        self.shutdown_tx();
        Ok(())
    }

    async fn read_u8(&mut self) -> icy_net::Result<u8> {
        let mut buf = [0u8; 1];
        loop {
            let n = self.read(&mut buf).await?;
            if n == 0 {
                // EOF - you might want to return an error here instead
                return Err("Connection closed".into());
            }
            return Ok(buf[0]);
        }
    }
}

#[allow(unused)]
pub async fn test_sender(con: &mut TestConnection, proto: &mut dyn Protocol, files: &[std::path::PathBuf]) -> TransferState {
    let mut state = proto.initiate_send(con, files).await.expect("init send failed");
    while !state.is_finished {
        proto.update_transfer(con, &mut state).await.expect("update send failed");
    }
    state
}

#[allow(unused)]
pub async fn test_receiver(con: &mut TestConnection, proto: &mut dyn Protocol) -> TransferState {
    let mut state = proto.initiate_recv(con).await.expect("init recv failed");
    while !state.is_finished {
        proto.update_transfer(con, &mut state).await.expect("update recv failed");
    }
    state
}
