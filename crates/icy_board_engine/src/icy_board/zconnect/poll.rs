//! Online caller for the deliberately limited ZIP/ZMODEM public-mail profile.
//!
//! Transport is Telnet, NOT TLS: credentials and mail travel in cleartext. Use
//! only on a trusted network/tunnel. `host` is only the dial address; optional
//! `remote_system` checks the peer's Chapter II SYS display name, not its
//! cryptographic identity. `username`, when present, is our account SYS name
//! in BLK1 and the JANUS Systemname;
//! otherwise it defaults to `local_system`. Standard login always dispatches via
//! zconnect/0zconnec and authenticates with the link password in BLK1.
//!
//! An error never removes downloaded archives. Discover already received files
//! in inbound even when the final handshake fails. Packet import is separate.

use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use fs4::FileExt;
use icy_net::{
    Connection,
    telnet::{TelnetConnection, TermCaps, TerminalEmulation},
    zconnect::{
        commands::{ZConnectCommandBlock, mails},
        session::{Identity, Limits, Login, Session},
    },
};

use super::{ZconnectConfig, ZconnectLink, acknowledge_outbound};

#[derive(Debug, Default)]
pub struct PollReport {
    pub uploaded: bool,
    pub downloaded: Vec<PathBuf>,
}

const MAX_DOWNLOADS: usize = 32;
const MAX_SESSION_BYTES: u64 = 256 * 1024 * 1024;

fn rooted(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() { path.to_path_buf() } else { root.join(path) }
}

fn poll_limits(link: &ZconnectLink) -> Limits {
    Limits {
        timeout: Duration::from_secs(u64::from(link.timeout_secs)),
        packet_bytes: 32 * 1024 * 1024,
        ..Limits::default()
    }
}

fn poll_identity<'a>(config: &'a ZconnectConfig, link: &'a ZconnectLink) -> Identity<'a> {
    let system = if link.username.is_empty() { &config.local_system } else { &link.username };
    Identity {
        system,
        sysop: &config.local_user,
        login_system: system,
        remote_system: &link.remote_system,
        password: &link.password,
    }
}

pub async fn poll(config: &ZconnectConfig, board_root: &Path, link_id: &str) -> crate::Res<PollReport> {
    config.validate()?;
    if !config.enabled {
        return Err("ZCONNECT is disabled".into());
    }
    let link = config.link(link_id).ok_or("Unknown ZCONNECT link")?;
    if link.host.is_empty() {
        return Err("ZCONNECT link is offline-only (no host)".into());
    }
    let login = match link.login.as_str() {
        "direct" => Login::Direct,
        "zconnect" => Login::Zconnect,
        "janus" => Login::Janus,
        _ => return Err("Unsupported ZCONNECT login profile".into()),
    };
    let limits = poll_limits(link);
    let outbound = rooted(board_root, &config.outbound).join(&link.id).join("mail.zip");
    let inbound = rooted(board_root, &config.inbound).join(&link.id);
    fs::create_dir_all(&inbound)?;
    let spool = outbound.parent().ok_or("Invalid ZCONNECT outbound path")?;
    fs::create_dir_all(spool)?;
    // Keep the inode stable. This is distinct from the offline transaction lock,
    // which acknowledge_outbound takes for its short atomic commit.
    let poll_lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(spool.join("poll.lock"))?;
    FileExt::try_lock(&poll_lock).map_err(|_| "A ZCONNECT poll is already active for this link")?;
    // Reject symlink packet aliases: the offline transaction owns this exact file.
    let outbound_size = match fs::symlink_metadata(&outbound) {
        Ok(meta) if meta.file_type().is_file() && meta.len() <= limits.packet_bytes => Some(meta.len()),
        Ok(_) => return Err("Unsafe or oversized ZCONNECT outbound packet".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    let identity = poll_identity(config, link);
    let address = if link.host.contains(':') {
        format!("[{}]:{}", link.host, link.port)
    } else {
        format!("{}:{}", link.host, link.port)
    };
    let connection = TelnetConnection::open(
        address,
        TermCaps {
            window_size: (80, 25),
            terminal: TerminalEmulation::Ascii,
        },
        limits.timeout,
    )
    .await?;
    poll_connection(
        connection,
        config,
        board_root,
        &link.id,
        login,
        &identity,
        limits,
        outbound,
        outbound_size,
        inbound,
    )
    .await
}

// Separate transport construction from the transaction so synthetic peers can
// exercise precisely the same flow without opening a socket.
#[allow(clippy::too_many_arguments)]
async fn poll_connection<C: Connection>(
    connection: C,
    config: &ZconnectConfig,
    board_root: &Path,
    link_id: &str,
    login: Login,
    identity: &Identity<'_>,
    limits: Limits,
    outbound: PathBuf,
    outbound_size: Option<u64>,
    inbound: PathBuf,
) -> crate::Res<PollReport> {
    let mut session = Session::new(connection, limits)?;
    let mut report = PollReport::default();
    let result: crate::Res<()> = async {
        session.login(login, identity).await?;
        session.negotiate(identity).await?;
        let mut pending_receipt = false;
        if let Some(bytes) = outbound_size {
            let command = ZConnectCommandBlock::default().put(mails::NEWS).format().file_size(bytes);
            let mut reply = session.request(&command).await?;
            if !session.execute(&mut reply, true).await? {
                return Err("ZCONNECT peer declined outbound public mail; packet retained".into());
            }
            session.send_zip(&outbound).await?;
            pending_receipt = true;
        }

        let mut delete_durable_download = false;
        let mut received_bytes = 0u64;
        for round in 0..=MAX_DOWNLOADS {
            let closing = round == MAX_DOWNLOADS || received_bytes == MAX_SESSION_BYTES;
            if !closing {
                session.limit_packet_bytes(MAX_SESSION_BYTES - received_bytes)?;
            }
            // DELETE has precedence over GET. Only the packet from the last
            // completed, synced receive is eligible; never delete on speculation.
            let mut command = if closing {
                ZConnectCommandBlock::default().logoff()
            } else {
                ZConnectCommandBlock::default().get(mails::NEWS)
            };
            if delete_durable_download {
                command = command.delete(mails::NEWS);
            }
            let mut reply = session.request(&command).await?;
            if pending_receipt {
                // Chapter II.4.2: ZMODEM alone is insufficient. request() has
                // validated ACK1 and checked BLK2 for a RETRANSMIT override.
                acknowledge_outbound(config, board_root, link_id)?;
                report.uploaded = true;
                pending_receipt = false;
            }
            if closing {
                session.execute(&mut reply, false).await?;
                return Ok(());
            }
            if !session.execute(&mut reply, true).await? {
                // Finish an explicit logoff cycle instead of dropping carrier.
                let mut reply = session.request(&ZConnectCommandBlock::default().logoff()).await?;
                session.execute(&mut reply, false).await?;
                return Ok(());
            }
            let path = session.receive_zip(&inbound, &reply).await?;
            received_bytes = received_bytes
                .checked_add(fs::metadata(&path)?.len())
                .ok_or("ZCONNECT download accounting overflow")?;
            report.downloaded.push(path);
            if received_bytes > MAX_SESSION_BYTES {
                // Preserve this archive and leave remote mail alone on failure.
                return Err("ZCONNECT session download limit exceeded".into());
            }
            delete_durable_download = true;
        }
        Ok(())
    }
    .await;
    // Preserve the original protocol/IO error; shutdown never implies receipt.
    let shutdown = session.shutdown().await;
    result?;
    shutdown?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use icy_net::{
        ConnectionType,
        zconnect::{
            BlockCode, ZConnectBlock, ZConnectState,
            commands::Execute,
            header::{Acer, TransferProtocol, ZConnectHeaderBlock},
        },
    };
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
    };

    struct Peer {
        bytes: VecDeque<u8>,
        sent: Arc<Mutex<Vec<u8>>>,
    }

    #[async_trait::async_trait]
    impl Connection for Peer {
        fn get_connection_type(&self) -> ConnectionType {
            ConnectionType::Channel
        }
        async fn read(&mut self, out: &mut [u8]) -> icy_net::Result<usize> {
            let n = self.bytes.len().min(out.len());
            for byte in &mut out[..n] {
                *byte = self.bytes.pop_front().unwrap();
            }
            Ok(n)
        }
        async fn try_read(&mut self, out: &mut [u8]) -> icy_net::Result<usize> {
            self.read(out).await
        }
        async fn send(&mut self, bytes: &[u8]) -> icy_net::Result<()> {
            self.sent.lock().unwrap().extend(bytes);
            Ok(())
        }
    }

    fn block(bytes: &mut Vec<u8>, state: ZConnectState, mut command: ZConnectCommandBlock) {
        command.set_state(state);
        bytes.extend(command.display().bytes());
    }

    fn control(bytes: &mut Vec<u8>, state: ZConnectState) {
        block(bytes, state, ZConnectCommandBlock::default());
    }

    fn negotiation() -> Vec<u8> {
        let mut bytes = b"BEGIN\r".to_vec();
        control(&mut bytes, ZConnectState::Ack(BlockCode::Block1));
        let mut header = ZConnectHeaderBlock::default();
        header.set_system("peer.example");
        header.set_sysop("sysop");
        header.add_phone(0, "TCP");
        header.add_acer(0, Acer::ZIP);
        header.add_protocol(0, TransferProtocol::ZModem);
        header.block(BlockCode::Block2);
        bytes.extend(header.display().bytes());
        control(&mut bytes, ZConnectState::Tme(BlockCode::Block2));
        control(&mut bytes, ZConnectState::Ack(BlockCode::Block3));
        control(&mut bytes, ZConnectState::Block(BlockCode::Block4));
        control(&mut bytes, ZConnectState::Tme(BlockCode::Block4));
        bytes
    }

    fn no_transfer_cycle(bytes: &mut Vec<u8>, response: ZConnectCommandBlock) {
        control(bytes, ZConnectState::Ack(BlockCode::Block1));
        block(bytes, ZConnectState::Block(BlockCode::Block2), response);
        control(bytes, ZConnectState::Tme(BlockCode::Block2));
        control(bytes, ZConnectState::Ack(BlockCode::Block3));
        block(
            bytes,
            ZConnectState::Block(BlockCode::Block4),
            ZConnectCommandBlock::default().execute(Execute::No),
        );
        control(bytes, ZConnectState::Tme(BlockCode::Block4));
    }

    fn identity() -> Identity<'static> {
        Identity {
            system: "local.example",
            sysop: "sysop",
            login_system: "local.example",
            remote_system: "peer.example",
            password: "secret",
        }
    }

    #[test]
    fn poll_uses_optional_expected_sys_and_exact_configured_timeout() {
        let config = ZconnectConfig {
            local_system: "local.example".into(),
            ..Default::default()
        };
        let mut link = ZconnectLink {
            host: "192.0.2.1".into(),
            ..Default::default()
        };
        assert!(poll_identity(&config, &link).remote_system.is_empty());
        assert_eq!(poll_identity(&config, &link).system, "local.example");
        link.username = "MYPOINT".into();
        assert_eq!(poll_identity(&config, &link).system, "MYPOINT");
        assert_eq!(poll_identity(&config, &link).login_system, "MYPOINT");
        link.remote_system = "The Remote BBS".into();
        assert_eq!(poll_identity(&config, &link).remote_system, "The Remote BBS");
        for seconds in [1, 30, 60, 3600] {
            link.timeout_secs = seconds;
            let limits = poll_limits(&link);
            assert_eq!(limits.timeout, Duration::from_secs(u64::from(seconds)));
            assert_eq!(limits.transfer_timeout, Limits::default().transfer_timeout);
            assert_eq!(limits.retries, Limits::default().retries);
        }
    }

    #[tokio::test]
    async fn empty_poll_completes_logoff_without_delete() {
        let temp = tempfile::tempdir().unwrap();
        let mut bytes = negotiation();
        no_transfer_cycle(&mut bytes, ZConnectCommandBlock::default().put(0));
        no_transfer_cycle(&mut bytes, ZConnectCommandBlock::default().logoff());
        let sent = Arc::new(Mutex::new(Vec::new()));
        let peer = Peer {
            bytes: bytes.into(),
            sent: sent.clone(),
        };
        let report = poll_connection(
            peer,
            &ZconnectConfig::default(),
            temp.path(),
            "peer",
            Login::Direct,
            &identity(),
            Limits::default(),
            temp.path().join("mail.zip"),
            None,
            temp.path().join("inbound"),
        )
        .await
        .unwrap();
        assert!(!report.uploaded);
        assert!(report.downloaded.is_empty());
        let sent = sent.lock().unwrap();
        let text = String::from_utf8_lossy(&sent);
        assert!(!text.contains("Delete:"));
        assert!(text.contains("Logoff"));
    }

    #[tokio::test]
    async fn disconnect_before_upload_receipt_keeps_pending_archive() {
        let temp = tempfile::tempdir().unwrap();
        let outbound = temp.path().join("mail.zip");
        fs::write(&outbound, b"pending archive").unwrap();
        let sent = Arc::new(Mutex::new(Vec::new()));
        let peer = Peer {
            bytes: negotiation().into(),
            sent: sent.clone(),
        };
        let result = poll_connection(
            peer,
            &ZconnectConfig::default(),
            temp.path(),
            "peer",
            Login::Direct,
            &identity(),
            Limits::default(),
            outbound.clone(),
            Some(15),
            temp.path().join("inbound"),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(fs::read(outbound).unwrap(), b"pending archive");
        assert!(!String::from_utf8_lossy(&sent.lock().unwrap()).contains("Delete:"));
    }
}
