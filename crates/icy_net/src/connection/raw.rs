#![allow(dead_code)]

use std::{io::ErrorKind, time::Duration};

use async_trait::async_trait;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, ToSocketAddrs},
};

use super::{Connection, ConnectionType};

pub struct RawConnection {
    tcp_stream: TcpStream,
}

impl RawConnection {
    pub async fn open<A: ToSocketAddrs>(addr: &A, timeout: Duration) -> crate::Result<Self> {
        let result = tokio::time::timeout(timeout, TcpStream::connect(addr)).await;
        match result {
            Ok(tcp_stream) => match tcp_stream {
                Ok(stream) => Ok(Self { tcp_stream: stream }),
                Err(err) => Err(Box::new(err)),
            },
            Err(err) => Err(Box::new(err)),
        }
    }

    pub async fn accept(tcp_stream: TcpStream) -> crate::Result<Self> {
        Ok(Self { tcp_stream })
    }
}

#[async_trait]
impl Connection for RawConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Raw
    }

    async fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        let result = self.tcp_stream.read(buf).await?;
        /*     if result == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "").into());
        }*/
        Ok(result)
    }

    async fn try_read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        match self.tcp_stream.try_read(buf) {
            Ok(size) => Ok(size),
            Err(e) => match e.kind() {
                ErrorKind::ConnectionAborted | ErrorKind::NotConnected => {
                    log::error!("telnet error reading from TCP stream: {}", e);
                    return Err(std::io::Error::new(ErrorKind::ConnectionAborted, format!("Connection aborted: {e}")).into());
                }
                ErrorKind::WouldBlock => Ok(0),
                _ => {
                    log::error!("Error {:?} reading from SSH connection: {:?}", e.kind(), e);
                    Ok(0)
                }
            },
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
