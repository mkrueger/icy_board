use crate::{Connection, ConnectionState, ConnectionType};
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use http::Uri;
use std::io;
use std::io::ErrorKind;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
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
    if is_secure { "wss" } else { "ws" }
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
        // First return any buffered data
        if self.data.len() > 0 {
            let len = buf.len().min(self.data.len());
            buf[..len].copy_from_slice(&self.data[..len]);
            self.data = self.data.slice(len..);
            return Ok(len);
        }

        // Non-blocking check for new messages using poll
        use std::task::Poll;
        let waker = futures_util::task::noop_waker();
        let mut cx = std::task::Context::from_waker(&waker);

        match self.socket.poll_next_unpin(&mut cx) {
            Poll::Ready(Some(Ok(msg))) => {
                let data = msg.into_data();
                let len = buf.len().min(data.len());
                buf[..len].copy_from_slice(&data[..len]);
                self.data = data.slice(len..);
                Ok(len)
            }
            Poll::Ready(Some(Err(e))) => match e {
                tokio_tungstenite::tungstenite::Error::Io(e) => {
                    if e.kind() == ErrorKind::UnexpectedEof {
                        return Ok(0);
                    }
                    return Err(e.into());
                }
                _ => Err(std::io::Error::new(ErrorKind::ConnectionAborted, format!("Connection aborted: {e}")).into()),
            },
            Poll::Ready(None) => Ok(0), // Stream ended
            Poll::Pending => Ok(0),     // No data available (non-blocking)
        }
    }

    async fn poll(&mut self) -> crate::Result<ConnectionState> {
        use std::task::Poll;

        // First check if we have buffered data - if we do, connection is still active
        if !self.data.is_empty() {
            return Ok(ConnectionState::Connected);
        }

        // Try to poll the stream without consuming messages
        // We need to check if the stream is ready without actually reading
        let waker = futures_util::task::noop_waker();
        let mut cx = std::task::Context::from_waker(&waker);

        match self.socket.poll_next_unpin(&mut cx) {
            Poll::Ready(Some(Ok(msg))) => {
                match msg {
                    Message::Close(_) => {
                        log::debug!("WebSocket received close frame");
                        Ok(ConnectionState::Disconnected)
                    }
                    Message::Binary(data) => {
                        // We got data, store it in our buffer for the next read
                        self.data = Bytes::from(data);
                        Ok(ConnectionState::Connected)
                    }
                    Message::Text(_) => Ok(ConnectionState::Connected),
                    Message::Ping(_) | Message::Pong(_) => {
                        // Control frames indicate the connection is still alive
                        Ok(ConnectionState::Connected)
                    }
                    Message::Frame(_) => {
                        // Raw frame, connection is still active
                        Ok(ConnectionState::Connected)
                    }
                }
            }
            Poll::Ready(Some(Err(e))) => match e {
                tokio_tungstenite::tungstenite::Error::ConnectionClosed | tokio_tungstenite::tungstenite::Error::AlreadyClosed => {
                    log::debug!("WebSocket connection closed");
                    Ok(ConnectionState::Disconnected)
                }
                tokio_tungstenite::tungstenite::Error::Io(ref io_err) => match io_err.kind() {
                    ErrorKind::UnexpectedEof | ErrorKind::ConnectionAborted | ErrorKind::ConnectionReset | ErrorKind::BrokenPipe => {
                        log::debug!("WebSocket IO error indicates disconnection: {:?}", io_err);
                        Ok(ConnectionState::Disconnected)
                    }
                    _ => {
                        log::warn!("WebSocket poll error: {:?}", e);
                        Err(Box::new(e))
                    }
                },
                _ => {
                    log::warn!("WebSocket error during poll: {:?}", e);
                    Err(Box::new(e))
                }
            },
            Poll::Ready(None) => {
                // Stream has ended
                log::debug!("WebSocket stream ended");
                Ok(ConnectionState::Disconnected)
            }
            Poll::Pending => {
                // No data available right now, but connection is still open
                Ok(ConnectionState::Connected)
            }
        }
    }

    async fn send(&mut self, buf: &[u8]) -> crate::Result<()> {
        let msg = Message::binary(Bytes::copy_from_slice(buf));
        if let Err(err) = self.socket.send(msg).await {
            return Err(io::Error::new(ErrorKind::ConnectionAborted, format!("Connection aborted: {err}")).into());
        }
        // Explicit flush to ensure data is sent immediately
        if let Err(err) = self.socket.flush().await {
            return Err(io::Error::new(ErrorKind::ConnectionAborted, format!("Flush failed: {err}")).into());
        }
        Ok(())
    }

    async fn shutdown(&mut self) -> crate::Result<()> {
        self.socket.close(None).await?;
        Ok(())
    }
}
