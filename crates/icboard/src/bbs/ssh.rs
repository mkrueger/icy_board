use std::{borrow::Cow, io::ErrorKind, sync::Arc, time::Duration};

use crate::Res;
use async_trait::async_trait;
use icy_board_engine::icy_board::{IcyBoard, bbs::BBS, login_server::SSH};
use icy_net::{Connection, ConnectionType};
use internal_russh_forked_ssh_key::{Certificate, PublicKey, rand_core::OsRng};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
    time::timeout,
};

use russh::{
    Channel, ChannelStream, Preferred, cipher, kex,
    server::{self, Msg, Session},
};

use super::handle_client;

pub async fn await_ssh_connections(ssh: SSH, board: Arc<tokio::sync::Mutex<IcyBoard>>, bbs: Arc<Mutex<BBS>>) -> Res<()> {
    let config = russh::server::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
        auth_rejection_time: std::time::Duration::from_secs(3),
        auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
        keys: vec![russh::keys::PrivateKey::random(&mut OsRng, russh::keys::Algorithm::Ed25519).unwrap()],
        preferred: Preferred {
            kex: Cow::Owned(kex::ALL_KEX_ALGORITHMS.iter().map(|k| **k).collect()),
            cipher: Cow::Owned(cipher::ALL_CIPHERS.iter().map(|k| **k).collect()),
            ..Preferred::default()
        },
        ..Default::default()
    };
    let config = Arc::new(config);
    let mut server_impl = Server { board, bbs };
    let configured_addr = if ssh.address.trim().is_empty() {
        "0.0.0.0".to_string()
    } else {
        ssh.address.clone()
    };

    match russh::server::Server::run_on_address(&mut server_impl, config.clone(), (configured_addr.as_str(), ssh.port)).await {
        Ok(_) => {
            log::info!("SSH listening on {}:{}", configured_addr, ssh.port);
            return Ok(());
        }
        Err(e) => {
            log::error!("SSH bind failed on {}:{} -> {e}; kind={:?}", configured_addr, ssh.port, e);
            // Only attempt fallback if user supplied a non-wildcard that failed
            if configured_addr != "0.0.0.0" && e.kind() == std::io::ErrorKind::AddrNotAvailable {
                let fallback = "0.0.0.0";
                log::warn!("Retrying SSH listener on fallback {}:{}", fallback, ssh.port);
                if let Err(e2) = russh::server::Server::run_on_address(&mut server_impl, config, (fallback, ssh.port)).await {
                    log::error!("SSH fallback bind also failed on {}:{} -> {e2}", fallback, ssh.port);
                    return Err(e2.into());
                } else {
                    log::info!("SSH listening on fallback {}:{}", fallback, ssh.port);
                    return Ok(());
                }
            } else {
                return Err(e.into());
            }
        }
    }
}

struct SshSession {
    board: Arc<tokio::sync::Mutex<IcyBoard>>,
    bbs: Arc<Mutex<BBS>>,
}

#[derive(Clone)]
struct Server {
    board: Arc<tokio::sync::Mutex<IcyBoard>>,
    bbs: Arc<Mutex<BBS>>,
}

impl server::Server for Server {
    type Handler = SshSession;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> SshSession {
        SshSession {
            board: self.board.clone(),
            bbs: self.bbs.clone(),
        }
    }

    fn handle_session_error(&mut self, _error: <Self::Handler as russh::server::Handler>::Error) {
        log::error!("SSH Session error: {:#?}", _error);
    }
}

impl server::Handler for SshSession {
    type Error = russh::Error;

    async fn channel_open_session(&mut self, channel: Channel<Msg>, session: &mut Session) -> Result<bool, Self::Error> {
        let bbs2 = self.bbs.clone();
        let node = self.bbs.lock().await.create_new_node(ConnectionType::SSH).await;
        let node_list = self.bbs.lock().await.get_open_connections().await.clone();
        let board = self.board.clone();

        let channel_id = channel.id();
        let session_handle = session.handle();
        let connection = SSHConnection::new(channel, channel_id, session_handle);

        let handle = std::thread::Builder::new()
            .name("SSH handle".to_string())
            .spawn(move || {
                tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                    if let Err(err) = handle_client(bbs2, board, node_list, node, Box::new(connection), None, "").await {
                        log::error!("Error running background client: {}", err);
                    }
                    log::info!("SSH session for node {} ended.", node);
                });
                Ok(())
            })
            .unwrap();

        self.bbs.lock().await.get_open_connections().await.lock().await[node].as_mut().unwrap().handle = Some(handle);

        Ok(true)
    }

    async fn auth_password(&mut self, _user: &str, _password: &str) -> Result<server::Auth, Self::Error> {
        Ok(server::Auth::Accept)
    }

    async fn auth_publickey(&mut self, _: &str, _key: &PublicKey) -> Result<server::Auth, Self::Error> {
        Ok(server::Auth::Accept)
    }

    async fn auth_openssh_certificate(&mut self, _user: &str, _certificate: &Certificate) -> Result<server::Auth, Self::Error> {
        Ok(server::Auth::Accept)
    }

    async fn tcpip_forward(&mut self, address: &str, port: &mut u32, session: &mut Session) -> Result<bool, Self::Error> {
        let handle = session.handle();
        let address = address.to_string();
        let port = *port;
        tokio::spawn(async move {
            let channel = handle.channel_open_forwarded_tcpip(address, port, "1.2.3.4", 1234).await.unwrap();
            let _ = channel.data(&b"Hello from a forwarded port"[..]).await;
            let _ = channel.eof().await;
        });
        Ok(true)
    }
}

pub struct SSHConnection {
    channel: ChannelStream<Msg>,
    channel_id: russh::ChannelId,
    handle: russh::server::Handle,
    closed: bool,
}

unsafe impl Send for SSHConnection {}
unsafe impl Sync for SSHConnection {}

impl SSHConnection {
    pub fn new(channel: Channel<Msg>, channel_id: russh::ChannelId, handle: russh::server::Handle) -> Self {
        Self {
            channel: channel.into_stream(),
            channel_id,
            handle,
            closed: false,
        }
    }

    async fn do_close(&mut self) {
        if self.closed {
            return;
        }

        // Explicit channel close
        if let Err(e) = self.handle.close(self.channel_id).await {
            log::debug!("SSH channel close failed: {e:?}");
        }

        self.closed = true;
    }
}

#[async_trait]
impl Connection for SSHConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::SSH
    }

    async fn read(&mut self, buf: &mut [u8]) -> icy_net::Result<usize> {
        match self.channel.read(buf).await {
            Ok(size) => Ok(size),
            Err(e) => match e.kind() {
                ErrorKind::ConnectionAborted | ErrorKind::NotConnected => {
                    log::error!("telnet error - connection aborted.");
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

    async fn try_read(&mut self, buf: &mut [u8]) -> icy_net::Result<usize> {
        // Non-blocking attempt: immediate timeout -> treat Pending as no data (return 0)
        match timeout(Duration::from_millis(0), self.channel.read(buf)).await {
            // Future completed within the timeout
            Ok(Ok(size)) => Ok(size),
            Ok(Err(e)) => match e.kind() {
                ErrorKind::ConnectionAborted | ErrorKind::NotConnected => {
                    log::error!("ssh try_read - connection aborted.");
                    Err(std::io::Error::new(ErrorKind::ConnectionAborted, format!("Connection aborted: {e}")).into())
                }
                ErrorKind::WouldBlock => Ok(0),
                _ => {
                    log::error!("ssh try_read error {:?}: {:?}", e.kind(), e);
                    Ok(0)
                }
            },
            // Timed out: underlying read not ready yet
            Err(_elapsed) => Ok(0),
        }
    }

    async fn send(&mut self, buf: &[u8]) -> icy_net::Result<()> {
        self.channel.write(buf).await?;
        Ok(())
    }

    async fn shutdown(&mut self) -> icy_net::Result<()> {
        self.channel.shutdown().await?;
        self.do_close().await;
        Ok(())
    }
}
