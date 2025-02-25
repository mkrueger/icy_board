use std::{borrow::Cow, io::ErrorKind, sync::Arc};

use crate::Res;
use async_trait::async_trait;
use icy_board_engine::icy_board::{IcyBoard, bbs::BBS, login_server::SSH};
use icy_net::{Connection, ConnectionType};
use russh_keys::{Certificate, ssh_key};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
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
        keys: vec![russh::keys::PrivateKey::random(&mut ssh_key::rand_core::OsRng, russh::keys::Algorithm::Ed25519).unwrap()],
        preferred: Preferred {
            kex: Cow::Owned(kex::ALL_KEX_ALGORITHMS.iter().map(|k| **k).collect()),
            cipher: Cow::Owned(cipher::ALL_CIPHERS.iter().map(|k| **k).collect()),
            ..Preferred::default()
        },
        ..Default::default()
    };
    let config = Arc::new(config);
    let mut sh = Server { board, bbs };
    let addr = if ssh.address.is_empty() { "0.0.0.0".to_string() } else { ssh.address.clone() };

    server::Server::run_on_address(&mut sh, config, (addr, ssh.port)).await.unwrap();
    Ok(())
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

    async fn channel_open_session(&mut self, channel: Channel<Msg>, _session: &mut Session) -> Result<bool, Self::Error> {
        let bbs2 = self.bbs.clone();
        let node = self.bbs.lock().await.create_new_node(ConnectionType::Telnet).await;
        let node_list = self.bbs.lock().await.get_open_connections().await.clone();
        let board = self.board.clone();
        let connection = SSHConnection::new(channel);

        std::thread::Builder::new()
            .name("SSH handle".to_string())
            .spawn(move || {
                tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                    if let Err(err) = handle_client(bbs2, board, node_list, node, Box::new(connection), None, "").await {
                        log::error!("Error running backround client: {}", err);
                    }
                });
            })
            .unwrap();
        Ok(true)
    }

    async fn auth_password(&mut self, _user: &str, _password: &str) -> Result<server::Auth, Self::Error> {
        Ok(server::Auth::Accept)
    }

    async fn auth_publickey(&mut self, _: &str, _key: &ssh_key::PublicKey) -> Result<server::Auth, Self::Error> {
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

impl Drop for Server {
    fn drop(&mut self) {
        /*let id = self.id;
        let clients = self.clients.clone();
        tokio::spawn(async move {
            let mut clients = clients.lock().await;
            clients.remove(&id);
        });*/
    }
}

pub struct SSHConnection {
    channel: ChannelStream<Msg>,
}
unsafe impl Send for SSHConnection {}
unsafe impl Sync for SSHConnection {}

impl SSHConnection {
    pub fn new(channel: Channel<Msg>) -> Self {
        let channel = channel.into_stream();
        Self { channel }
    }
}

#[async_trait]
impl Connection for SSHConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Channel
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

    async fn send(&mut self, buf: &[u8]) -> icy_net::Result<()> {
        self.channel.write(buf).await?;
        Ok(())
    }

    async fn shutdown(&mut self) -> icy_net::Result<()> {
        Ok(())
    }
}
