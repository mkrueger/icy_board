//! SSH client connections with explicit password, private-key, agent, and automatic
//! authentication. Client credentials and server host-key trust are independent:
//! [`SSHConnection::open`] preserves the historical accept-any host behavior, while
//! [`SSHConnection::open_with_options`] requires an explicit [`HostKeyPolicy`].

#![allow(dead_code)]
use async_trait::async_trait;
use russh::keys::{PrivateKey, ssh_key};
use russh::{client::Msg, *};
use std::{
    borrow::Cow,
    collections::HashMap,
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use thiserror::Error;
use zeroize::{Zeroize, Zeroizing};

use crate::ConnectionState;
use crate::connection::proxy::{ProxyConfig, connect_tcp};
use crate::{Connection, ConnectionType, telnet::TermCaps};
use tokio::{io::AsyncWriteExt, sync::Mutex};

pub struct SSHConnection {
    client: SshClient,
    channel: Channel<Msg>,
    read_buffer: Vec<u8>, // Add internal buffer for non-blocking reads
}

#[derive(Debug)]
pub struct Credentials {
    pub user_name: String,
    pub authentication: SshAuthentication,
    pub proxy_command: Option<String>,
}

impl Credentials {
    pub fn password(user_name: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            user_name: user_name.into(),
            authentication: SshAuthentication::Password {
                password: SecretString::new(password),
            },
            proxy_command: None,
        }
    }

    pub fn private_key(user_name: impl Into<String>, path: impl Into<PathBuf>, passphrase: Option<SecretString>) -> Self {
        Self {
            user_name: user_name.into(),
            authentication: SshAuthentication::PrivateKey { path: path.into(), passphrase },
            proxy_command: None,
        }
    }

    pub fn agent(user_name: impl Into<String>) -> Self {
        Self {
            user_name: user_name.into(),
            authentication: SshAuthentication::Agent { public_key: None },
            proxy_command: None,
        }
    }
}

#[derive(Clone, Default, Eq, PartialEq)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretString([REDACTED])")
    }
}

impl Drop for SecretString {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[derive(Debug)]
pub enum SshAuthentication {
    Password {
        password: SecretString,
    },
    PrivateKey {
        path: PathBuf,
        passphrase: Option<SecretString>,
    },
    Agent {
        public_key: Option<SshPublicKeySelector>,
    },
    Auto {
        /// Tried in order, before the agent and password. A local key-loading error
        /// stops authentication; only a server rejection advances to the next entry.
        private_keys: Vec<PrivateKeyCredential>,
        /// When true, agent identities follow explicit private-key files.
        use_agent: bool,
        /// When present, password authentication is the final fallback.
        password: Option<SecretString>,
    },
}

impl SshAuthentication {
    pub fn configured_methods(&self) -> Vec<SshAuthenticationMethod> {
        match self {
            Self::Password { .. } => vec![SshAuthenticationMethod::Password],
            Self::PrivateKey { .. } => vec![SshAuthenticationMethod::PublicKey],
            Self::Agent { .. } => vec![SshAuthenticationMethod::Agent],
            Self::Auto {
                private_keys,
                use_agent,
                password,
            } => {
                let mut methods = vec![SshAuthenticationMethod::PublicKey; private_keys.len()];
                if *use_agent {
                    methods.push(SshAuthenticationMethod::Agent);
                }
                if password.is_some() {
                    methods.push(SshAuthenticationMethod::Password);
                }
                methods
            }
        }
    }
}

#[derive(Debug)]
pub struct PrivateKeyCredential {
    pub path: PathBuf,
    pub passphrase: Option<SecretString>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SshPublicKeySelector {
    Fingerprint(String),
    PublicKeyFile(PathBuf),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SshHostKeyFingerprint(pub String);

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum HostKeyPolicy {
    KnownHosts { path: PathBuf, accept_new: bool },
    Fingerprint(SshHostKeyFingerprint),
    InsecureAcceptAny,
}

#[derive(Debug)]
pub struct SshConnectionOptions {
    pub credentials: Credentials,
    pub host_key_policy: HostKeyPolicy,
    pub connect_timeout: Duration,
    pub authentication_timeout: Duration,
    /// Optional proxy for the underlying TCP connection (e.g. SOCKS5 for Tor/I2P).
    pub proxy: Option<ProxyConfig>,
}

impl SshConnectionOptions {
    /// Compatibility settings used by [`SSHConnection::open`]. New callers should
    /// construct this type directly and choose a non-permissive host-key policy.
    pub fn insecure_compatibility(credentials: Credentials) -> Self {
        Self {
            credentials,
            host_key_policy: HostKeyPolicy::InsecureAcceptAny,
            connect_timeout: Duration::from_secs(5),
            authentication_timeout: Duration::from_secs(30),
            proxy: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SshAuthenticationMethod {
    Password,
    PublicKey,
    Agent,
}

#[derive(Debug, Error)]
pub enum SshAuthenticationError {
    #[error("SSH key file was not found: {path}", path = .path.display())]
    KeyFileNotFound { path: PathBuf },
    #[error("SSH key file cannot be read: {path}", path = .path.display())]
    KeyFileUnreadable {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("unsupported SSH private-key format: {path}", path = .path.display())]
    UnsupportedKeyFormat { path: PathBuf },
    #[error("SSH private key requires a passphrase: {path}", path = .path.display())]
    PassphraseRequired { path: PathBuf },
    #[error("SSH private-key passphrase is invalid: {path}", path = .path.display())]
    InvalidPassphrase { path: PathBuf },
    #[error("SSH agent is unavailable")]
    AgentUnavailable,
    #[error("SSH agent has no identities")]
    AgentHasNoIdentities,
    #[error("selected SSH agent key was not found")]
    SelectedAgentKeyNotFound,
    #[error("SSH authentication was rejected")]
    AuthenticationRejected {
        attempted: Vec<SshAuthenticationMethod>,
        server_methods: Vec<SshAuthenticationMethod>,
    },
    #[error("SSH server host key is unknown")]
    HostKeyUnknown,
    #[error("SSH server host key does not match")]
    HostKeyMismatch,
    #[error("SSH transport failed: {0}")]
    Transport(#[source] Box<dyn std::error::Error + Send + Sync>),
}

async fn load_private_key(credential: &PrivateKeyCredential) -> Result<Arc<PrivateKey>, SshAuthenticationError> {
    let path = &credential.path;
    let contents = tokio::fs::read_to_string(path).await.map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            SshAuthenticationError::KeyFileNotFound { path: path.clone() }
        } else {
            SshAuthenticationError::KeyFileUnreadable { path: path.clone(), source }
        }
    })?;

    let contents = Zeroizing::new(contents);
    let path = path.clone();
    let passphrase = credential.passphrase.clone();
    tokio::task::spawn_blocking(move || decode_loaded_private_key(&path, &contents, passphrase.as_ref()))
        .await
        .map_err(transport)?
}

fn decode_loaded_private_key(path: &Path, contents: &str, passphrase: Option<&SecretString>) -> Result<Arc<PrivateKey>, SshAuthenticationError> {
    match russh::keys::decode_secret_key(contents, None) {
        Ok(key) => Ok(Arc::new(key)),
        Err(russh::keys::Error::KeyIsEncrypted) => {
            let Some(passphrase) = passphrase else {
                return Err(SshAuthenticationError::PassphraseRequired { path: path.to_path_buf() });
            };
            russh::keys::decode_secret_key(contents, Some(passphrase.expose_secret()))
                .map(Arc::new)
                .map_err(|_| SshAuthenticationError::InvalidPassphrase { path: path.to_path_buf() })
        }
        Err(_) => Err(SshAuthenticationError::UnsupportedKeyFormat { path: path.to_path_buf() }),
    }
}

impl SSHConnection {
    pub async fn open(addr: impl Into<String>, caps: TermCaps, credentials: Credentials) -> crate::Result<Self> {
        Self::open_with_options(addr, caps, SshConnectionOptions::insecure_compatibility(credentials)).await
    }

    pub async fn open_with_options(addr: impl Into<String>, caps: TermCaps, options: SshConnectionOptions) -> crate::Result<Self> {
        let ssh = SshClient::connect(addr, options).await?;
        let channel = ssh.session.channel_open_session().await?;
        let terminal_type: String = format!("{:?}", caps.terminal).to_lowercase();
        channel
            .request_pty(false, &terminal_type, caps.window_size.0 as u32, caps.window_size.1 as u32, 1, 1, &[])
            .await?;
        channel.request_shell(false).await?;
        Ok(Self {
            client: ssh,
            channel,
            read_buffer: Vec::new(), // Initialize empty buffer
        })
    }

    fn default_port() -> u16 {
        22
    }

    // Helper method to fill buffer from channel messages without blocking
    async fn fill_buffer_nonblocking(&mut self) -> crate::Result<()> {
        // Use a very short timeout to make this non-blocking
        let timeout = Duration::from_millis(1);

        loop {
            match tokio::time::timeout(timeout, self.channel.wait()).await {
                Ok(Some(msg)) => {
                    match msg {
                        ChannelMsg::Data { data } => {
                            // Add data to our buffer
                            self.read_buffer.extend_from_slice(&data);
                        }
                        ChannelMsg::Eof => {
                            // Channel received EOF, connection is ending
                            return Ok(());
                        }
                        ChannelMsg::Close => {
                            // Channel is closing
                            return Ok(());
                        }
                        _ => {
                            // Other messages, continue
                        }
                    }
                }
                Ok(None) => {
                    // Channel closed
                    return Ok(());
                }
                Err(_) => {
                    // Timeout - no more messages available right now
                    return Ok(());
                }
            }
        }
    }
}

#[async_trait]
impl Connection for SSHConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::SSH
    }

    async fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        // First check if we have buffered data
        if !self.read_buffer.is_empty() {
            let to_read = buf.len().min(self.read_buffer.len());
            buf[..to_read].copy_from_slice(&self.read_buffer[..to_read]);
            self.read_buffer.drain(..to_read);
            return Ok(to_read);
        }

        // No buffered data, wait for new data from the channel
        loop {
            let Some(msg) = self.channel.wait().await else {
                // Channel closed
                return Ok(0);
            };

            match msg {
                ChannelMsg::Data { data } => {
                    // We got data, copy what we can to the buffer
                    let to_read = buf.len().min(data.len());
                    buf[..to_read].copy_from_slice(&data[..to_read]);

                    // If there's leftover data, store it in our buffer
                    if data.len() > to_read {
                        self.read_buffer.extend_from_slice(&data[to_read..]);
                    }

                    return Ok(to_read);
                }
                ChannelMsg::Eof | ChannelMsg::Close => {
                    // Connection is closing
                    return Ok(0);
                }
                _ => {
                    // Other messages, continue waiting
                    continue;
                }
            }
        }
    }

    async fn poll(&mut self) -> crate::Result<ConnectionState> {
        // Check if the session is closed
        if self.client.session.is_closed() {
            return Ok(ConnectionState::Disconnected);
        }

        // Try to fill buffer without blocking
        self.fill_buffer_nonblocking().await?;

        // Use timeout to check if channel is still responsive
        let timeout = Duration::from_millis(1);
        match tokio::time::timeout(timeout, self.channel.wait()).await {
            Ok(Some(msg)) => {
                match msg {
                    ChannelMsg::Data { data } => {
                        // We got data during poll, buffer it
                        self.read_buffer.extend_from_slice(&data);
                        Ok(ConnectionState::Connected)
                    }
                    ChannelMsg::Eof | ChannelMsg::Close => {
                        log::debug!("SSH channel received EOF/Close");
                        Ok(ConnectionState::Disconnected)
                    }
                    _ => Ok(ConnectionState::Connected),
                }
            }
            Ok(None) => {
                // Channel is closed
                Ok(ConnectionState::Disconnected)
            }
            Err(_) => {
                // Timeout - no messages pending, connection is still active
                Ok(ConnectionState::Connected)
            }
        }
    }

    async fn try_read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        // First check if we have buffered data
        if !self.read_buffer.is_empty() {
            let to_read = buf.len().min(self.read_buffer.len());
            buf[..to_read].copy_from_slice(&self.read_buffer[..to_read]);
            self.read_buffer.drain(..to_read);
            return Ok(to_read);
        }

        // Try to fill buffer without blocking
        self.fill_buffer_nonblocking().await?;

        // Check buffer again after attempting to fill
        if !self.read_buffer.is_empty() {
            let to_read = buf.len().min(self.read_buffer.len());
            buf[..to_read].copy_from_slice(&self.read_buffer[..to_read]);
            self.read_buffer.drain(..to_read);
            Ok(to_read)
        } else {
            // No data available
            Ok(0)
        }
    }

    async fn send(&mut self, buf: &[u8]) -> crate::Result<()> {
        self.channel.make_writer().write_all(buf).await?;
        Ok(())
    }

    async fn shutdown(&mut self) -> crate::Result<()> {
        self.channel.eof().await?;
        self.channel.close().await?;

        Ok(())
    }
}

#[derive(Clone)]
struct Server {
    clients: Arc<Mutex<HashMap<usize, (ChannelId, russh::server::Handle)>>>,
    id: usize,
}

struct Client {
    host: String,
    port: u16,
    host_key_policy: HostKeyPolicy,
}

impl russh::client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(&mut self, key: &ssh_key::PublicKey) -> Result<bool, Self::Error> {
        match &self.host_key_policy {
            HostKeyPolicy::InsecureAcceptAny => Ok(true),
            HostKeyPolicy::Fingerprint(expected) => {
                if key.fingerprint(ssh_key::HashAlg::Sha256).to_string() == expected.0 {
                    Ok(true)
                } else {
                    Err(russh::Error::KeyChanged { line: 0 })
                }
            }
            HostKeyPolicy::KnownHosts { path, accept_new } => match russh::keys::known_hosts::check_known_hosts_path(&self.host, self.port, key, path) {
                Ok(true) => Ok(true),
                Ok(false) if *accept_new => {
                    prepare_known_hosts_file(path)?;
                    russh::keys::known_hosts::learn_known_hosts_path(&self.host, self.port, key, path)?;
                    Ok(true)
                }
                Ok(false) => Err(russh::Error::UnknownKey),
                Err(russh::keys::Error::KeyChanged { line }) => Err(russh::Error::KeyChanged { line }),
                Err(error) => Err(russh::Error::Keys(error)),
            },
        }
    }
}

pub struct SshClient {
    session: client::Handle<Client>,
}

impl SshClient {
    async fn connect(addr: impl Into<String>, options: SshConnectionOptions) -> Result<Self, SshAuthenticationError> {
        let (connect_addr, host, port) = split_ssh_address(addr.into());

        let mut preferred = Preferred::DEFAULT.clone();
        preferred.kex = Cow::Owned(kex::ALL_KEX_ALGORITHMS.iter().map(|k| **k).collect());
        preferred.cipher = Cow::Owned(cipher::ALL_CIPHERS.iter().map(|k| **k).collect());
        let config = client::Config {
            inactivity_timeout: None,
            preferred,
            // keepalive_interval: Some(Duration::from_secs(30)),
            // keepalive_max: 3,
            ..<_>::default()
        };
        let config = Arc::new(config);
        let sh = Client {
            host,
            port,
            host_key_policy: options.host_key_policy,
        };
        let tcp_stream = connect_tcp(&connect_addr, options.proxy.as_ref(), options.connect_timeout)
            .await
            .map_err(SshAuthenticationError::Transport)?;
        tcp_stream.set_nodelay(true).map_err(transport)?;
        let mut session: client::Handle<Client> = russh::client::connect_stream(config, tcp_stream, sh).await.map_err(map_transport_error)?;

        let user_name = options.credentials.user_name;
        let auth = authenticate(&mut session, &user_name, options.credentials.authentication, options.authentication_timeout);
        tokio::time::timeout(options.authentication_timeout, auth).await.map_err(transport)??;

        Ok(Self { session })
    }

    async fn call(&mut self, command: &str) -> crate::Result<u32> {
        let mut channel = self.session.channel_open_session().await?;
        channel.exec(true, command).await?;

        let mut code = None;
        let mut stdout = tokio::io::stdout();

        loop {
            // There's an event available on the session channel
            let Some(msg) = channel.wait().await else {
                break;
            };
            match msg {
                // Write data to the terminal
                ChannelMsg::Data { ref data } => {
                    stdout.write_all(data).await?;
                    stdout.flush().await?;
                }
                // The command has returned an exit code
                ChannelMsg::ExitStatus { exit_status } => {
                    code = Some(exit_status);
                    // cannot leave the loop immediately, there might still be more data to receive
                }
                _ => {}
            }
        }
        Ok(code.expect("program did not exit cleanly"))
    }

    async fn close(&mut self) -> crate::Result<()> {
        self.session.disconnect(Disconnect::ByApplication, "", "English").await?;
        Ok(())
    }
}

async fn authenticate(
    session: &mut client::Handle<Client>,
    user_name: &str,
    authentication: SshAuthentication,
    operation_timeout: Duration,
) -> Result<(), SshAuthenticationError> {
    let mut attempted = Vec::new();
    let mut server_methods = Vec::new();

    match authentication {
        SshAuthentication::Password { password } => {
            if authenticate_password(session, user_name, &password, &mut attempted, &mut server_methods).await? {
                return Ok(());
            }
        }
        SshAuthentication::PrivateKey { path, passphrase } => {
            let credential = PrivateKeyCredential { path, passphrase };
            if authenticate_private_key(session, user_name, &credential, &mut attempted, &mut server_methods).await? {
                return Ok(());
            }
        }
        SshAuthentication::Agent { public_key } => {
            if authenticate_agent(session, user_name, public_key.as_ref(), operation_timeout, &mut attempted, &mut server_methods).await? {
                return Ok(());
            }
        }
        SshAuthentication::Auto {
            private_keys,
            use_agent,
            password,
        } => {
            for credential in private_keys {
                if method_is_available(&server_methods, SshAuthenticationMethod::PublicKey)
                    && authenticate_private_key(session, user_name, &credential, &mut attempted, &mut server_methods).await?
                {
                    return Ok(());
                }
            }
            if use_agent
                && method_is_available(&server_methods, SshAuthenticationMethod::PublicKey)
                && authenticate_agent(session, user_name, None, operation_timeout, &mut attempted, &mut server_methods).await?
            {
                return Ok(());
            }
            if let Some(password) = password
                && method_is_available(&server_methods, SshAuthenticationMethod::Password)
                && authenticate_password(session, user_name, &password, &mut attempted, &mut server_methods).await?
            {
                return Ok(());
            }
        }
    }

    Err(SshAuthenticationError::AuthenticationRejected { attempted, server_methods })
}

async fn authenticate_password(
    session: &mut client::Handle<Client>,
    user_name: &str,
    password: &SecretString,
    attempted: &mut Vec<SshAuthenticationMethod>,
    server_methods: &mut Vec<SshAuthenticationMethod>,
) -> Result<bool, SshAuthenticationError> {
    attempted.push(SshAuthenticationMethod::Password);
    let result = session
        .authenticate_password(user_name, password.expose_secret().to_owned())
        .await
        .map_err(map_transport_error)?;
    Ok(record_auth_result(result, server_methods))
}

async fn authenticate_private_key(
    session: &mut client::Handle<Client>,
    user_name: &str,
    credential: &PrivateKeyCredential,
    attempted: &mut Vec<SshAuthenticationMethod>,
    server_methods: &mut Vec<SshAuthenticationMethod>,
) -> Result<bool, SshAuthenticationError> {
    let key = load_private_key(credential).await?;
    let hash_alg = session.best_supported_rsa_hash().await.map_err(map_transport_error)?.flatten();
    attempted.push(SshAuthenticationMethod::PublicKey);
    let result = session
        .authenticate_publickey(user_name, russh::keys::PrivateKeyWithHashAlg::new(key, hash_alg))
        .await
        .map_err(map_transport_error)?;
    Ok(record_auth_result(result, server_methods))
}

#[cfg(unix)]
async fn authenticate_agent(
    session: &mut client::Handle<Client>,
    user_name: &str,
    selector: Option<&SshPublicKeySelector>,
    operation_timeout: Duration,
    attempted: &mut Vec<SshAuthenticationMethod>,
    server_methods: &mut Vec<SshAuthenticationMethod>,
) -> Result<bool, SshAuthenticationError> {
    let mut agent = tokio::time::timeout(operation_timeout, russh::keys::agent::client::AgentClient::connect_env())
        .await
        .map_err(|_| SshAuthenticationError::AgentUnavailable)?
        .map_err(|_| SshAuthenticationError::AgentUnavailable)?;
    let identities = tokio::time::timeout(operation_timeout, agent.request_identities())
        .await
        .map_err(|_| SshAuthenticationError::AgentUnavailable)?
        .map_err(|_| SshAuthenticationError::AgentUnavailable)?;
    if identities.is_empty() {
        return Err(SshAuthenticationError::AgentHasNoIdentities);
    }
    let selected_key = selected_public_key(selector).await?;
    let mut eligible = 0;
    for identity in identities {
        let hash_alg = session.best_supported_rsa_hash().await.map_err(map_transport_error)?.flatten();
        let result = match identity {
            russh::keys::agent::AgentIdentity::PublicKey { key, .. } => {
                if !selector_matches(selector, selected_key.as_ref(), &key) {
                    continue;
                }
                eligible += 1;
                tokio::time::timeout(operation_timeout, session.authenticate_publickey_with(user_name, key, hash_alg, &mut agent)).await
            }
            russh::keys::agent::AgentIdentity::Certificate { certificate, .. } => {
                let key = ssh_key::PublicKey::new(certificate.public_key().clone(), "");
                if !selector_matches(selector, selected_key.as_ref(), &key) {
                    continue;
                }
                eligible += 1;
                tokio::time::timeout(
                    operation_timeout,
                    session.authenticate_certificate_with(user_name, certificate, hash_alg, &mut agent),
                )
                .await
            }
        }
        .map_err(|_| SshAuthenticationError::AgentUnavailable)?
        .map_err(|_| SshAuthenticationError::AgentUnavailable)?;
        attempted.push(SshAuthenticationMethod::Agent);
        if record_auth_result(result, server_methods) {
            return Ok(true);
        }
        if !method_is_available(server_methods, SshAuthenticationMethod::PublicKey) {
            break;
        }
    }
    if selector.is_some() && eligible == 0 {
        return Err(SshAuthenticationError::SelectedAgentKeyNotFound);
    }
    Ok(false)
}

#[cfg(not(unix))]
async fn authenticate_agent(
    _: &mut client::Handle<Client>,
    _: &str,
    _: Option<&SshPublicKeySelector>,
    _: Duration,
    _: &mut Vec<SshAuthenticationMethod>,
    _: &mut Vec<SshAuthenticationMethod>,
) -> Result<bool, SshAuthenticationError> {
    Err(SshAuthenticationError::AgentUnavailable)
}

async fn selected_public_key(selector: Option<&SshPublicKeySelector>) -> Result<Option<ssh_key::PublicKey>, SshAuthenticationError> {
    let Some(SshPublicKeySelector::PublicKeyFile(path)) = selector else {
        return Ok(None);
    };
    let contents = tokio::fs::read_to_string(path)
        .await
        .map_err(|_| SshAuthenticationError::SelectedAgentKeyNotFound)?;
    ssh_key::PublicKey::from_openssh(&contents)
        .map(Some)
        .map_err(|_| SshAuthenticationError::SelectedAgentKeyNotFound)
}

fn selector_matches(selector: Option<&SshPublicKeySelector>, selected_key: Option<&ssh_key::PublicKey>, key: &ssh_key::PublicKey) -> bool {
    match selector {
        None => true,
        Some(SshPublicKeySelector::Fingerprint(expected)) => key.fingerprint(ssh_key::HashAlg::Sha256).to_string() == *expected,
        Some(SshPublicKeySelector::PublicKeyFile(_)) => selected_key == Some(key),
    }
}

fn record_auth_result(result: client::AuthResult, server_methods: &mut Vec<SshAuthenticationMethod>) -> bool {
    match result {
        client::AuthResult::Success => true,
        client::AuthResult::Failure { remaining_methods, .. } => {
            *server_methods = remaining_methods.iter().filter_map(method_from_russh).collect();
            false
        }
    }
}

fn method_from_russh(method: &MethodKind) -> Option<SshAuthenticationMethod> {
    match method {
        MethodKind::Password => Some(SshAuthenticationMethod::Password),
        MethodKind::PublicKey => Some(SshAuthenticationMethod::PublicKey),
        _ => None,
    }
}

fn method_is_available(server_methods: &[SshAuthenticationMethod], method: SshAuthenticationMethod) -> bool {
    server_methods.is_empty() || server_methods.contains(&method)
}

fn split_ssh_address(addr: String) -> (String, String, u16) {
    if let Some(end) = addr.find("]:")
        && addr.starts_with('[')
        && let Ok(port) = addr[end + 2..].parse()
    {
        return (addr.clone(), addr[1..end].to_string(), port);
    }
    if let Some((host, port)) = addr.rsplit_once(':')
        && !host.contains(':')
        && let Ok(port) = port.parse()
    {
        return (addr.clone(), host.to_string(), port);
    }
    if addr.parse::<std::net::Ipv6Addr>().is_ok() {
        return (format!("[{addr}]:22"), addr, 22);
    }
    (format!("{addr}:22"), addr, 22)
}

fn map_transport_error(error: russh::Error) -> SshAuthenticationError {
    match error {
        russh::Error::UnknownKey => SshAuthenticationError::HostKeyUnknown,
        russh::Error::KeyChanged { .. } => SshAuthenticationError::HostKeyMismatch,
        error => transport(error),
    }
}

fn transport(error: impl std::error::Error + Send + Sync + 'static) -> SshAuthenticationError {
    SshAuthenticationError::Transport(Box::new(error))
}

fn prepare_known_hosts_file(path: &PathBuf) -> Result<(), russh::Error> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut options = std::fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    match options.open(path) {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telnet::TerminalEmulation;
    use russh::keys::ssh_key::{Algorithm, LineEnding};
    use russh::{MethodSet, server};

    fn generated_key(algorithm: Algorithm) -> PrivateKey {
        PrivateKey::random(&mut russh::keys::key::safe_rng(), algorithm).unwrap()
    }

    fn write_key(path: &std::path::Path, key: &PrivateKey) {
        let encoded = key.to_openssh(LineEnding::LF).unwrap();
        std::fs::write(path, encoded.as_bytes()).unwrap();
    }

    #[test]
    fn secret_debug_output_is_redacted() {
        let credentials = Credentials::password("sysop", "never-print-this");
        let debug = format!("{credentials:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("never-print-this"));
    }

    #[test]
    fn constructors_select_the_requested_authentication_mode() {
        assert!(matches!(Credentials::password("u", "p").authentication, SshAuthentication::Password { .. }));
        assert!(matches!(
            Credentials::private_key("u", "id_ed25519", None).authentication,
            SshAuthentication::PrivateKey { .. }
        ));
        assert!(matches!(Credentials::agent("u").authentication, SshAuthentication::Agent { .. }));
    }

    #[test]
    fn automatic_authentication_keeps_caller_order() {
        let authentication = SshAuthentication::Auto {
            private_keys: vec![
                PrivateKeyCredential {
                    path: "first".into(),
                    passphrase: None,
                },
                PrivateKeyCredential {
                    path: "second".into(),
                    passphrase: None,
                },
            ],
            use_agent: true,
            password: Some(SecretString::new("password")),
        };
        assert_eq!(
            authentication.configured_methods(),
            vec![
                SshAuthenticationMethod::PublicKey,
                SshAuthenticationMethod::PublicKey,
                SshAuthenticationMethod::Agent,
                SshAuthenticationMethod::Password,
            ]
        );
    }

    #[tokio::test]
    async fn loads_an_unencrypted_ed25519_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("id_ed25519");
        write_key(&path, &generated_key(Algorithm::Ed25519));
        let loaded = load_private_key(&PrivateKeyCredential { path, passphrase: None }).await.unwrap();
        assert_eq!(loaded.algorithm(), Algorithm::Ed25519);
    }

    #[tokio::test]
    async fn loads_an_unencrypted_rsa_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("id_rsa");
        write_key(&path, &generated_key(Algorithm::Rsa { hash: None }));
        let loaded = load_private_key(&PrivateKeyCredential { path, passphrase: None }).await.unwrap();
        assert!(matches!(loaded.algorithm(), Algorithm::Rsa { .. }));
    }

    #[tokio::test]
    async fn encrypted_key_requires_the_right_passphrase() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("id_ed25519");
        let key = generated_key(Algorithm::Ed25519)
            .encrypt(&mut russh::keys::key::safe_rng(), "correct-passphrase")
            .unwrap();
        write_key(&path, &key);

        let missing = load_private_key(&PrivateKeyCredential {
            path: path.clone(),
            passphrase: None,
        })
        .await
        .unwrap_err();
        assert!(matches!(missing, SshAuthenticationError::PassphraseRequired { .. }));

        let invalid = load_private_key(&PrivateKeyCredential {
            path: path.clone(),
            passphrase: Some(SecretString::new("wrong-passphrase")),
        })
        .await
        .unwrap_err();
        assert!(matches!(invalid, SshAuthenticationError::InvalidPassphrase { .. }));

        let loaded = load_private_key(&PrivateKeyCredential {
            path,
            passphrase: Some(SecretString::new("correct-passphrase")),
        })
        .await
        .unwrap();
        assert_eq!(loaded.algorithm(), Algorithm::Ed25519);
    }

    #[tokio::test]
    async fn key_file_errors_are_distinct() {
        let dir = tempfile::tempdir().unwrap();
        let missing = load_private_key(&PrivateKeyCredential {
            path: dir.path().join("missing"),
            passphrase: None,
        })
        .await
        .unwrap_err();
        assert!(matches!(missing, SshAuthenticationError::KeyFileNotFound { .. }));

        let unreadable = load_private_key(&PrivateKeyCredential {
            path: dir.path().to_path_buf(),
            passphrase: None,
        })
        .await
        .unwrap_err();
        assert!(matches!(unreadable, SshAuthenticationError::KeyFileUnreadable { .. }));

        let malformed_path = dir.path().join("malformed");
        std::fs::write(&malformed_path, "this is not a private key").unwrap();
        let malformed = load_private_key(&PrivateKeyCredential {
            path: malformed_path,
            passphrase: None,
        })
        .await
        .unwrap_err();
        assert!(matches!(malformed, SshAuthenticationError::UnsupportedKeyFormat { .. }));
    }

    #[test]
    fn selectors_match_sha256_fingerprints_and_public_key_files() {
        let key = generated_key(Algorithm::Ed25519).public_key().clone();
        let fingerprint = key.fingerprint(ssh_key::HashAlg::Sha256).to_string();
        assert!(selector_matches(Some(&SshPublicKeySelector::Fingerprint(fingerprint)), None, &key));
        assert!(selector_matches(Some(&SshPublicKeySelector::PublicKeyFile("key.pub".into())), Some(&key), &key));
    }

    #[test]
    fn ssh_addresses_keep_host_and_port_separate() {
        assert_eq!(split_ssh_address("example.org".into()), ("example.org:22".into(), "example.org".into(), 22));
        assert_eq!(
            split_ssh_address("example.org:2222".into()),
            ("example.org:2222".into(), "example.org".into(), 2222)
        );
        assert_eq!(split_ssh_address("::1".into()), ("[::1]:22".into(), "::1".into(), 22));
        assert_eq!(split_ssh_address("[::1]:2222".into()), ("[::1]:2222".into(), "::1".into(), 2222));
    }

    #[derive(Clone)]
    struct TestServer {
        password: Option<String>,
        public_key: Option<ssh_key::PublicKey>,
    }

    impl server::Handler for TestServer {
        type Error = russh::Error;

        async fn auth_password(&mut self, _: &str, password: &str) -> Result<server::Auth, Self::Error> {
            if self.password.as_deref() == Some(password) {
                Ok(server::Auth::Accept)
            } else {
                Ok(self.reject())
            }
        }

        async fn auth_publickey(&mut self, _: &str, key: &ssh_key::PublicKey) -> Result<server::Auth, Self::Error> {
            if self.public_key.as_ref() == Some(key) {
                Ok(server::Auth::Accept)
            } else {
                Ok(self.reject())
            }
        }

        async fn channel_open_session(
            &mut self,
            _: Channel<server::Msg>,
            reply: server::ChannelOpenHandle,
            _: &mut server::Session,
        ) -> Result<(), Self::Error> {
            reply.accept().await;
            Ok(())
        }
    }

    impl TestServer {
        fn reject(&self) -> server::Auth {
            let mut methods = Vec::new();
            if self.public_key.is_some() {
                methods.push(MethodKind::PublicKey);
            }
            if self.password.is_some() {
                methods.push(MethodKind::Password);
            }
            server::Auth::Reject {
                proceed_with_methods: Some(MethodSet::from(methods.as_slice())),
                partial_success: false,
            }
        }
    }

    async fn spawn_server(server: TestServer, host_key: PrivateKey) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let config = Arc::new(server::Config {
            auth_rejection_time: Duration::from_millis(1),
            auth_rejection_time_initial: Some(Duration::ZERO),
            keys: vec![host_key],
            ..Default::default()
        });
        let task = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let running = server::run_stream(config, socket, server).await.unwrap();
            let _ = running.await;
        });
        (address, task)
    }

    fn test_caps() -> TermCaps {
        TermCaps {
            window_size: (80, 25),
            terminal: TerminalEmulation::Ansi,
        }
    }

    #[tokio::test]
    async fn password_authentication_remains_compatible() {
        let host_key = generated_key(Algorithm::Ed25519);
        let (address, server) = spawn_server(
            TestServer {
                password: Some("secret".into()),
                public_key: None,
            },
            host_key,
        )
        .await;
        let connection = SSHConnection::open(address.to_string(), test_caps(), Credentials::password("sysop", "secret")).await;
        assert!(connection.is_ok());
        server.abort();
    }

    #[tokio::test]
    async fn private_key_authentication_accepts_and_rejects_keys() {
        let dir = tempfile::tempdir().unwrap();
        let accepted_path = dir.path().join("accepted");
        let accepted = generated_key(Algorithm::Ed25519);
        write_key(&accepted_path, &accepted);
        let (address, server) = spawn_server(
            TestServer {
                password: None,
                public_key: Some(accepted.public_key().clone()),
            },
            generated_key(Algorithm::Ed25519),
        )
        .await;
        let connection = SSHConnection::open(address.to_string(), test_caps(), Credentials::private_key("sysop", accepted_path, None)).await;
        assert!(connection.is_ok());
        server.abort();

        let rejected_path = dir.path().join("rejected");
        write_key(&rejected_path, &generated_key(Algorithm::Ed25519));
        let (address, server) = spawn_server(
            TestServer {
                password: None,
                public_key: Some(accepted.public_key().clone()),
            },
            generated_key(Algorithm::Ed25519),
        )
        .await;
        let result = SSHConnection::open(address.to_string(), test_caps(), Credentials::private_key("sysop", rejected_path, None)).await;
        let Err(error) = result else {
            panic!("server accepted an unrelated private key");
        };
        assert!(matches!(
            error.downcast_ref::<SshAuthenticationError>(),
            Some(SshAuthenticationError::AuthenticationRejected { .. })
        ));
        server.abort();
    }

    #[tokio::test]
    async fn rsa_and_encrypted_private_keys_authenticate() {
        let dir = tempfile::tempdir().unwrap();
        let rsa_path = dir.path().join("id_rsa");
        let rsa = generated_key(Algorithm::Rsa { hash: None });
        write_key(&rsa_path, &rsa);
        let (address, server) = spawn_server(
            TestServer {
                password: None,
                public_key: Some(rsa.public_key().clone()),
            },
            generated_key(Algorithm::Ed25519),
        )
        .await;
        assert!(
            SSHConnection::open(address.to_string(), test_caps(), Credentials::private_key("sysop", rsa_path, None),)
                .await
                .is_ok()
        );
        server.abort();

        let encrypted_path = dir.path().join("id_ed25519");
        let plain = generated_key(Algorithm::Ed25519);
        let encrypted = plain.clone().encrypt(&mut russh::keys::key::safe_rng(), "passphrase").unwrap();
        write_key(&encrypted_path, &encrypted);
        let (address, server) = spawn_server(
            TestServer {
                password: None,
                public_key: Some(plain.public_key().clone()),
            },
            generated_key(Algorithm::Ed25519),
        )
        .await;
        assert!(
            SSHConnection::open(
                address.to_string(),
                test_caps(),
                Credentials::private_key("sysop", encrypted_path, Some(SecretString::new("passphrase"))),
            )
            .await
            .is_ok()
        );
        server.abort();
    }

    #[tokio::test]
    async fn automatic_mode_falls_back_from_a_rejected_key_to_password() {
        let dir = tempfile::tempdir().unwrap();
        let rejected_path = dir.path().join("rejected");
        write_key(&rejected_path, &generated_key(Algorithm::Ed25519));
        let credentials = Credentials {
            user_name: "sysop".into(),
            authentication: SshAuthentication::Auto {
                private_keys: vec![PrivateKeyCredential {
                    path: rejected_path,
                    passphrase: None,
                }],
                use_agent: false,
                password: Some(SecretString::new("secret")),
            },
            proxy_command: None,
        };
        let (address, server) = spawn_server(
            TestServer {
                password: Some("secret".into()),
                public_key: Some(generated_key(Algorithm::Ed25519).public_key().clone()),
            },
            generated_key(Algorithm::Ed25519),
        )
        .await;
        let connection = SSHConnection::open(address.to_string(), test_caps(), credentials).await;
        assert!(connection.is_ok());
        server.abort();
    }

    #[tokio::test]
    async fn known_hosts_policy_distinguishes_unknown_and_changed_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("known_hosts");
        let first = generated_key(Algorithm::Ed25519);
        let second = generated_key(Algorithm::Ed25519);
        let mut unknown = Client {
            host: "example.org".into(),
            port: 22,
            host_key_policy: HostKeyPolicy::KnownHosts {
                path: path.clone(),
                accept_new: false,
            },
        };
        assert!(matches!(
            russh::client::Handler::check_server_key(&mut unknown, first.public_key()).await,
            Err(russh::Error::UnknownKey)
        ));

        let mut accept_new = Client {
            host: "example.org".into(),
            port: 22,
            host_key_policy: HostKeyPolicy::KnownHosts {
                path: path.clone(),
                accept_new: true,
            },
        };
        assert!(russh::client::Handler::check_server_key(&mut accept_new, first.public_key()).await.unwrap());
        assert!(matches!(
            russh::client::Handler::check_server_key(&mut accept_new, second.public_key()).await,
            Err(russh::Error::KeyChanged { .. })
        ));
    }

    #[tokio::test]
    async fn open_with_options_returns_typed_host_key_errors() {
        let dir = tempfile::tempdir().unwrap();
        let (address, server) = spawn_server(
            TestServer {
                password: Some("secret".into()),
                public_key: None,
            },
            generated_key(Algorithm::Ed25519),
        )
        .await;
        let result = SSHConnection::open_with_options(
            address.to_string(),
            test_caps(),
            SshConnectionOptions {
                credentials: Credentials::password("sysop", "secret"),
                host_key_policy: HostKeyPolicy::KnownHosts {
                    path: dir.path().join("known_hosts"),
                    accept_new: false,
                },
                connect_timeout: Duration::from_secs(2),
                authentication_timeout: Duration::from_secs(2),
                proxy: None,
            },
        )
        .await;
        let Err(error) = result else {
            panic!("unknown host key was accepted");
        };
        assert!(matches!(
            error.downcast_ref::<SshAuthenticationError>(),
            Some(SshAuthenticationError::HostKeyUnknown)
        ));
        server.abort();

        let (address, server) = spawn_server(
            TestServer {
                password: Some("secret".into()),
                public_key: None,
            },
            generated_key(Algorithm::Ed25519),
        )
        .await;
        let unrelated = generated_key(Algorithm::Ed25519);
        let result = SSHConnection::open_with_options(
            address.to_string(),
            test_caps(),
            SshConnectionOptions {
                credentials: Credentials::password("sysop", "secret"),
                host_key_policy: HostKeyPolicy::Fingerprint(SshHostKeyFingerprint(unrelated.public_key().fingerprint(ssh_key::HashAlg::Sha256).to_string())),
                connect_timeout: Duration::from_secs(2),
                authentication_timeout: Duration::from_secs(2),
                proxy: None,
            },
        )
        .await;
        let Err(error) = result else {
            panic!("mismatched host-key fingerprint was accepted");
        };
        assert!(matches!(
            error.downcast_ref::<SshAuthenticationError>(),
            Some(SshAuthenticationError::HostKeyMismatch)
        ));
        server.abort();
    }

    struct SlowServer;

    impl server::Handler for SlowServer {
        type Error = russh::Error;

        async fn auth_password(&mut self, _: &str, _: &str) -> Result<server::Auth, Self::Error> {
            tokio::time::sleep(Duration::from_millis(250)).await;
            Ok(server::Auth::Accept)
        }
    }

    #[tokio::test]
    async fn authentication_respects_its_timeout() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let config = Arc::new(server::Config {
            keys: vec![generated_key(Algorithm::Ed25519)],
            ..Default::default()
        });
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let running = server::run_stream(config, socket, SlowServer).await.unwrap();
            let _ = running.await;
        });
        let result = SSHConnection::open_with_options(
            address.to_string(),
            test_caps(),
            SshConnectionOptions {
                credentials: Credentials::password("sysop", "secret"),
                host_key_policy: HostKeyPolicy::InsecureAcceptAny,
                connect_timeout: Duration::from_secs(2),
                authentication_timeout: Duration::from_millis(20),
                proxy: None,
            },
        )
        .await;
        let Err(error) = result else {
            panic!("authentication exceeded its configured timeout");
        };
        assert!(matches!(
            error.downcast_ref::<SshAuthenticationError>(),
            Some(SshAuthenticationError::Transport(_))
        ));
        server.abort();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn agent_authentication_uses_the_selected_identity() {
        static AGENT_ENV: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
        let _environment_guard = AGENT_ENV.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("agent.sock");
        let listener = tokio::net::UnixListener::bind(&socket_path).unwrap();
        let incoming = futures_util::stream::unfold(listener, |listener| async move {
            let item = listener.accept().await.map(|(stream, _)| stream);
            Some((item, listener))
        });
        let agent_server = tokio::spawn(russh::keys::agent::server::serve::<tokio::net::UnixStream, _, _>(Box::pin(incoming), ()));

        let unrelated = generated_key(Algorithm::Ed25519);
        let accepted = generated_key(Algorithm::Ed25519);
        let mut setup = russh::keys::agent::client::AgentClient::connect_uds(&socket_path).await.unwrap();
        setup.add_identity(&unrelated, &[]).await.unwrap();
        setup.add_identity(&accepted, &[]).await.unwrap();
        let fingerprint = accepted.public_key().fingerprint(ssh_key::HashAlg::Sha256).to_string();

        let old_socket = std::env::var_os("SSH_AUTH_SOCK");
        unsafe { std::env::set_var("SSH_AUTH_SOCK", &socket_path) };
        let credentials = Credentials {
            user_name: "sysop".into(),
            authentication: SshAuthentication::Agent {
                public_key: Some(SshPublicKeySelector::Fingerprint(fingerprint)),
            },
            proxy_command: None,
        };
        let (address, ssh_server) = spawn_server(
            TestServer {
                password: None,
                public_key: Some(accepted.public_key().clone()),
            },
            generated_key(Algorithm::Ed25519),
        )
        .await;
        let connection = SSHConnection::open(address.to_string(), test_caps(), credentials).await;

        match old_socket {
            Some(value) => unsafe { std::env::set_var("SSH_AUTH_SOCK", value) },
            None => unsafe { std::env::remove_var("SSH_AUTH_SOCK") },
        }
        assert!(connection.is_ok());
        ssh_server.abort();
        agent_server.abort();
    }
}
