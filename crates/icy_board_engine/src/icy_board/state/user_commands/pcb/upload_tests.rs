//! TRANSFER.C successful/getnames/receive and the attempt08 prompt oracle.
use super::*;
use crate::icy_board::{
    IcyBoard,
    bbs::BBS,
    conferences::Conference,
    security_expr::SecurityExpression,
    state::{KeyChar, KeySource},
    user_base::User,
    xfer_protocols::SupportedProtocols,
};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::{Mutex, mpsc};

async fn fixture(input: &str) -> (TempDir, IcyBoardState, ChannelConnection) {
    let root = tempfile::tempdir().unwrap();
    for directory in ["public", "private"] {
        std::fs::create_dir(root.path().join(directory)).unwrap();
    }
    let conference = Conference {
        pub_upload_location: root.path().join("public"),
        private_upload_location: root.path().join("private"),
        pub_upload_metadata: root.path().join("public-meta"),
        private_upload_metadata: root.path().join("private-meta"),
        ..Default::default()
    };
    let mut board = IcyBoard::new();
    board.protocols = SupportedProtocols::generate_pcboard_defaults();
    board.config.file_transfer.promote_to_batch_transfers = false;
    board.config.file_transfer.disable_drive_size_check = true;
    board.config.upload_processing.publish_policy = UploadPublishPolicy::Immediate;
    board.config.upload_processing.notify_sysop = false;
    board.config.paths.statistics_file = root.path().join("statistics.toml");
    board.config.paths.transfer_log = root.path().join("transfer.log");
    let user = User {
        name: "UPLOAD TEST".into(),
        security_level: 255,
        protocol: "N".into(),
        ..Default::default()
    };
    board.users.new_user(user.clone());
    board.conferences.clear();
    board.conferences.push(conference.clone());
    let bbs = Arc::new(Mutex::new(BBS::new(1)));
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let (peer, connection) = ChannelConnection::create_pair();
    let mut state = IcyBoardState::new(bbs, Arc::new(Mutex::new(board)), nodes, node, Box::new(connection)).await;
    state.session.current_user = Some(user);
    state.session.cur_user_id = 0;
    state.session.cur_security = 255;
    state.session.user_name = "UPLOAD TEST".into();
    state.session.current_conference = conference;
    state.session.page_len = 0;
    state.session.bytes_remaining = -1;
    state.char_buffer.extend(input.chars().map(|ch| KeyChar::new(KeySource::User, ch)));
    (root, state, peer)
}

async fn output(peer: &mut ChannelConnection) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let count = peer.try_read(&mut buffer).await.unwrap();
        if count == 0 {
            return String::from_utf8_lossy(&bytes).into_owned();
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
}

fn request(name: &str, private: bool, description: &str) -> UploadRequest {
    UploadRequest {
        name: name.into(),
        private,
        description: vec![description.into()],
        local_source: None,
        local_cps: 0,
    }
}

fn completed(name: &str, bytes: &[u8]) -> CompletedUpload {
    let file = NamedTempFile::new().unwrap();
    std::fs::write(file.path(), bytes).unwrap();
    CompletedUpload {
        name: name.into(),
        source: file.into_temp_path(),
        cps: 1,
    }
}

async fn finish_bounded(state: &mut IcyBoardState, receipt: UploadReceipt, requests: &[UploadRequest], retain_requested_name: bool) {
    timeout(Duration::from_secs(3), state.finish_uploads(receipt, requests, retain_requested_name, "Z"))
        .await
        .expect("upload finalization stalled")
        .unwrap();
}

async fn assert_description(state: &mut IcyBoardState, root: &Path, directory: &str, name: &str, description: &str) {
    let location = root.join(directory);
    let base = state.get_filebase(&location, &root.join(format!("{directory}-meta"))).await.unwrap();
    let metadata = base.lock().await.read_metadata(&location.join(name)).unwrap();
    assert!(
        metadata
            .iter()
            .any(|m| m.metadata_type == MetadataType::FileID && m.data == description.as_bytes())
    );
    assert!(metadata.iter().any(|m| m.metadata_type == MetadataType::Uploader && m.data == b"UPLOAD TEST"));
}

#[tokio::test]
async fn explicit_batch_and_promoted_u_collect_numbered_names_before_protocol() {
    for explicit in [true, false] {
        let (_root, mut state, mut peer) = fixture("ONE.BIN\rfirst description\r\rTWO.BIN\r/second private description\r\r\rN\r").await;
        if !explicit {
            state.get_board().await.config.file_transfer.promote_to_batch_transfers = true;
            state.session.current_user.as_mut().unwrap().protocol = "Z".into();
            // Default Z is usable, so cancel at the batch-ready prompt instead.
            state.char_buffer.pop_back();
            state.char_buffer.pop_back();
            state.char_buffer.extend("A\r".chars().map(|ch| KeyChar::new(KeySource::User, ch)));
        }
        timeout(Duration::from_secs(3), state.upload_files(explicit)).await.unwrap().unwrap();
        let text = output(&mut peer).await;
        assert!(text.contains("(1)") && text.contains("(2)") && text.contains("(3)"), "{text}");
        assert!(text.contains("ONE.BIN") && text.contains("TWO.BIN"), "{text}");
        assert!(!text.contains("Transfer Successful"), "{text}");
        assert!(state.session.flagged_files.is_empty());
        assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_uploads, 0);
    }
}

#[tokio::test]
async fn no_batch_uploads_limits_names_but_not_batch_protocol_and_low_security_falls_back() {
    let (_root, mut state, mut peer) = fixture("ONE.BIN\rdescription one\r\rA\r").await;
    state.get_board().await.config.file_transfer.disallow_batch_uploads = true;
    state.session.current_user.as_mut().unwrap().protocol = "Z".into();
    timeout(Duration::from_secs(3), state.batch_upload_command()).await.unwrap().unwrap();
    let text = output(&mut peer).await;
    assert!(text.contains("(1)"), "{text}");
    assert!(!text.contains("(2)"), "{text}");
    assert!(text.contains("Goodbye") || text.contains("oodbye"), "{text}");

    let (_root, mut state, mut peer) = fixture("\r").await;
    state.session.cur_security = 10;
    state.session.user_command_level.batch_file_transfer = SecurityExpression::from_req_security(20);
    timeout(Duration::from_secs(3), state.batch_upload_command()).await.unwrap().unwrap();
    let text = output(&mut peer).await;
    assert!(text.contains("Filename to Upload"), "{text}");
    assert!(!text.contains("(1)"), "{text}");
}

#[tokio::test]
async fn stacked_protocol_and_names_are_not_consumed_as_descriptions() {
    let (_root, mut state, mut peer) = fixture("first description\r\rsecond description\r\r\rN\r").await;
    state.session.tokens.extend(["N", "ONE.BIN", "TWO.BIN"].map(str::to_string));
    timeout(Duration::from_secs(3), state.batch_upload_command()).await.unwrap().unwrap();
    let text = output(&mut peer).await;
    assert!(text.contains("ONE.BIN") && text.contains("TWO.BIN") && text.contains("(3)"), "{text}");
    assert!(!text.contains("description for N"), "{text}");
    assert!(state.session.tokens.is_empty());
}

#[tokio::test]
async fn normal_u_scans_all_stacked_names_without_promoting_or_an_extra_name_prompt() {
    let (_root, mut state, mut peer) = fixture("first description\r\r\\second description\r\rN\r").await;
    state.session.tokens.extend(["N", "ONE.BIN", "TWO.BIN"].map(str::to_string));
    timeout(Duration::from_secs(3), state.upload_file()).await.unwrap().unwrap();
    let text = output(&mut peer).await;
    assert!(text.contains("ONE.BIN") && text.contains("TWO.BIN"), "{text}");
    assert!(!text.contains("Filename to Upload") && !text.contains("(3)"), "{text}");
    assert!(state.session.tokens.is_empty() && state.char_buffer.is_empty());
    assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_uploads, 0);
}

#[tokio::test]
async fn each_upload_command_resets_transfer_totals_not_lifetime_or_download_totals() {
    let (_root, mut state, _peer) = fixture("\r").await;
    state.transfer_statistics.uploaded_files = 7;
    state.transfer_statistics.uploaded_bytes = 1234;
    state.transfer_statistics.uploaded_cps = 456;
    state.transfer_statistics.downloaded_files = 3;
    state.session.current_user.as_mut().unwrap().stats.num_uploads = 8;
    timeout(Duration::from_secs(3), state.upload_file()).await.unwrap().unwrap();
    assert_eq!(state.transfer_statistics.uploaded_files, 0);
    assert_eq!(state.transfer_statistics.uploaded_bytes, 0);
    assert_eq!(state.transfer_statistics.uploaded_cps, 0);
    assert_eq!(state.transfer_statistics.downloaded_files, 3);
    assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_uploads, 8);
}

#[tokio::test]
async fn none_default_does_not_promote_u_and_blank_protocol_aborts() {
    let (_root, mut state, mut peer) = fixture("ONE.BIN\rfirst description\r\r\r").await;
    state.get_board().await.config.file_transfer.promote_to_batch_transfers = true;
    assert!(!state.promotes_to_batch(false).await);
    timeout(Duration::from_secs(3), state.upload_file()).await.unwrap().unwrap();
    let text = output(&mut peer).await;
    assert!(!text.contains("(1)") && !text.contains("(2)"), "{text}");
    assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_uploads, 0);
}

#[tokio::test]
async fn local_upload_without_protocol_uses_one_picker_preserves_original_and_cancel_is_silent() {
    for cancel in [false, true] {
        let (root, mut state, mut peer) = fixture(if cancel { "" } else { "local description\r\r" }).await;
        let original = root.path().join("ORIGINAL.BIN");
        std::fs::write(&original, b"selected original").unwrap();
        state.session.is_local = true;
        let (tx, mut rx) = mpsc::channel(1);
        state.node_state.lock().await[state.node].as_mut().unwrap().local_file_picker = Some(tx);
        let (result, ()) = timeout(Duration::from_secs(3), async {
            tokio::join!(state.upload_file(), async {
                let request = rx.recv().await.unwrap();
                assert_eq!(request.kind, LocalFilePickerKind::UploadFile);
                request.response.send(if cancel { None } else { Some(original.clone()) }).unwrap();
            })
        })
        .await
        .unwrap();
        result.unwrap();
        assert!(rx.try_recv().is_err());
        assert_eq!(std::fs::read(&original).unwrap(), b"selected original");
        assert_eq!(root.path().join("public/ORIGINAL.BIN").exists(), !cancel);
        let text = output(&mut peer).await;
        assert!(!text.contains("Filename to Upload") && !text.contains("Protocol Type"), "{text}");
        assert!(!text.as_bytes().contains(&0x18));
        assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_uploads, u64::from(!cancel));
    }
}

#[tokio::test]
async fn receive_direction_none_and_local_native_guard() {
    let (_root, mut state, mut peer) = fixture("").await;
    state.get_board().await.protocols.iter_mut().find(|p| p.char_code == "Z").unwrap().send_command = TransferProtocolType::External("send-only".into());
    assert_eq!(state.get_upload_protocol("z", true).await, Some(TransferProtocolType::ZModem));
    assert_eq!(state.get_protocol("Z".into()).await, Some(TransferProtocolType::External("send-only".into())));
    assert!(state.get_upload_protocol("N", false).await.is_none());
    assert!(state.get_upload_protocol("X", true).await.is_none());
    state.session.is_local = true;
    let receipt = state.receive_uploads(&TransferProtocolType::ZModem, 1).await.unwrap();
    assert!(receipt.failed && receipt.files.is_empty());
    assert!(output(&mut peer).await.is_empty());
}

#[tokio::test]
async fn local_explicit_batch_picks_each_file_once_with_independent_routing() {
    let (root, mut state, _peer) = fixture("first local description\r\r/second local description\r\r").await;
    state.session.is_local = true;
    let paths: Vec<_> = ["FIRST.BIN", "SECOND.BIN"]
        .iter()
        .map(|name| {
            let path = root.path().join(name);
            std::fs::write(&path, name.as_bytes()).unwrap();
            path
        })
        .collect();
    let (tx, mut rx) = mpsc::channel(1);
    state.node_state.lock().await[state.node].as_mut().unwrap().local_file_picker = Some(tx);
    let (result, ()) = timeout(Duration::from_secs(3), async {
        tokio::join!(state.batch_upload_command(), async {
            for path in &paths {
                let request = rx.recv().await.unwrap();
                assert_eq!(request.kind, LocalFilePickerKind::UploadFile);
                request.response.send(Some(path.clone())).unwrap();
            }
            rx.recv().await.unwrap().response.send(None).unwrap();
        })
    })
    .await
    .unwrap();
    result.unwrap();
    assert!(rx.try_recv().is_err());
    assert!(root.path().join("public/FIRST.BIN").exists());
    assert!(root.path().join("private/SECOND.BIN").exists());
    assert!(paths.iter().all(|p| p.exists()));
    assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_uploads, 2);
}

#[tokio::test]
async fn independent_descriptions_private_routing_requested_xmodem_name_and_cleanup() {
    let (root, mut state, _peer) = fixture("").await;
    let first = completed("one.bin", b"one");
    let second = completed("TWO.BIN", b"two!");
    let paths = [first.source.to_path_buf(), second.source.to_path_buf()];
    state
        .finish_uploads(
            UploadReceipt {
                files: vec![second, first],
                ..Default::default()
            },
            &[request("ONE.BIN", false, "first description"), request("TWO.BIN", true, "/second description")],
            false,
            "Z",
        )
        .await
        .unwrap();
    for (directory, name, description) in [("public", "ONE.BIN", "first description"), ("private", "TWO.BIN", "/second description")] {
        let location = root.path().join(directory);
        let base = state.get_filebase(&location, &root.path().join(format!("{directory}-meta"))).await.unwrap();
        let metadata = base.lock().await.read_metadata(&location.join(name)).unwrap();
        assert!(
            metadata
                .iter()
                .any(|m| m.metadata_type == MetadataType::FileID && m.data == description.as_bytes())
        );
    }
    assert!(paths.iter().all(|p| !p.exists()));
    assert_eq!(state.session.current_user.as_ref().unwrap().stats.total_upld_bytes, 7);
    let file = completed("generated-temp-name", b"xmodem");
    state
        .finish_uploads(
            UploadReceipt {
                files: vec![file],
                ..Default::default()
            },
            &[request("ASKED.BIN", false, "requested description")],
            true,
            "X",
        )
        .await
        .unwrap();
    assert_eq!(std::fs::read(root.path().join("public/ASKED.BIN")).unwrap(), b"xmodem");
}

#[tokio::test]
async fn described_and_unannounced_batch_files_have_independent_descriptions_and_privacy() {
    // A blank FIRST line after receiving is not cancellation (TRANSFER.C 754+).
    let (root, mut state, mut peer) = fixture("\rfirst unannounced description\r\r\\second unannounced description\r\r").await;
    let files = vec![
        completed("KNOWN1.BIN", b"known one"),
        completed("NEW1.BIN", b"new one"),
        completed("KNOWN2.BIN", b"known two"),
        completed("NEW2.BIN", b"new two"),
    ];
    let paths: Vec<_> = files.iter().map(|file| file.source.to_path_buf()).collect();
    finish_bounded(
        &mut state,
        UploadReceipt { files, ..Default::default() },
        &[
            request("KNOWN1.BIN", true, "/first known description"),
            request("KNOWN2.BIN", false, "second known description"),
        ],
        false,
    )
    .await;
    timeout(Duration::from_secs(3), async {
        for (directory, name, description) in [
            ("private", "KNOWN1.BIN", "/first known description"),
            ("public", "KNOWN2.BIN", "second known description"),
            ("public", "NEW1.BIN", "first unannounced description"),
            ("private", "NEW2.BIN", "\\second unannounced description"),
        ] {
            assert_description(&mut state, root.path(), directory, name, description).await;
        }
    })
    .await
    .unwrap();
    assert!(paths.iter().all(|path| !path.exists()));
    assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_uploads, 4);
    let text = output(&mut peer).await;
    assert!(
        text.contains("NEW1.BIN") && text.contains("NEW2.BIN") && text.to_ascii_lowercase().contains("longer"),
        "{text}"
    );
    assert!(state.char_buffer.is_empty());
}

#[tokio::test]
async fn unannounced_archive_diz_supplies_metadata_without_prompt_and_honors_private_default() {
    use std::io::Write;
    for (conference_private, description) in [
        (false, "Archive DIZ description"),
        (true, "Archive DIZ description"),
        (false, "\\Private archive DIZ"),
    ] {
        let (root, mut state, mut peer) = fixture("").await;
        state.session.current_conference.private_uploads = conference_private;
        let source = NamedTempFile::new().unwrap();
        let mut zip = zip::ZipWriter::new(source.as_file());
        zip.start_file("FILE_ID.DIZ", zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(description.as_bytes()).unwrap();
        zip.finish().unwrap();
        let temporary = source.path().to_path_buf();
        finish_bounded(
            &mut state,
            UploadReceipt {
                files: vec![
                    completed("KNOWN.BIN", b"known"),
                    CompletedUpload {
                        name: "NEW.ZIP".into(),
                        source: source.into_temp_path(),
                        cps: 1,
                    },
                ],
                ..Default::default()
            },
            &[request("KNOWN.BIN", true, "/known private description")],
            false,
        )
        .await;
        let directory = if conference_private || description.starts_with('\\') {
            "private"
        } else {
            "public"
        };
        timeout(
            Duration::from_secs(3),
            assert_description(&mut state, root.path(), directory, "NEW.ZIP", description),
        )
        .await
        .unwrap();
        assert!(!temporary.exists());
        assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_uploads, 2);
        let text = output(&mut peer).await;
        assert!(!text.contains("Enter a description"), "{text}");
    }
}

#[tokio::test]
async fn failed_or_logged_off_unannounced_uploads_skip_input_and_clean_all_temporaries() {
    for logoff in [false, true] {
        let (root, mut state, _peer) = fixture("must not be consumed\r").await;
        state.session.request_logoff = logoff;
        let remaining_input = state.char_buffer.len();
        let files = vec![completed("NEW1.BIN", b"one"), completed("NEW2.BIN", b"two")];
        let paths: Vec<_> = files.iter().map(|file| file.source.to_path_buf()).collect();
        finish_bounded(
            &mut state,
            UploadReceipt {
                files,
                failed: !logoff,
                errors: 0,
            },
            &[],
            false,
        )
        .await;
        assert_eq!(state.char_buffer.len(), remaining_input);
        assert!(paths.iter().all(|path| !path.exists()));
        assert!(!root.path().join("public/NEW1.BIN").exists());
        assert!(!root.path().join("private/NEW2.BIN").exists());
        assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_uploads, 0);
    }
}

#[tokio::test]
async fn canceling_posttransfer_description_cleans_pending_files_before_any_publication() {
    let (root, mut state, mut peer) = fixture("").await;
    let files = vec![completed("KNOWN.BIN", b"known"), completed("NEW1.BIN", b"one"), completed("NEW2.BIN", b"two")];
    let paths: Vec<_> = files.iter().map(|file| file.source.to_path_buf()).collect();
    // Cancel the entire future while description entry awaits input. Both the
    // pending described file and all remaining received files must be dropped.
    assert!(
        timeout(
            Duration::from_millis(100),
            state.finish_uploads(
                UploadReceipt { files, ..Default::default() },
                &[request("KNOWN.BIN", false, "known description")],
                false,
                "Z",
            ),
        )
        .await
        .is_err()
    );
    assert!(paths.iter().all(|path| !path.exists()));
    assert!(!root.path().join("public/KNOWN.BIN").exists());
    assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_uploads, 0);
    assert!(output(&mut peer).await.contains("NEW1.BIN"));
}

struct LostUploadCarrier;

#[async_trait::async_trait]
impl Connection for LostUploadCarrier {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Channel
    }

    async fn read(&mut self, _buf: &mut [u8]) -> icy_net::Result<usize> {
        panic!("description input must not read after carrier loss");
    }

    async fn try_read(&mut self, _buf: &mut [u8]) -> icy_net::Result<usize> {
        panic!("description input must not read after carrier loss");
    }

    async fn send(&mut self, _buf: &[u8]) -> icy_net::Result<()> {
        Ok(())
    }

    async fn poll(&mut self) -> icy_net::Result<icy_net::ConnectionState> {
        Ok(icy_net::ConnectionState::Disconnected)
    }
}

#[tokio::test]
async fn lost_carrier_skips_unannounced_description_even_with_buffered_input() {
    let (_root, mut state, _peer) = fixture("must not be consumed\r").await;
    state.connection = Box::new(LostUploadCarrier);
    let remaining_input = state.char_buffer.len();
    let file = completed("NEW.BIN", b"unannounced");
    let path = file.source.to_path_buf();
    finish_bounded(
        &mut state,
        UploadReceipt {
            files: vec![file],
            ..Default::default()
        },
        &[],
        false,
    )
    .await;
    assert!(!path.exists());
    assert_eq!(state.char_buffer.len(), remaining_input);
    assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_uploads, 0);
}

#[tokio::test]
async fn no_batch_requested_name_discards_extra_payloads_without_description_input() {
    let (root, mut state, _peer) = fixture("").await;
    let files = vec![completed("SENDER.BIN", b"first"), completed("EXTRA.BIN", b"second")];
    let paths: Vec<_> = files.iter().map(|file| file.source.to_path_buf()).collect();
    finish_bounded(
        &mut state,
        UploadReceipt { files, ..Default::default() },
        &[request("REQUESTED.BIN", false, "requested description")],
        true,
    )
    .await;
    assert_eq!(std::fs::read(root.path().join("public/REQUESTED.BIN")).unwrap(), b"first");
    assert!(!root.path().join("public/SENDER.BIN").exists());
    assert!(!root.path().join("public/EXTRA.BIN").exists());
    assert!(paths.iter().all(|path| !path.exists()));
    assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_uploads, 1);
}

#[tokio::test]
async fn rejected_duplicates_empty_and_failed_receipts_never_claim_success_or_credit() {
    let (root, mut state, mut peer) = fixture("").await;
    std::fs::write(root.path().join("public/OLD.BIN"), b"old").unwrap();
    let files = vec![
        completed("OLD.BIN", b"replacement"),
        completed("../escape", b"bad"),
        completed("EMPTY.BIN", b""),
    ];
    let paths: Vec<_> = files.iter().map(|f| f.source.to_path_buf()).collect();
    state
        .finish_uploads(
            UploadReceipt {
                files,
                failed: true,
                errors: 1,
            },
            &[request("OLD.BIN", false, "duplicate"), request("EMPTY.BIN", false, "empty file")],
            false,
            "Z",
        )
        .await
        .unwrap();
    assert!(paths.iter().all(|p| !p.exists()));
    assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_uploads, 0);
    assert_eq!(state.get_board().await.statistics.total.uploads, 0);
    assert_eq!(std::fs::read(root.path().join("public/OLD.BIN")).unwrap(), b"old");
    let text = output(&mut peer).await;
    assert!(!text.contains("Successful") && !text.contains("Thanks"), "{text}");
}

#[test]
fn unsafe_names_credit_order_and_completed_guard_cleanup() {
    for name in ["", ".", "..", "../secret", "a/b", "a\\b", "C:secret", "a\0b", "a\nb", "*.zip", " file"] {
        assert!(normalize_upload_name(name).is_none(), "{name:?}");
    }
    assert_eq!(normalize_upload_name("FILE."), Some("FILE".into()));
    assert_eq!(upload_credits(19, 10, 10, 25), (19, 2));
    assert_eq!(upload_credits(19, 0, 10, 25), (19, 0));
    let source = NamedTempFile::new().unwrap().keep().unwrap().1;
    let mut transfer = TransferState::new("test".into());
    transfer.recieve_state.finished_files.push(("file".into(), source.clone()));
    drop(UploadReceiveGuard(transfer));
    assert!(!source.exists());
}

#[tokio::test]
async fn accepted_credit_preserves_unlimited_bytes_and_event_time() {
    let (_root, mut state, _peer) = fixture("").await;
    state.get_board().await.config.file_transfer.upload_credit_bytes = 10;
    state.get_board().await.config.file_transfer.upload_credit_time = 25;
    let before = state.session.time_limit;
    state.credit_completed_upload("one", 240, 10).await.unwrap();
    assert_eq!(state.session.time_limit, before + 1);
    assert_eq!(state.session.bytes_remaining, -1);
    assert_eq!(state.session.current_user.as_ref().unwrap().stats.today_dnld_bytes, -240);
    state.session.time_adjusted_for_event = true;
    state.credit_completed_upload("two", 240, 10).await.unwrap();
    assert_eq!(state.session.time_limit, before + 1);
}

#[tokio::test]
async fn fractional_upload_credit_is_carried_in_private_session_state() {
    let (_root, mut state, _peer) = fixture("").await;
    state.get_board().await.config.file_transfer.upload_credit_time = 10;
    let before = state.session.time_limit;
    timeout(Duration::from_secs(3), async {
        state.credit_completed_upload("one", 300, 10).await.unwrap();
        assert_eq!(state.session.time_limit, before);
        assert_eq!(state.session.upload_credit_seconds, 30);
        state.credit_completed_upload("two", 300, 10).await.unwrap();
        assert_eq!(state.session.time_limit, before + 1);
        assert_eq!(state.session.upload_credit_seconds, 0);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn local_staging_survives_quarantine_and_manual_approval() {
    for policy in [UploadPublishPolicy::AfterProcessing, UploadPublishPolicy::ManualApproval] {
        let (root, mut state, _peer) = fixture("").await;
        let original = root.path().join("original.bin");
        std::fs::write(&original, b"original contents").unwrap();
        {
            let mut board = state.get_board().await;
            board.config.upload_processing.publish_policy = policy;
            board.config.upload_processing.quarantine_path = root.path().join("quarantine");
        }
        let staged = stage_local_upload(&original).unwrap();
        let staged_path = staged.to_path_buf();
        let file = CompletedUpload {
            name: "original.bin".into(),
            source: staged,
            cps: 0,
        };
        state
            .finish_uploads(
                UploadReceipt {
                    files: vec![file],
                    ..Default::default()
                },
                &[request("original.bin", false, "quarantine description")],
                false,
                "Local",
            )
            .await
            .unwrap();
        assert_eq!(std::fs::read(&original).unwrap(), b"original contents");
        assert!(!staged_path.exists());
        let records = UploadQuarantine::new(root.path().join("quarantine")).list().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].description, vec!["quarantine description"]);
        assert_eq!(
            records[0].status,
            if policy == UploadPublishPolicy::AfterProcessing {
                QuarantineStatus::Published
            } else {
                QuarantineStatus::AwaitingApproval
            }
        );
        assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_uploads, 1);
    }
}

#[cfg(unix)]
#[tokio::test]
async fn rejected_scanner_keeps_quarantine_without_credit() {
    let (root, mut state, _peer) = fixture("").await;
    {
        let mut board = state.get_board().await;
        let config = &mut board.config.upload_processing;
        config.publish_policy = UploadPublishPolicy::AfterProcessing;
        config.quarantine_path = root.path().join("quarantine");
        config.scanner.enabled = true;
        config.scanner.executable = "/bin/false".into();
        config.scanner.arguments = vec!["{file}".into()];
    }
    let file = completed("infected.bin", b"test scanner rejection");
    let temporary = file.source.to_path_buf();
    finish_bounded(
        &mut state,
        UploadReceipt {
            files: vec![file],
            ..Default::default()
        },
        &[request("infected.bin", false, "rejected description")],
        false,
    )
    .await;
    assert!(!temporary.exists());
    assert!(!root.path().join("public/infected.bin").exists());
    let quarantine = UploadQuarantine::new(root.path().join("quarantine"));
    let records = quarantine.list().unwrap();
    assert_eq!(records[0].status, QuarantineStatus::NeedsReview);
    assert!(records[0].processing_report.iter().any(|line| line == "virus scanner: infected"));
    assert!(quarantine.payload_path(&records[0]).exists());
    assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_uploads, 0);
}

#[tokio::test]
async fn native_xmodem_uses_requested_name_and_remote_cancel_never_completes_a_file() {
    for cancel in [false, true] {
        let (root, mut state, mut peer) = fixture("").await;
        let source = root.path().join("SENDER.BIN");
        std::fs::write(&source, vec![42; 512]).unwrap();
        let (receipt, ()) = timeout(Duration::from_secs(10), async {
            tokio::join!(state.receive_uploads(&TransferProtocolType::XModem, 1), async {
                let mut sender = XYmodem::new(XYModemVariant::XModem);
                if cancel {
                    assert_eq!(peer.read_u8().await.unwrap(), 0x15);
                    sender.cancel_transfer(&mut peer).await.unwrap();
                    return;
                }
                let mut sent = sender.initiate_send(&mut peer, std::slice::from_ref(&source)).await.unwrap();
                while !sent.is_finished {
                    sender.update_transfer(&mut peer, &mut sent).await.unwrap();
                    tokio::task::yield_now().await;
                }
            })
        })
        .await
        .expect("native XMODEM upload stalled");
        let receipt = receipt.unwrap();
        assert_eq!(receipt.files.len(), usize::from(!cancel));
        state
            .finish_uploads(receipt, &[request("REQUESTED.BIN", false, "requested description")], true, "X")
            .await
            .unwrap();
        assert_eq!(root.path().join("public/REQUESTED.BIN").exists(), !cancel);
        assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_uploads, u64::from(!cancel));
        assert_eq!(std::fs::read(&source).unwrap(), vec![42; 512]);
    }
}

#[tokio::test]
async fn native_upload_command_retains_single_names_but_enabled_batch_describes_extra_files() {
    use icy_net::protocol::{Header, ZFrameType};
    for (batch, no_batch) in [(false, false), (true, true), (true, false)] {
        let append = batch && !no_batch;
        let mut input = "known description\r\r".to_string();
        if append {
            input.push('\r'); // End batch name collection.
        }
        if batch {
            input.push('\r'); // Start at the batch-ready prompt.
        }
        if append {
            input.push_str("\\extra received description\r\r");
        }
        let (root, mut state, mut peer) = fixture(&input).await;
        let requested = if append { "SENDER.BIN" } else { "ASKED.BIN" };
        state.session.tokens.push_back(requested.into());
        state.session.current_user.as_mut().unwrap().protocol = "Z".into();
        state.get_board().await.config.file_transfer.disallow_batch_uploads = no_batch;
        let sources = tempfile::tempdir().unwrap();
        let paths: Vec<_> = ["SENDER.BIN", "EXTRA.BIN"]
            .iter()
            .map(|name| {
                let path = sources.path().join(name);
                std::fs::write(&path, vec![42; 2048]).unwrap();
                path
            })
            .collect();
        let (result, ()) = timeout(Duration::from_secs(10), async {
            tokio::join!(state.upload_files(batch), async {
                let mut can_count = 0;
                loop {
                    if let Ok(Some(header)) = Header::read(&mut peer, &mut can_count).await
                        && header.frame_type == ZFrameType::RIinit
                    {
                        break;
                    }
                }
                let mut sender = Zmodem::new(1024);
                let mut transfer = sender.initiate_send(&mut peer, &paths).await.unwrap();
                while !transfer.is_finished {
                    if sender.update_transfer(&mut peer, &mut transfer).await.is_err() {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
        })
        .await
        .expect("native upload command stalled");
        result.unwrap();
        assert!(root.path().join("public").join(requested).exists());
        assert_eq!(root.path().join("public/SENDER.BIN").exists(), append);
        assert_eq!(root.path().join("private/EXTRA.BIN").exists(), append);
        assert!(!root.path().join("public/EXTRA.BIN").exists());
        assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_uploads, if append { 2 } else { 1 });
        assert!(state.char_buffer.is_empty());
        assert!(paths.iter().all(|path| path.exists()));
        timeout(Duration::from_secs(3), async {
            assert_description(&mut state, root.path(), "public", requested, "known description").await;
            if append {
                assert_description(&mut state, root.path(), "private", "EXTRA.BIN", "\\extra received description").await;
            }
        })
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn paired_zmodem_receives_native_names_and_stops_no_batch_after_first_payload() {
    use icy_net::protocol::{Header, ZFrameType};
    for (limit, cancel_after_first) in [(1, false), (32000, false), (32000, true)] {
        let (root, mut state, _unused_peer) = fixture("").await;
        let (mut peer, connection) = ChannelConnection::create_pair();
        state.connection = Box::new(connection);
        let sources = tempfile::tempdir().unwrap();
        let paths: Vec<_> = ["ONE.BIN", "TWO.BIN"]
            .iter()
            .map(|name| {
                let path = sources.path().join(name);
                std::fs::write(&path, vec![42; 2048]).unwrap();
                path
            })
            .collect();
        let (receipt, ()) = timeout(Duration::from_secs(10), async {
            tokio::join!(state.receive_uploads(&TransferProtocolType::ZModem, limit), async {
                let mut can_count = 0;
                loop {
                    if let Ok(Some(header)) = Header::read(&mut peer, &mut can_count).await
                        && header.frame_type == ZFrameType::RIinit
                    {
                        break;
                    }
                }
                let mut sender = Zmodem::new(1024);
                let mut transfer = sender.initiate_send(&mut peer, &paths).await.unwrap();
                while !transfer.is_finished {
                    if sender.update_transfer(&mut peer, &mut transfer).await.is_err() {
                        break;
                    }
                    if cancel_after_first && !transfer.send_state.finished_files.is_empty() {
                        sender.cancel_transfer(&mut peer).await.unwrap();
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
        })
        .await
        .expect("paired upload stalled");
        let receipt = receipt.unwrap();
        assert_eq!(receipt.failed, cancel_after_first);
        assert_eq!(receipt.files.len(), if limit == 1 || cancel_after_first { 1 } else { 2 });
        for file in &receipt.files {
            assert_eq!(std::fs::read(&file.source).unwrap(), vec![42; 2048]);
        }
        let temps: Vec<_> = receipt.files.iter().map(|f| f.source.to_path_buf()).collect();
        drop(receipt);
        assert!(temps.iter().all(|p| !p.exists()));
        assert!(paths.iter().all(|p| p.exists()));
        assert!(!root.path().join("public/ONE.BIN").exists());
    }
}
