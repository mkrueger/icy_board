#![allow(dead_code)]
use std::{io::ErrorKind, time::Duration};

use async_trait::async_trait;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, ToSocketAddrs},
};

use crate::ConnectionState;

use super::{Connection, ConnectionType};

pub struct RawConnection {
    tcp_stream: TcpStream,
    read_buffer: Vec<u8>,
}

impl RawConnection {
    pub async fn open<A: ToSocketAddrs>(addr: &A, timeout: Duration) -> crate::Result<Self> {
        let result = tokio::time::timeout(timeout, TcpStream::connect(addr)).await;
        match result {
            Ok(tcp_stream) => match tcp_stream {
                Ok(stream) => Ok(Self {
                    tcp_stream: stream,
                    read_buffer: Vec::new(),
                }),
                Err(err) => Err(Box::new(err)),
            },
            Err(err) => Err(Box::new(err)),
        }
    }

    pub async fn accept(tcp_stream: TcpStream) -> crate::Result<Self> {
        Ok(Self {
            tcp_stream,
            read_buffer: Vec::new(),
        })
    }
}

#[async_trait]
impl Connection for RawConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Raw
    }

    async fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        // First, check if we have buffered data from a previous poll
        if !self.read_buffer.is_empty() {
            let to_read = buf.len().min(self.read_buffer.len());
            buf[..to_read].copy_from_slice(&self.read_buffer[..to_read]);
            self.read_buffer.drain(..to_read);
            return Ok(to_read);
        }

        // No buffered data, read from the stream
        let result = self.tcp_stream.read(buf).await?;
        Ok(result)
    }

    async fn try_read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        // First, check if we have buffered data from a previous poll
        if !self.read_buffer.is_empty() {
            let to_read = buf.len().min(self.read_buffer.len());
            buf[..to_read].copy_from_slice(&self.read_buffer[..to_read]);
            self.read_buffer.drain(..to_read);
            return Ok(to_read);
        }

        // No buffered data, try to read from the stream
        match self.tcp_stream.try_read(buf) {
            Ok(size) => Ok(size),
            Err(e) => match e.kind() {
                ErrorKind::ConnectionAborted | ErrorKind::NotConnected => {
                    log::error!("raw connection error reading from TCP stream: {}", e);
                    return Err(std::io::Error::new(ErrorKind::ConnectionAborted, format!("Connection aborted: {e}")).into());
                }
                ErrorKind::WouldBlock => Ok(0),
                _ => {
                    log::error!("Error {:?} reading from raw connection: {:?}", e.kind(), e);
                    Ok(0)
                }
            },
        }
    }

    async fn poll(&mut self) -> crate::Result<ConnectionState> {
        // Try to peek at data without consuming it
        // We use a small buffer to check if data is available
        let mut buf = [0u8; 1];
        match self.tcp_stream.try_read(&mut buf) {
            Ok(0) => {
                // A successful read of 0 bytes means the connection was closed cleanly
                Ok(ConnectionState::Disconnected)
            }
            Ok(n) => {
                // We got data - store it in our buffer for later reading
                // This prevents data loss during polling
                self.read_buffer.extend_from_slice(&buf[..n]);
                Ok(ConnectionState::Connected)
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                // No data available, but connection is still open
                // This is the normal case for an active connection with no pending data
                Ok(ConnectionState::Connected)
            }
            Err(e)
                if matches!(
                    e.kind(),
                    ErrorKind::ConnectionAborted | ErrorKind::ConnectionReset | ErrorKind::NotConnected | ErrorKind::BrokenPipe | ErrorKind::UnexpectedEof
                ) =>
            {
                log::debug!("Raw connection closed: {:?}", e);
                Ok(ConnectionState::Disconnected)
            }
            Err(e) => {
                log::warn!("Raw connection poll error: {:?}", e);
                Err(Box::new(e))
            }
        }
    }

    async fn send(&mut self, buf: &[u8]) -> crate::Result<()> {
        self.tcp_stream.write_all(buf).await?;
        Ok(())
    }

    async fn shutdown(&mut self) -> crate::Result<()> {
        self.tcp_stream.shutdown().await?;
        Ok(())
    }
}
