use super::*;

fn message_fixture() -> JamMessage {
    let message = JamMessage::default()
        .with_from(BString::from("AUTHOR"))
        .with_to(BString::from("READER"))
        .with_subject(BString::from("Original subject"))
        .with_msg_id(BString::from("network-message-id"))
        .with_reply_id(BString::from("network-parent-id"))
        .with_password(&BString::from("SECRET"))
        .with_attributes(attributes::MSG_PRIVATE | attributes::MSG_LOCAL | attributes::MSG_RECEIPTREQ)
        .with_sub_field(MessageSubfield::new(SubfieldType::PackoutDate, BString::from("2030-01-02T03:04:05Z")))
        .with_sub_field(MessageSubfield::new(SubfieldType::DateWritten, BString::from("2020-01-02T03:04:05Z")))
        .with_sub_field(MessageSubfield::new(SubfieldType::Address0, BString::from("2:123/456")))
        .with_sub_field(MessageSubfield::new(SubfieldType::AddressD, BString::from("reader@example.org")))
        .with_sub_field(MessageSubfield::new(SubfieldType::Unknown(424242), BString::from(vec![0, 255, 42])))
        .with_text(BString::from("Original body\r\nSecond line\r\n"));
    let mut header = message.header().clone();
    header.reply_to = 41;
    header.reply_first = 42;
    header.reply_next = 43;
    header.date_written = 1577934245;
    header.date_received = 1577934246;
    header.date_processed = 1577934247;
    header.times_read = 17;
    header.attributes2 = 0xabcdef;
    header.cost = 123;
    JamMessage::from_stored(header, message.text().clone())
}

fn header_bytes(header: &JamMessageHeader) -> Vec<u8> {
    let mut bytes = Vec::new();
    header.write(&mut bytes).unwrap();
    bytes
}

fn assert_transfer_metadata(original: &JamMessage, stored: &JamMessage, same_base: bool) {
    let mut expected = original.header().clone();
    expected.offset = stored.header().offset;
    expected.message_number = stored.header().message_number;
    expected.txt_len = stored.header().txt_len;
    expected.reply_first = 0;
    expected.reply_next = 0;
    if !same_base { expected.reply_to = 0; }
    assert_eq!(header_bytes(&expected), header_bytes(stored.header()));
    assert_eq!(original.text(), stored.text());
    assert!(stored.header().is_password_valid("SECRET"));
}

#[tokio::test]
async fn copy_preserves_all_metadata_but_not_cross_base_storage_or_numeric_threads() {
    let temp = tempfile::tempdir().unwrap();
    let mut source = JamMessageBase::create(temp.path().join("source")).unwrap();
    let mut target = JamMessageBase::create(temp.path().join("target")).unwrap();
    source.write_message(&message_fixture()).unwrap();
    // Different numbering and body offset catch accidental source-index reuse.
    target.write_message(&JamMessage::default().with_text(BString::from("padding"))).unwrap();
    let original = source.read_message(1).unwrap();
    let draft = transfer_draft(source.read_message(1).unwrap(), false);
    finish_transfer(async { target.write_message(&draft)?; Ok(()) }, &temp.path().join("target"), false,
        || panic!("COPY must never delete its source")).await.unwrap();
    assert_transfer_metadata(&original, &target.read_message(2).unwrap(), false);
    assert!(!source.read_header(1).unwrap().is_deleted());
    assert_eq!(source.read_message(1).unwrap().text(), original.text());
    assert_eq!(target.search_to(&BString::from("READER")).unwrap(), vec![2]);
}

#[tokio::test]
async fn move_commits_destination_before_deleting_source() {
    let temp = tempfile::tempdir().unwrap();
    let mut source = JamMessageBase::create(temp.path().join("source")).unwrap();
    let target_path = temp.path().join("target");
    source.write_message(&message_fixture()).unwrap();
    let original = source.read_message(1).unwrap();
    let draft = transfer_draft(source.read_message(1).unwrap(), false);
    finish_transfer(async {
        let mut target = JamMessageBase::create(&target_path)?;
        target.write_message(&draft)?;
        Ok(())
    }, &target_path, true, || {
        let target = JamMessageBase::open(&target_path)?;
        assert_transfer_metadata(&original, &target.read_message(1)?, false);
        delete_unchanged_message(&mut source, 1, &original)
    }).await.unwrap();
    assert!(matches!(source.read_header(1), Err(jamjam::Error::Jam(jamjam::jam::JamError::MessageDeleted))));
}

#[tokio::test]
async fn destination_open_failure_never_deletes_the_source() {
    let temp = tempfile::tempdir().unwrap();
    let mut source = JamMessageBase::create(temp.path().join("source")).unwrap();
    source.write_message(&message_fixture()).unwrap();
    let original = header_bytes(&source.read_header(1).unwrap());
    let result = finish_transfer(async {
        JamMessageBase::open(temp.path().join("missing"))?;
        Ok(())
    }, &temp.path().join("missing"), true, || { source.delete_message(1)?; Ok(()) }).await;
    assert!(matches!(result, Err(TransferFailure::Destination(_))));
    assert_eq!(original, header_bytes(&source.read_header(1).unwrap()));
    assert_eq!(message_fixture().text(), source.read_message(1).unwrap().text());
}

#[tokio::test]
async fn destination_text_write_failure_never_deletes_the_source() {
    let temp = tempfile::tempdir().unwrap();
    let mut source = JamMessageBase::create(temp.path().join("source")).unwrap();
    source.write_message(&message_fixture()).unwrap();
    let target_path = temp.path().join("target");
    let mut target = JamMessageBase::create(&target_path).unwrap();
    // Deterministic even as root: a directory cannot be opened as a text file.
    std::fs::remove_file(target_path.with_extension("jdt")).unwrap();
    std::fs::create_dir(target_path.with_extension("jdt")).unwrap();
    let draft = transfer_draft(source.read_message(1).unwrap(), false);
    let result = finish_transfer(async { target.write_message(&draft)?; Ok(()) }, &target_path, true,
        || { source.delete_message(1)?; Ok(()) }).await;
    assert!(matches!(result, Err(TransferFailure::Destination(_))));
    assert!(!source.read_header(1).unwrap().is_deleted());
    assert_eq!(source.read_message(1).unwrap().text(), draft.text());
}

#[tokio::test]
async fn source_deletion_failure_keeps_the_successful_destination_copy() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("source");
    let mut source = JamMessageBase::create(&source_path).unwrap();
    let mut target = JamMessageBase::create(temp.path().join("target")).unwrap();
    source.write_message(&message_fixture()).unwrap();
    let original = source.read_message(1).unwrap();
    let draft = transfer_draft(source.read_message(1).unwrap(), false);
    let result = finish_transfer(async {
        target.write_message(&draft)?;
        std::fs::rename(source_path.with_extension("jdx"), source_path.with_extension("saved-index"))?;
        std::fs::create_dir(source_path.with_extension("jdx"))?;
        Ok(())
    }, &temp.path().join("target"), true, || { source.delete_message(1)?; Ok(()) }).await;
    assert!(matches!(result, Err(TransferFailure::Source(_))));
    std::fs::remove_dir(source_path.with_extension("jdx")).unwrap();
    std::fs::rename(source_path.with_extension("saved-index"), source_path.with_extension("jdx")).unwrap();
    assert!(!source.read_header(1).unwrap().is_deleted());
    assert_transfer_metadata(&original, &target.read_message(1).unwrap(), false);
}

#[test]
fn same_base_copy_keeps_parent_but_never_duplicates_child_or_sibling_links() {
    let original = message_fixture();
    let draft = transfer_draft(message_fixture(), true);
    assert_eq!(draft.header().reply_to, 41);
    assert_eq!(draft.header().reply_first, 0);
    assert_eq!(draft.header().reply_next, 0);
    assert_transfer_metadata(&original, &draft, true);
}

#[test]
fn edit_replaces_shorter_and_longer_bodies_without_changing_indexes_or_metadata() {
    let temp = tempfile::tempdir().unwrap();
    let mut base = JamMessageBase::create(temp.path().join("edit")).unwrap();
    base.write_message(&message_fixture()).unwrap();
    base.write_message(&JamMessage::default().with_text(BString::from("unrelated"))).unwrap();
    for body in ["short".to_string(), "much longer body\r\n".repeat(200)] {
        let original = base.read_message(1).unwrap();
        let mut header = original.header().clone();
        header.set_to(BString::from("NEW RECIPIENT"));
        let draft = JamMessage::from_stored(header, BString::from(body.clone()));
        replace_message(&mut base, 1, &original, &draft).unwrap();
        let edited = base.read_message(1).unwrap();
        let mut expected = draft.header().clone();
        expected.offset = edited.header().offset;
        expected.txt_len = edited.header().txt_len;
        assert_eq!(header_bytes(&expected), header_bytes(edited.header()));
        assert_eq!(edited.text(), &BString::from(body));
        assert_eq!(base.highest_message_number(), 2);
        assert_eq!(base.search_to(&BString::from("NEW RECIPIENT")).unwrap(), vec![1]);
        assert_eq!(base.read_message(2).unwrap().text(), &BString::from("unrelated"));
    }
}

#[test]
fn edit_rejects_concurrent_changes_without_overwriting_or_appending_text() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("edit");
    let mut base = JamMessageBase::create(&path).unwrap();
    base.write_message(&message_fixture()).unwrap();
    let original = base.read_message(1).unwrap();
    raw::set_attributes(&mut base, 1, attributes::MSG_READ, 0).unwrap();
    let before = header_bytes(&base.read_header(1).unwrap());
    let before_len = std::fs::metadata(path.with_extension("jdt")).unwrap().len();
    let draft = JamMessage::from_stored(original.header().clone(), BString::from("edited"));
    assert!(replace_message(&mut base, 1, &original, &draft).is_err());
    assert_eq!(before, header_bytes(&base.read_header(1).unwrap()));
    assert_eq!(before_len, std::fs::metadata(path.with_extension("jdt")).unwrap().len());
    assert_eq!(original.text(), base.read_message(1).unwrap().text());
}

#[test]
fn clear_password_removes_sender_marker_without_touching_other_extensions() {
    let mut header = message_fixture().header().clone();
    set_security_kind(&mut header, true);
    assert!(header.needs_password());
    clear_password(&mut header);
    assert!(!header.needs_password());
    assert!(!requires_read_password(&header, false));
    assert!(header.sub_fields.iter().any(|field| field.field_type() == SubfieldType::PackoutDate));
    assert!(!header.sub_fields.iter().any(|field| field.content() == "ICYBOARD-SECURITY: S"));
}

#[test]
fn attachment_paths_reject_traversal_wildcards_and_alias_escape() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("stored.zip"), b"attachment").unwrap();
    let field = |name: &str| MessageSubfield::new(SubfieldType::EnclFwAlias, BString::from(name));
    let (path, alias) = attachment_path(temp.path(), &field("stored.zip\0display.zip")).unwrap();
    assert_eq!(path, temp.path().join("stored.zip"));
    assert_eq!(alias, "display.zip");
    for name in ["", ".", "..", "../secret", "/etc/passwd", "C:\\SECRET", "*.zip", "stored.zip\0../evil", "stored.zip\0safe\0extra"] {
        assert!(attachment_path(temp.path(), &field(name)).is_err(), "accepted {name:?}");
    }
}

#[cfg(unix)]
#[test]
fn attachment_paths_reject_symlinks_even_if_the_target_is_regular() {
    let temp = tempfile::tempdir().unwrap();
    let outside = tempfile::NamedTempFile::new().unwrap();
    std::os::unix::fs::symlink(outside.path(), temp.path().join("secret.zip")).unwrap();
    let field = MessageSubfield::new(SubfieldType::EnclFile, BString::from("secret.zip"));
    assert!(attachment_path(temp.path(), &field).is_err());
}

async fn action_state(root: &Path) -> (IcyBoardState, icy_net::channel::ChannelConnection) {
    use crate::icy_board::{IcyBoard, bbs::BBS, conferences::Conference, message_area::{AreaList, MessageArea}, user_base::User};
    use icy_net::{ConnectionType, channel::ChannelConnection};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let bbs = Arc::new(Mutex::new(BBS::new(1)));
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let (peer, connection) = ChannelConnection::create_pair();
    let mut board = IcyBoard::new();
    board.config.paths.statistics_file = root.join("statistics.toml");
    board.users.new_user(User { name: "AUTHOR".into(), security_level: 255, ..Default::default() });
    let caller = board.users[0].clone();
    board.conferences.clear();
    for name in ["source", "target"] {
        board.conferences.push(Conference {
            is_public: true,
            areas: Some(Arc::new(AreaList::new(vec![MessageArea { name: name.into(), path: root.join(name), ..Default::default() }]))),
            ..Default::default()
        });
    }
    let conference = board.conferences[0].clone();
    let mut state = IcyBoardState::new(bbs, Arc::new(Mutex::new(board)), nodes, node, Box::new(connection)).await;
    state.session.current_user = Some(caller);
    state.session.cur_user_id = 0;
    state.session.user_name = "AUTHOR".into();
    state.session.cur_security = 255;
    state.session.is_sysop = true;
    state.session.page_len = 0;
    state.session.current_conference = conference;
    state.session.current_conference_number = 0;
    state.session.current_message_area = 0;
    (state, peer)
}

#[tokio::test]
async fn actual_copy_action_persists_password_dates_and_network_extensions() {
    let temp = tempfile::tempdir().unwrap();
    let (mut state, _peer) = action_state(temp.path()).await;
    let mut source = JamMessageBase::create(temp.path().join("source")).unwrap();
    source.write_message(&message_fixture()).unwrap();
    let original = source.read_message(1).unwrap();
    assert!(state.copy_message_to_conference(&mut source, 1, 1, 0, false).await.unwrap());
    let target = JamMessageBase::open(temp.path().join("target")).unwrap();
    assert_transfer_metadata(&original, &target.read_message(1).unwrap(), false);
    assert!(!source.read_header(1).unwrap().is_deleted());
}

#[tokio::test]
async fn actual_move_action_retains_source_on_destination_open_failure() {
    let temp = tempfile::tempdir().unwrap();
    let (mut state, _peer) = action_state(temp.path()).await;
    let mut source = JamMessageBase::create(temp.path().join("source")).unwrap();
    source.write_message(&message_fixture()).unwrap();
    // A malformed existing destination must not be silently created over.
    std::fs::write(temp.path().join("target.jhr"), b"not a JAM header").unwrap();
    assert!(!state.copy_message_to_conference(&mut source, 1, 1, 0, true).await.unwrap());
    assert!(!source.read_header(1).unwrap().is_deleted());
    assert_eq!(message_fixture().text(), source.read_message(1).unwrap().text());
}

#[tokio::test]
async fn action_target_rejects_bounds_readonly_and_area_security() {
    use crate::icy_board::security_expr::SecurityExpression;
    let temp = tempfile::tempdir().unwrap();
    let (mut state, _peer) = action_state(temp.path()).await;
    assert!(state.read_action_target(u16::MAX, 0).await.unwrap().is_none());
    assert!(state.read_action_target(1, usize::MAX).await.unwrap().is_none());
    assert!(state.read_action_target(1, 0).await.unwrap().is_some());
    state.get_board().await.conferences[1].is_read_only = true;
    assert!(state.read_action_target(1, 0).await.unwrap().is_none());
    state.get_board().await.conferences[1].is_read_only = false;
    std::sync::Arc::make_mut(state.get_board().await.conferences[1].areas.as_mut().unwrap())[0].is_read_only = true;
    assert!(state.read_action_target(1, 0).await.unwrap().is_none());
    std::sync::Arc::make_mut(state.get_board().await.conferences[1].areas.as_mut().unwrap())[0].is_read_only = false;
    state.session.cur_security = 10;
    std::sync::Arc::make_mut(state.get_board().await.conferences[1].areas.as_mut().unwrap())[0].req_level_to_enter = SecurityExpression::from_req_security(100);
    assert!(state.read_action_target(1, 0).await.unwrap().is_none());
}

#[tokio::test]
async fn aborted_reply_or_edit_redisplays_without_deleting_source() {
    let temp = tempfile::tempdir().unwrap();
    let mut source = JamMessageBase::create(temp.path().join("source")).unwrap();
    source.write_message(&message_fixture()).unwrap();
    assert!(matches!(IcyBoardState::after_message_save(EditResult::Abort), AfterAction::Redisplay));
    assert!(matches!(IcyBoardState::after_message_save(EditResult::SendMessage), AfterAction::Redisplay));
    assert!(matches!(IcyBoardState::after_message_save(EditResult::SendNext), AfterAction::Next));
    assert!(!source.read_header(1).unwrap().is_deleted());
}

async fn reply_action_state(root: &Path) -> (IcyBoardState, icy_net::channel::ChannelConnection) {
    use crate::icy_board::{icb_text::DEFAULT_DISPLAY_TEXT, security_expr::SecurityExpression, user_base::FSEMode};
    let (mut state, peer) = action_state(root).await;
    state.get_board().await.default_display_text = DEFAULT_DISPLAY_TEXT.clone();
    state.get_board().await.config.message.validate_to_name = false;
    state.get_board().await.config.paths.email_msgbase = root.join("email");
    state.session.cur_security = 10;
    state.session.is_sysop = false;
    state.session.fse_mode = FSEMode::No;
    state.session.current_conference.long_to_names = true;
    state.session.current_conference.disallow_private_msgs = true;
    state.session.current_conference.sec_request_rr = SecurityExpression::from_req_security(255);
    state.session.user_command_level.cmd_e = SecurityExpression::from_req_security(0);
    state.session.user_command_level.cmd_k = SecurityExpression::from_req_security(0);
    (state, peer)
}

#[tokio::test]
async fn reader_reply_sk_deletes_actual_source_and_advances_without_a_second_kill() {
    use crate::icy_board::state::{KeyChar, KeySource};
    use jamjam::jam::pack::PackOptions;
    use std::time::Duration;
    use tokio::time::timeout;

    for email in [false, true] {
        for other in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let (mut state, _peer) = reply_action_state(temp.path()).await;
            let path = temp.path().join(if email { "email" } else { "source" });
            let mut source = JamMessageBase::create(&path).unwrap();
            source.write_message(&JamMessage::default()
                .with_from(BString::from("REMOTE"))
                .with_to(BString::from("AUTHOR"))
                .with_attributes(attributes::MSG_PRIVATE)
                .with_text(BString::from("source quote"))).unwrap();
            let input = if other { "REMOTE\r\r\rQ 1 1\rSK\r" } else { "\rQ 1 1\rSK\r" };
            state.char_buffer.extend(input.chars().map(|ch| KeyChar::new(KeySource::User, ch)));
            let cmd = ReadCommand { func: if other { MsgFunc::ReplyOther } else { MsgFunc::Reply }, ..Default::default() };
            let result = timeout(Duration::from_secs(3), state.run_read_action(&cmd, &mut source, 1)).await.unwrap().unwrap();
            assert!(matches!(result, AfterAction::Next));
            let mut saved = JamMessageBase::open(&path).unwrap();
            assert!(matches!(saved.read_header(1), Err(jamjam::Error::Jam(jamjam::jam::JamError::MessageDeleted))));
            assert_eq!(saved.read_message(2).unwrap().text(), &BString::from("-> source quote"));

            // A pack can reuse the killed slot before navigation. Confirmation
            // must not look it up again or attempt to kill the replacement.
            saved.pack(&PackOptions::default().with_renumber_from(1)).unwrap();
            let before = header_bytes(&saved.read_header(1).unwrap());
            assert!(matches!(IcyBoardState::after_message_save(EditResult::SendKill), AfterAction::Next));
            assert_eq!(before, header_bytes(&saved.read_header(1).unwrap()));
        }
    }
}

#[tokio::test]
async fn reader_reply_sk_redisplays_instead_of_killing_a_source_edited_during_composition() {
    use crate::icy_board::{icb_text::DEFAULT_DISPLAY_TEXT, state::{KeyChar, KeySource}};
    use icy_net::Connection;
    use std::time::Duration;
    use tokio::time::timeout;

    let temp = tempfile::tempdir().unwrap();
    let (mut state, mut peer) = reply_action_state(temp.path()).await;
    const PROMPT: &str = "[reader-reply-compose]";
    // The active session owns a copy made at construction; changing the board
    // defaults afterwards cannot change the prompt emitted by this session.
    state.display_text = DEFAULT_DISPLAY_TEXT.clone();
    state.display_text.update_record_number(IceText::TextEntryCommand as usize, PROMPT).unwrap();
    assert_eq!(state.get_display_text(IceText::TextEntryCommand).unwrap(), PROMPT);
    let path = temp.path().join("source");
    let mut source = JamMessageBase::create(&path).unwrap();
    source.write_message(&JamMessage::default().with_from(BString::from("REMOTE"))
        .with_to(BString::from("AUTHOR")).with_attributes(attributes::MSG_PRIVATE)
        .with_text(BString::from("source quote"))).unwrap();
    state.char_buffer.push_back(KeyChar::new(KeySource::User, '\r'));
    let cmd = ReadCommand { func: MsgFunc::Reply, ..Default::default() };
    let caller = async {
        let mut output = Vec::new();
        while !output.windows(PROMPT.len()).any(|bytes| bytes == PROMPT.as_bytes()) {
            let mut bytes = [0; 4096];
            let count = peer.read(&mut bytes).await.unwrap();
            assert_ne!(count, 0);
            output.extend_from_slice(&bytes[..count]);
        }
        let mut writer = JamMessageBase::open(&path).unwrap();
        assert!(writer.try_lock().unwrap());
        writer.unlock();
        writer.transaction(|_| {
            std::fs::write(path.with_extension("jdt"), b"edited quote")?;
            Ok(())
        }).unwrap();
        peer.send(b"Q 1 1\rSK\r").await.unwrap();
        peer
    };
    let (result, _peer) = timeout(Duration::from_secs(3), async {
        tokio::join!(state.run_read_action(&cmd, &mut source, 1), caller)
    }).await.unwrap();
    assert!(matches!(result.unwrap(), AfterAction::Redisplay));
    let source = JamMessageBase::open(&path).unwrap();
    assert_eq!(source.read_message(1).unwrap().text(), &BString::from("edited quote"));
    assert_eq!(source.read_message(2).unwrap().text(), &BString::from("-> source quote"));
}

#[tokio::test]
async fn attachment_view_restores_tokens_and_file_list_even_for_an_invalid_archive() {
    let temp = tempfile::tempdir().unwrap();
    let (mut state, _peer) = action_state(temp.path()).await;
    state.session.current_conference.attachment_location = temp.path().to_path_buf();
    std::fs::write(temp.path().join("stored.bin"), b"not an archive").unwrap();
    let mut source = JamMessageBase::create(temp.path().join("source")).unwrap();
    source.write_message(&message_fixture().with_sub_field(MessageSubfield::new(
        SubfieldType::EnclFwAlias, BString::from("stored.bin\0display.zip")))).unwrap();
    state.session.tokens.push_back("KEEP".into());
    state.session.disp_options.in_file_list = Some(temp.path().join("old-list"));
    assert!(matches!(state.read_attachment(MsgFunc::ViewFile, &mut source, 1).await.unwrap(), AfterAction::Prompt));
    assert_eq!(state.session.tokens.iter().cloned().collect::<Vec<_>>(), vec!["KEEP".to_string()]);
    assert_eq!(state.session.disp_options.in_file_list, Some(temp.path().join("old-list")));
    assert!(temp.path().join("stored.bin").exists());
    assert!(!temp.path().join("display.zip").exists());
}

#[tokio::test]
async fn attachment_copy_does_not_overwrite_destination_files_or_remove_source() {
    let temp = tempfile::tempdir().unwrap();
    let (mut state, _peer) = action_state(temp.path()).await;
    let from = temp.path().join("from-attachments");
    let to = temp.path().join("to-attachments");
    std::fs::create_dir(&from).unwrap();
    std::fs::create_dir(&to).unwrap();
    std::fs::write(from.join("file.zip"), b"source attachment").unwrap();
    std::fs::write(to.join("file.zip"), b"unrelated attachment").unwrap();
    state.session.current_conference.attachment_location = from.clone();
    state.get_board().await.conferences[1].attachment_location = to.clone();
    let message = message_fixture().with_sub_field(MessageSubfield::new(SubfieldType::EnclFile, BString::from("file.zip")));
    assert!(state.copy_action_attachments(&message, 1, 0).await.is_err());
    assert_eq!(std::fs::read(from.join("file.zip")).unwrap(), b"source attachment");
    assert_eq!(std::fs::read(to.join("file.zip")).unwrap(), b"unrelated attachment");
}

#[test]
fn snapshot_survives_pack_as_owned_text_and_rejects_stale_authorization() {
    use jamjam::jam::pack::PackOptions;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("source");
    let mut base = JamMessageBase::create(&path).unwrap();
    for body in ["padding", "allowed", "PRIVATE"] {
        base.write_message(&JamMessage::default().with_text(BString::from(body))).unwrap();
    }
    let original = action_snapshot(&mut base, 2, None).unwrap();
    let mut writer = JamMessageBase::open(&path).unwrap();
    assert!(writer.try_lock().unwrap(), "snapshot must release its lock before any await");
    writer.unlock();
    writer.delete_message(1).unwrap();
    writer.pack(&PackOptions::default()).unwrap();
    // The old offset is still valid, but now points at somebody else's text.
    assert_eq!(base.read_message_text(original.header()).unwrap(), BString::from("PRIVATE"));
    assert_eq!(original.text(), &BString::from("allowed"));
    assert!(action_snapshot(&mut base, 2, Some(original.header())).is_err());
    let fresh = action_snapshot(&mut base, 2, None).unwrap();
    assert_eq!(fresh.text(), &BString::from("allowed"));
    assert_eq!(fresh.header().offset, 0);
}

#[test]
fn post_password_snapshot_rejects_changed_security_before_reading_text() {
    for change in 0..3 {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("source");
        let mut base = JamMessageBase::create(&path).unwrap();
        base.write_message(&message_fixture()).unwrap();
        let original = action_snapshot(&mut base, 1, None).unwrap();
        let mut changed = original.header().clone();
        match change {
            0 => changed.password_crc = JamMessageBase::crc(&BString::from("NEW PASSWORD")),
            1 => changed.set_to(BString::from("OTHER READER")),
            _ => changed.attributes |= attributes::MSG_NODISP,
        }
        raw::update_header(&mut base, 1, &changed).unwrap();
        std::fs::remove_file(path.with_extension("jdt")).unwrap();
        let error = action_snapshot(&mut base, 1, Some(original.header())).err().unwrap();
        assert!(error.to_string().contains("Message changed"), "must reject before trying to read missing text: {error}");
    }
}

#[tokio::test]
async fn password_prompt_holds_no_jam_lock_and_rejects_packed_replacement() {
    use icy_net::Connection;
    use jamjam::jam::pack::PackOptions;
    use crate::icy_board::{icb_text::DEFAULT_DISPLAY_TEXT, security_expr::SecurityExpression};
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("source");
    let (mut state, mut peer) = action_state(temp.path()).await;
    const PROMPT: &str = "[read-action-password]";
    // State construction clones the board's text; input_field reads this
    // session copy, not later changes to board.default_display_text.
    state.display_text = DEFAULT_DISPLAY_TEXT.clone();
    state.display_text.update_record_number(IceText::PasswordToReadMessage as usize, PROMPT).unwrap();
    {
        let mut board = state.get_board().await;
        board.config.sysop_command_level.read_all_mail = SecurityExpression::from_req_security(255);
    }
    state.session.is_sysop = false;
    state.session.cur_security = 10;
    state.session.user_name = "READER".into();
    let mut base = JamMessageBase::create(&path).unwrap();
    base.write_message(&message_fixture()).unwrap();
    base.write_message(&JamMessage::default().with_to(BString::from("STRANGER"))
        .with_attributes(attributes::MSG_PRIVATE).with_text(BString::from("other secret"))).unwrap();
    let mut writer = JamMessageBase::open(&path).unwrap();
    let caller = async {
        let mut output = Vec::new();
        while !output.windows(PROMPT.len()).any(|bytes| bytes == PROMPT.as_bytes()) {
            let mut bytes = [0; 4096];
            let count = peer.read(&mut bytes).await.unwrap();
            assert_ne!(count, 0);
            output.extend_from_slice(&bytes[..count]);
        }
        assert!(writer.try_lock().unwrap(), "password prompt must not hold a JAM lock");
        writer.unlock();
        writer.delete_message(1).unwrap();
        writer.pack(&PackOptions::default().with_renumber_from(1)).unwrap();
        peer.send(b"SECRET\r").await.unwrap();
        peer
    };
    let (result, _peer) = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        tokio::join!(state.read_action_message(&mut base, 1), caller)
    }).await.unwrap();
    assert!(result.unwrap().is_none());
    assert_eq!(state.session.last_password, "SECRET", "reject the stale snapshot only after successful password input");
    assert_eq!(base.read_message(1).unwrap().text(), &BString::from("other secret"));
}

#[test]
fn header_edit_rejects_concurrent_receipt_and_never_resurrects_deleted_mail() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("source");
    let mut base = JamMessageBase::create(&path).unwrap();
    base.write_message(&message_fixture()).unwrap();
    let original = base.read_header(1).unwrap();
    let mut draft = original.clone();
    draft.set_subject(BString::from("Edited subject"));
    let mut writer = JamMessageBase::open(&path).unwrap();
    let mut receipt = original.clone();
    receipt.times_read += 1;
    receipt.date_received += 1;
    receipt.attributes |= attributes::MSG_READ;
    raw::update_header(&mut writer, 1, &receipt).unwrap();
    let before_len = std::fs::metadata(path.with_extension("jhr")).unwrap().len();
    assert!(replace_header(&mut base, 1, &original, &draft).is_err());
    assert_eq!(header_bytes(&receipt), header_bytes(&base.read_header(1).unwrap()));
    assert_eq!(before_len, std::fs::metadata(path.with_extension("jhr")).unwrap().len());
    writer.delete_message(1).unwrap();
    let deleted_len = std::fs::metadata(path.with_extension("jhr")).unwrap().len();
    assert!(replace_header(&mut base, 1, &original, &draft).is_err());
    assert!(matches!(base.read_header(1), Err(jamjam::Error::Jam(jamjam::jam::JamError::MessageDeleted))));
    assert_eq!(deleted_len, std::fs::metadata(path.with_extension("jhr")).unwrap().len());
}

#[test]
fn header_edit_updates_one_field_and_recipient_index_when_unchanged() {
    let temp = tempfile::tempdir().unwrap();
    let mut base = JamMessageBase::create(temp.path().join("source")).unwrap();
    base.write_message(&message_fixture()).unwrap();
    let original = base.read_message(1).unwrap();
    let mut draft = original.header().clone();
    replace_sub_field(&mut draft, SubfieldType::RecvName, "NEW READER");
    replace_header(&mut base, 1, original.header(), &draft).unwrap();
    assert_eq!(header_bytes(&draft), header_bytes(&base.read_header(1).unwrap()));
    assert_eq!(base.read_message(1).unwrap().text(), original.text());
    assert_eq!(base.search_to(&BString::from("NEW READER")).unwrap(), vec![1]);
    assert!(base.search_to(&BString::from("READER")).unwrap().is_empty());
}

#[test]
fn body_edit_and_move_reject_in_place_text_changes_with_an_unchanged_header() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("source");
    let mut base = JamMessageBase::create(&path).unwrap();
    base.write_message(&message_fixture()).unwrap();
    let original = action_snapshot(&mut base, 1, None).unwrap();
    let replacement = vec![b'X'; original.text().len()];
    base.transaction(|base| {
        std::fs::write(base.path().with_extension("jdt"), &replacement)?;
        Ok(())
    }).unwrap();
    let draft = JamMessage::from_stored(original.header().clone(), BString::from("new edit"));
    assert!(replace_message(&mut base, 1, &original, &draft).is_err());
    assert!(delete_unchanged_message(&mut base, 1, &original).is_err());
    assert_eq!(header_bytes(original.header()), header_bytes(&base.read_header(1).unwrap()));
    assert_eq!(std::fs::read(path.with_extension("jdt")).unwrap(), replacement);
}

#[tokio::test]
async fn move_retains_both_copies_on_source_header_body_or_pack_conflict() {
    use jamjam::jam::pack::PackOptions;
    for change in 0..3 {
        let temp = tempfile::tempdir().unwrap();
        let source_path = temp.path().join("source");
        let target_path = temp.path().join("target");
        let mut source = JamMessageBase::create(&source_path).unwrap();
        source.write_message(&JamMessage::default().with_text(BString::from("padding"))).unwrap();
        source.write_message(&message_fixture()).unwrap();
        let original = action_snapshot(&mut source, 2, None).unwrap();
        let draft = transfer_draft(JamMessage::from_stored(original.header().clone(), original.text().clone()), false);
        let mut writer = JamMessageBase::open(&source_path).unwrap();
        let result = finish_transfer(async {
            let mut target = JamMessageBase::create(&target_path)?;
            target.write_message(&draft)?;
            match change {
                0 => { raw::set_attributes(&mut writer, 2, attributes::MSG_READ, 0)?; }
                1 => writer.transaction(|base| {
                    use std::io::{Seek, SeekFrom};
                    let mut file = std::fs::OpenOptions::new().write(true).open(base.path().with_extension("jdt"))?;
                    file.seek(SeekFrom::Start(original.header().offset as u64))?;
                    file.write_all(&vec![b'X'; original.text().len()])?;
                    Ok(())
                })?,
                _ => { writer.delete_message(1)?; writer.pack(&PackOptions::default())?; }
            }
            Ok(())
        }, &target_path, true, || delete_unchanged_message(&mut source, 2, &original)).await;
        assert!(matches!(result, Err(TransferFailure::Source(_))));
        assert!(source.read_header(2).is_ok());
        assert_transfer_metadata(&original, &JamMessageBase::open(&target_path).unwrap().read_message(1).unwrap(), false);
    }
}

#[tokio::test]
async fn move_requires_every_destination_file_to_sync_before_source_deletion() {
    for extension in ["jhr", "jdt", "jdx"] {
        let temp = tempfile::tempdir().unwrap();
        let target_path = temp.path().join("target");
        let mut source = JamMessageBase::create(temp.path().join("source")).unwrap();
        source.write_message(&message_fixture()).unwrap();
        let original = action_snapshot(&mut source, 1, None).unwrap();
        let draft = transfer_draft(JamMessage::from_stored(original.header().clone(), original.text().clone()), false);
        let result = finish_transfer(async {
            let mut target = JamMessageBase::create(&target_path)?;
            target.write_message(&draft)?;
            // Missing files must not be silently ignored by a durability barrier.
            std::fs::rename(target_path.with_extension(extension), target_path.with_extension("saved"))?;
            Ok(())
        }, &target_path, true, || panic!("source deletion must not run after a sync failure")).await;
        assert!(matches!(result, Err(TransferFailure::Destination(_))));
        assert_eq!(source.read_message(1).unwrap().text(), original.text());
        std::fs::rename(target_path.with_extension("saved"), target_path.with_extension(extension)).unwrap();
        assert_transfer_metadata(&original, &JamMessageBase::open(&target_path).unwrap().read_message(1).unwrap(), false);
    }
}

#[tokio::test]
async fn actual_move_action_syncs_and_deletes_only_the_unchanged_source() {
    let temp = tempfile::tempdir().unwrap();
    let (mut state, _peer) = action_state(temp.path()).await;
    let mut source = JamMessageBase::create(temp.path().join("source")).unwrap();
    source.write_message(&message_fixture()).unwrap();
    let original = action_snapshot(&mut source, 1, None).unwrap();
    assert!(state.copy_message_to_conference(&mut source, 1, 1, 0, true).await.unwrap());
    assert!(matches!(source.read_header(1), Err(jamjam::Error::Jam(jamjam::jam::JamError::MessageDeleted))));
    let target = JamMessageBase::open(temp.path().join("target")).unwrap();
    assert_transfer_metadata(&original, &target.read_message(1).unwrap(), false);
}

async fn attachment_action_state(root: &Path) -> (IcyBoardState, icy_net::channel::ChannelConnection, PathBuf, PathBuf) {
    let (mut state, peer) = action_state(root).await;
    let source = root.join("source-attachments");
    let target = root.join("target-attachments");
    std::fs::create_dir(&source).unwrap();
    std::fs::create_dir(&target).unwrap();
    state.session.current_conference.attachment_location = source.clone();
    state.get_board().await.conferences[1].attachment_location = target.clone();
    (state, peer, source, target)
}

fn enclosed_message(names: &[&str]) -> JamMessage {
    names.iter().fold(message_fixture(), |message, name| {
        message.with_sub_field(MessageSubfield::new(SubfieldType::EnclFile, BString::from(*name)))
    })
}

#[tokio::test]
async fn attachment_partial_copy_failure_rolls_back_only_new_files() {
    let temp = tempfile::tempdir().unwrap();
    let (mut state, _peer, source, target) = attachment_action_state(temp.path()).await;
    for name in ["shared.zip", "new.zip", "conflict.zip"] {
        std::fs::write(source.join(name), name.as_bytes()).unwrap();
    }
    std::fs::write(target.join("shared.zip"), b"shared.zip").unwrap();
    std::fs::write(target.join("conflict.zip"), b"other file").unwrap();
    let message = enclosed_message(&["shared.zip", "new.zip", "conflict.zip"]);
    assert!(state.copy_action_attachments(&message, 1, 0).await.is_err());
    assert!(!target.join("new.zip").exists());
    assert_eq!(std::fs::read(target.join("shared.zip")).unwrap(), b"shared.zip");
    assert_eq!(std::fs::read(target.join("conflict.zip")).unwrap(), b"other file");
    assert_eq!(std::fs::read_dir(&source).unwrap().count(), 3);
    assert_eq!(std::fs::read_dir(&target).unwrap().count(), 2, "no staging files left behind");
}

#[tokio::test]
async fn identical_existing_attachment_is_reused_without_overwrite_or_rollback() {
    let temp = tempfile::tempdir().unwrap();
    let (mut state, _peer, source, target) = attachment_action_state(temp.path()).await;
    std::fs::write(source.join("shared.zip"), b"same contents").unwrap();
    std::fs::write(target.join("shared.zip"), b"same contents").unwrap();
    let before = std::fs::metadata(target.join("shared.zip")).unwrap();
    let message = enclosed_message(&["shared.zip", "shared.zip"]);
    let copies = state.copy_action_attachments(&message, 1, 0).await.unwrap();
    assert!(copies.files.is_empty());
    drop(copies);
    let after = std::fs::metadata(target.join("shared.zip")).unwrap();
    assert_eq!(before.modified().unwrap(), after.modified().unwrap());
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(before.ino(), after.ino());
    }
    assert_eq!(std::fs::read(target.join("shared.zip")).unwrap(), b"same contents");
}

#[tokio::test]
async fn attachment_copy_guard_rolls_back_on_drop_but_keeps_committed_copies() {
    let temp = tempfile::tempdir().unwrap();
    let (mut state, _peer, source, target) = attachment_action_state(temp.path()).await;
    std::fs::write(source.join("file.zip"), b"enclosure").unwrap();
    let message = enclosed_message(&["file.zip", "file.zip"]);
    let copies = state.copy_action_attachments(&message, 1, 0).await.unwrap();
    assert_eq!(copies.files.len(), 1);
    drop(copies);
    assert!(!target.join("file.zip").exists());
    assert!(source.join("file.zip").exists());
    let mut copies = state.copy_action_attachments(&message, 1, 0).await.unwrap();
    copies.commit();
    drop(copies);
    assert_eq!(std::fs::read(target.join("file.zip")).unwrap(), b"enclosure");
}

#[tokio::test]
async fn actual_move_rolls_back_attachment_copies_on_destination_open_or_append_failure() {
    for append_failure in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let (mut state, _peer, from, to) = attachment_action_state(temp.path()).await;
        std::fs::write(from.join("file.zip"), b"enclosure").unwrap();
        let mut source = JamMessageBase::create(temp.path().join("source")).unwrap();
        source.write_message(&enclosed_message(&["file.zip"])).unwrap();
        if append_failure {
            JamMessageBase::create(temp.path().join("target")).unwrap();
            std::fs::remove_file(temp.path().join("target.jdt")).unwrap();
            std::fs::create_dir(temp.path().join("target.jdt")).unwrap();
        } else {
            std::fs::write(temp.path().join("target.jhr"), b"invalid JAM").unwrap();
        }
        assert!(!state.copy_message_to_conference(&mut source, 1, 1, 0, true).await.unwrap());
        assert!(source.read_header(1).is_ok());
        assert_eq!(std::fs::read(from.join("file.zip")).unwrap(), b"enclosure");
        assert_eq!(std::fs::read_dir(to).unwrap().count(), 0);
    }
}

#[tokio::test]
async fn appended_attachment_survives_cancellation_during_post_save_bookkeeping() {
    let temp = tempfile::tempdir().unwrap();
    let (mut state, _peer, source, target) = attachment_action_state(temp.path()).await;
    std::fs::write(source.join("file.zip"), b"enclosure").unwrap();
    let message = enclosed_message(&["file.zip"]);
    let mut copies = state.copy_action_attachments(&message, 1, 0).await.unwrap();
    let board = state.board.clone();
    let blocked_statistics = board.lock().await;
    let target_path = temp.path().join("target");
    {
        let mut save = std::pin::pin!(state.send_action_message(1, 0, &target_path, message, IceText::MessageCopied, &mut copies));
        std::future::poll_fn(|cx| {
            assert!(save.as_mut().poll(cx).is_pending());
            std::task::Poll::Ready(())
        }).await;
        // Dropping this suspended future simulates a disconnect/cancellation.
    }
    drop(blocked_statistics);
    assert!(copies.files.is_empty(), "append must commit before the first bookkeeping await");
    drop(copies);
    assert!(JamMessageBase::open(&target_path).unwrap().read_header(1).is_ok());
    assert_eq!(std::fs::read(target.join("file.zip")).unwrap(), b"enclosure");
}

#[tokio::test]
async fn edit_attachment_cleanup_preserves_originals_and_rolls_back_conflicts() {
    for conflict in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let (mut state, _peer) = action_state(temp.path()).await;
        state.session.current_conference.attachment_location = temp.path().to_path_buf();
        let old_name = "icb-attach-original.bin";
        let new_name = "icb-attach-new.bin";
        std::fs::write(temp.path().join(old_name), b"old enclosure").unwrap();
        let mut base = JamMessageBase::create(temp.path().join("source")).unwrap();
        base.write_message(&enclosed_message(&[old_name])).unwrap();
        let original = action_snapshot(&mut base, 1, None).unwrap();
        let mut cleanup = state.message_attachment_cleanup(&original);
        std::fs::write(temp.path().join(new_name), b"new enclosure").unwrap();
        let draft = JamMessage::from_stored(original.header().clone(), BString::from("edited body"))
            .with_sub_field(MessageSubfield::new(SubfieldType::EnclFile, BString::from(new_name)));
        cleanup.track(&draft).unwrap();
        if conflict {
            base.delete_message(1).unwrap();
            assert!(replace_message(&mut base, 1, &original, &draft).is_err());
        } else {
            replace_message(&mut base, 1, &original, &draft).unwrap();
            cleanup.commit();
            assert_eq!(base.read_message(1).unwrap().text(), draft.text());
        }
        drop(cleanup);
        assert!(temp.path().join(old_name).exists());
        assert_eq!(temp.path().join(new_name).exists(), !conflict);
    }
}

#[tokio::test]
async fn actual_edit_sk_saves_without_killing_existing_mail() {
    use crate::icy_board::{icb_text::DEFAULT_DISPLAY_TEXT, state::{KeyChar, KeySource}, user_base::FSEMode};
    let temp = tempfile::tempdir().unwrap();
    let (mut state, _peer) = action_state(temp.path()).await;
    state.get_board().await.default_display_text = DEFAULT_DISPLAY_TEXT.clone();
    state.session.fse_mode = FSEMode::No;
    state.char_buffer.extend("\rSK\r".chars().map(|ch| KeyChar::new(KeySource::User, ch)));
    let mut base = JamMessageBase::create(temp.path().join("source")).unwrap();
    base.write_message(&message_fixture()).unwrap();
    let before = base.read_header(1).unwrap().offset;
    let result = tokio::time::timeout(std::time::Duration::from_secs(3), state.edit_read_message(&mut base, 1)).await.unwrap().unwrap();
    assert!(matches!(result, AfterAction::Redisplay));
    assert!(base.read_header(1).unwrap().offset > before, "EDIT must actually replace the body");
    assert_eq!(base.highest_message_number(), 1, "EDIT must not create another indexed message");
    assert!(matches!(after_existing_edit(EditResult::SendKill), AfterAction::Redisplay));
    assert!(matches!(after_existing_edit(EditResult::SendNext), AfterAction::Next));
}

#[tokio::test]
async fn actual_header_edit_rejects_changes_made_during_new_info_prompt() {
    use icy_net::Connection;
    use crate::icy_board::{icb_text::DEFAULT_DISPLAY_TEXT, state::{KeyChar, KeySource}};
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("source");
    let (mut state, mut peer) = action_state(temp.path()).await;
    const PROMPT: &str = "[read-action-new-info]";
    // Install the marker in the active session copy used by input_field.
    state.display_text = DEFAULT_DISPLAY_TEXT.clone();
    state.display_text.update_record_number(IceText::NewInfo as usize, PROMPT).unwrap();
    state.char_buffer.extend("S\r".chars().map(|ch| KeyChar::new(KeySource::User, ch)));
    let mut base = JamMessageBase::create(&path).unwrap();
    base.write_message(&message_fixture()).unwrap();
    let mut writer = JamMessageBase::open(&path).unwrap();
    let mut concurrent = writer.read_header(1).unwrap();
    concurrent.times_read += 1;
    concurrent.set_subject(BString::from("Concurrent subject"));
    let caller = async {
        let mut output = Vec::new();
        while !output.windows(PROMPT.len()).any(|bytes| bytes == PROMPT.as_bytes()) {
            let mut bytes = [0; 4096];
            let count = peer.read(&mut bytes).await.unwrap();
            assert_ne!(count, 0);
            output.extend_from_slice(&bytes[..count]);
        }
        assert!(writer.try_lock().unwrap(), "header editor must release JAM while prompting");
        writer.unlock();
        raw::update_header(&mut writer, 1, &concurrent).unwrap();
        peer.send(b"My stale subject\r").await.unwrap();
        peer
    };
    let (result, _peer) = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        tokio::join!(state.edit_header(&mut base, 1), caller)
    }).await.unwrap();
    result.unwrap();
    assert_eq!(state.session.last_answer.as_deref(), Some("My stale subject"), "the proposed edit must be read before rejecting it");
    assert_eq!(header_bytes(&concurrent), header_bytes(&base.read_header(1).unwrap()));
}

#[tokio::test]
async fn copied_attachments_remain_referenced_when_move_source_comparison_fails() {
    let temp = tempfile::tempdir().unwrap();
    let (mut state, _peer, from, to) = attachment_action_state(temp.path()).await;
    std::fs::write(from.join("file.zip"), b"enclosure").unwrap();
    let source_path = temp.path().join("source");
    let target_path = temp.path().join("target");
    let mut source = JamMessageBase::create(&source_path).unwrap();
    source.write_message(&enclosed_message(&["file.zip"])).unwrap();
    let original = action_snapshot(&mut source, 1, None).unwrap();
    let draft = transfer_draft(JamMessage::from_stored(original.header().clone(), original.text().clone()), false);
    let mut copies = state.copy_action_attachments(&draft, 1, 0).await.unwrap();
    let mut writer = JamMessageBase::open(&source_path).unwrap();
    let result = finish_transfer(async {
        state.send_action_message(1, 0, &target_path, draft, IceText::MessageMoved, &mut copies).await?;
        raw::set_attributes(&mut writer, 1, attributes::MSG_READ, 0)?;
        Ok(())
    }, &target_path, true, || delete_unchanged_message(&mut source, 1, &original)).await;
    assert!(matches!(result, Err(TransferFailure::Source(_))));
    drop(copies);
    assert!(source.read_header(1).is_ok());
    assert!(JamMessageBase::open(&target_path).unwrap().read_header(1).is_ok());
    assert_eq!(std::fs::read(from.join("file.zip")).unwrap(), b"enclosure");
    assert_eq!(std::fs::read(to.join("file.zip")).unwrap(), b"enclosure");
}

#[cfg(unix)]
#[tokio::test]
async fn identical_destination_symlink_is_not_an_acceptable_existing_attachment() {
    let temp = tempfile::tempdir().unwrap();
    let (mut state, _peer, from, to) = attachment_action_state(temp.path()).await;
    std::fs::write(from.join("file.zip"), b"enclosure").unwrap();
    std::os::unix::fs::symlink(from.join("file.zip"), to.join("file.zip")).unwrap();
    assert!(state.copy_action_attachments(&enclosed_message(&["file.zip"]), 1, 0).await.is_err());
    assert!(std::fs::symlink_metadata(to.join("file.zip")).unwrap().file_type().is_symlink());
    assert_eq!(std::fs::read(from.join("file.zip")).unwrap(), b"enclosure");
}