use std::{path::Path, sync::Arc, time::Duration};

use crate::{
    icy_board::{
        IcyBoard,
        bbs::BBS,
        conferences::Conference,
        doors::DropFile,
        icb_config::{ExternalEditorConfig, ExternalEditorMode},
        icb_text::DEFAULT_DISPLAY_TEXT,
        message_area::{AreaList, MessageArea},
        read_data_with_encoding_detection,
        security_expr::SecurityExpression,
        state::{IcyBoardState, KeyChar, KeySource},
        user_base::{FSEMode, User},
    },
    vm::{DiskIO, run},
};
use icy_net::{ConnectionType, channel::ChannelConnection};
use jamjam::jam::{
    JamMessage, JamMessageBase, attributes,
    msg_header::{MessageSubfield, SubfieldType},
};

async fn fixture(root: &Path, input: &str, stop: bool) -> (IcyBoardState, ChannelConnection) {
    let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let (peer, connection) = ChannelConnection::create_pair();
    let mut board = IcyBoard::new();
    board.root_path = root.into();
    board.config.paths.statistics_file = root.join("statistics.toml");
    board.default_display_text = DEFAULT_DISPLAY_TEXT.clone();
    board.config.message.validate_to_name = false;
    board.users.new_user(User {
        name: "READER".into(),
        security_level: 10,
        ..Default::default()
    });
    let user = board.users[0].clone();
    board.conferences.clear();
    for number in 0..2 {
        board.conferences.push(Conference {
            name: format!("Conference {number}"),
            is_public: true,
            long_to_names: true,
            sec_request_rr: SecurityExpression::from_req_security(255),
            areas: Some(Arc::new(AreaList::new(vec![MessageArea {
                path: root.join(format!("area{number}")),
                name: format!("Area {number}"),
                ..Default::default()
            }]))),
            ..Default::default()
        });
        let mut base = JamMessageBase::create(root.join(format!("area{number}"))).unwrap();
        base.write_message(
            &JamMessage::default()
                .with_from("READER".into())
                .with_to("ALICE".into())
                .with_subject("Original subject".into())
                .with_text("Original body".into())
                .with_msg_id("original-id".into())
                .with_attributes(attributes::MSG_PRIVATE | attributes::MSG_LOCAL)
                .with_sub_field(MessageSubfield::new(SubfieldType::FTSKludge, "UNKNOWN: retained".into())),
        )
        .unwrap();
    }
    let editor = crate::vm::tests::compile(&format!(
        r#"
STRING directory, text
GETTOKEN directory
FOPEN 1, directory + "/MSGTMP", O_RD, S_DN
FGET 1, text
FCLOSE 1
FCREATE 1, directory + "/MSGTMP", O_WR, S_DN
FPUTLN 1, "Edited: " + text
FCLOSE 1
FCREATE 1, directory + "/RESULT.ED", O_WR, S_DN
FPUTLN 1, "0"
FPUTLN 1, "Final subject"
FPUTLN 1, "Test PPE"
FCLOSE 1
{}
"#,
        if stop { "STOP" } else { "EXIT" }
    ));
    let editor_path = root.join("editor.ppe");
    std::fs::write(&editor_path, editor.to_buffer().unwrap()).unwrap();
    board.config.message.external_editor = ExternalEditorConfig {
        mode: ExternalEditorMode::Ppe,
        path: editor_path.to_string_lossy().into(),
        drop_file: DropFile::None,
        ..Default::default()
    };
    let conference = board.conferences[0].clone();
    let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(board)), nodes, node, Box::new(connection)).await;
    state.session.current_user = Some(user);
    state.session.user_name = "READER".into();
    state.session.alias_name.clear();
    state.session.cur_user_id = 0;
    state.session.cur_security = 10;
    state.session.user_command_level.cmd_e = SecurityExpression::from_req_security(0);
    state.session.user_command_level.cmd_j = SecurityExpression::from_req_security(0);
    state.session.user_command_level.edit_own_messages = SecurityExpression::from_req_security(0);
    state.session.current_conference = conference;
    state.session.time_limit = 30;
    state.session.page_len = 0;
    state.session.fse_mode = FSEMode::Yes;
    state
        .char_buffer
        .extend(input.chars().map(|character| KeyChar::new(KeySource::User, character)));
    state.session.tokens.push_back("caller argument".into());
    state.ppe_nesting = 1;
    (state, peer)
}

async fn execute(state: &mut IcyBoardState, root: &Path, source: &str) -> String {
    let executable = crate::vm::tests::compile(source);
    let mut io = DiskIO::new(root.to_str().unwrap(), None);
    let result = tokio::time::timeout(Duration::from_secs(5), run(&root.join("caller.ppe"), &executable, &mut io, state))
        .await
        .expect("message API timed out");
    assert!(result.unwrap());
    assert_eq!(state.session.current_conference_number, 0);
    assert_eq!(state.session.current_message_area, 0);
    assert_eq!(state.session.tokens.front().map(String::as_str), Some("caller argument"));
    assert_eq!(state.ppe_nesting, 1);
    read_data_with_encoding_detection(&std::fs::read(root.join("status.txt")).unwrap())
        .unwrap()
        .trim()
        .to_string()
}

#[tokio::test]
async fn message_api_post_new_and_editable_initial_text() {
    for predefined in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let (mut state, _peer) = fixture(root.path(), if predefined { "\r\r\r" } else { "ALL\rSubject\rN\r" }, false).await;
        let source = format!(
            r#"
MSGHEADER header
header.To = "ALL"
header.Subject = "Prepared subject"
MSG saved = Session.PostMessage(Board.Conferences[1].Areas[0]{})
ERROR failure = Error.Last()
FCREATE 1, "status.txt", O_WR, S_DN
FPUTLN 1, saved.Valid, ":", saved.Number, ":", saved.Subject, ":", saved.Text(), ":", failure.OK, ":", header.Subject
FCLOSE 1
EXIT
"#,
            if predefined { ", header, \"Prepared body\"" } else { "" }
        );
        let result = execute(&mut state, root.path(), &source).await;
        assert_eq!(
            result,
            format!("1:2:Final subject:Edited: {}:1:Prepared subject", if predefined { "Prepared body" } else { "" })
        );
        let base = JamMessageBase::open(root.path().join("area1")).unwrap();
        let message = base.read_message(2).unwrap();
        assert_eq!(message.from().unwrap().to_string(), "READER");
        assert!(!message.header().is_private());
        assert_eq!(JamMessageBase::open(root.path().join("area0")).unwrap().highest_message_number(), 1);
    }
}

#[tokio::test]
async fn message_api_reply_uses_source_area_defaults_and_thread() {
    let root = tempfile::tempdir().unwrap();
    let (mut state, _peer) = fixture(root.path(), "\r\r\r", false).await;
    let result = execute(
        &mut state,
        root.path(),
        r#"
MSG original = Board.Conferences[1].Areas[0].Read(1)
MSGHEADER header = Session.ReplyHeader(original)
MSG saved = Session.ReplyMessage(original, header, "Prepared reply")
FCREATE 1, "status.txt", O_WR, S_DN
FPUTLN 1, header.To, ":", header.IsPrivate, ":", saved.Valid, ":", saved.Number, ":", saved.ReplyTo, ":", saved.IsPrivate, ":", saved.Text()
FCLOSE 1
EXIT
"#,
    )
    .await;
    assert_eq!(result, "ALICE:1:1:2:1:1:Edited: Prepared reply");
    let base = JamMessageBase::open(root.path().join("area1")).unwrap();
    let saved = base.read_header(2).unwrap();
    assert!(
        saved
            .sub_fields
            .iter()
            .any(|field| field.field_type() == SubfieldType::ReplyID && field.content().to_string() == "original-id")
    );
    assert_eq!(JamMessageBase::open(root.path().join("area0")).unwrap().highest_message_number(), 1);
}

#[tokio::test]
async fn message_api_edit_preserves_number_metadata_and_header_input() {
    let root = tempfile::tempdir().unwrap();
    let (mut state, _peer) = fixture(root.path(), "BOB\r\r", false).await;
    let result = execute(
        &mut state,
        root.path(),
        r#"
MSG original = Board.Conferences[1].Areas[0].Read(1)
MSGHEADER header = original.Header
header.Subject = "Prepared subject"
MSG saved = Session.EditMessage(original, header)
ERROR failure = Error.Last()
MSG fresh = Board.Conferences[1].Areas[0].Read(1)
FCREATE 1, "status.txt", O_WR, S_DN
FPUTLN 1, saved.Valid, ":", saved.Number, ":", saved.Subject, ":", fresh.Subject, ":", header.Subject, ":", original.Subject, ":", saved.Text(), ":", saved.To, ":", header.To, ":", failure.OK
FCLOSE 1
EXIT
"#,
    )
    .await;
    assert_eq!(
        result,
        "1:1:Final subject:Final subject:Prepared subject:Original subject:Edited: Original body:BOB:ALICE:1"
    );
    let base = JamMessageBase::open(root.path().join("area1")).unwrap();
    assert_eq!(base.highest_message_number(), 1);
    let saved = base.read_header(1).unwrap();
    assert!(saved.is_private());
    assert!(
        saved
            .sub_fields
            .iter()
            .any(|field| field.field_type() == SubfieldType::FTSKludge && field.content().to_string() == "UNKNOWN: retained")
    );
}

#[tokio::test]
async fn message_api_abort_does_not_write_any_message() {
    for call in ["Session.PostMessage(area)", "Session.ReplyMessage(original)", "Session.EditMessage(original)"] {
        let root = tempfile::tempdir().unwrap();
        let (mut state, _peer) = fixture(root.path(), "ALL\rSubject\rN\r", true).await;
        let result = execute(
            &mut state,
            root.path(),
            &format!(
                r#"
AREA area = Board.Conferences[1].Areas[0]
MSG original = area.Read(1)
MSG saved = {call}
ERROR failure = Error.Last()
FCREATE 1, "status.txt", O_WR, S_DN
FPUTLN 1, saved.Valid, ":", failure.OK
FCLOSE 1
EXIT
"#
            ),
        )
        .await;
        assert_eq!(result, "0:1", "{call}");
        let base = JamMessageBase::open(root.path().join("area1")).unwrap();
        assert_eq!(base.highest_message_number(), 1);
        assert_eq!(base.read_message(1).unwrap().text().to_string(), "Original body");
    }
}

#[tokio::test]
async fn message_api_rejects_stale_snapshot_and_private_access() {
    for stale in [true, false] {
        let root = tempfile::tempdir().unwrap();
        let (mut state, _peer) = fixture(root.path(), "", false).await;
        if !stale {
            state.session.user_name = "STRANGER".into();
        }
        let change = if stale {
            "INTEGER changed = SETMSGHDR(1, 1, 3, \"Concurrent subject\")"
        } else {
            ""
        };
        let result = execute(
            &mut state,
            root.path(),
            &format!(
                r#"
MSG original = Board.Conferences[1].Areas[0].Read(1)
{change}
MSG saved = Session.EditMessage(original)
ERROR failure = Error.Last()
FCREATE 1, "status.txt", O_WR, S_DN
FPUTLN 1, saved.Valid, ":", failure.Kind = ErrKind.Msg, ":", failure.Code = ErrCode.{}
FCLOSE 1
EXIT
"#,
                if stale { "Invalid" } else { "Denied" }
            ),
        )
        .await;
        assert_eq!(result, "0:1:1");
        let base = JamMessageBase::open(root.path().join("area1")).unwrap();
        assert_eq!(base.highest_message_number(), 1);
        assert_eq!(base.read_message(1).unwrap().text().to_string(), "Original body");
    }
}

#[tokio::test]
async fn message_api_editor_error_runs_on_error_handler() {
    let root = tempfile::tempdir().unwrap();
    let (mut state, _peer) = fixture(root.path(), "", false).await;
    state.get_board().await.config.message.external_editor.path = root.path().join("missing.ppe").to_string_lossy().into();
    let result = execute(
        &mut state,
        root.path(),
        r#"
ON ERROR GOTO failed
MSG original = Board.Conferences[1].Areas[0].Read(1)
MSG saved = Session.EditMessage(original)
EXIT
:failed
ERROR failure = Error.Last()
FCREATE 1, "status.txt", O_WR, S_DN
FPUTLN 1, failure.Kind = ErrKind.Msg, ":", failure.OK
FCLOSE 1
EXIT
"#,
    )
    .await;
    assert_eq!(result, "1:0");
    assert_eq!(
        JamMessageBase::open(root.path().join("area1"))
            .unwrap()
            .read_message(1)
            .unwrap()
            .text()
            .to_string(),
        "Original body"
    );
}

#[tokio::test]
async fn message_api_saved_result_survives_post_commit_output_error() {
    let root = tempfile::tempdir().unwrap();
    let (mut state, peer) = fixture(root.path(), "", false).await;
    drop(peer);
    let message = JamMessage::default()
        .with_from("READER".into())
        .with_to("ALL".into())
        .with_subject("Saved".into())
        .with_text("Body".into());
    let mut cleanup = state.message_attachment_cleanup(&message);
    let mut saved = None;
    let result = state
        .send_message_with_result(1, 0, message, crate::icy_board::icb_text::IceText::SavingMessage, &mut cleanup, &mut saved)
        .await;
    assert!(result.is_err());
    let saved = saved.expect("committed message must remain available after output failure");
    assert_eq!(saved.number, 2);
    assert_eq!(saved.area, Some((1, 0)));
    let base = JamMessageBase::open(root.path().join("area1")).unwrap();
    assert_eq!(base.read_message(saved.number).unwrap().text().to_string(), "Body");
}

#[tokio::test]
async fn message_api_edit_rejects_changes_during_the_editor() {
    let root = tempfile::tempdir().unwrap();
    let (mut state, _peer) = fixture(root.path(), "", false).await;
    let editor = crate::vm::tests::compile(
        r#"
STRING directory
GETTOKEN directory
INTEGER changed = SETMSGHDR(1, 1, 3, "Concurrent subject")
FCREATE 1, directory + "/MSGTMP", O_WR, S_DN
FPUTLN 1, "Stale replacement"
FCLOSE 1
EXIT
"#,
    );
    std::fs::write(root.path().join("editor.ppe"), editor.to_buffer().unwrap()).unwrap();
    let result = execute(
        &mut state,
        root.path(),
        r#"
MSG original = Board.Conferences[1].Areas[0].Read(1)
MSG saved = Session.EditMessage(original)
ERROR failure = Error.Last()
FCREATE 1, "status.txt", O_WR, S_DN
FPUTLN 1, saved.Valid, ":", failure.OK
FCLOSE 1
EXIT
"#,
    )
    .await;
    assert_eq!(result, "0:0");
    let base = JamMessageBase::open(root.path().join("area1")).unwrap();
    let message = base.read_message(1).unwrap();
    assert_eq!(message.header().subject().unwrap().to_string(), "Concurrent subject");
    assert_eq!(message.text().to_string(), "Original body");
}

#[tokio::test]
async fn message_api_default_private_reply_addresses_the_author() {
    let root = tempfile::tempdir().unwrap();
    let (mut state, _peer) = fixture(root.path(), "", false).await;
    let mut base = JamMessageBase::open(root.path().join("area1")).unwrap();
    let mut header = base.read_header(1).unwrap();
    header.set_from("ALICE".into());
    header.set_to("READER".into());
    jamjam::jam::raw::update_header(&mut base, 1, &header).unwrap();
    let result = execute(
        &mut state,
        root.path(),
        r#"
MSG original = Board.Conferences[1].Areas[0].Read(1)
MSG saved = Session.ReplyMessage(original)
FCREATE 1, "status.txt", O_WR, S_DN
FPUTLN 1, saved.Valid, ":", saved.To, ":", saved.IsPrivate, ":", saved.Text()
FCLOSE 1
EXIT
"#,
    )
    .await;
    assert_eq!(result, "1:ALICE:1:Edited: -> Original body");
}

#[test]
fn message_api_header_record_roundtrips_through_nested_records_and_arrays() {
    assert_eq!(
        crate::vm::tests::run_ppl(
            r#"
TYPE Envelope
    MSGHEADER Header
ENDTYPE
Envelope source
source.Header.Subject = "Original"
Envelope copy = source
copy.Header.Subject = "Changed"
MSGHEADER headers[]
REDIM headers, 1
headers[0] = copy.Header
PRINT source.Header.Subject, ":", headers[0].Subject, ":", headers[1].IsPrivate
"#
        ),
        "Original:Changed:0"
    );
}

#[tokio::test]
async fn message_api_internal_editor_keeps_quotes_with_initial_text() {
    let root = tempfile::tempdir().unwrap();
    let (mut state, _peer) = fixture(root.path(), "\r\r\r\rQ 1 1\rS\r", false).await;
    state.session.fse_mode = FSEMode::No;
    let result = execute(
        &mut state,
        root.path(),
        r#"
MSG original = Board.Conferences[1].Areas[0].Read(1)
MSGHEADER header = Session.ReplyHeader(original)
MSG saved = Session.ReplyMessage(original, header, "Prepared reply")
FCREATE 1, "status.txt", O_WR, S_DN
FPUTLN 1, saved.Valid, ":", saved.Text()
FCLOSE 1
EXIT
"#,
    )
    .await;
    assert!(result.starts_with("1:"), "{result:?}");
    assert!(result.contains("Prepared reply"), "{result:?}");
    assert!(result.contains("-> Original body"), "{result:?}");
}

#[tokio::test]
async fn message_api_initial_text_cannot_bypass_line_limit() {
    let root = tempfile::tempdir().unwrap();
    let (mut state, _peer) = fixture(root.path(), "", false).await;
    state.get_board().await.config.message.max_msg_lines = 1;
    let result = execute(
        &mut state,
        root.path(),
        r#"
MSGHEADER header
MSG saved = Session.PostMessage(Board.Conferences[1].Areas[0], header, "first" + CHR(13) + "second")
ERROR failure = Error.Last()
FCREATE 1, "status.txt", O_WR, S_DN
FPUTLN 1, saved.Valid, ":", failure.Code = ErrCode.Limit
FCLOSE 1
EXIT
"#,
    )
    .await;
    assert_eq!(result, "0:1");
    assert_eq!(JamMessageBase::open(root.path().join("area1")).unwrap().highest_message_number(), 1);
}

#[tokio::test]
async fn message_api_edit_privacy_requires_native_protection_permission() {
    for allowed in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let (mut state, _peer) = fixture(root.path(), if allowed { "\r\r\r" } else { "" }, false).await;
        state.get_board().await.config.sysop_command_level.protect_unprotect_messages = SecurityExpression::from_req_security(if allowed { 0 } else { 255 });
        let result = execute(
            &mut state,
            root.path(),
            r#"
MSG original = Board.Conferences[1].Areas[0].Read(1)
MSGHEADER header = original.Header
header.IsPrivate = FALSE
MSG saved = Session.EditMessage(original, header)
ERROR failure = Error.Last()
FCREATE 1, "status.txt", O_WR, S_DN
FPUTLN 1, saved.Valid, ":", failure.Code = ErrCode.Denied
FCLOSE 1
EXIT
"#,
        )
        .await;
        assert_eq!(result, if allowed { "1:0" } else { "0:1" });
        let base = JamMessageBase::open(root.path().join("area1")).unwrap();
        assert_eq!(base.highest_message_number(), 1);
        assert_eq!(base.read_header(1).unwrap().is_private(), !allowed);
        assert_eq!(
            base.read_message(1).unwrap().text().to_string(),
            if allowed { "Edited: Original body" } else { "Original body" }
        );
    }
}

#[tokio::test]
async fn message_api_sender_defaults_are_editable_with_native_permission() {
    for call in [
        "Session.PostMessage(area, header)",
        "Session.ReplyMessage(original, header)",
        "Session.EditMessage(original, header)",
    ] {
        let root = tempfile::tempdir().unwrap();
        let (mut state, _peer) = fixture(root.path(), "MODERATOR\r\r\r\r", false).await;
        state.get_board().await.config.sysop_command_level.edit_message_headers = SecurityExpression::from_req_security(0);
        state.get_board().await.config.sysop_command_level.protect_unprotect_messages = SecurityExpression::from_req_security(255);
        let result = execute(
            &mut state,
            root.path(),
            &format!(
                r#"
AREA area = Board.Conferences[1].Areas[0]
MSG original = area.Read(1)
MSGHEADER header = original.Header
MSG saved = {call}
ERROR failure = Error.Last()
FCREATE 1, "status.txt", O_WR, S_DN
FPUTLN 1, saved.Valid, ":", saved.From, ":", header.From, ":", failure.OK
FCLOSE 1
EXIT
"#
            ),
        )
        .await;
        assert_eq!(result, "1:MODERATOR:READER:1", "{call}");
    }
}

/// An editor without RESULT.ED leaves the stored header to the dialogue answers.
fn plain_editor(root: &Path) {
    let editor = crate::vm::tests::compile(
        r#"
STRING directory
GETTOKEN directory
FCREATE 1, directory + "/MSGTMP", O_WR, S_DN
FPUTLN 1, "Edited body"
FCLOSE 1
EXIT
"#,
    );
    std::fs::write(root.join("editor.ppe"), editor.to_buffer().unwrap()).unwrap();
}

#[tokio::test]
async fn message_api_empty_answers_keep_supplied_post_defaults() {
    let root = tempfile::tempdir().unwrap();
    let (mut state, _peer) = fixture(root.path(), "\r\r\r\r", false).await;
    plain_editor(root.path());
    state.get_board().await.config.sysop_command_level.edit_message_headers = SecurityExpression::from_req_security(0);
    let result = execute(
        &mut state,
        root.path(),
        r#"
MSGHEADER header
header.From = "MODERATOR"
header.To = "ALL"
header.Subject = "Prepared subject"
MSG saved = Session.PostMessage(Board.Conferences[1].Areas[0], header)
FCREATE 1, "status.txt", O_WR, S_DN
FPUTLN 1, saved.Valid, ":", saved.From, ":", saved.To, ":", saved.Subject
FCLOSE 1
EXIT
"#,
    )
    .await;
    assert_eq!(result, "1:MODERATOR:ALL:Prepared subject");
}

#[tokio::test]
async fn message_api_empty_answers_keep_the_edited_messages_header() {
    let root = tempfile::tempdir().unwrap();
    let (mut state, _peer) = fixture(root.path(), "\r\r\r", false).await;
    plain_editor(root.path());
    let mut base = JamMessageBase::open(root.path().join("area1")).unwrap();
    let mut header = base.read_header(1).unwrap();
    header.set_from("ALICE".into());
    header.set_to("READER".into());
    jamjam::jam::raw::update_header(&mut base, 1, &header).unwrap();
    drop(base);
    state.get_board().await.config.sysop_command_level.edit_any_message = SecurityExpression::from_req_security(0);
    state.get_board().await.config.sysop_command_level.edit_message_headers = SecurityExpression::from_req_security(0);
    state.get_board().await.config.sysop_command_level.protect_unprotect_messages = SecurityExpression::from_req_security(255);
    let result = execute(
        &mut state,
        root.path(),
        r#"
MSG original = Board.Conferences[1].Areas[0].Read(1)
MSG saved = Session.EditMessage(original, original.Header)
FCREATE 1, "status.txt", O_WR, S_DN
FPUTLN 1, saved.Valid, ":", saved.From, ":", saved.To, ":", saved.Subject, ":", saved.IsPrivate, ":", saved.Text()
FCLOSE 1
EXIT
"#,
    )
    .await;
    assert_eq!(result, "1:ALICE:READER:Original subject:1:Edited body");
}

#[tokio::test]
async fn message_api_edit_keeps_a_message_without_a_subject() {
    let root = tempfile::tempdir().unwrap();
    let (mut state, _peer) = fixture(root.path(), "\r\r", false).await;
    plain_editor(root.path());
    let mut base = JamMessageBase::open(root.path().join("area1")).unwrap();
    let mut header = base.read_header(1).unwrap();
    header.set_subject("".into());
    jamjam::jam::raw::update_header(&mut base, 1, &header).unwrap();
    drop(base);
    state.get_board().await.conferences[1].long_to_names = false;
    state.get_board().await.config.sysop_command_level.protect_unprotect_messages = SecurityExpression::from_req_security(255);
    let result = execute(
        &mut state,
        root.path(),
        r#"
MSG original = Board.Conferences[1].Areas[0].Read(1)
MSG saved = Session.EditMessage(original, original.Header)
FCREATE 1, "status.txt", O_WR, S_DN
FPUTLN 1, saved.Valid, ":", saved.Subject, ":", saved.Text()
FCLOSE 1
EXIT
"#,
    )
    .await;
    assert_eq!(result, "1::Edited body");
}

#[tokio::test]
async fn message_api_credit_denial_reports_denied_before_editor() {
    for call in ["Session.PostMessage(area, header)", "Session.ReplyMessage(original, header)"] {
        let root = tempfile::tempdir().unwrap();
        let (mut state, _peer) = fixture(root.path(), "\r\r\r", false).await;
        crate::icy_board::state::user_commands::pcb::d_download::enable_activity_accounting(
            &mut state,
            crate::icy_board::accounting_cfg::AccountingConfig {
                charge_per_msg_write_private: 10.0,
                ..Default::default()
            },
        )
        .await;
        state.session.current_user.as_mut().unwrap().account.as_mut().unwrap().starting_balance = 9.0;
        let result = execute(
            &mut state,
            root.path(),
            &format!(
                r#"
AREA area = Board.Conferences[1].Areas[0]
MSG original = area.Read(1)
MSGHEADER header = original.Header
MSG saved = {call}
ERROR failure = Error.Last()
FCREATE 1, "status.txt", O_WR, S_DN
FPUTLN 1, saved.Valid, ":", failure.Code = ErrCode.Denied
FCLOSE 1
EXIT
"#
            ),
        )
        .await;
        assert_eq!(result, "0:1", "{call}");
        assert_eq!(JamMessageBase::open(root.path().join("area1")).unwrap().highest_message_number(), 1);
    }
}
