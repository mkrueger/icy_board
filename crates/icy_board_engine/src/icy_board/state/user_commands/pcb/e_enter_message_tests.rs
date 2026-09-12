//! Reply-entry regressions owned by reply_message, not the reader dispatcher.
use super::*;
use std::{path::Path, sync::Arc, time::Duration};

use crate::icy_board::{
    IcyBoard,
    bbs::BBS,
    conferences::Conference,
    icb_text::DEFAULT_DISPLAY_TEXT,
    message_area::{AreaList, MessageArea},
    security_expr::SecurityExpression,
    state::{KeyChar, KeySource},
    user_base::{FSEMode, User},
};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection};
use jamjam::jam::pack::PackOptions;
use tokio::{sync::Mutex, time::timeout};

const PASSWORD_PROMPT: &str = "[reply-password-test]";
const COMPOSE_PROMPT: &str = "[reply-compose-test]";

fn source_message(body: &str) -> JamMessage {
    JamMessage::default()
        .with_from(BString::from("AUTHOR"))
        .with_to(BString::from("READER"))
        .with_subject(BString::from("Source subject"))
        .with_msg_id(BString::from("source-message-id"))
        .with_attributes(attributes::MSG_PRIVATE | attributes::MSG_LOCAL)
        .with_text(BString::from(body))
}

fn has_reply_date(header: &JamMessageHeader) -> bool {
    header
        .sub_fields
        .iter()
        .any(|field| field.field_type() == SubfieldType::FTSKludge && field.content().starts_with(b"ICYBOARD-REPLY-DATE: "))
}

fn header_bytes(header: &JamMessageHeader) -> Vec<u8> {
    let mut bytes = Vec::new();
    header.write(&mut bytes).unwrap();
    bytes
}

#[test]
fn authorized_snapshot_reads_body_and_releases_the_read_lock() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("source");
    let mut base = JamMessageBase::create(&path).unwrap();
    base.write_message(&source_message("approved body")).unwrap();
    let approved = base.read_transaction(|base| base.read_header(1)).unwrap();
    let (fresh, body) = read_authorized_reply(&mut base, &approved, "READER", "", false).unwrap().unwrap();
    assert_eq!(body, BString::from("approved body"));
    assert_eq!(header_bytes(&fresh), header_bytes(&approved));
    let mut writer = JamMessageBase::open(&path).unwrap();
    assert!(writer.try_lock().unwrap(), "no read lock may survive into an await");
    writer.unlock();
}

#[test]
fn post_password_snapshot_rejects_changed_password_recipient_or_visibility_before_reading_body() {
    for change in 0..3 {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("source");
        let mut base = JamMessageBase::create(&path).unwrap();
        base.write_message(&source_message("secret body").with_password(&BString::from("OLD"))).unwrap();
        let approved = base.read_header(1).unwrap();
        let mut changed = approved.clone();
        match change {
            0 => changed.password_crc = JamMessageBase::crc(&BString::from("NEW")),
            1 => changed.set_to(BString::from("SOMEONE ELSE")),
            _ => changed.attributes |= attributes::MSG_NODISP,
        }
        jamjam::jam::raw::update_header(&mut base, 1, &changed).unwrap();
        // If the guard reads a body at all, this missing text file makes it fail.
        std::fs::remove_file(path.with_extension("jdt")).unwrap();
        assert!(read_authorized_reply(&mut base, &approved, "READER", "", false).unwrap().is_none());
    }
}

#[test]
fn snapshot_rechecks_private_access_even_for_an_unchanged_header() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("source");
    let mut base = JamMessageBase::create(&path).unwrap();
    base.write_message(&source_message("private body")).unwrap();
    let approved = base.read_header(1).unwrap();
    std::fs::remove_file(path.with_extension("jdt")).unwrap();
    assert!(read_authorized_reply(&mut base, &approved, "STRANGER", "", false).unwrap().is_none());
}

#[test]
fn pack_between_approval_and_quote_never_reads_the_old_text_offset() {
    let root = tempfile::tempdir().unwrap();
    let mut base = JamMessageBase::create(root.path().join("source")).unwrap();
    base.write_message(&source_message("padding removed by pack")).unwrap();
    base.write_message(&source_message("approved body")).unwrap();
    let approved = base.read_header(2).unwrap();
    base.delete_message(1).unwrap();
    base.pack(&PackOptions::default()).unwrap();
    assert_ne!(base.read_header(2).unwrap().offset, approved.offset);
    assert!(read_authorized_reply(&mut base, &approved, "READER", "", false).unwrap().is_none());
}

#[test]
fn reply_date_skips_a_renumbered_replacement_even_when_its_crc_matches() {
    let root = tempfile::tempdir().unwrap();
    let mut base = JamMessageBase::create(root.path().join("source")).unwrap();
    base.write_message(&source_message("original body")).unwrap();
    // Deliberately duplicate MsgID/CRC: identity checking must not use CRC alone.
    base.write_message(&source_message("unrelated body").with_subject(BString::from("Other subject")))
        .unwrap();
    let approved = base.read_header(1).unwrap();
    let (approved, body) = read_authorized_reply(&mut base, &approved, "READER", "", false).unwrap().unwrap();
    base.delete_message(1).unwrap();
    base.pack(&PackOptions::default().with_renumber_from(1)).unwrap();
    let before = header_bytes(&base.read_header(1).unwrap());
    record_reply_date(&mut base, &approved, &body).unwrap();
    assert_eq!(before, header_bytes(&base.read_header(1).unwrap()));
    assert!(!has_reply_date(&base.read_header(1).unwrap()));
}

#[test]
fn reply_date_skips_deleted_source_and_same_length_body_replacement() {
    for deleted in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("source");
        let mut base = JamMessageBase::create(&path).unwrap();
        base.write_message(&source_message("original")).unwrap();
        let approved = base.read_header(1).unwrap();
        if deleted {
            base.delete_message(1).unwrap();
            record_reply_date(&mut base, &approved, &BString::from("original")).unwrap();
            assert!(reply_header(&base, 1).unwrap().is_none());
        } else {
            std::fs::write(path.with_extension("jdt"), b"replaced").unwrap();
            record_reply_date(&mut base, &approved, &BString::from("original")).unwrap();
            assert_eq!(header_bytes(&approved), header_bytes(&base.read_header(1).unwrap()));
        }
    }
}

#[test]
fn reply_date_preserves_concurrent_read_receipt_and_message_body() {
    let root = tempfile::tempdir().unwrap();
    let mut base = JamMessageBase::create(root.path().join("source")).unwrap();
    base.write_message(&source_message("original body")).unwrap();
    let approved = base.read_header(1).unwrap();
    let mut read = approved.clone();
    read.date_received = 123456;
    read.times_read = 9;
    read.attributes |= attributes::MSG_READ;
    jamjam::jam::raw::update_header(&mut base, 1, &read).unwrap();
    record_reply_date(&mut base, &approved, &BString::from("original body")).unwrap();
    let saved = base.read_message(1).unwrap();
    assert!(has_reply_date(saved.header()));
    assert_eq!(saved.header().date_received, 123456);
    assert_eq!(saved.header().times_read, 9);
    assert!(saved.header().is_read());
    assert_eq!(saved.text(), &BString::from("original body"));
}

fn base_header(path: &Path, number: u32) -> JamMessageHeader {
    JamMessageBase::open(path).unwrap().read_header(number).unwrap()
}

async fn input_state(root: &Path, input: &str) -> (IcyBoardState, ChannelConnection) {
    let bbs = Arc::new(Mutex::new(BBS::new(1)));
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let (peer, connection) = ChannelConnection::create_pair();
    let mut board = IcyBoard::new();
    board.root_path = root.to_path_buf();
    board.config.paths.statistics_file = root.join("statistics.toml");
    board.config.paths.email_msgbase = root.join("email");
    board.config.message.validate_to_name = false;
    board.default_display_text = DEFAULT_DISPLAY_TEXT.clone();
    board
        .default_display_text
        .update_record_number(IceText::PasswordToReadMessage as usize, PASSWORD_PROMPT)
        .unwrap();
    board
        .default_display_text
        .update_record_number(IceText::TextEntryCommand as usize, COMPOSE_PROMPT)
        .unwrap();
    board.users.new_user(User {
        name: "READER".into(),
        security_level: 10,
        ..Default::default()
    });
    let caller = board.users[0].clone();
    board.conferences.clear();
    board.conferences.push(Conference {
        long_to_names: true,
        disallow_private_msgs: true,
        sec_request_rr: SecurityExpression::from_req_security(255),
        areas: Some(Arc::new(AreaList::new(vec![MessageArea {
            path: root.join("area"),
            ..Default::default()
        }]))),
        ..Default::default()
    });
    let conference = board.conferences[0].clone();
    let mut state = IcyBoardState::new(bbs, Arc::new(Mutex::new(board)), nodes, node, Box::new(connection)).await;
    state.session.current_user = Some(caller);
    state.session.user_name = "READER".into();
    state.session.alias_name.clear();
    state.session.cur_user_id = 0;
    state.session.cur_security = 10;
    state.session.user_command_level.cmd_e = SecurityExpression::from_req_security(0);
    state.session.page_len = 0;
    state.session.fse_mode = FSEMode::No;
    state.session.current_conference = conference;
    state.char_buffer.extend(input.chars().map(|ch| KeyChar::new(KeySource::User, ch)));
    (state, peer)
}

#[tokio::test]
async fn email_reply_and_ro_quote_the_actual_base_stay_private_and_do_not_touch_current_area() {
    for ask_other in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let email_path = root.path().join("email");
        let area_path = root.path().join("area");
        let mut email = JamMessageBase::create(&email_path).unwrap();
        email.write_message(&source_message("email quote")).unwrap();
        let mut area = JamMessageBase::create(&area_path).unwrap();
        area.write_message(&source_message("WRONG AREA BODY")).unwrap();
        let area_before = header_bytes(&area.read_header(1).unwrap());
        let input = if ask_other { "OTHER\r\r\rQ 1 1\rSN\r" } else { "\rQ 1 1\rSN\r" };
        let (mut state, _peer) = input_state(root.path(), input).await;
        // Email must not depend on whether the current conference has an area
        // or allows posting. Its destination is the email base, conf=-1/area=0.
        state.session.current_conference.is_read_only = true;
        state.session.current_conference.areas = None;
        let result = timeout(Duration::from_secs(3), state.reply_from_base(&email_path, 1, ask_other, true))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result, EditResult::SendNext);
        let email = JamMessageBase::open(&email_path).unwrap();
        let reply = email.read_message(2).unwrap();
        assert_eq!(reply.text(), &BString::from("-> email quote"));
        assert_eq!(reply.to().unwrap().to_string(), if ask_other { "OTHER" } else { "AUTHOR" });
        assert!(reply.header().is_private());
        assert_eq!(reply.header().reply_to, 1);
        assert!(
            reply
                .header()
                .sub_fields
                .iter()
                .any(|field| field.field_type() == SubfieldType::ReplyID && field.content() == "source-message-id")
        );
        assert!(has_reply_date(&email.read_header(1).unwrap()));
        assert_eq!(area_before, header_bytes(&base_header(&area_path, 1)));
        assert!(JamMessageBase::open(&area_path).unwrap().read_header(2).is_err());
    }
}

#[tokio::test]
async fn standalone_sk_kills_source_keeps_area_thread_and_leaves_email_alone() {
    let root = tempfile::tempdir().unwrap();
    let mut area = JamMessageBase::create(root.path().join("area")).unwrap();
    area.write_message(&source_message("area quote")).unwrap();
    let mut email = JamMessageBase::create(root.path().join("email")).unwrap();
    email.write_message(&source_message("WRONG EMAIL BODY")).unwrap();
    let email_before = header_bytes(&email.read_header(1).unwrap());
    let (mut state, _peer) = input_state(root.path(), "\rQ 1 1\rSK\r").await;
    state.session.user_command_level.cmd_k = SecurityExpression::from_req_security(0);
    state.session.tokens.push_back("1".into());
    timeout(Duration::from_secs(3), state.reply_message_command()).await.unwrap().unwrap();
    let area = JamMessageBase::open(root.path().join("area")).unwrap();
    let reply = area.read_message(2).unwrap();
    assert_eq!(reply.text(), &BString::from("-> area quote"));
    assert_eq!(reply.header().reply_to, 1);
    assert!(matches!(area.read_header(1), Err(jamjam::Error::Jam(jamjam::jam::JamError::MessageDeleted))));
    assert_eq!(email_before, header_bytes(&email.read_header(1).unwrap()));
}

#[tokio::test]
async fn reply_sk_returns_save_only_when_kill_command_or_ownership_is_denied() {
    for command_denied in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("area");
        let mut base = JamMessageBase::create(&path).unwrap();
        let mut message = source_message("original");
        if !command_denied {
            // with_to appends a field; it would leave READER as the first
            // recipient and legitimately allow this caller to kill the source.
            let mut header = message.header().clone();
            header.set_to(BString::from("SOMEONE ELSE"));
            header.attributes = attributes::MSG_LOCAL;
            message = JamMessage::from_stored(header, message.text().clone());
        }
        base.write_message(&message).unwrap();
        let original = base.read_header(1).unwrap();
        assert_eq!(original.to().unwrap().to_string(), if command_denied { "READER" } else { "SOMEONE ELSE" });
        assert_eq!(original.from().unwrap().to_string(), "AUTHOR");
        let before = header_bytes(&original);
        let (mut state, _peer) = input_state(root.path(), "\rQ 1 1\rSK\r").await;
        state.session.is_sysop = false;
        state.get_board().await.config.sysop_command_level.read_all_mail = SecurityExpression::from_req_security(255);
        state.session.user_command_level.cmd_k = SecurityExpression::from_req_security(if command_denied { 255 } else { 0 });
        assert!(!state.is_sysop());
        assert!(
            !state
                .get_board()
                .await
                .config
                .sysop_command_level
                .read_all_mail
                .session_can_access(&state.session)
        );
        assert_eq!(state.session.user_command_level.cmd_k.session_can_access(&state.session), !command_denied);
        let result = timeout(Duration::from_secs(3), state.reply_current_message(1, false)).await.unwrap().unwrap();
        assert_eq!(result, EditResult::SendMessage, "command_denied={command_denied}");
        let base = JamMessageBase::open(&path).unwrap();
        assert_eq!(before, header_bytes(&base.read_header(1).unwrap()));
        assert_eq!(base.read_message(1).unwrap().text(), &BString::from("original"));
        assert_eq!(base.read_message(2).unwrap().text(), &BString::from("-> original"));
    }
}

#[tokio::test]
async fn reply_sk_rejects_source_modified_or_identically_replaced_during_composition() {
    for change in 0..3 {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("area");
        let mut base = JamMessageBase::create(&path).unwrap();
        let message = source_message("original");
        base.write_message(&message).unwrap();
        if change == 2 {
            base.write_message(&message).unwrap();
        }
        let approved = header_bytes(&base.read_header(1).unwrap());
        let (mut state, mut peer) = input_state(root.path(), "\r").await;
        state.session.user_command_level.cmd_k = SecurityExpression::from_req_security(0);
        let caller = async {
            let mut output = Vec::new();
            while !output.windows(COMPOSE_PROMPT.len()).any(|bytes| bytes == COMPOSE_PROMPT.as_bytes()) {
                let mut bytes = [0; 4096];
                let count = peer.read(&mut bytes).await.unwrap();
                assert_ne!(count, 0);
                output.extend_from_slice(&bytes[..count]);
            }
            assert!(base.try_lock().unwrap(), "composition must not retain a JAM lock");
            base.unlock();
            match change {
                0 => {
                    let mut header = base.read_header(1).unwrap();
                    header.set_subject(BString::from("Concurrent edit"));
                    jamjam::jam::raw::update_header(&mut base, 1, &header).unwrap();
                }
                1 => base
                    .transaction(|_| {
                        // Equal length, same header and generation: compare the body too.
                        std::fs::write(path.with_extension("jdt"), b"replaced")?;
                        Ok(())
                    })
                    .unwrap(),
                _ => {
                    base.delete_message(1).unwrap();
                    base.pack(&PackOptions::default().with_renumber_from(1)).unwrap();
                    assert_eq!(
                        approved,
                        header_bytes(&base.read_header(1).unwrap()),
                        "only generation distinguishes this replacement"
                    );
                }
            }
            let before = header_bytes(&base.read_header(1).unwrap());
            peer.send(b"Q 1 1\rSK\r").await.unwrap();
            (peer, before)
        };
        let (result, (_peer, before)) = timeout(Duration::from_secs(3), async { tokio::join!(state.reply_current_message(1, false), caller) })
            .await
            .unwrap();
        assert_eq!(result.unwrap(), EditResult::SendMessage);
        let base = JamMessageBase::open(&path).unwrap();
        assert_eq!(before, header_bytes(&base.read_header(1).unwrap()));
        assert_eq!(
            base.read_message(1).unwrap().text(),
            &BString::from(if change == 1 { "replaced" } else { "original" })
        );
        assert_eq!(base.read_message(2).unwrap().text(), &BString::from("-> original"));
    }
}

#[tokio::test]
async fn cross_base_reply_sk_kills_only_the_original_source() {
    let root = tempfile::tempdir().unwrap();
    let source_path = root.path().join("other-email");
    let mut source = JamMessageBase::create(&source_path).unwrap();
    source.write_message(&source_message("source quote")).unwrap();
    let mut email = JamMessageBase::create(root.path().join("email")).unwrap();
    email.write_message(&source_message("unrelated destination")).unwrap();
    let destination_before = header_bytes(&email.read_header(1).unwrap());
    let (mut state, _peer) = input_state(root.path(), "\rQ 1 1\rSK\r").await;
    state.session.user_command_level.cmd_k = SecurityExpression::from_req_security(0);
    let result = timeout(Duration::from_secs(3), state.reply_from_base(&source_path, 1, false, true))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result, EditResult::SendKill);
    let source = JamMessageBase::open(&source_path).unwrap();
    assert!(matches!(source.read_header(1), Err(jamjam::Error::Jam(jamjam::jam::JamError::MessageDeleted))));
    let email = JamMessageBase::open(root.path().join("email")).unwrap();
    assert_eq!(destination_before, header_bytes(&email.read_header(1).unwrap()));
    assert_eq!(email.read_message(2).unwrap().text(), &BString::from("-> source quote"));
    assert_eq!(email.read_header(2).unwrap().reply_to, 0);
}

#[tokio::test]
async fn cross_base_reply_drops_numeric_link_but_keeps_reply_id_and_updates_source_only() {
    let root = tempfile::tempdir().unwrap();
    let source_path = root.path().join("other-email");
    let mut source = JamMessageBase::create(&source_path).unwrap();
    source.write_message(&source_message("other base quote")).unwrap();
    let mut email = JamMessageBase::create(root.path().join("email")).unwrap();
    email.write_message(&source_message("unrelated destination")).unwrap();
    let destination_before = header_bytes(&email.read_header(1).unwrap());
    let (mut state, _peer) = input_state(root.path(), "\rQ 1 1\rS\r").await;
    assert_eq!(
        timeout(Duration::from_secs(3), state.reply_from_base(&source_path, 1, false, true))
            .await
            .unwrap()
            .unwrap(),
        EditResult::SendMessage
    );
    let email = JamMessageBase::open(root.path().join("email")).unwrap();
    let reply = email.read_message(2).unwrap();
    assert_eq!(reply.header().reply_to, 0);
    assert_eq!(reply.text(), &BString::from("-> other base quote"));
    assert!(
        reply
            .header()
            .sub_fields
            .iter()
            .any(|field| field.field_type() == SubfieldType::ReplyID && field.content() == "source-message-id")
    );
    assert_eq!(destination_before, header_bytes(&email.read_header(1).unwrap()));
    assert!(has_reply_date(&base_header(&source_path, 1)));
}

#[tokio::test]
async fn pack_during_password_prompt_is_unlocked_and_aborts_without_quoting_replacement() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("email");
    let mut base = JamMessageBase::create(&path).unwrap();
    base.write_message(&source_message("approved secret").with_password(&BString::from("SECRET")))
        .unwrap();
    base.write_message(&source_message("replacement secret").with_to(BString::from("STRANGER")))
        .unwrap();
    let (mut state, mut peer) = input_state(root.path(), "").await;
    let caller = async {
        let mut output = Vec::new();
        while !output.windows(PASSWORD_PROMPT.len()).any(|bytes| bytes == PASSWORD_PROMPT.as_bytes()) {
            let mut bytes = [0; 4096];
            let count = peer.read(&mut bytes).await.unwrap();
            assert_ne!(count, 0);
            output.extend_from_slice(&bytes[..count]);
        }
        // A failed probe fails immediately instead of deadlocking the runtime.
        assert!(base.try_lock().unwrap(), "password input must not hold a JAM lock");
        base.unlock();
        base.delete_message(1).unwrap();
        base.pack(&PackOptions::default().with_renumber_from(1)).unwrap();
        peer.send(b"SECRET\r").await.unwrap();
        peer
    };
    let (result, _peer) = timeout(Duration::from_secs(3), async {
        tokio::join!(state.reply_from_base(&path, 1, false, true), caller)
    })
    .await
    .unwrap();
    assert_eq!(result.unwrap(), EditResult::Abort);
    let remaining = JamMessageBase::open(&path).unwrap().read_message(1).unwrap();
    assert_eq!(remaining.text(), &BString::from("replacement secret"));
    assert!(!has_reply_date(remaining.header()));
    assert!(JamMessageBase::open(&path).unwrap().read_header(2).is_err());
}
