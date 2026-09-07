use super::*;
use std::{path::Path, sync::Arc, time::Duration};

use crate::icy_board::state::user_commands::mods::messagereader::message_security::set_security_kind;
use crate::icy_board::{
    IcyBoard,
    bbs::BBS,
    conferences::Conference,
    icb_text::DEFAULT_DISPLAY_TEXT,
    message_area::{AreaList, MessageArea},
    security_expr::SecurityExpression,
    state::{KeyChar, KeySource},
    user_base::User,
};
use bstr::BString;
use icy_net::{Connection, ConnectionType, channel::ChannelConnection};
use jamjam::jam::{attributes, pack::PackOptions, raw};
use tokio::{sync::Mutex, time::timeout};

const PROMPT: &str = "[kill-password]";
const DENIED: &str = "[kill-denied]";
const MISSING: &str = "[kill-missing]";
const KILLED: &str = "[kill-done]";
const WRONG: &str = "[kill-wrong-password]";

fn message(private: bool, kind: char) -> JamMessage {
    let mut message = JamMessage::default()
        .with_from(BString::from("AUTHOR"))
        .with_to(BString::from("READER"))
        .with_subject(BString::from("Secret subject"))
        .with_msg_id(BString::from("same-id-deliberately"))
        .with_attributes(if private { attributes::MSG_PRIVATE } else { 0 })
        .with_text(BString::from("Secret body"));
    if kind != ' ' {
        message = message.with_password(&BString::from("SECRET"));
        let mut header = message.header().clone();
        set_security_kind(&mut header, kind == 'S');
        message = JamMessage::from_stored(header, message.text().clone());
    }
    message
}

fn access(user: &str) -> KillAccess<'_> {
    KillAccess {
        user,
        alias: "",
        command: true,
        read_all: false,
    }
}

fn header_bytes(header: &JamMessageHeader) -> Vec<u8> {
    let mut bytes = Vec::new();
    header.write(&mut bytes).unwrap();
    bytes
}

fn is_deleted(base: &JamMessageBase, number: u32) -> bool {
    matches!(base.read_header(number), Err(jamjam::Error::Jam(jamjam::jam::JamError::MessageDeleted)))
}

#[test]
fn original_kill_permission_matrix() {
    for private in [false, true] {
        for kind in [' ', 'S', 'G'] {
            let message = message(private, kind);
            for (user, expected) in [
                ("OTHER", KillPermission::Denied),
                ("READER", if kind == 'S' { KillPermission::Denied } else { KillPermission::Allowed }),
                (
                    "AUTHOR",
                    if kind == ' ' {
                        KillPermission::Allowed
                    } else {
                        KillPermission::SenderPassword
                    },
                ),
            ] {
                assert_eq!(
                    access(user).permission(message.header()),
                    expected,
                    "private={private}, kind={kind}, user={user}"
                );
            }
            let mut sysop = access("OTHER");
            sysop.read_all = true;
            assert_eq!(sysop.permission(message.header()), KillPermission::Allowed);
            sysop.command = false;
            assert_eq!(sysop.permission(message.header()), KillPermission::Denied, "read-all never bypasses cmd_k");
        }
    }
}

#[test]
fn aliases_generic_recipients_and_hidden_messages_fail_closed() {
    let message = message(true, ' ');
    let mut caller = access("OTHER");
    caller.alias = "reader";
    assert_eq!(caller.permission(message.header()), KillPermission::Allowed);
    caller.alias = "author";
    assert_eq!(caller.permission(message.header()), KillPermission::Allowed);
    let mut header = message.header().clone();
    header.set_to(BString::from(" @USER@"));
    caller.alias = "@USER@";
    assert_eq!(caller.permission(&header), KillPermission::Denied);
    assert_eq!(access("AUTHOR").permission(&header), KillPermission::Allowed);
    caller.read_all = true;
    for flag in [attributes::MSG_DELETED, attributes::MSG_NODISP] {
        let mut hidden = header.clone();
        hidden.attributes |= flag;
        assert_eq!(caller.permission(&hidden), KillPermission::Denied);
    }
    header.set_to(BString::from(""));
    header.set_from(BString::from(""));
    assert_eq!(access("").permission(&header), KillPermission::Denied);
}

#[test]
fn denied_snapshot_never_reads_body_and_all_paths_release_locks() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("area");
    let mut base = JamMessageBase::create(&path).unwrap();
    base.write_message(&message(true, ' ')).unwrap();
    let snapshot = kill_snapshot(&mut base, 1, &access("AUTHOR")).unwrap().unwrap();
    let mut writer = JamMessageBase::open(&path).unwrap();
    assert!(writer.try_lock().unwrap());
    writer.unlock();
    assert_eq!(snapshot.message.text(), &BString::from("Secret body"));
    std::fs::remove_file(path.with_extension("jdt")).unwrap();
    assert!(kill_snapshot(&mut base, 1, &access("OTHER")).unwrap().is_none());
    assert!(kill_snapshot(&mut base, 1, &access("AUTHOR")).is_err());
    assert!(writer.try_lock().unwrap(), "failed snapshot must release its lock too");
    writer.unlock();
}

#[test]
fn commit_rechecks_authorization_and_password_even_for_unchanged_message() {
    let root = tempfile::tempdir().unwrap();
    let mut base = JamMessageBase::create(root.path().join("area")).unwrap();
    base.write_message(&message(true, 'S')).unwrap();
    let snapshot = kill_snapshot(&mut base, 1, &access("AUTHOR")).unwrap().unwrap();
    let mut author = access("AUTHOR");
    assert!(!kill_unchanged(&mut base, 1, &snapshot, &author, false).unwrap());
    author.command = false;
    assert!(!kill_unchanged(&mut base, 1, &snapshot, &author, true).unwrap());
    assert!(!kill_unchanged(&mut base, 1, &snapshot, &access("OTHER"), true).unwrap());
    assert!(!kill_unchanged(&mut base, 1, &snapshot, &access("READER"), true).unwrap());
    assert!(kill_unchanged(&mut base, 1, &snapshot, &access("AUTHOR"), true).unwrap());
    assert!(is_deleted(&base, 1));
}

#[test]
fn changed_header_or_same_length_body_is_not_deleted() {
    for change in 0..5 {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("area");
        let mut base = JamMessageBase::create(&path).unwrap();
        base.write_message(&message(true, 'S')).unwrap();
        let snapshot = kill_snapshot(&mut base, 1, &access("AUTHOR")).unwrap().unwrap();
        let mut writer = JamMessageBase::open(&path).unwrap();
        let mut changed = snapshot.message.header().clone();
        match change {
            0 => changed.password_crc = JamMessageBase::crc(&BString::from("NEW")),
            1 => changed.set_to(BString::from("OTHER")),
            2 => changed.attributes |= attributes::MSG_NODISP,
            3 => changed.times_read += 1,
            _ => {}
        }
        writer
            .transaction(|writer| {
                if change == 4 {
                    // Same header, CRC, offset, length and base generation.
                    std::fs::write(path.with_extension("jdt"), b"Edited body")?;
                    Ok(())
                } else {
                    raw::update_header(writer, 1, &changed)
                }
            })
            .unwrap();
        let before = header_bytes(&writer.read_header(1).unwrap());
        assert!(!kill_unchanged(&mut base, 1, &snapshot, &access("AUTHOR"), true).unwrap());
        assert_eq!(before, header_bytes(&base.read_header(1).unwrap()));
        assert!(writer.try_lock().unwrap());
        writer.unlock();
    }
}

#[test]
fn deleted_snapshot_is_not_deleted_again() {
    let root = tempfile::tempdir().unwrap();
    let mut base = JamMessageBase::create(root.path().join("area")).unwrap();
    base.write_message(&message(true, ' ')).unwrap();
    let snapshot = kill_snapshot(&mut base, 1, &access("AUTHOR")).unwrap().unwrap();
    base.delete_message(1).unwrap();
    let generation = base.mod_counter();
    assert!(kill_snapshot(&mut base, 1, &access("AUTHOR")).is_err());
    assert!(kill_unchanged(&mut base, 1, &snapshot, &access("AUTHOR"), true).is_err());
    assert_eq!(base.mod_counter(), generation);
}

#[test]
fn pack_rejects_even_byte_identical_renumbered_replacement() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("area");
    let mut base = JamMessageBase::create(&path).unwrap();
    let duplicate = message(true, 'S');
    base.write_message(&duplicate).unwrap();
    base.write_message(&duplicate).unwrap();
    let snapshot = kill_snapshot(&mut base, 1, &access("AUTHOR")).unwrap().unwrap();
    let mut writer = JamMessageBase::open(&path).unwrap();
    writer.delete_message(1).unwrap();
    writer.pack(&PackOptions::default().with_renumber_from(1)).unwrap();
    assert_eq!(header_bytes(snapshot.message.header()), header_bytes(&writer.read_header(1).unwrap()));
    assert_eq!(snapshot.message.text(), writer.read_message(1).unwrap().text());
    assert!(!kill_unchanged(&mut base, 1, &snapshot, &access("AUTHOR"), true).unwrap());
    assert!(!base.read_header(1).unwrap().is_deleted());
}

async fn state(root: &Path, user: &str, input: &str) -> (IcyBoardState, ChannelConnection) {
    let bbs = Arc::new(Mutex::new(BBS::new(1)));
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let (peer, connection) = ChannelConnection::create_pair();
    let mut board = IcyBoard::new();
    board.root_path = root.to_path_buf();
    board.config.paths.statistics_file = root.join("statistics.toml");
    board.default_display_text = DEFAULT_DISPLAY_TEXT.clone();
    for (text, marker) in [
        (IceText::YourPassword, PROMPT),
        (IceText::YouCanNotKillMessage, DENIED),
        (IceText::NoSuchMessageNumber, MISSING),
        (IceText::MessageKilled, KILLED),
        (IceText::WrongPasswordEntered, WRONG),
    ] {
        board.default_display_text.update_record_number(text as usize, marker).unwrap();
    }
    board.config.sysop_command_level.read_all_mail = SecurityExpression::from_req_security(100);
    // There is no CMNT marker: read-comments alone must not authorize a kill.
    board.config.sysop_command_level.read_all_comments = SecurityExpression::from_req_security(0);
    board.users.new_user(User {
        name: user.into(),
        security_level: 10,
        ..Default::default()
    });
    let caller = board.users[0].clone();
    let conference = Conference {
        is_public: true,
        areas: Some(Arc::new(AreaList::new(vec![
            MessageArea {
                path: "area0".into(),
                ..Default::default()
            },
            MessageArea {
                path: "area1".into(),
                ..Default::default()
            },
        ]))),
        ..Default::default()
    };
    let mut state = IcyBoardState::new(bbs, Arc::new(Mutex::new(board)), nodes, node, Box::new(connection)).await;
    state.session.current_user = Some(caller);
    state.session.user_name = user.into();
    state.session.alias_name.clear();
    state.session.cur_user_id = 0;
    state.session.cur_security = 10;
    state.session.is_sysop = false;
    state.session.user_command_level.cmd_k = SecurityExpression::from_req_security(0);
    state.session.current_conference = conference;
    state.session.current_message_area = 1;
    state.session.page_len = 0;
    state.char_buffer.extend(input.chars().map(|ch| KeyChar::new(KeySource::User, ch)));
    (state, peer)
}

async fn output(peer: &mut ChannelConnection) -> String {
    let mut output = Vec::new();
    loop {
        let mut bytes = [0; 4096];
        let count = peer.try_read(&mut bytes).await.unwrap();
        if count == 0 {
            return String::from_utf8_lossy(&output).into_owned();
        }
        output.extend_from_slice(&bytes[..count]);
    }
}

#[tokio::test]
async fn actual_kill_private_and_public_recipient_sender_and_unrelated_matrix() {
    for private in [false, true] {
        for kind in [' ', 'S', 'G'] {
            for user in ["AUTHOR", "READER", "OTHER"] {
                let root = tempfile::tempdir().unwrap();
                let mut base = JamMessageBase::create(root.path().join("area1")).unwrap();
                base.write_message(&message(private, kind)).unwrap();
                let prompt = user == "AUTHOR" && kind != ' ';
                let allowed = user == "AUTHOR" || (user == "READER" && kind != 'S');
                let (mut state, mut peer) = state(root.path(), user, if prompt { "SECRET\r" } else { "" }).await;
                if !allowed {
                    // Neither knowing the password nor an is_sysop flag grants
                    // ownership or the configured read-all security expression.
                    state.session.last_password = "SECRET".into();
                    state.session.is_sysop = true;
                }
                timeout(Duration::from_secs(3), state.try_to_kill_message(&mut base, 1)).await.unwrap().unwrap();
                assert_eq!(is_deleted(&base, 1), allowed, "private={private}, kind={kind}, user={user}");
                let output = output(&mut peer).await;
                assert!(output.contains(if allowed { KILLED } else { DENIED }), "{output:?}");
                assert_eq!(output.contains(PROMPT), prompt, "{output:?}");
                assert!(output.contains('1'));
                assert!(!output.contains("Secret subject") && !output.contains("Secret body"));
            }
        }
    }
}

#[tokio::test]
async fn sysop_override_still_requires_k_and_read_access() {
    for private in [false, true] {
        for (command, hidden) in [(true, false), (false, false), (true, true)] {
            let root = tempfile::tempdir().unwrap();
            let mut base = JamMessageBase::create(root.path().join("area1")).unwrap();
            let original = message(private, 'S');
            let mut header = original.header().clone();
            if hidden {
                header.attributes |= attributes::MSG_NODISP;
            }
            base.write_message(&JamMessage::from_stored(header, original.text().clone())).unwrap();
            let (mut state, mut peer) = state(root.path(), "OTHER", "").await;
            state.session.cur_security = 100;
            // The expression, not the is_sysop bit, grants read-all.
            state.session.user_command_level.cmd_k = SecurityExpression::from_req_security(if command { 0 } else { 255 });
            timeout(Duration::from_secs(3), state.try_to_kill_message(&mut base, 1)).await.unwrap().unwrap();
            assert_eq!(is_deleted(&base, 1), command && !hidden);
            assert!(!output(&mut peer).await.contains(PROMPT));
        }
    }
}

#[tokio::test]
async fn incorrect_sender_password_reports_wrong_password_then_cannot_kill() {
    for kind in ['S', 'G'] {
        let root = tempfile::tempdir().unwrap();
        let mut base = JamMessageBase::create(root.path().join("area1")).unwrap();
        base.write_message(&message(true, kind)).unwrap();
        let (mut state, mut peer) = state(root.path(), "AUTHOR", "WRONG\rWRONG\rWRONG\r").await;
        timeout(Duration::from_secs(3), state.try_to_kill_message(&mut base, 1)).await.unwrap().unwrap();
        assert!(!base.read_header(1).unwrap().is_deleted());
        let output = output(&mut peer).await;
        assert!(output.contains(WRONG) && output.contains(DENIED), "{output:?}");
        assert!(!output.contains(KILLED));
    }
}

#[tokio::test]
async fn absent_and_deleted_numbers_report_no_such_message() {
    let root = tempfile::tempdir().unwrap();
    let mut base = JamMessageBase::create(root.path().join("area1")).unwrap();
    base.write_message(&message(true, 'S')).unwrap();
    base.delete_message(1).unwrap();
    let (mut state, mut peer) = state(root.path(), "AUTHOR", "").await;
    for number in [0, 1, 2, u32::MAX] {
        timeout(Duration::from_secs(3), state.try_to_kill_message(&mut base, number))
            .await
            .unwrap()
            .unwrap();
        let output = output(&mut peer).await;
        assert!(output.contains(MISSING) && output.contains(&number.to_string()), "{output:?}");
        assert!(!output.contains(PROMPT) && !output.contains(KILLED));
    }
}

#[tokio::test]
async fn standalone_kill_uses_checked_current_area_and_resolves_relative_path() {
    let root = tempfile::tempdir().unwrap();
    for area in ["area0", "area1"] {
        JamMessageBase::create(root.path().join(area))
            .unwrap()
            .write_message(&message(true, ' '))
            .unwrap();
    }
    let (mut state, _peer) = state(root.path(), "AUTHOR", "").await;
    state.session.tokens.push_back("1".into());
    timeout(Duration::from_secs(3), state.delete_message()).await.unwrap().unwrap();
    assert!(!is_deleted(&JamMessageBase::open(root.path().join("area0")).unwrap(), 1));
    assert!(is_deleted(&JamMessageBase::open(root.path().join("area1")).unwrap(), 1));
    for missing in 0..3 {
        state.session.current_message_area = usize::MAX;
        if missing == 1 {
            state.session.current_conference.areas = Some(Arc::new(AreaList::new(vec![])));
        }
        if missing == 2 {
            state.session.current_conference.areas = None;
        }
        timeout(Duration::from_secs(3), state.delete_message()).await.unwrap().unwrap();
    }
}

#[tokio::test]
async fn password_input_holds_no_lock_and_changed_or_renumbered_message_survives() {
    for change in 0..3 {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("area1");
        let mut base = JamMessageBase::create(&path).unwrap();
        let original = message(true, 'S');
        base.write_message(&original).unwrap();
        base.write_message(&original).unwrap();
        let (mut state, mut peer) = state(root.path(), "AUTHOR", "").await;
        let mut writer = JamMessageBase::open(&path).unwrap();
        let caller = async {
            let mut output = Vec::new();
            while !output.windows(PROMPT.len()).any(|bytes| bytes == PROMPT.as_bytes()) {
                let mut bytes = [0; 4096];
                let count = peer.read(&mut bytes).await.unwrap();
                assert_ne!(count, 0);
                output.extend_from_slice(&bytes[..count]);
            }
            assert!(writer.try_lock().unwrap(), "password input must not hold a JAM lock");
            writer.unlock();
            match change {
                0 => {
                    writer.delete_message(1).unwrap();
                    writer.pack(&PackOptions::default().with_renumber_from(1)).unwrap();
                }
                1 => writer
                    .transaction(|_| {
                        std::fs::write(path.with_extension("jdt"), b"Edited bodySecret body")?;
                        Ok(())
                    })
                    .unwrap(),
                _ => {
                    let mut header = writer.read_header(1).unwrap();
                    header.password_crc = JamMessageBase::crc(&BString::from("NEW"));
                    raw::update_header(&mut writer, 1, &header).unwrap();
                }
            }
            peer.send(b"SECRET\r").await.unwrap();
            peer
        };
        let (result, mut peer) = timeout(Duration::from_secs(3), async { tokio::join!(state.try_to_kill_message(&mut base, 1), caller) })
            .await
            .unwrap();
        result.unwrap();
        assert!(!base.read_header(1).unwrap().is_deleted());
        let output = output(&mut peer).await;
        assert!(output.contains(DENIED) && !output.contains(KILLED), "{output:?}");
    }
}
