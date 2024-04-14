use super::{Connection, ConnectionType};
use std::borrow::Borrow;
use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

pub struct ChannelConnection {
    buffer: VecDeque<u8>,
    rx: Receiver<Vec<u8>>,
    tx: Sender<Vec<u8>>,
}

impl ChannelConnection {
    pub fn new(rx: Receiver<Vec<u8>>, tx: Sender<Vec<u8>>) -> Self {
        Self {
            buffer: VecDeque::new(),
            rx,
            tx,
        }
    }

    pub fn create_pair() -> (ChannelConnection, ChannelConnection) {
        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        (ChannelConnection::new(rx1, tx2), ChannelConnection::new(rx2, tx1))
    }
}

impl Connection for ChannelConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Channel
    }
}

impl Read for ChannelConnection {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
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
            Err(err) => match err {
                mpsc::TryRecvError::Empty => Ok(0),
                mpsc::TryRecvError::Disconnected => Err(io::Error::new(io::ErrorKind::BrokenPipe, err)),
            },
        }
    }
}

impl Write for ChannelConnection {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Err(err) = self.tx.send(buf.to_vec()) {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, err));
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reader_writer_pair() {
        let (mut p1, mut p2) = ChannelConnection::create_pair();
        p1.write_all("Hello World".as_bytes()).unwrap();

        let mut buf = [0u8; 11];
        p2.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, "Hello World".as_bytes());
    }

    #[test]
    fn test_reader_writer_pair2() {
        let (mut p1, mut p2) = ChannelConnection::create_pair();
        p1.write_all("Hello".as_bytes()).unwrap();
        p1.write_all(" ".as_bytes()).unwrap();
        p1.write_all("World".as_bytes()).unwrap();

        let mut buf = [0u8; 11];
        p2.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, "Hello World".as_bytes());
    }

    #[test]
    fn read_chars() {
        let (mut p1, mut p2) = ChannelConnection::create_pair();
        p1.write_all("Hello World".as_bytes()).unwrap();

        let mut s = String::new();
        for _ in 0..11 {
            let mut buf = [0; 1];
            let len = p2.read(&mut buf).unwrap();
            s.push(buf[0] as char);
        }
        assert_eq!(&s, "Hello World");
    }
}
