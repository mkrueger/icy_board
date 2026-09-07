//! Native, single-file message uploads. Remote names are display metadata only.
use std::{
    collections::HashSet,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use bstr::BString;
use icy_net::protocol::TransferState;
use jamjam::jam::{
    JamMessage, JamMessageBase, attributes,
    msg_header::{MessageSubfield, SubfieldType},
};
use tempfile::{NamedTempFile, TempPath};
use tokio::time::{Duration, Instant, timeout, timeout_at};

use super::u_upload_file::create_protocol;
use crate::{
    Res,
    icy_board::{
        icb_text::IceText,
        state::{IcyBoardState, Session, functions::display_flags},
    },
    vm::TerminalTarget,
};

const MAX_ATTACHMENT_BYTES: u64 = 16 * 1024 * 1024;
const STORED_PREFIX: &str = "icb-attach-";

fn safe_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 255 && name != "." && name != ".." && !name.chars().any(|ch| ch.is_control() || "/\\:*?[]".contains(ch))
}

fn may_attach(session: &Session) -> bool {
    let conference = &session.current_conference;
    !conference.attachment_location.as_os_str().is_empty()
        && conference.sec_attachments.session_can_access(session)
        && conference.areas.as_ref().is_none_or(|areas| {
            areas
                .get(session.current_message_area)
                .is_some_and(|area| area.req_level_to_save_attach.session_can_access(session))
        })
}

fn attachment_names(message: &JamMessage) -> HashSet<String> {
    message
        .header()
        .sub_fields
        .iter()
        .filter(|field| matches!(field.field_type(), SubfieldType::EnclFile | SubfieldType::EnclFwAlias))
        .filter_map(|field| std::str::from_utf8(field.content()).ok())
        .filter_map(|value| value.split('\0').next())
        .map(str::to_owned)
        .collect()
}

/// A compose/replace caller must create this BEFORE editing, call `track`
/// immediately after editing (before another await), and `commit` immediately
/// after the first successful JAM write/replacement. Never owns old enclosures:
/// copies and edited messages can share those with other stored messages.
pub(crate) struct MessageAttachmentCleanup {
    root: PathBuf,
    original: HashSet<String>,
    files: Vec<TempPath>,
}

impl MessageAttachmentCleanup {
    pub(crate) fn track(&mut self, message: &JamMessage) -> Res<()> {
        for name in attachment_names(message).difference(&self.original) {
            if !safe_name(name) || !name.starts_with(STORED_PREFIX) {
                continue;
            }
            let path = self.root.join(name);
            if self.files.iter().any(|file| file.to_path_buf() == path) {
                continue;
            }
            self.files.push(TempPath::try_from_path(path)?);
        }
        Ok(())
    }

    pub(crate) fn commit(&mut self) {
        for mut file in self.files.drain(..) {
            // The reference is already durable. Never unlink on notification,
            // statistics, a later carbon-copy failure, or disconnect.
            if let Some(name) = file.file_name().and_then(|name| name.to_str()) {
                self.original.insert(name.to_owned());
            }
            file.disable_cleanup(true);
        }
    }
}

/// The native receiver owns in-progress NamedTempFiles. Completed files are
/// kept in TransferState, including while update_transfer is suspended; this
/// wrapper reclaims those too if the entire async future is dropped.
struct ReceiveGuard(TransferState);

impl Drop for ReceiveGuard {
    fn drop(&mut self) {
        for (_, path) in self.0.recieve_state.finished_files.drain(..) {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn transfer_rejected(transfer: &TransferState, count: usize, logoff: bool) -> bool {
    let info = &transfer.recieve_state;
    logoff
        || transfer.request_cancel
        || count > 1
        || info.file_size > MAX_ATTACHMENT_BYTES
        || info.cur_bytes_transfered > MAX_ATTACHMENT_BYTES
        || info.total_bytes_transfered > MAX_ATTACHMENT_BYTES
}

/// Copy bounded contents into our configured directory. The source must be a
/// native receive temporary, never a caller-supplied filesystem path. Random
/// names and persist_noclobber prevent overwriting another message's enclosure.
fn stage_attachment(root: &Path, source: &Path, display: &str) -> Res<(TempPath, MessageSubfield)> {
    if !safe_name(display) || root.as_os_str().is_empty() {
        return Err(std::io::Error::other("Invalid attachment filename").into());
    }
    let metadata = std::fs::symlink_metadata(source)?;
    if !metadata.file_type().is_file() || metadata.len() == 0 || metadata.len() > MAX_ATTACHMENT_BYTES {
        return Err(std::io::Error::other("Invalid attachment receive file").into());
    }
    let root = root.canonicalize()?;
    let mut staged = NamedTempFile::new_in(&root)?;
    let count = std::io::copy(&mut File::open(source)?.take(MAX_ATTACHMENT_BYTES + 1), &mut staged)?;
    if count == 0 || count > MAX_ATTACHMENT_BYTES || count != metadata.len() {
        return Err(std::io::Error::other("Attachment size changed during receipt").into());
    }
    staged.as_file().sync_all()?;
    let stored = format!("{STORED_PREFIX}{:032x}.bin", fastrand::u128(..));
    // Construct RAII ownership before publication, without any intervening await.
    let path = root.join(&stored);
    staged.persist_noclobber(&path)?;
    let owned = TempPath::try_from_path(path)?;
    File::open(&root)?.sync_all()?;
    // EnclFwAlias is jamjam's spelling of the JAM file+display-alias field.
    // Do not add a second EnclFile record: the reader would offer it twice.
    let field = MessageSubfield::new(SubfieldType::EnclFwAlias, BString::from(format!("{stored}\0{display}")));
    Ok((owned, field))
}

impl IcyBoardState {
    pub(crate) fn message_attachment_cleanup(&self, message: &JamMessage) -> MessageAttachmentCleanup {
        MessageAttachmentCleanup {
            root: self.resolve_path(&self.session.current_conference.attachment_location),
            original: attachment_names(message),
            files: Vec::new(),
        }
    }

    /// False means deny/cancel/invalid transfer; the caller must resume its draft.
    /// Successful callers own persistence via `message_attachment_cleanup`.
    pub(crate) async fn attach_message_file(&mut self, message: &mut JamMessage) -> Res<bool> {
        if !may_attach(&self.session) {
            self.display_text(IceText::AttachNotAllOWed, display_flags::NEWLINE).await?;
            return Ok(false);
        }
        if self.session.request_logoff {
            return Ok(false);
        }
        if let Some(window) = self.event_window().await
            && window.uploads_blocked(&chrono::Local::now())
        {
            self.display_text(IceText::UploadsDisabled, display_flags::NEWLINE).await?;
            return Ok(false);
        }
        let root = self.resolve_path(&self.session.current_conference.attachment_location);
        let outcome: Res<Option<(TempPath, MessageSubfield)>> = async {
            // Bad configuration is reported before putting the terminal in receive mode.
            if !root.is_dir() {
                return Err(std::io::Error::other("Attachment directory unavailable").into());
            }
            self.display_text(IceText::UploadMode, display_flags::NEWLINE).await?;
            let answer = self.ask_transfer_protocol("N").await?;
            if answer.is_empty() || answer.eq_ignore_ascii_case("N") || self.session.request_logoff {
                return Ok(None);
            }
            let protocol = self
                .get_board()
                .await
                .protocols
                .iter()
                .find(|p| p.is_enabled && p.char_code.eq_ignore_ascii_case(&answer))
                .and_then(|p| create_protocol(&p.recv_command));
            let Some(mut protocol) = protocol else { return Ok(None) };
            let deadline = Instant::now() + Duration::from_secs(120);
            let mut transfer = match timeout_at(deadline, protocol.initiate_recv(&mut *self.connection)).await {
                Ok(Ok(state)) => ReceiveGuard(state),
                _ => {
                    let _ = timeout(Duration::from_secs(2), protocol.cancel_transfer(&mut *self.connection)).await;
                    return Ok(None);
                }
            };
            let mut received = Vec::new();
            let mut failed = false;
            loop {
                // No await between draining native completed paths and adoption.
                for (name, path) in transfer.0.recieve_state.finished_files.drain(..) {
                    match TempPath::try_from_path(&path) {
                        Ok(path) => received.push((name, path)),
                        Err(_) => {
                            let _ = std::fs::remove_file(path);
                            failed = true;
                        }
                    }
                }
                failed |= transfer_rejected(&transfer.0, received.len(), self.session.request_logoff);
                if failed || transfer.0.is_finished {
                    break;
                }
                self.check_time_left().await;
                if !matches!(
                    timeout_at(deadline, protocol.update_transfer(&mut *self.connection, &mut transfer.0)).await,
                    Ok(Ok(()))
                ) {
                    failed = true;
                }
            }
            if failed {
                let _ = timeout(Duration::from_secs(2), protocol.cancel_transfer(&mut *self.connection)).await;
                return Ok(None);
            }
            if received.len() != 1 {
                return Ok(None);
            }
            let (name, source) = &received[0];
            // XMODEM has no filename header. Never interpret its empty name as a path.
            let display = if name.is_empty() { "attachment.bin" } else { name.as_str() };
            stage_attachment(&root, source, display).map(Some)
        }
        .await;
        let (mut owned, field) = match outcome {
            Ok(Some(attachment)) => attachment,
            outcome => {
                if let Err(error) = outcome {
                    log::warn!("Message attachment rejected: {error}");
                }
                self.display_text(IceText::TransferAborted, display_flags::NEWLINE).await?;
                return Ok(false);
            }
        };
        // Until this notification completes, dropping this future deletes the
        // durable staging file and leaves the draft entirely unchanged.
        self.display_text(IceText::TransferSuccessful, display_flags::NEWLINE).await?;
        if self.session.request_logoff {
            return Ok(false);
        }
        let mut header = message.header().clone();
        header.sub_fields.push(field);
        header.attributes |= attributes::MSG_FILEATTACH;
        *message = JamMessage::from_stored(header, message.text().clone());
        owned.disable_cleanup(true);
        Ok(true)
    }

    /// Only the attachment path needs this adapter. send_message currently
    /// swallows base-open failures and can fail AFTER a successful JAM append.
    /// Commit cleanup at the actual append, not at its later UI/statistics result.
    pub(crate) async fn send_message_with_attachment_cleanup(
        &mut self,
        conf: i32,
        area: i32,
        message: JamMessage,
        text: IceText,
        cleanup: &mut MessageAttachmentCleanup,
    ) -> Res<()> {
        if cleanup.files.is_empty() {
            return self.send_message(conf, area, message, text).await;
        }
        let mut base = if conf < 0 {
            let to = message.to().map(ToString::to_string).unwrap_or_default();
            self.get_email_msgbase(&to).await?
        } else {
            let board = self.get_board().await;
            let target = board
                .conferences
                .get(conf as usize)
                .ok_or_else(|| std::io::Error::other("Invalid attachment conference"))?;
            let target_area = target
                .areas
                .as_ref()
                .and_then(|areas| areas.get(area as usize))
                .ok_or_else(|| std::io::Error::other("Invalid attachment message area"))?;
            if target.attachment_location.as_os_str().is_empty()
                || !target.sec_attachments.session_can_access(&self.session)
                || !target_area.req_level_to_save_attach.session_can_access(&self.session)
                || self.resolve_path(&target.attachment_location).canonicalize()? != cleanup.root.canonicalize()?
            {
                return Err(std::io::Error::other("Attachment destination does not match its conference").into());
            }
            let path = target_area.path.clone();
            drop(board);
            if path.with_extension("jhr").exists() {
                JamMessageBase::open(path)?
            } else {
                JamMessageBase::create(path)?
            }
        };
        let number = base.write_message(&message)?;
        cleanup.commit();
        if let Some(user) = &mut self.session.current_user {
            user.stats.messages_left += 1;
        }
        self.get_board().await.statistics.add_message();
        self.get_board().await.save_statistics()?;
        self.display_text(text, display_flags::DEFAULT).await?;
        self.println(TerminalTarget::Both, &number.to_string()).await?;
        self.new_line().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icy_board::{
        message_area::{AreaList, MessageArea},
        security_expr::SecurityExpression,
    };
    use std::{io::Write, sync::Arc};

    fn upload() -> NamedTempFile {
        let mut source = NamedTempFile::new().unwrap();
        source.write_all(b"attachment contents").unwrap();
        source
    }

    fn cleanup(root: &Path, original: &JamMessage) -> MessageAttachmentCleanup {
        MessageAttachmentCleanup {
            root: root.to_path_buf(),
            original: attachment_names(original),
            files: Vec::new(),
        }
    }

    fn staged_message(root: &Path) -> (PathBuf, JamMessage) {
        let source = upload();
        let (mut owned, field) = stage_attachment(root, source.path(), "report.zip").unwrap();
        let path = owned.to_path_buf();
        owned.disable_cleanup(true);
        (path, JamMessage::default().with_sub_field(field).with_text(BString::from("draft body")))
    }

    async fn input_state(root: &Path, input: &str) -> (IcyBoardState, icy_net::channel::ChannelConnection) {
        use crate::icy_board::{
            IcyBoard,
            bbs::BBS,
            conferences::Conference,
            state::{KeyChar, KeySource},
            user_base::{FSEMode, User},
        };
        use icy_net::{ConnectionType, channel::ChannelConnection};
        use tokio::sync::Mutex;

        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (peer, connection) = ChannelConnection::create_pair();
        let mut board = IcyBoard::new();
        board.config.paths.statistics_file = root.join("statistics.toml");
        board.users.new_user(User {
            name: "ATTACHMENT TEST".into(),
            security_level: 255,
            ..Default::default()
        });
        let caller = board.users[0].clone();
        board.conferences.clear();
        board.conferences.push(Conference {
            attachment_location: root.to_path_buf(),
            areas: Some(Arc::new(AreaList::new(vec![MessageArea {
                path: root.join("messages"),
                ..Default::default()
            }]))),
            ..Default::default()
        });
        let conference = board.conferences[0].clone();
        let mut state = IcyBoardState::new(bbs, Arc::new(Mutex::new(board)), nodes, node, Box::new(connection)).await;
        state.session.current_user = Some(caller);
        state.session.cur_user_id = 0;
        state.session.cur_security = 255;
        state.session.page_len = 0;
        state.session.fse_mode = FSEMode::No;
        state.session.current_conference = conference;
        state.char_buffer.extend(input.chars().map(|ch| KeyChar::new(KeySource::User, ch)));
        (state, peer)
    }

    #[tokio::test]
    async fn native_zmodem_attachment_transfers_bytes_rejects_names_and_cleans_cancelled_upload() {
        use crate::icy_board::xfer_protocols::SupportedProtocols;
        use icy_net::protocol::{Header, Protocol, ZFrameType, Zmodem};

        // These are real Linux filenames: the native sender must carry the
        // unsafe display names over the wire, not have the test call safe_name.
        for (name, cancel, accepted) in [
            ("report.bin", false, true),
            ("bad\\name.bin", false, false),
            ("bad:name.bin", false, false),
            ("cancel.bin", true, false),
        ] {
            let root = tempfile::tempdir().unwrap();
            let sources = tempfile::tempdir().unwrap();
            let source = sources.path().join(name);
            let bytes: Vec<u8> = (0..4096).map(|n| (n % 256) as u8).collect();
            std::fs::write(&source, &bytes).unwrap();
            let (mut state, mut peer) = input_state(root.path(), "Z\r").await;
            // Bare boards have no configured protocols, including Zmodem.
            state.get_board().await.protocols = SupportedProtocols::generate_pcboard_defaults();
            let mut draft = JamMessage::default().with_subject("original subject".into()).with_text("draft body".into());
            let mut cleanup = state.message_attachment_cleanup(&draft);

            let (result, sent) = timeout(Duration::from_secs(10), async {
                tokio::join!(state.attach_message_file(&mut draft), async {
                    // Consume the terminal menu and initial ZRINIT before
                    // starting sz: its strict parser does not accept UI text.
                    // Native sz's ZRQINIT obtains a fresh native ZRINIT.
                    let mut can_count = 0;
                    loop {
                        if let Ok(Some(header)) = Header::read(&mut peer, &mut can_count).await
                            && header.frame_type == ZFrameType::RIinit
                        {
                            break;
                        }
                    }
                    let mut protocol = Zmodem::new(1024);
                    let mut sent = protocol.initiate_send(&mut peer, std::slice::from_ref(&source)).await.unwrap();
                    while !sent.is_finished {
                        protocol.update_transfer(&mut peer, &mut sent).await.unwrap();
                        if cancel && !sent.send_state.file_name.is_empty() {
                            // ZFILE/ZRPOS has opened a receive temporary, but
                            // no ZEOF has published a completed attachment.
                            assert!(sent.send_state.finished_files.is_empty());
                            protocol.cancel_transfer(&mut peer).await.unwrap();
                            return sent;
                        }
                        tokio::task::yield_now().await;
                    }
                    sent
                })
            })
            .await
            .expect("native attachment transfer stalled");
            assert_eq!(result.unwrap(), accepted, "{name}");
            assert_eq!(draft.text(), &BString::from("draft body"));
            assert_eq!(draft.header().subject().unwrap().to_string(), "original subject");
            assert_eq!(std::fs::read(&source).unwrap(), bytes, "sender source must be untouched");
            if !cancel {
                assert!(sent.is_finished);
                assert_eq!(sent.send_state.finished_files.len(), 1);
                assert_eq!(sent.send_state.finished_files[0].1, source);
                assert_eq!(sent.send_state.total_bytes_transfered, bytes.len() as u64);
            }
            if accepted {
                assert_ne!(draft.header().attributes & attributes::MSG_FILEATTACH, 0);
                let fields: Vec<_> = draft
                    .header()
                    .sub_fields
                    .iter()
                    .filter(|field| field.field_type() == SubfieldType::EnclFwAlias)
                    .collect();
                assert_eq!(fields.len(), 1);
                let parts: Vec<_> = std::str::from_utf8(fields[0].content()).unwrap().split('\0').collect();
                assert_eq!(parts.len(), 2);
                assert_eq!(parts[1], name);
                assert!(parts[0].starts_with(STORED_PREFIX));
                assert_ne!(parts[0], name);
                let stored = root.path().join(parts[0]);
                assert_eq!(stored.parent(), Some(root.path()));
                assert_eq!(std::fs::read(&stored).unwrap(), bytes);
                assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
                cleanup.track(&draft).unwrap();
                drop(cleanup);
                assert!(!stored.exists(), "abandoning the draft must reclaim its real upload");
            } else {
                assert!(attachment_names(&draft).is_empty());
                assert_eq!(draft.header().attributes & attributes::MSG_FILEATTACH, 0);
            }
            assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0, "{name}");
        }
    }

    #[tokio::test]
    async fn denied_attachment_resumes_composition_without_losing_the_draft() {
        use crate::icy_board::state::user_commands::mods::editor::EditResult;
        let root = tempfile::tempdir().unwrap();
        let (mut state, _peer) = input_state(root.path(), "draft before\r\rSA\rcontinued\r\rSN\r").await;
        state.session.current_conference.attachment_location = PathBuf::new();
        let mut draft = JamMessage::default().with_subject(BString::from("original subject"));
        let result = timeout(Duration::from_secs(3), state.edit_message_context(&mut draft, Vec::new()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result, EditResult::SendNext);
        assert_eq!(draft.text(), &BString::from("draft before\ncontinued"));
        assert_eq!(draft.header().subject().unwrap().to_string(), "original subject");
        assert!(attachment_names(&draft).is_empty());
    }

    #[tokio::test]
    async fn cancelled_protocol_resumes_composition_without_losing_the_draft() {
        use crate::icy_board::state::user_commands::mods::editor::EditResult;
        let root = tempfile::tempdir().unwrap();
        let (mut state, _peer) = input_state(root.path(), "draft before\r\rSA\rN\rcontinued\r\rS\r").await;
        let mut draft = JamMessage::default();
        let result = timeout(Duration::from_secs(3), state.edit_message_context(&mut draft, Vec::new()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result, EditResult::SendMessage);
        assert_eq!(draft.text(), &BString::from("draft before\ncontinued"));
        assert!(attachment_names(&draft).is_empty());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn attachment_save_adapter_reports_base_open_failure_and_reclaims_upload() {
        let root = tempfile::tempdir().unwrap();
        let (mut state, _peer) = input_state(root.path(), "").await;
        std::fs::write(root.path().join("messages.jhr"), b"invalid JAM").unwrap();
        let (path, draft) = staged_message(root.path());
        let mut guard = state.message_attachment_cleanup(&JamMessage::default());
        guard.track(&draft).unwrap();
        assert!(
            state
                .send_message_with_attachment_cleanup(0, 0, draft, IceText::SavingComment, &mut guard)
                .await
                .is_err()
        );
        drop(guard);
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn attachment_save_adapter_preserves_file_when_statistics_cannot_be_saved() {
        let root = tempfile::tempdir().unwrap();
        let (mut state, _peer) = input_state(root.path(), "").await;
        // A directory cannot be replaced with the statistics file.
        state.get_board().await.config.paths.statistics_file = root.path().to_path_buf();
        let (path, draft) = staged_message(root.path());
        let mut guard = state.message_attachment_cleanup(&JamMessage::default());
        guard.track(&draft).unwrap();
        assert!(
            state
                .send_message_with_attachment_cleanup(0, 0, draft, IceText::SavingComment, &mut guard)
                .await
                .is_ok()
        );
        drop(guard);
        assert!(path.exists());
        let base = JamMessageBase::open(root.path().join("messages")).unwrap();
        assert!(attachment_names(&base.read_message(1).unwrap()).contains(path.file_name().unwrap().to_str().unwrap()));
    }

    #[test]
    fn conference_and_area_security_are_both_required() {
        let mut session = Session::default();
        assert!(!may_attach(&session));
        session.current_conference.attachment_location = PathBuf::from("attachments");
        session.current_conference.sec_attachments = SecurityExpression::from_req_security(40);
        session.cur_security = 39;
        assert!(!may_attach(&session));
        session.cur_security = 40;
        assert!(may_attach(&session));
        session.current_conference.areas = Some(Arc::new(AreaList::new(vec![MessageArea {
            req_level_to_save_attach: SecurityExpression::from_req_security(50),
            ..Default::default()
        }])));
        assert!(!may_attach(&session));
        session.cur_security = 50;
        assert!(may_attach(&session));
        session.current_message_area = 1;
        assert!(!may_attach(&session));
    }

    #[test]
    fn display_names_reject_paths_wildcards_controls_and_alias_injection() {
        for name in [
            "",
            ".",
            "..",
            "../secret",
            "/etc/passwd",
            "C:\\SECRET",
            "a/b",
            "a\\b",
            "*.zip",
            "a?b",
            "[ab]",
            "x:y",
            "a\nb",
            "a\0b",
            "a\u{1b}b",
        ] {
            assert!(!safe_name(name), "accepted {name:?}");
        }
        assert!(!safe_name(&"a".repeat(256)));
        for name in ["report.zip", "my report.txt", "résumé.txt", "attachment.bin"] {
            assert!(safe_name(name));
        }
        let root = tempfile::tempdir().unwrap();
        let source = upload();
        assert!(stage_attachment(root.path(), source.path(), "../escape").is_err());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn receive_limits_cover_advertised_current_total_batch_and_cancellation() {
        let mut state = TransferState::new("test".into());
        assert!(!transfer_rejected(&state, 1, false));
        assert!(transfer_rejected(&state, 2, false));
        assert!(transfer_rejected(&state, 1, true));
        state.request_cancel = true;
        assert!(transfer_rejected(&state, 1, false));
        state.request_cancel = false;
        state.recieve_state.file_size = MAX_ATTACHMENT_BYTES + 1;
        assert!(transfer_rejected(&state, 1, false));
        state.recieve_state.file_size = 0;
        state.recieve_state.cur_bytes_transfered = MAX_ATTACHMENT_BYTES + 1;
        assert!(transfer_rejected(&state, 1, false));
        state.recieve_state.cur_bytes_transfered = 0;
        state.recieve_state.total_bytes_transfered = MAX_ATTACHMENT_BYTES + 1;
        assert!(transfer_rejected(&state, 1, false));
    }

    #[test]
    fn empty_oversized_missing_and_directory_receive_files_are_rejected() {
        let root = tempfile::tempdir().unwrap();
        let source = NamedTempFile::new().unwrap();
        assert!(stage_attachment(root.path(), source.path(), "empty.bin").is_err());
        source.as_file().set_len(MAX_ATTACHMENT_BYTES + 1).unwrap();
        assert!(stage_attachment(root.path(), source.path(), "large.bin").is_err());
        assert!(stage_attachment(root.path(), &root.path().join("missing"), "missing.bin").is_err());
        assert!(stage_attachment(root.path(), root.path(), "directory.bin").is_err());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn receive_symlinks_are_rejected_without_touching_the_target() {
        let root = tempfile::tempdir().unwrap();
        let source = upload();
        let link = root.path().join("link");
        std::os::unix::fs::symlink(source.path(), &link).unwrap();
        assert!(stage_attachment(root.path(), &link, "innocent.zip").is_err());
        assert_eq!(std::fs::read(source.path()).unwrap(), b"attachment contents");
    }

    #[test]
    fn stored_names_are_random_and_reader_alias_encoding_roundtrips_in_jam() {
        let root = tempfile::tempdir().unwrap();
        let source = upload();
        std::fs::write(root.path().join("report.zip"), b"preexisting shared file").unwrap();
        let (first, field) = stage_attachment(root.path(), source.path(), "report.zip").unwrap();
        let (second, _) = stage_attachment(root.path(), source.path(), "report.zip").unwrap();
        assert_ne!(first.to_path_buf(), second.to_path_buf());
        assert_eq!(first.parent(), Some(root.path()));
        assert_eq!(field.field_type(), SubfieldType::EnclFwAlias);
        let encoded = std::str::from_utf8(field.content()).unwrap();
        let parts: Vec<_> = encoded.split('\0').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], first.file_name().unwrap().to_str().unwrap());
        assert!(safe_name(parts[0]));
        assert_eq!(parts[1], "report.zip");
        let message = JamMessage::default().with_sub_field(field).with_attributes(attributes::MSG_FILEATTACH);
        let mut base = JamMessageBase::create(root.path().join("messages")).unwrap();
        base.write_message(&message).unwrap();
        let stored = base.read_message(1).unwrap();
        assert_eq!(attachment_names(&stored), attachment_names(&message));
        assert_eq!(std::fs::read(root.path().join("report.zip")).unwrap(), b"preexisting shared file");
    }

    #[test]
    fn dropped_receive_state_reclaims_completed_native_temps() {
        let source = upload();
        let (_, path) = source.keep().unwrap();
        let mut receive = ReceiveGuard(TransferState::new("test".into()));
        receive.0.recieve_state.finished_files.push(("../../not-a-path".into(), path.clone()));
        drop(receive);
        assert!(!path.exists());
    }

    #[test]
    fn aborted_draft_cleanup_never_owns_a_preexisting_shared_enclosure() {
        let root = tempfile::tempdir().unwrap();
        let (shared, original) = staged_message(root.path());
        let (new_path, added) = staged_message(root.path());
        let mut header = original.header().clone();
        header.sub_fields.extend(added.header().sub_fields.clone());
        let draft = JamMessage::from_stored(header, original.text().clone());
        let mut guard = cleanup(root.path(), &original);
        guard.track(&draft).unwrap();
        guard.track(&draft).unwrap();
        assert_eq!(guard.files.len(), 1);
        drop(guard);
        assert!(shared.exists());
        assert!(!new_path.exists());
        assert_eq!(original.text(), draft.text());
    }

    #[test]
    fn failed_jam_append_removes_only_the_new_attachment() {
        let root = tempfile::tempdir().unwrap();
        let mut base = JamMessageBase::create(root.path().join("messages")).unwrap();
        // Force append to fail before JAM can publish a reference.
        std::fs::remove_file(root.path().join("messages.jdt")).unwrap();
        std::fs::create_dir(root.path().join("messages.jdt")).unwrap();
        let (path, draft) = staged_message(root.path());
        let mut guard = cleanup(root.path(), &JamMessage::default());
        guard.track(&draft).unwrap();
        assert!(base.write_message(&draft).is_err());
        drop(guard);
        assert!(!path.exists());
    }

    #[test]
    fn committed_attachment_survives_later_errors_and_retracking() {
        let root = tempfile::tempdir().unwrap();
        let (path, draft) = staged_message(root.path());
        let mut guard = cleanup(root.path(), &JamMessage::default());
        guard.track(&draft).unwrap();
        let mut base = JamMessageBase::create(root.path().join("messages")).unwrap();
        base.write_message(&draft).unwrap();
        guard.commit();
        // A later carbon-copy/statistics/terminal error must not break the first message.
        guard.track(&draft).unwrap();
        assert!(guard.files.is_empty());
        drop(guard);
        assert!(path.exists());
        assert_eq!(attachment_names(&base.read_message(1).unwrap()), attachment_names(&draft));
    }

    #[tokio::test]
    async fn cancelled_future_reclaims_durable_staging_and_preserves_shared_files() {
        let root = tempfile::tempdir().unwrap();
        let (shared, original) = staged_message(root.path());
        let (new_path, added) = staged_message(root.path());
        let mut guard = cleanup(root.path(), &original);
        guard.track(&added).unwrap();
        let task = tokio::spawn(async move {
            let _guard = guard;
            std::future::pending::<()>().await;
        });
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(!new_path.exists());
        assert!(shared.exists());
    }
}
