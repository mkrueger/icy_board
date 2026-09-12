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
#[ignore = "requires ICB_LIQUID_READ_PPE pointing to the compiled reader package"]
async fn message_api_liquid_read_real_package() {
    use icy_engine::TextPane;
    use icy_net::Connection;

    let path = std::path::PathBuf::from(std::env::var_os("ICB_LIQUID_READ_PPE").expect("ICB_LIQUID_READ_PPE is required"));
    let executable = crate::executable::Executable::read_file(&path, false).unwrap();
    let editor_kind = std::env::var("ICB_LIQUID_READ_EDITOR").unwrap_or_else(|_| "ppe".into());
    assert!(
        matches!(editor_kind.as_str(), "ppe" | "internal" | "iceedit" | "gedit" | "lredit" | "ledit"),
        "unknown editor: {editor_kind}"
    );
    let dos_editor = matches!(editor_kind.as_str(), "iceedit" | "gedit");
    let editor_language = std::env::var("ICB_LIQUID_EDIT_LANGUAGE").unwrap_or_else(|_| "en".into());
    for abort in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let (mut state, mut peer) = fixture(root.path(), "", abort).await;
        if editor_kind == "internal" {
            state.session.fse_mode = FSEMode::No;
            state.get_board().await.config.message.external_editor.mode = ExternalEditorMode::Internal;
        }
        if matches!(editor_kind.as_str(), "lredit" | "ledit") {
            state.get_board().await.config.message.external_editor = ExternalEditorConfig {
                mode: ExternalEditorMode::Ppe,
                path: std::env::var("ICB_LIQUID_EDIT_PPE").expect("ICB_LIQUID_EDIT_PPE is required"),
                arguments: editor_language.clone(),
                ..Default::default()
            };
        }
        if dos_editor {
            let source_variable = if editor_kind == "gedit" { "ICB_GEDIT_SOURCE" } else { "ICB_ICEEDIT_SOURCE" };
            let source = std::env::var_os(source_variable).expect(source_variable);
            let installation = root.path().join("dos-editor");
            std::fs::create_dir(&installation).unwrap();
            for entry in std::fs::read_dir(source).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_file() {
                    std::fs::copy(entry.path(), installation.join(entry.file_name())).unwrap();
                }
            }
            let driver = std::env::var_os("ICB_FOSSIL_DRIVER");
            if let Some(driver) = &driver {
                std::fs::copy(driver, installation.join("X00.EXE")).unwrap();
            }
            let command = if editor_kind == "gedit" {
                "SET GEDIT=BBS:DORINFO\r\nGEDIT.EXE 1 57600 30 15 -N1 -A1 -R25"
            } else {
                "ICEEDIT.EXE /D:C:\\DOOR /N:1 /T:30 /K:15"
            };
            std::fs::write(
                installation.join("ICBSTART.BAT"),
                format!("@ECHO OFF\r\n{}{command}\r\n", if driver.is_some() { "X00.EXE E\r\n" } else { "" }),
            )
            .unwrap();
            let assets = std::env::var_os("ICB_DOS_ASSETS").expect("ICB_DOS_ASSETS");
            std::fs::create_dir_all(root.path().join("assets/dos")).unwrap();
            for name in ["freedos.img", "seabios.bin", "vgabios.bin"] {
                std::fs::copy(Path::new(&assets).join(name), root.path().join("assets/dos").join(name)).unwrap();
            }
            state.get_board().await.config.message.external_editor = ExternalEditorConfig {
                mode: ExternalEditorMode::Dos,
                path: installation.to_string_lossy().into(),
                arguments: "ICBSTART.BAT".into(),
                drop_file: DropFile::DorInfo,
                timeout_seconds: 40,
                ..Default::default()
            };
        }
        state.set_terminal_size(80, 25);
        state.session.disp_options.grapics_mode = crate::icy_board::state::GraphicsMode::Graphics;
        state.session.term_caps.is_utf8 = true;
        let mut io = DiskIO::new(root.path().to_str().unwrap(), None);
        let mut inputs = ["\r", "r", "\r", "\r", "\r"].map(String::from).to_vec();
        if editor_kind == "internal" {
            inputs.truncate(2);
            inputs.extend(["ICB reader reply\r", "\r"].map(String::from));
            inputs.extend(if abort { ["A\r", "Y\r"] } else { ["Q 1 1\r", "S\r"] }.map(String::from));
        }
        if editor_kind == "lredit" {
            inputs.truncate(2);
            inputs.extend(["ICB reader reply\r", if abort { "/A\r" } else { "/S\r" }].map(String::from));
        }
        if editor_kind == "ledit" {
            inputs.truncate(2);
            inputs.extend(["\x1b[F\rICB reader reply", if abort { "\x01" } else { "\x13" }].map(String::from));
        }
        if dos_editor {
            let default_keys = match (editor_kind.as_str(), abort) {
                ("gedit", true) => "ICB reader reply\r\x0fay\r",
                ("iceedit", true) => "ICB reader reply\r\x01y\r",
                ("gedit", false) => "\x11\r\x0bICB reader reply\r\x1an\r",
                _ => "\x11\r\x11ICB reader reply\r\x1a",
            };
            let keys = std::env::var("ICB_LIQUID_READ_KEYS").unwrap_or_else(|_| default_keys.into());
            inputs.extend(keys.chars().map(|character| character.to_string()));
            inputs.push("\r".into());
        }
        let editor_input_end = inputs.len();
        inputs.extend(["\x1b".into(), "\x1b".into()]);
        let mut typed_text_visible = false;
        let mut editor_header_visible = false;
        let mut transcript = Vec::new();
        let drive = async {
            let trigger = if editor_kind == "gedit" { "[ ^Q=Quote ]" } else { "\x1b[5;1H" };
            let mut screen = crate::icy_board::state::virtual_screen::VirtualScreen::new(icy_parser_core::AnsiParser::default());
            for (index, input) in inputs.iter().enumerate() {
                let deadline = tokio::time::Instant::now() + Duration::from_millis(500);
                loop {
                    if dos_editor && index > 5 && index < editor_input_end && tokio::time::Instant::now() >= deadline {
                        peer.send(input.as_bytes()).await.unwrap();
                        break;
                    }
                    let mut packet = [0; 4096];
                    // InKey waits 100 ms for a possible ANSI suffix after Escape.
                    match tokio::time::timeout(Duration::from_millis(150), peer.read(&mut packet)).await {
                        Ok(read) => {
                            let count = read.unwrap();
                            assert_ne!(count, 0);
                            transcript.extend_from_slice(&packet[..count]);
                            screen.write_bytes(&packet[..count]);
                            typed_text_visible |= (0..25).any(|row| {
                                (0..80)
                                    .map(|column| screen.buffer.char_at(icy_engine::Position::new(column, row)).ch)
                                    .collect::<String>()
                                    .contains("ICB reader reply")
                            });
                        }
                        Err(_) if dos_editor && index == 5 && !String::from_utf8_lossy(&transcript).contains(trigger) => {}
                        Err(_) => {
                            if editor_kind == "lredit" && index == 2 {
                                let rows = (0..6)
                                    .map(|row| {
                                        (0..80)
                                            .map(|column| screen.buffer.char_at(icy_engine::Position::new(column, row)).ch)
                                            .collect::<String>()
                                    })
                                    .collect::<Vec<_>>();
                                assert!(rows[0].starts_with("LiQUiD Read / Editor"), "{rows:?}");
                                let expected = if editor_language == "de" {
                                    ["Von: READER", "An: ALICE", "Betreff: Original subject"]
                                } else {
                                    ["From: READER", "To: ALICE", "Subject: Original subject"]
                                };
                                for (row, text) in rows[1..4].iter().zip(expected) {
                                    assert!(row.starts_with(text), "missing {text}: {row}");
                                }
                                assert!(rows[4].starts_with("Area: Area 0"), "{rows:?}");
                                assert_eq!(rows[4].chars().skip(62).take(5).collect::<String>(), "[YES]", "{rows:?}");
                                assert!(rows[5].contains("Original body"), "{rows:?}");
                                editor_header_visible = true;
                            }
                            if editor_kind == "ledit" && index == 2 {
                                let rows = (0..25)
                                    .map(|row| {
                                        (0..80)
                                            .map(|column| screen.buffer.char_at(icy_engine::Position::new(column, row)).ch)
                                            .collect::<String>()
                                    })
                                    .collect::<Vec<_>>();
                                assert!(rows[1].starts_with("| To   : ALICE"), "{rows:?}");
                                assert!(rows[1].contains("LiQUiD Edit"), "{rows:?}");
                                assert!(rows[2].contains("Original subject"), "{rows:?}");
                                assert!(rows[2].contains("^A Abort  ^S Save"), "{rows:?}");
                                assert!(rows[4..22].iter().any(|row| row.contains("Original body")), "{rows:?}");
                                for (row, text) in rows.iter().enumerate().take(23) {
                                    assert_eq!(text.chars().nth(77), Some(if matches!(row, 0 | 3 | 22) { '+' } else { '|' }), "{rows:?}");
                                    assert!(text.chars().skip(78).all(|character| character == ' '), "{rows:?}");
                                }
                                editor_header_visible = true;
                            }
                            peer.send(input.as_bytes()).await.unwrap();
                            break;
                        }
                    }
                }
            }
            std::future::pending::<()>().await;
        };
        let result = tokio::time::timeout(Duration::from_secs(if dos_editor { 60 } else { 5 }), async {
            tokio::select! {
                result = run(&path, &executable, &mut io, &mut state) => result,
                () = drive => unreachable!(),
            }
        })
        .await;
        if let Some(path) = std::env::var_os("ICB_LIQUID_READ_TRANSCRIPT") {
            std::fs::write(path, &transcript).unwrap();
        }
        let screen = (0..25)
            .map(|row| {
                (0..80)
                    .map(|column| state.user_screen.buffer.char_at(icy_engine::Position::new(column, row)).ch)
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            result.is_ok(),
            "LiQUiD Read did not return; editor={editor_kind}, abort={abort}, buffered={}\n{screen}",
            state.char_buffer.len()
        );
        assert!(result.unwrap().unwrap());
        assert!(screen.contains("Original subject"), "reader did not restore its list:\n{screen}");
        let rows = screen.lines().collect::<Vec<_>>();
        assert_eq!(rows[3].chars().skip(7).take(16).collect::<String>(), "Original subject", "{screen}");
        assert_eq!(rows[12].chars().skip(2).take(13).collect::<String>(), "Original body", "{screen}");
        assert_eq!(state.session.tokens.iter().cloned().collect::<Vec<_>>(), ["caller argument"]);
        let base = JamMessageBase::open(root.path().join("area0")).unwrap();
        assert_eq!(base.highest_message_number(), if abort { 1 } else { 2 }, "abort={abort}\n{screen}");
        if editor_kind != "ppe" {
            assert!(typed_text_visible, "editor input was not rendered; editor={editor_kind}, abort={abort}");
            assert!(!state.session.request_logoff);
        }
        if matches!(editor_kind.as_str(), "lredit" | "ledit") {
            assert!(editor_header_visible);
        }
        if !abort {
            let reply = base.read_message(2).unwrap();
            assert_eq!(reply.header().reply_to, 1);
            assert!(reply.header().is_private());
            assert!(reply.text().to_string().contains("Original body"), "saved body: {:?}", reply.text());
            if matches!(editor_kind.as_str(), "lredit" | "ledit") {
                let expected_pid = if editor_kind == "ledit" { "LiQUiD Edit 1.1.0" } else { "LiQUiD Read Editor 0.1.0" };
                assert!(
                    reply
                        .header()
                        .sub_fields
                        .iter()
                        .any(|field| { field.field_type() == SubfieldType::PID && field.content().to_string() == expected_pid })
                );
            }
            if editor_kind != "ppe" {
                assert!(reply.text().to_string().contains("ICB reader reply"));
                assert_eq!(rows[4].chars().skip(7).take(16).collect::<String>(), "Original subject", "{screen}");
            } else {
                assert!(screen.contains("Final subject"), "saved reply missing from refreshed list:\n{screen}");
            }
        }
        assert_eq!(state.session.last_msg_read, 1);
        assert_eq!(state.session.current_conference_number, 0);
        assert_eq!(state.session.current_message_area, 0);
    }
}

#[tokio::test]
#[ignore = "requires ICB_LIQUID_READ_PPE pointing to the compiled reader package"]
async fn message_api_liquid_read_empty_filtered_and_reopened() {
    use icy_engine::TextPane;
    use icy_net::Connection;

    let path = std::path::PathBuf::from(std::env::var_os("ICB_LIQUID_READ_PPE").expect("ICB_LIQUID_READ_PPE is required"));
    let executable = crate::executable::Executable::read_file(&path, false).unwrap();
    let root = tempfile::tempdir().unwrap();
    let base_path = root.path().join("reader-cases");
    let mut base = JamMessageBase::create(&base_path).unwrap();
    for phase in 0..3 {
        if phase == 1 {
            for (subject, recipient, flags, password) in [
                ("Visible first", "ALL", 0, false),
                ("Hidden private", "OTHER", attributes::MSG_PRIVATE, false),
                ("Hidden deleted", "ALL", attributes::MSG_DELETED, false),
                ("Hidden password", "READER", 0, true),
                ("Visible alias", "ALT READER", attributes::MSG_PRIVATE, false),
            ] {
                let mut message = JamMessage::default()
                    .with_from("WRITER".into())
                    .with_to(recipient.into())
                    .with_subject(subject.into())
                    .with_text(format!("{subject} body").into())
                    .with_attributes(flags);
                if password {
                    message = message.with_password(&"secret".into());
                }
                base.write_message(&message).unwrap();
            }
        }
        let session_root = tempfile::tempdir().unwrap();
        let (mut state, mut peer) = fixture(session_root.path(), "", false).await;
        let mut conference = state.session.current_conference.clone();
        conference.areas = Some(Arc::new(AreaList::new(vec![MessageArea {
            path: base_path.clone(),
            name: "Reader cases".into(),
            ..Default::default()
        }])));
        state.get_board().await.conferences[0] = conference.clone();
        state.session.current_conference = conference;
        state.session.alias_name = "ALT READER".into();
        state.set_terminal_size(80, 25);
        state.session.disp_options.grapics_mode = crate::icy_board::state::GraphicsMode::Graphics;
        state.session.term_caps.is_utf8 = true;
        let last_read = r#"
FCREATE 1, "status.txt", O_WR, S_DN
FPUTLN 1, U_LMR(AreaId(Session.Conference.Number, Session.Area.Number))
FCLOSE 1
EXIT
"#;
        assert_eq!(execute(&mut state, session_root.path(), last_read).await, if phase == 2 { "5" } else { "0" });
        let inputs: &[&str] = match phase {
            0 => &["\r", "\x1b"],
            1 => &["\r", "\x1b", "\x1b[B", "\r", "\x1b", "\x1b"],
            _ => &["\r", "\x1b", "\x1b"],
        };
        let drive = async {
            let mut screen = crate::icy_board::state::virtual_screen::VirtualScreen::new(icy_parser_core::AnsiParser::default());
            for input in inputs {
                loop {
                    let mut packet = [0; 4096];
                    match tokio::time::timeout(Duration::from_millis(150), peer.read(&mut packet)).await {
                        Ok(read) => {
                            let count = read.unwrap();
                            assert_ne!(count, 0);
                            screen.write_bytes(&packet[..count]);
                            let rendered = (0..25)
                                .map(|row| {
                                    (0..80)
                                        .map(|column| screen.buffer.char_at(icy_engine::Position::new(column, row)).ch)
                                        .collect::<String>()
                                })
                                .collect::<Vec<_>>()
                                .join("\n");
                            for hidden in ["Hidden private", "Hidden deleted", "Hidden password"] {
                                assert!(!rendered.contains(hidden), "leaked {hidden}:\n{rendered}");
                            }
                        }
                        Err(_) => {
                            peer.send(input.as_bytes()).await.unwrap();
                            break;
                        }
                    }
                }
            }
            std::future::pending::<()>().await;
        };
        let mut io = DiskIO::new(session_root.path().to_str().unwrap(), None);
        let result = tokio::time::timeout(Duration::from_secs(5), async {
            tokio::select! {
                result = run(&path, &executable, &mut io, &mut state) => result,
                () = drive => unreachable!(),
            }
        })
        .await
        .expect("reader case timed out");
        assert!(result.unwrap());
        let rows = (0..25)
            .map(|row| {
                (0..80)
                    .map(|column| state.user_screen.buffer.char_at(icy_engine::Position::new(column, row)).ch)
                    .collect::<String>()
            })
            .collect::<Vec<_>>();
        if phase == 0 {
            assert_eq!(rows[12].chars().skip(2).take(16).collect::<String>(), "No valid message");
        } else {
            assert_eq!(rows[3].chars().skip(7).take(13).collect::<String>(), "Visible first");
            assert_eq!(rows[4].chars().skip(7).take(13).collect::<String>(), "Visible alias");
            assert_eq!(rows[4].chars().nth(5), Some('5'));
            let preview = if phase == 1 { "Visible alias body" } else { "Visible first body" };
            assert_eq!(rows[12].chars().skip(2).take(preview.len()).collect::<String>(), preview);
        }
        assert_eq!(execute(&mut state, session_root.path(), last_read).await, if phase == 0 { "0" } else { "5" });
        assert_eq!(state.session.tokens.iter().cloned().collect::<Vec<_>>(), ["caller argument"]);
        assert!(!state.session.request_logoff);
    }
}

#[tokio::test]
#[ignore = "requires ICB_LIQUID_EDIT_PPE pointing to the compiled LiQUiD editor"]
async fn message_api_liquid_editor_edit_and_abort_contract() {
    let editor_path = std::env::var("ICB_LIQUID_EDIT_PPE").expect("ICB_LIQUID_EDIT_PPE is required");
    for language in ["en", "de"] {
        for scenario in ["edit", "abort", "empty", "oversized", "long_line", "field_boundary", "unicode"] {
            let root = tempfile::tempdir().unwrap();
            let input = match scenario {
                "edit" => "/E 1\rChanged body\rdiscard me\r/D 2\r/H\rChanged subject\r/S\r",
                "abort" => "/E 1\rChanged body\r/H\rChanged subject\r/A\r",
                "empty" => "/D 1\r/S\r",
                "oversized" => "",
                "long_line" | "field_boundary" => "/E 1\r/S\r",
                "unicode" => "/E 1\rGr\u{fc}\u{df}e\r/S\r",
                _ => unreachable!(),
            };
            let (mut state, _peer) = fixture(root.path(), input, false).await;
            state.get_board().await.config.message.external_editor = ExternalEditorConfig {
                mode: ExternalEditorMode::Ppe,
                path: editor_path.clone(),
                arguments: language.into(),
                ..Default::default()
            };
            state.get_board().await.config.message.max_msg_lines = 200;
            state.get_board().await.config.switches.disable_high_ascii_filter = true;
            state.session.term_caps.is_utf8 = true;
            let original_text = match scenario {
                "oversized" => vec!["line"; 101].join("\n"),
                "long_line" => "x".repeat(300),
                "field_boundary" => "x".repeat(78),
                _ => "Original body".into(),
            };
            let mut base = JamMessageBase::open(root.path().join("area1")).unwrap();
            let mut header = base.read_header(1).unwrap();
            std::fs::write(root.path().join("area1.jdt"), original_text.as_bytes()).unwrap();
            header.offset = 0;
            header.txt_len = original_text.len() as u32;
            jamjam::jam::raw::update_header(&mut base, 1, &header).unwrap();
            drop(base);
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
            let saved = matches!(scenario, "edit" | "long_line" | "field_boundary" | "unicode");
            assert_eq!(result, if saved { "1:1" } else { "0:1" }, "{language}/{scenario}");
            let base = JamMessageBase::open(root.path().join("area1")).unwrap();
            assert_eq!(base.highest_message_number(), 1);
            let message = base.read_message(1).unwrap();
            let expected_text = match scenario {
                "edit" => "Changed body",
                "unicode" => "Gr\u{fc}\u{df}e",
                _ => &original_text,
            };
            assert_eq!(message.text().to_string().trim_end(), expected_text, "{language}/{scenario}");
            assert_eq!(
                message.header().subject().unwrap().to_string(),
                if scenario == "edit" { "Changed subject" } else { "Original subject" }
            );
            assert!(message.header().is_private());
            assert_eq!(state.session.tokens.iter().cloned().collect::<Vec<_>>(), ["caller argument"]);
        }
    }
}

#[tokio::test]
async fn message_api_ledit_standalone_editor_contract() {
    use icy_engine::TextPane;
    use icy_net::Connection;

    let editor_source = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ppe/ledit/src/ledit.pps")).unwrap();
    let editor = crate::vm::tests::compile(&editor_source);
    let editor_bytes = editor.to_buffer().unwrap();
    for language in ["", "de"] {
        for scenario in [
            "edit",
            "abort",
            "escape",
            "reply",
            "post",
            "unicode",
            "scroll",
            "wide",
            "oversized",
            "empty",
            "empty_save",
            "keys",
            "macros",
        ] {
            let root = tempfile::tempdir().unwrap();
            let (mut state, mut peer) = fixture(root.path(), if scenario == "post" { "\r\r\r" } else { "" }, false).await;
            std::fs::write(root.path().join("editor.ppe"), &editor_bytes).unwrap();
            state.get_board().await.config.message.external_editor.arguments = language.into();
            state.get_board().await.config.message.max_msg_lines = 250;
            state.get_board().await.config.switches.disable_high_ascii_filter = true;
            state.set_terminal_size(80, 25);
            state.session.disp_options.grapics_mode = crate::icy_board::state::GraphicsMode::Graphics;
            state.session.term_caps.is_utf8 = true;
            let original_text = match scenario {
                "wide" => "x".repeat(77),
                "oversized" => vec!["line"; 201].join("\n"),
                "scroll" => (1..=25).map(|number| format!("Line {number:02}")).collect::<Vec<_>>().join("\n"),
                "empty" | "empty_save" => String::new(),
                "macros" => "@CLS@ @HANGUP@ @X0C Original body".into(),
                _ => "Original body".into(),
            };
            let mut base = JamMessageBase::open(root.path().join("area1")).unwrap();
            let mut header = base.read_header(1).unwrap();
            std::fs::write(root.path().join("area1.jdt"), original_text.as_bytes()).unwrap();
            header.offset = 0;
            header.txt_len = original_text.len() as u32;
            jamjam::jam::raw::update_header(&mut base, 1, &header).unwrap();
            drop(base);
            let operation = match scenario {
                "reply" => "Session.ReplyMessage(original)",
                "post" => "Session.PostMessage(Board.Conferences[1].Areas[0], header, \"Original body\")",
                _ => "Session.EditMessage(original)",
            };
            let source = format!(
                r#"
MSG original = Board.Conferences[1].Areas[0].Read(1)
MSGHEADER header
header.To = "ALICE"
header.Subject = "Original subject"
MSG saved = {operation}
ERROR failure = Error.Last()
FCREATE 1, "status.txt", O_WR, S_DN
FPUTLN 1, saved.Valid, ":", failure.OK
FCLOSE 1
EXIT
"#
            );
            let changed_text = if scenario == "unicode" { "Gr\u{fc}\u{df}e" } else { "Standalone reply" };
            let editing_keys = if scenario == "scroll" {
                "\x1b[6~\x1b[6~\x1b[F!".to_string()
            } else if scenario == "empty_save" {
                String::new()
            } else if scenario == "keys" {
                format!("\x1b[H\x1b[C\x1b[3~r\x1b[F\rdiscard\x1b[H\x08{}!\r{changed_text}", "\x1b[3~".repeat(7))
            } else {
                format!("\x1b[F!\r{changed_text}")
            };
            let exit_keys = match scenario {
                "abort" => "\x01",
                "escape" => "\x1b",
                _ => "\x13",
            };
            let rejected = matches!(scenario, "wide" | "oversized");
            let mut rendered_initial = false;
            let mut rendered_edit = false;
            let drive = async {
                let mut screen = crate::icy_board::state::virtual_screen::VirtualScreen::new(icy_parser_core::AnsiParser::default());
                for (phase, keys) in [editing_keys.as_str(), exit_keys].into_iter().enumerate() {
                    loop {
                        let mut packet = [0; 4096];
                        match tokio::time::timeout(Duration::from_millis(150), peer.read(&mut packet)).await {
                            Ok(read) => {
                                let count = read.unwrap();
                                assert_ne!(count, 0);
                                screen.write_bytes(&packet[..count]);
                            }
                            Err(_) => break,
                        }
                    }
                    let rows = (0..25)
                        .map(|row| {
                            (0..80)
                                .map(|column| screen.buffer.char_at(icy_engine::Position::new(column, row)).ch)
                                .collect::<String>()
                        })
                        .collect::<Vec<_>>();
                    assert!(!rejected, "rejected draft reached input: {language}/{scenario}");
                    for row in 0..23 {
                        assert_eq!(
                            rows[row].chars().nth(77),
                            Some(if row == 0 || row == 3 || row == 22 { '+' } else { '|' }),
                            "{language}/{scenario}, row {row}: {rows:?}"
                        );
                        assert!(
                            rows[row].chars().skip(78).all(|character| character == ' '),
                            "{language}/{scenario}, overflow: {rows:?}"
                        );
                    }
                    if phase == 0 {
                        assert!(rows[1].starts_with("| To   : ALICE"), "{rows:?}");
                        assert!(rows[1].contains("LiQUiD Edit"), "{rows:?}");
                        assert!(rows[2].contains("Original subject"), "{rows:?}");
                        assert!(rows[2].contains("^A Abort  ^S Save"), "{rows:?}");
                        if scenario == "scroll" {
                            assert!(rows[4].contains("Line 01"), "{rows:?}");
                            assert!(rows[21].contains("Line 18"), "{rows:?}");
                        } else if !matches!(scenario, "empty" | "empty_save") {
                            assert!(rows[4..22].iter().any(|row| row.contains("Original body")), "{rows:?}");
                        }
                        if scenario == "macros" {
                            assert!(rows[4].contains(&original_text), "{rows:?}");
                        }
                        rendered_initial = true;
                    } else {
                        if scenario != "empty_save" {
                            assert!(
                                rows[4..22]
                                    .iter()
                                    .any(|row| row.contains(if scenario == "scroll" { "Line 25!" } else { changed_text })),
                                "{language}/{scenario}: {rows:?}"
                            );
                        }
                        rendered_edit = true;
                    }
                    peer.send(keys.as_bytes()).await.unwrap();
                }
                std::future::pending::<()>().await;
            };
            let result = tokio::select! {
                result = execute(&mut state, root.path(), &source) => result,
                () = drive => unreachable!(),
            };
            let saved = !matches!(scenario, "abort" | "escape" | "wide" | "oversized" | "empty_save");
            let final_screen = (0..25)
                .map(|row| {
                    (0..80)
                        .map(|column| state.user_screen.buffer.char_at(icy_engine::Position::new(column, row)).ch)
                        .collect::<String>()
                })
                .collect::<Vec<_>>();
            assert_eq!(result, if saved { "1:1" } else { "0:1" }, "{language}/{scenario}: {final_screen:?}");
            assert_eq!(rendered_initial && rendered_edit, !rejected, "{language}/{scenario}");
            assert!(!state.session.request_logoff);
            assert_eq!(state.session.tokens.iter().cloned().collect::<Vec<_>>(), ["caller argument"]);
            let base = JamMessageBase::open(root.path().join("area1")).unwrap();
            let new_message = matches!(scenario, "reply" | "post");
            assert_eq!(base.highest_message_number(), if new_message { 2 } else { 1 });
            let message = base.read_message(if new_message { 2 } else { 1 }).unwrap();
            assert_eq!(message.header().subject().unwrap().to_string(), "Original subject");
            assert_eq!(message.to().unwrap().to_string(), "ALICE");
            assert_eq!(message.header().is_private(), scenario != "post");
            let body = message.text().to_string();
            if !saved {
                assert_eq!(body, original_text, "{language}/{scenario}");
            } else {
                if scenario == "scroll" {
                    assert_eq!(body.trim_end(), format!("{original_text}!"));
                } else if scenario == "reply" {
                    assert!(body.contains("Original body"), "{body:?}");
                    assert!(body.contains(changed_text), "{body:?}");
                    assert_eq!(message.header().reply_to, 1);
                } else {
                    assert_eq!(body.trim_end(), format!("{original_text}!\n{changed_text}"), "{language}/{scenario}");
                }
                assert!(
                    message
                        .header()
                        .sub_fields
                        .iter()
                        .any(|field| field.field_type() == SubfieldType::PID && field.content().to_string() == "LiQUiD Edit 1.1.0")
                );
            }
            assert_eq!(JamMessageBase::open(root.path().join("area0")).unwrap().highest_message_number(), 1);
        }
    }
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
