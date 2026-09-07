use std::io::Read;

use codepages::tables::CP437_TO_UNICODE;
use tempfile::TempPath;
use tokio::time::{Duration, Instant, timeout, timeout_at};

use crate::icy_board::state::{
    local_transfer::{LocalFilePickerKind, stage_local_upload},
    user_commands::pcb::u_upload_file::create_protocol,
};

use super::{EditState, IceText, IcyBoardState, Res, display_flags};

impl EditState {
    // Bound memory independently of configurable message limits. The protocol
    // owns an unnamed, random temporary file until it reports completion.
    pub(super) const MAX_UPLOAD_BYTES: usize = 1024 * 1024;

    pub(super) fn append_uploaded_text(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() > Self::MAX_UPLOAD_BYTES || self.max_line_length == 0 {
            return false;
        }
        let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
        let text = match std::str::from_utf8(bytes) {
            Ok(text) => text.to_string(),
            Err(_) => bytes.iter().map(|byte| CP437_TO_UNICODE[*byte as usize]).collect(),
        };
        let text = text.trim_end_matches('\x1a').replace("\r\n", "\n").replace('\r', "\n");
        let mut lines = Vec::new();
        for raw in text.lines() {
            let mut clean = String::new();
            let mut column = 0;
            for ch in raw.chars() {
                if ch == '\t' {
                    let count = 8 - column % 8;
                    clean.push_str(&" ".repeat(count));
                    column += count;
                } else if !ch.is_control() {
                    clean.push(ch);
                    column += 1;
                }
            }
            loop {
                let (line, rest) = Self::wrap_once(&clean, self.max_line_length);
                lines.push(line.trim_end().to_string());
                if self.msg.len().saturating_add(lines.len()) > self.max_lines {
                    return false;
                }
                if rest.is_empty() {
                    break;
                }
                clean = rest;
            }
        }
        if lines.is_empty() {
            return false;
        }
        self.msg.extend(lines);
        self.cursor = (0, self.msg.len()).into();
        true
    }

    pub(super) async fn upload_text(&mut self, state: &mut IcyBoardState) -> Res<()> {
        if self.msg.len() >= self.max_lines {
            state.display_text(IceText::TextEntryFull, display_flags::NEWLINE).await?;
            return Ok(());
        }
        if let Some(window) = state.event_window().await
            && window.uploads_blocked(&chrono::Local::now())
        {
            state.display_text(IceText::UploadsDisabled, display_flags::NEWLINE).await?;
            return Ok(());
        }
        state.display_text(IceText::UploadMode, display_flags::NEWLINE).await?;
        if state.session.is_local {
            let Some(source) = state.request_local_path(LocalFilePickerKind::UploadFile).await? else {
                return Ok(());
            };
            if state.session.request_logoff {
                return Ok(());
            }
            let result = tokio::task::spawn_blocking(move || -> Res<Vec<u8>> {
                // Reject known oversized/special sources before copying. Only
                // the bridge's temporary copy is ever owned by this command.
                let metadata = std::fs::symlink_metadata(&source)?;
                if !metadata.file_type().is_file() || metadata.len() > Self::MAX_UPLOAD_BYTES as u64 {
                    return Err(std::io::Error::other("Invalid text upload file").into());
                }
                let staged = stage_local_upload(&source)?;
                let mut bytes = Vec::new();
                std::fs::File::open(&staged)?.take(Self::MAX_UPLOAD_BYTES as u64 + 1).read_to_end(&mut bytes)?;
                Ok(bytes)
            })
            .await?;
            let accepted = match result {
                Ok(bytes) => !state.session.request_logoff && self.append_uploaded_text(&bytes),
                Err(error) => {
                    log::warn!("Local message text upload rejected: {error}");
                    false
                }
            };
            state
                .display_text(
                    if accepted { IceText::TransferSuccessful } else { IceText::TransferAborted },
                    display_flags::NEWLINE,
                )
                .await?;
            return Ok(());
        }
        let answer = state.ask_transfer_protocol("N").await?;
        if answer.is_empty() || answer.eq_ignore_ascii_case("N") || state.session.request_logoff {
            return Ok(());
        }
        let protocol = state
            .get_board()
            .await
            .protocols
            .iter()
            .find(|p| p.is_enabled && p.char_code.eq_ignore_ascii_case(&answer))
            .and_then(|p| create_protocol(&p.recv_command));
        let Some(mut protocol) = protocol else {
            // ASCII/external protocol placeholders cannot receive framed data.
            state.display_text(IceText::TransferAborted, display_flags::NEWLINE).await?;
            return Ok(());
        };
        let deadline = Instant::now() + Duration::from_secs(120);
        let result = timeout_at(deadline, protocol.initiate_recv(&mut *state.connection)).await;
        let mut transfer = match result {
            Ok(Ok(transfer)) => transfer,
            _ => {
                let _ = timeout(Duration::from_secs(2), protocol.cancel_transfer(&mut *state.connection)).await;
                state.display_text(IceText::TransferAborted, display_flags::NEWLINE).await?;
                return Ok(());
            }
        };
        // Adopt all completed paths immediately. TempPath deletes them on every
        // exit path, including cancellation of this async editor future. Remote
        // filenames are never used as filesystem paths.
        let mut received = Vec::new();
        let mut failed = false;
        loop {
            for (_, path) in transfer.recieve_state.finished_files.drain(..) {
                match TempPath::try_from_path(&path) {
                    Ok(path) => received.push(path),
                    Err(_) => {
                        let _ = std::fs::remove_file(path);
                        failed = true;
                    }
                }
            }
            let info = &transfer.recieve_state;
            if info.total_bytes_transfered > Self::MAX_UPLOAD_BYTES as u64
                || info.cur_bytes_transfered > Self::MAX_UPLOAD_BYTES as u64
                || info.file_size > Self::MAX_UPLOAD_BYTES as u64
                || received.len() > 1
                || transfer.request_cancel
                || state.session.request_logoff
            {
                failed = true;
            }
            if failed || transfer.is_finished {
                break;
            }
            state.check_time_left().await;
            if !matches!(
                timeout_at(deadline, protocol.update_transfer(&mut *state.connection, &mut transfer)).await,
                Ok(Ok(()))
            ) {
                failed = true;
            }
        }
        if failed {
            let _ = timeout(Duration::from_secs(2), protocol.cancel_transfer(&mut *state.connection)).await;
        }
        let mut accepted = false;
        if !failed && received.len() == 1 && !state.session.request_logoff {
            let mut bytes = Vec::new();
            if let Ok(file) = std::fs::File::open(&received[0])
                && file.take(Self::MAX_UPLOAD_BYTES as u64 + 1).read_to_end(&mut bytes).is_ok()
            {
                accepted = self.append_uploaded_text(&bytes);
            }
        }
        state
            .display_text(
                if accepted { IceText::TransferSuccessful } else { IceText::TransferAborted },
                display_flags::NEWLINE,
            )
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icy_board::{
        IcyBoard,
        bbs::BBS,
        state::{KeyChar, KeySource},
        user_base::User,
        xfer_protocols::SupportedProtocols,
    };
    use icy_net::{
        ConnectionType,
        channel::ChannelConnection,
        protocol::{Header, Protocol, ZFrameType, Zmodem},
    };
    use std::sync::Arc;
    use tokio::sync::Mutex;

    async fn local_state() -> (IcyBoardState, ChannelConnection) {
        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (peer, connection) = ChannelConnection::create_pair();
        // No protocols configured: a local upload must not prompt for one.
        let mut state = IcyBoardState::new(bbs, Arc::new(Mutex::new(IcyBoard::new())), nodes, node, Box::new(connection)).await;
        state.session.is_local = true;
        state.session.is_sysop = false;
        state.session.current_user = Some(User::default());
        state.session.page_len = 0;
        (state, peer)
    }

    fn draft() -> EditState {
        EditState {
            msg: vec!["existing draft".into()],
            max_lines: 100,
            max_line_length: 79,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn local_text_picker_cancel_and_missing_capability_preserve_draft() {
        let (mut state, _peer) = local_state().await;
        let mut editor = draft();
        let cursor = editor.cursor;
        timeout(Duration::from_secs(3), editor.upload_text(&mut state)).await.unwrap().unwrap();
        assert_eq!(editor.msg, ["existing draft"]);
        assert_eq!(editor.cursor, cursor);

        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        state.node_state.lock().await[state.node].as_mut().unwrap().local_file_picker = Some(tx);
        for disconnect in [false, true] {
            let (result, ()) = timeout(Duration::from_secs(3), async {
                tokio::join!(editor.upload_text(&mut state), async {
                    let request = rx.recv().await.unwrap();
                    assert_eq!(request.kind, LocalFilePickerKind::UploadFile);
                    if !disconnect {
                        request.response.send(None).unwrap();
                    }
                })
            })
            .await
            .expect("local text cancellation must not enter protocol receive");
            result.unwrap();
            assert_eq!(editor.msg, ["existing draft"]);
            assert_eq!(editor.cursor, cursor);
        }
    }

    #[tokio::test]
    async fn local_text_picker_preserves_original_and_enforces_byte_and_line_limits() {
        for case in ["valid", "oversize", "line-limit", "directory", "missing"] {
            let root = tempfile::tempdir().unwrap();
            let source = root.path().join("body.txt");
            let bytes = if case == "oversize" {
                vec![b'x'; EditState::MAX_UPLOAD_BYTES + 1]
            } else {
                b"caf\x82\r\nsecond\tline\r\n\x1a".to_vec()
            };
            std::fs::write(&source, &bytes).unwrap();
            let selected = match case {
                "directory" => root.path().to_path_buf(),
                "missing" => root.path().join("missing.txt"),
                _ => source.clone(),
            };
            let (mut state, _peer) = local_state().await;
            let (tx, mut rx) = tokio::sync::mpsc::channel(1);
            state.node_state.lock().await[state.node].as_mut().unwrap().local_file_picker = Some(tx);
            let mut editor = draft();
            if case == "line-limit" {
                editor.max_lines = 2;
            }
            let cursor = editor.cursor;
            let (result, ()) = timeout(Duration::from_secs(5), async {
                tokio::join!(editor.upload_text(&mut state), async {
                    let request = rx.recv().await.unwrap();
                    assert_eq!(request.kind, LocalFilePickerKind::UploadFile);
                    request.response.send(Some(selected)).unwrap();
                })
            })
            .await
            .expect("local text selection must not enter protocol receive");
            result.unwrap();
            if case == "valid" {
                assert_eq!(editor.msg, ["existing draft", "café", "second  line"]);
                assert_eq!(editor.cursor, icy_engine::Position::new(0, 3));
            } else {
                assert_eq!(editor.msg, ["existing draft"], "{case}");
                assert_eq!(editor.cursor, cursor, "{case}");
            }
            assert_eq!(std::fs::read(&source).unwrap(), bytes, "{case}");
            assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
        }
    }

    #[tokio::test]
    async fn native_zmodem_body_upload_appends_received_text_and_preserves_cancelled_draft() {
        for cancel in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let source = root.path().join("body.txt");
            let bytes = b"caf\x82\r\nsecond\tline\r\n\x1a";
            std::fs::write(&source, bytes).unwrap();
            let bbs = Arc::new(Mutex::new(BBS::new(1)));
            let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
            let nodes = bbs.lock().await.open_connections.clone();
            let (mut peer, connection) = ChannelConnection::create_pair();
            let mut board = IcyBoard::new();
            // Bare boards have no configured protocols, including Zmodem.
            board.protocols = SupportedProtocols::generate_pcboard_defaults();
            // Deliberately asymmetric: uploads must use RECV, never SEND.
            for protocol in board.protocols.iter_mut() {
                protocol.send_command = icy_net::protocol::TransferProtocolType::None;
            }
            let mut state = IcyBoardState::new(bbs, Arc::new(Mutex::new(board)), nodes, node, Box::new(connection)).await;
            let (tx, mut rx) = tokio::sync::mpsc::channel(1);
            state.node_state.lock().await[state.node].as_mut().unwrap().local_file_picker = Some(tx);
            state.session.current_user = Some(User::default());
            state.session.page_len = 0;
            state.char_buffer.extend("Z\r".chars().map(|ch| KeyChar::new(KeySource::User, ch)));
            let mut editor = EditState {
                msg: vec!["existing draft".into()],
                max_lines: 100,
                max_line_length: 79,
                ..Default::default()
            };
            let original_cursor = editor.cursor;

            let (result, sent) = timeout(Duration::from_secs(10), async {
                tokio::join!(editor.upload_text(&mut state), async {
                    // Discard only the terminal preamble and first ZRINIT;
                    // the native sender requests its own fresh handshake.
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
            .expect("native body upload stalled");
            result.unwrap();
            assert!(rx.try_recv().is_err(), "remote sessions must never request a host picker");
            if cancel {
                assert_eq!(editor.msg, ["existing draft"]);
                assert_eq!(editor.cursor, original_cursor);
            } else {
                assert!(sent.is_finished);
                assert_eq!(sent.send_state.finished_files.len(), 1);
                assert_eq!(sent.send_state.finished_files[0].1, source);
                assert_eq!(sent.send_state.total_bytes_transfered, bytes.len() as u64);
                assert_eq!(editor.msg, ["existing draft", "café", "second  line"]);
                assert_eq!(editor.cursor, icy_engine::Position::new(0, 3));
            }
            assert_eq!(std::fs::read(&source).unwrap(), bytes);
            assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
        }
    }
}
