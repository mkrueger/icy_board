#![allow(dead_code)]

use std::{
    io::{self, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    time::Duration,
};

use crate::{Connection, ConnectionType, NetError};

pub struct RawConnection {
    tcp_stream: TcpStream,
}

impl RawConnection {
    pub fn open<A: ToSocketAddrs>(addr: &A, timeout: Duration) -> crate::Result<Self> {
        for addr in addr.to_socket_addrs()? {
            let tcp_stream = TcpStream::connect_timeout(&addr, timeout)?;

            tcp_stream.set_write_timeout(Some(timeout))?;
            tcp_stream.set_read_timeout(Some(timeout))?;
            tcp_stream.set_nonblocking(false)?;

            return Ok(Self { tcp_stream });
        }
        Err(NetError::CouldNotConnect.into())
    }
}

impl Connection for RawConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Raw
    }

    fn shutdown(&mut self) -> crate::Result<()> {
        self.tcp_stream.shutdown(std::net::Shutdown::Both)?;
        Ok(())
    }
}

impl Read for RawConnection {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.tcp_stream.read(buf)
    }
}

impl Write for RawConnection {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tcp_stream.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.tcp_stream.flush()
    }
}