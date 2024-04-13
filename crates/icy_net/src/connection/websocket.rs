use http::Uri;
use rustls::RootCertStore;
use std::io::ErrorKind;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use tungstenite::{client::IntoClientRequest, stream::MaybeTlsStream, Error, Message, WebSocket};

use crate::{Connection, ConnectionType};
/*
struct NoCertVerifier {}

impl ServerCertVerifier for NoCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }
}*/
pub struct WebSocketConnection {
    is_secure: bool,
    socket: WebSocket<MaybeTlsStream<TcpStream>>,
}

impl WebSocketConnection {
    pub fn connect(address: &String, is_secure: bool) -> crate::Result<Self> {
        // build an ws:// or wss:// address
        //  :TODO: default port if not supplied in address
        let url = format!("{}://{}", Self::schema_prefix(is_secure), address);

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

        Ok(Self { is_secure, socket })
    }

    fn schema_prefix(is_secure: bool) -> &'static str {
        if is_secure {
            "wss"
        } else {
            "ws"
        }
    }
}

impl Connection for WebSocketConnection {
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

impl Read for WebSocketConnection {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.socket.read() {
            Ok(msg) => {
                let data = msg.into_data();
                let len = buf.len().min(data.len());
                buf[..len].copy_from_slice(&data[..len]);

                Ok(len)
            }
            Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(0),
            Err(e) => Err(std::io::Error::new(ErrorKind::ConnectionAborted, format!("Connection aborted: {e}"))),
        }
    }
}

impl Write for WebSocketConnection {
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
