use std::{borrow::Cow, io::ErrorKind, sync::Arc, time::Duration};

use crate::Res;
use async_trait::async_trait;
use icy_board_engine::icy_board::{IcyBoard, bbs::BBS, login_server::SSH};
use icy_net::{Connection, ConnectionType};
use rand::rngs::StdRng;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::Mutex,
    time::timeout,
};

use russh::{
    Channel, ChannelStream, Preferred, cipher, kex,
    keys::{Certificate, PublicKey},
    server::{self, ChannelOpenHandle, Msg, Session},
};

use super::handle_client;
use tokio_util::sync::CancellationToken;

pub async fn await_ssh_connections(ssh: SSH, board: Arc<tokio::sync::Mutex<IcyBoard>>, bbs: Arc<Mutex<BBS>>, generation: CancellationToken) -> Res<()> {
    PreparedSsh::bind(ssh).await?.run(board, bbs, generation).await
}

/// An SSH listener and server configuration prepared without starting transports.
pub struct PreparedSsh {
    listener: TcpListener,
    config: Arc<russh::server::Config>,
}

impl PreparedSsh {
    /// Actual bound address, including port-zero allocation or fallback binding.
    pub fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        self.listener.local_addr()
    }

    pub async fn bind(ssh: SSH) -> Res<Self> {
        let mut rng: StdRng = rand::make_rng();
        let config = russh::server::Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
            auth_rejection_time: std::time::Duration::from_secs(3),
            auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
            keys: vec![russh::keys::PrivateKey::random(&mut rng, russh::keys::Algorithm::Ed25519)?],
            preferred: Preferred {
                kex: Cow::Owned(kex::ALL_KEX_ALGORITHMS.iter().map(|k| **k).collect()),
                cipher: Cow::Owned(cipher::ALL_CIPHERS.iter().map(|k| **k).collect()),
                ..Preferred::default()
            },
            ..Default::default()
        };
        let config = Arc::new(config);
        let configured_addr = if ssh.address.trim().is_empty() {
            "0.0.0.0".to_string()
        } else {
            ssh.address.clone()
        };

        let listener = match TcpListener::bind((configured_addr.as_str(), ssh.port)).await {
            Ok(listener) => listener,
            Err(e) => {
                log::error!("SSH bind failed on {}:{} -> {e}; kind={:?}", configured_addr, ssh.port, e);
                // Only attempt fallback if user supplied a non-wildcard that failed
                if configured_addr != "0.0.0.0" && e.kind() == std::io::ErrorKind::AddrNotAvailable {
                    let fallback = "0.0.0.0";
                    log::warn!("Retrying SSH listener on fallback {}:{}", fallback, ssh.port);
                    TcpListener::bind((fallback, ssh.port)).await?
                } else {
                    return Err(e.into());
                }
            }
        };
        log::info!("SSH listening on {}", listener.local_addr()?);
        Ok(Self { listener, config })
    }

    pub async fn run(self, board: Arc<tokio::sync::Mutex<IcyBoard>>, bbs: Arc<Mutex<BBS>>, generation: CancellationToken) -> Res<()> {
        let Self { listener, config } = self;
        // Own transport tasks as well as the accept loop. russh's convenience server
        // spawns detached transports; dropping it is not a completed shutdown.
        let mut transports = tokio::task::JoinSet::new();
        let result = loop {
            tokio::select! {
                _ = generation.cancelled() => break Ok(()),
                result = transports.join_next(), if !transports.is_empty() => {
                    if let Some(Err(err)) = result { log::error!("SSH transport failed: {err}"); }
                }
                accepted = listener.accept() => {
                    let (stream, _) = match accepted {
                        Ok(accepted) => accepted,
                        Err(err) => break Err(err.into()),
                    };
                    let handler = SshSession { board: board.clone(), bbs: bbs.clone(), generation: generation.clone() };
                    let config = config.clone();
                    let cancel = generation.clone();
                    transports.spawn(async move {
                        let mut session = tokio::select! {
                            _ = cancel.cancelled() => return,
                            result = server::run_stream(config, stream, handler) => match result {
                                Ok(session) => session,
                                Err(err) => { log::debug!("SSH setup failed: {err}"); return; }
                            }
                        };
                        tokio::select! {
                            result = &mut session => { if let Err(err) = result { log::debug!("SSH session ended: {err}"); } },
                            _ = cancel.cancelled() => {
                                let _ = session.handle().disconnect(russh::Disconnect::ByApplication, "Board maintenance".into(), String::new()).await;
                                let _ = session.await;
                            }
                        }
                    });
                }
            }
        };
        drop(listener);
        generation.cancel();
        while let Some(result) = transports.join_next().await {
            if let Err(err) = result {
                log::error!("SSH transport shutdown failed: {err}");
            }
        }
        result
    }
}

struct SshSession {
    board: Arc<tokio::sync::Mutex<IcyBoard>>,
    bbs: Arc<Mutex<BBS>>,
    generation: CancellationToken,
}

impl server::Handler for SshSession {
    type Error = russh::Error;

    async fn channel_open_session(&mut self, channel: Channel<Msg>, reply: ChannelOpenHandle, session: &mut Session) -> Result<(), Self::Error> {
        let bbs2 = self.bbs.clone();
        let mut admission = self.bbs.lock().await;
        // russh can retain authenticated transports after its accept loop exits.
        // An old generation must never open a new BBS node after event restart.
        if self.generation.is_cancelled() {
            return Ok(());
        }
        let node_list = admission.open_connections.clone();
        let board = self.board.clone();

        let channel_id = channel.id();
        let session_handle = session.handle();
        let connection = SSHConnection::new(channel, channel_id, session_handle);

        let node = admission
            .spawn_node(ConnectionType::SSH, move |node, _| {
                std::thread::Builder::new().name("SSH handle".to_string()).spawn(move || {
                    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                        if let Err(err) = handle_client(bbs2, board, node_list, node, Box::new(connection), None, "").await {
                            log::error!("Error running background client: {}", err);
                        }
                        log::info!("SSH session for node {} ended.", node);
                    });
                    Ok(())
                })
            })
            .await?;
        drop(admission);
        if node.is_none() {
            // Dropping the unanswered reply rejects the channel; no BBS thread exists.
            return Ok(());
        }

        reply.accept().await;
        Ok(())
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

    async fn tcpip_forward(&mut self, _address: &str, _port: &mut u32, _session: &mut Session) -> Result<bool, Self::Error> {
        // This BBS is not a forwarding service. Do not spawn an untracked dummy
        // forwarding task which can outlive a listener generation.
        Ok(false)
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
        self.channel.write_all(buf).await?;
        Ok(())
    }

    async fn shutdown(&mut self) -> icy_net::Result<()> {
        self.channel.shutdown().await?;
        self.do_close().await;
        Ok(())
    }
}
