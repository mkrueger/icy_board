use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
};

use super::{Connection, ConnectionType};

pub struct ChannelConnection {
    rx: UnboundedReceiver<Vec<u8>>,
    tx: UnboundedSender<Vec<u8>>,
}

impl ChannelConnection {
    pub fn new(rx: UnboundedReceiver<Vec<u8>>, tx: UnboundedSender<Vec<u8>>) -> Self {
        Self { rx, tx }
    }

    pub fn create_pair() -> (ChannelConnection, ChannelConnection) {
        let (tx1, rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, rx2) = tokio::sync::mpsc::unbounded_channel();
        (ChannelConnection::new(rx1, tx2), ChannelConnection::new(rx2, tx1))
    }
}

impl Connection for ChannelConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Channel
    }
}

impl AsyncRead for ChannelConnection {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        let p = self.rx.poll_recv(cx);
        match p {
            Poll::Ready(Some(data)) => {
                buf.put_slice(&data);
                Poll::Ready(Ok(()))
            }
            Poll::Ready(None) => Poll::Ready(Ok(())),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl AsyncWrite for ChannelConnection {
    fn poll_write(self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let p = self.tx.send(buf.to_vec());
        if let Err(err) = p {
            return Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, err)));
        }
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn test_reader_writer_pair() {
        let (mut p1, mut p2) = ChannelConnection::create_pair();
        tokio::spawn(async move {
            p1.write_all("Hello World".as_bytes()).await.unwrap();
        })
        .await
        .unwrap();

        let mut buf = [0u8; 11];
        p2.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, "Hello World".as_bytes());
    }
}
