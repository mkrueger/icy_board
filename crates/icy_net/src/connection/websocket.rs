use http::Uri;
use rustls::{RootCertStore, ServerConnection, StreamOwned};
use std::fs::File;
use std::io::{self, Read, Write};
use std::io::{BufReader, ErrorKind};
use std::net::TcpStream;
use std::path::Path;
use std::sync::Arc;
use tungstenite::{client::IntoClientRequest, stream::MaybeTlsStream, Error, Message, WebSocket};

use crate::{Connection, ConnectionType};

pub struct WebSocketConnection<S: Read + Write> {
    is_secure: bool,
    socket: WebSocket<S>,
    data: Vec<u8>,
}

pub fn accept_websocket(stream: TcpStream) -> crate::Result<WebSocketConnection<TcpStream>> {
    let socket = tungstenite::accept(stream)?;
    Ok(WebSocketConnection {
        is_secure: false,
        socket,
        data: Vec::new(),
    })
}

pub fn accept_websocket_secure(
    stream: TcpStream,
    cert_file: &Path,
    private_key_file: &Path,
) -> crate::Result<WebSocketConnection<StreamOwned<ServerConnection, TcpStream>>> {
    if cert_file.exists() && private_key_file.exists() {
        let mut f1 = File::open(cert_file)?;
        let mut reader = BufReader::new(&mut f1);
        let mut f2 = File::open(private_key_file)?;
        let mut reader2 = BufReader::new(&mut f2);
        return accept_secure2(&mut reader, &mut reader2, stream);
    }
    Err("No cert or private key found".into())
}

fn accept_secure2(
    key_reader: &mut dyn io::BufRead,
    cert_reader: &mut dyn io::BufRead,
    stream: TcpStream,
) -> crate::Result<WebSocketConnection<StreamOwned<ServerConnection, TcpStream>>> {
    let certs = rustls_pemfile::certs(cert_reader).collect::<Result<Vec<_>, _>>()?;
    if let Some(private_key) = rustls_pemfile::private_key(key_reader)? {
        let config = rustls::ServerConfig::builder().with_no_client_auth().with_single_cert(certs, private_key)?;

        let tls_session = ServerConnection::new(Arc::new(config))?;
        let stream: StreamOwned<ServerConnection, TcpStream> = rustls::StreamOwned::new(tls_session, stream);
        let socket = tungstenite::accept_hdr(
            stream,
            |req: &tungstenite::handshake::server::Request, mut response: tungstenite::handshake::server::Response| {
                for (ref header, _value) in req.headers() {
                    log::warn!("* {}", header);
                }
                *response.status_mut() = http::StatusCode::OK;
                Ok(response)
            },
        )?;
        Ok(WebSocketConnection {
            is_secure: true,
            socket,
            data: Vec::new(),
        })
    } else {
        Err("No private key found".into())
    }
}

pub fn connect(address: &String, is_secure: bool) -> crate::Result<WebSocketConnection<MaybeTlsStream<TcpStream>>> {
    // build an ws:// or wss:// address
    //  :TODO: default port if not supplied in address
    let url = format!("{}://{}", schema_prefix(is_secure), address);

    let req = Uri::try_from(url)?.into_client_request()?;

    let mut root_store: RootCertStore = RootCertStore::empty();

    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = rustls::ClientConfig::builder().with_root_certificates(root_store).with_no_client_auth();

    // enable this line to test non-secure (ie: invalid certs) wss:// -- we could make this an option in the UI
    //config.dangerous().set_certificate_verifier(Arc::new(NoCertVerifier{}));

    let config = std::sync::Arc::new(config);

    let stream = TcpStream::connect(address)?;
    let connector: tungstenite::Connector = tungstenite::Connector::Rustls(config);
    let (mut socket, _) = tungstenite::client_tls_with_config(req, stream, None, Some(connector))?;

    let s = socket.get_mut();
    match s {
        MaybeTlsStream::Plain(s) => {
            s.set_nonblocking(true)?;
        }
        MaybeTlsStream::Rustls(tls) => {
            tls.sock.set_nonblocking(true)?;
        }
        _ => (),
    }

    Ok(WebSocketConnection {
        is_secure,
        socket,
        data: Vec::new(),
    })
}

fn schema_prefix(is_secure: bool) -> &'static str {
    if is_secure {
        "wss"
    } else {
        "ws"
    }
}

impl<S: Read + Write> Connection for WebSocketConnection<S> {
    fn get_connection_type(&self) -> ConnectionType {
        if self.is_secure {
            ConnectionType::SecureWebsocket
        } else {
            ConnectionType::Websocket
        }
    }

    fn shutdown(&mut self) -> crate::Result<()> {
        self.socket.close(None)?;
        Ok(())
    }
}

impl<S: Read + Write> Read for WebSocketConnection<S> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.data.len() > 0 {
            let len = buf.len().min(self.data.len());
            buf[..len].copy_from_slice(&self.data[..len]);
            self.data.drain(..len);
            return Ok(len);
        }
        match self.socket.read() {
            Ok(msg) => {
                let mut data = msg.into_data();
                let len = buf.len().min(data.len());
                buf[..len].copy_from_slice(&data[..len]);
                data.drain(..len);
                self.data = data;
                Ok(len)
            }
            Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(0),
            Err(e) => Err(std::io::Error::new(ErrorKind::ConnectionAborted, format!("Connection aborted: {e}"))),
        }
    }
}

impl<S: Read + Write> Write for WebSocketConnection<S> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let msg = Message::binary(buf);
        if let Err(err) = self.socket.send(msg) {
            // write + flush
            return Err(io::Error::new(ErrorKind::ConnectionAborted, format!("Connection aborted: {err}")));
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Err(err) = self.socket.flush() {
            return Err(io::Error::new(ErrorKind::ConnectionAborted, format!("Connection aborted: {err}")));
        }
        Ok(())
    }
}
