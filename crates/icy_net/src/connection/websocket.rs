use crate::{Connection, ConnectionType};
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use http::Uri;
use std::io;
use std::io::ErrorKind;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

pub struct WebSocketConnection<S: AsyncRead + AsyncWrite + Unpin> {
    is_secure: bool,
    socket: WebSocketStream<S>,
    data: Bytes,
}

pub fn init_websocket_providers() {
    rustls::crypto::aws_lc_rs::default_provider().install_default().unwrap();
}

pub async fn accept_sec_websocket(stream: TcpStream) -> crate::Result<WebSocketConnection<MaybeTlsStream<TcpStream>>> {
    let url = format!("{}://{}", schema_prefix(true), "localhost");
    let request = Uri::try_from(url)?.into_client_request()?;
    let (socket, _) = tokio_tungstenite::client_async_tls(request, stream).await?;
    Ok(WebSocketConnection {
        is_secure: true,
        socket,
        data: Bytes::new(),
    })
}

pub async fn accept_websocket(stream: TcpStream) -> crate::Result<WebSocketConnection<TcpStream>> {
    let socket = tokio_tungstenite::accept_async(stream).await?;
    Ok(WebSocketConnection {
        is_secure: false,
        socket,
        data: Bytes::new(),
    })
}

pub async fn connect(address: &String, is_secure: bool) -> crate::Result<WebSocketConnection<MaybeTlsStream<TcpStream>>> {
    // build an ws:// or wss:// address
    //  :TODO: default port if not supplied in address
    let url = format!("{}://{}", schema_prefix(is_secure), address);
    let request = Uri::try_from(url)?.into_client_request()?;
    let (socket, _) = tokio_tungstenite::connect_async(request).await?;
    Ok(WebSocketConnection {
        is_secure,
        socket,
        data: Bytes::new(),
    })
}

fn schema_prefix(is_secure: bool) -> &'static str {
    if is_secure {
        "wss"
    } else {
        "ws"
    }
}

#[async_trait]
impl<S: AsyncRead + AsyncWrite + Unpin + Send> Connection for WebSocketConnection<S> {
    fn get_connection_type(&self) -> ConnectionType {
        if self.is_secure {
            ConnectionType::SecureWebsocket
        } else {
            ConnectionType::Websocket
        }
    }

    async fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        if self.data.len() > 0 {
            let len = buf.len().min(self.data.len());
            buf[..len].copy_from_slice(&self.data[..len]);
            self.data = self.data.slice(len..);
            return Ok(len);
        }
        match self.socket.next().await {
            Some(Ok(msg)) => {
                let data = msg.into_data();
                let len = buf.len().min(data.len());
                buf[..len].copy_from_slice(&data[..len]);
                self.data = data.slice(len..);
                Ok(len)
            }
            Some(Err(e)) => match e {
                tokio_tungstenite::tungstenite::Error::Io(e) => {
                    if e.kind() == ErrorKind::UnexpectedEof {
                        return Ok(0);
                    }
                    return Err(e.into());
                }
                _ => Err(std::io::Error::new(ErrorKind::ConnectionAborted, format!("Connection aborted: {e}")).into()),
            },
            None => Err(std::io::Error::new(ErrorKind::ConnectionAborted, "Connection aborted").into()),
        }
    }

    async fn try_read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        if self.data.len() > 0 {
            let len = buf.len().min(self.data.len());
            buf[..len].copy_from_slice(&self.data[..len]);
            self.data = self.data.slice(len..);
            return Ok(len);
        }
        match self.socket.next().await {
            Some(Ok(msg)) => {
                let data = msg.into_data();
                let len = buf.len().min(data.len());
                buf[..len].copy_from_slice(&data[..len]);
                self.data = data.slice(len..);
                Ok(len)
            }
            Some(Err(e)) => match e {
                tokio_tungstenite::tungstenite::Error::Io(e) => {
                    if e.kind() == ErrorKind::UnexpectedEof {
                        return Ok(0);
                    }
                    return Err(e.into());
                }
                _ => Err(std::io::Error::new(ErrorKind::ConnectionAborted, format!("Connection aborted: {e}")).into()),
            },
            None => Err(std::io::Error::new(ErrorKind::ConnectionAborted, "Connection aborted").into()),
        }
    }

    async fn send(&mut self, buf: &[u8]) -> crate::Result<()> {
        let msg = Message::binary(Bytes::copy_from_slice(buf));
        if let Err(err) = self.socket.send(msg).await {
            // write + flush
            return Err(io::Error::new(ErrorKind::ConnectionAborted, format!("Connection aborted: {err}")).into());
        }
        Ok(())
    }

    async fn shutdown(&mut self) -> crate::Result<()> {
        self.socket.close(None).await?;
        Ok(())
    }
}
