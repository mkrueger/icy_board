use std::{path::Path, sync::Arc, time::Duration};

use icy_engine::{Position, TextPane};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection};

use crate::{
    icy_board::{
        IcyBoard,
        bbs::BBS,
        conferences::Conference,
        file_directory::{DirectoryList, FileDirectory},
        icb_text::DEFAULT_DISPLAY_TEXT,
        security_expr::SecurityExpression,
        state::{GraphicsMode, IcyBoardState, virtual_screen::VirtualScreen},
        user_base::User,
    },
    vm::{DiskIO, run},
};

async fn fixture(root: &Path, language: &str, scenario: &str) -> (IcyBoardState, ChannelConnection) {
    let mut directories = DirectoryList::default();
    if matches!(scenario, "files" | "empty-index" | "missing-index") {
        let path = root.join("private-path-files");
        let metadata = root.join("file-index");
        std::fs::create_dir_all(&path).unwrap();
        if scenario != "missing-index" {
            let mut base = dizbase::file_base::FileBase::open(&path, &metadata).unwrap();
            if scenario == "files" {
                for number in 0..1030 {
                    let file = path.join(format!("file{number:04}.txt"));
                    std::fs::write(&file, b"Demo file\n").unwrap();
                    if number == 0 {
                        std::fs::File::create(&file).unwrap().set_len(2_147_483_648).unwrap();
                    }
                    base.add_file(&file, Vec::new()).unwrap();
                }
                base.set_description(
                    &path.join("file0000.txt"),
                    &format!("@HANGUP@ @CLS@ Gr\u{fc}\u{df}e {}TAIL", "description ".repeat(100)),
                )
                .unwrap();
                base.set_description(&path.join("file1025.txt"), "needle description-only match").unwrap();
            }
        }
        directories.push(FileDirectory {
            name: "General".into(),
            path,
            metadata_path: metadata,
            download_security: "FALSE".parse().unwrap(),
            ..Default::default()
        });
    } else if scenario == "single" {
        directories.push(FileDirectory {
            name: "General".into(),
            ..Default::default()
        });
    } else if scenario != "empty" {
        directories.push(FileDirectory {
            name: "HIDDEN DIRECTORY".into(),
            list_security: SecurityExpression::from_req_security(255),
            ..Default::default()
        });
    }
    if scenario == "browse" {
        for number in 0..1000 {
            directories.push(FileDirectory {
                name: format!("Directory {number:03}"),
                path: root.join(format!("private-path-{number}")),
                download_security: SecurityExpression::from_req_security(if number == 15 { 255 } else { 0 }),
                is_free: number == 15,
                has_new_files: number == 15,
                ..Default::default()
            });
        }
        directories.push(FileDirectory {
            name: format!("@HANGUP@ @CLS@ Gr\u{fc}\u{df}e {}", "long ".repeat(30)),
            ..Default::default()
        });
    }
    let conference = Conference {
        name: format!("E1 @CLS@ {}", "conference ".repeat(12)),
        is_public: true,
        directories: Some(Arc::new(directories)),
        ..Default::default()
    };
    let mut board = IcyBoard::new();
    board.root_path = root.into();
    board.config.paths.statistics_file = root.join("statistics.toml");
    board.default_display_text = DEFAULT_DISPLAY_TEXT.clone();
    board.users.new_user(User {
        name: "BROWSER".into(),
        security_level: 10,
        ..Default::default()
    });
    let user = board.users[0].clone();
    board.conferences.clear();
    board.conferences.push(conference.clone());
    let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let (peer, connection) = ChannelConnection::create_pair();
    let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(board)), nodes, node, Box::new(connection)).await;
    state.session.current_user = Some(user);
    state.session.cur_user_id = 0;
    state.session.user_name = "BROWSER".into();
    state.session.cur_security = 10;
    state.session.current_conference = conference;
    state.session.time_limit = 30;
    state.session.page_len = 0;
    state.session.disp_options.grapics_mode = GraphicsMode::Graphics;
    state.session.term_caps.is_utf8 = true;
    state.set_terminal_size(80, 25);
    state.session.tokens.extend([language.into(), "caller argument".into()]);
    (state, peer)
}

async fn read_frame(peer: &mut ChannelConnection, screen: &mut VirtualScreen) -> Vec<String> {
    loop {
        let mut packet = [0; 4096];
        match tokio::time::timeout(Duration::from_millis(150), peer.read(&mut packet)).await {
            Ok(read) => {
                let count = read.unwrap();
                assert_ne!(count, 0, "connection closed while waiting for browser frame");
                screen.write_bytes(&packet[..count]);
            }
            Err(_) => break,
        }
    }
    (0..25)
        .map(|row| (0..80).map(|column| screen.buffer.char_at(Position::new(column, row)).ch).collect())
        .collect()
}

fn assert_frame(rows: &[String], language: &str, scenario: &str) {
    assert!(rows[1].starts_with("| Files / E1 @CLS@"), "{language}/{scenario}: {rows:?}");
    for (row, text) in rows.iter().enumerate() {
        assert!(!text.contains("HIDDEN DIRECTORY"), "{language}/{scenario}: {rows:?}");
        assert!(!text.contains("private-path-"), "{language}/{scenario}: {rows:?}");
        assert!(
            text.chars().skip(78).all(|character| character == ' '),
            "{language}/{scenario}, row {row}: {rows:?}"
        );
        if row < 3 {
            assert_eq!(text.chars().nth(77), Some(if row == 1 { '|' } else { '+' }), "{language}/{scenario}: {rows:?}");
        }
    }
    assert!(rows[23].trim().is_empty() && rows[24].trim().is_empty(), "{language}/{scenario}: {rows:?}");
}

#[tokio::test]
async fn a4_directory_flag_rechecks_live_rights_and_index() {
    use crate::{compiler::user_data::UserDataValue, executable::VariableValue, icy_board::state::ppl_error::*, vm::VirtualMachine};
    for (scenario, expected) in [
        ("list", ERR_DENIED),
        ("download", ERR_DENIED),
        ("conference", ERR_DENIED),
        ("user", ERR_DENIED),
        ("path", ERR_INVALID),
        ("metadata", ERR_INVALID),
        ("removed", ERR_INVALID),
        ("missing-index", ERR_UNAVAILABLE),
        ("corrupt-index", ERR_IO),
        ("offline", ERR_UNAVAILABLE),
        ("unindexed", ERR_UNAVAILABLE),
        ("deleted", ERR_UNAVAILABLE),
    ] {
        let root = tempfile::tempdir().unwrap();
        let (mut state, mut peer) = fixture(root.path(), "en", "empty").await;
        let path = root.path().join("files");
        let metadata_path = root.path().join("index");
        std::fs::create_dir(&path).unwrap();
        let file = path.join("file.txt");
        if scenario != "unindexed" {
            std::fs::write(&file, b"test").unwrap();
        }
        let mut base = dizbase::file_base::FileBase::open(&path, &metadata_path).unwrap();
        if scenario == "deleted" {
            base.iter_mut().next().unwrap().set_deleted(true);
            base.save().unwrap();
        }
        drop(base);
        let snapshot = FileDirectory {
            path,
            metadata_path,
            valid: true,
            ..Default::default()
        };
        let mut live = snapshot.clone();
        match scenario {
            "list" => live.list_security = "FALSE".parse().unwrap(),
            "download" => live.download_security = "FALSE".parse().unwrap(),
            "path" => live.path = root.path().join("replacement"),
            "metadata" => live.metadata_path = root.path().join("replacement-index"),
            "offline" => std::fs::remove_file(&file).unwrap(),
            "unindexed" => std::fs::write(&file, b"test").unwrap(),
            "missing-index" => std::fs::remove_file(dizbase::file_base::FileBase::database_path(&snapshot.metadata_path)).unwrap(),
            "corrupt-index" => std::fs::write(dizbase::file_base::FileBase::database_path(&snapshot.metadata_path), b"not sqlite").unwrap(),
            _ => (),
        }
        let mut directories = DirectoryList::default();
        if scenario != "removed" {
            directories.push(live);
        }
        {
            let mut board = state.get_board().await;
            board.conferences[0].directories = Some(Arc::new(directories));
            if scenario == "conference" {
                board.conferences[0].required_security = "FALSE".parse().unwrap();
            }
        }
        if scenario == "user" {
            state.session.current_user = None;
        }
        state.session.batch_limit = 10;
        let previous = root.path().join("previous.zip");
        state.session.flagged_files.push(previous.clone());
        let registry = crate::parser::icy_board_registry();
        let mut io = DiskIO::new(root.path().to_str().unwrap(), None);
        let mut vm = VirtualMachine::new(root.path().join("flag.ppe"), &registry, &mut io, &mut state);
        let result = snapshot
            .call_function(
                &mut vm,
                &unicase::Ascii::new("Flag".into()),
                &[VariableValue::new_unbounded_string("file.txt".into())],
            )
            .await
            .unwrap();
        assert!(!result.as_bool(), "{scenario}");
        assert_eq!(vm.last_error.kind, ERR_KIND_FILE, "{scenario}");
        assert_eq!(vm.last_error.code, expected, "{scenario}");
        assert_eq!(vm.icy_board_state.session.flagged_files, [previous], "{scenario}");
        assert_eq!(
            vm.icy_board_state.session.tokens.iter().map(String::as_str).collect::<Vec<_>>(),
            ["en", "caller argument"]
        );
        let mut screen = VirtualScreen::new(icy_parser_core::AnsiParser::default());
        let rows = read_frame(&mut peer, &mut screen).await;
        assert!(rows.iter().all(|row| row.trim().is_empty()), "{scenario}: {rows:?}");
    }
}

#[tokio::test]
async fn e1_file_browser_download_command_returns_to_browser() {
    use crate::icy_board::state::local_transfer::{LocalFilePickerKind, LocalFilePickerRequest};
    let source = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ppe/files/src/main.pps")).unwrap();
    let executable = super::compile(&source);
    for language in ["en", "de"] {
        for scenario in ["success", "cancel", "denied", "pending", "empty"] {
            let root = tempfile::tempdir().unwrap();
            let (mut state, mut peer) = fixture(root.path(), language, "empty").await;
            let path = root.path().join("files");
            let metadata_path = root.path().join("index");
            let destination = root.path().join("destination");
            std::fs::create_dir(&path).unwrap();
            std::fs::create_dir(&destination).unwrap();
            let mut base = dizbase::file_base::FileBase::open(&path, &metadata_path).unwrap();
            for name in ["marked.txt", "unmarked.txt", "previous.txt"] {
                let file = path.join(name);
                std::fs::write(&file, format!("contents of {name}\n")).unwrap();
                base.add_file(&file, Vec::new()).unwrap();
            }
            drop(base);
            let mut directories = DirectoryList::default();
            directories.push(FileDirectory {
                name: "General".into(),
                path: path.clone(),
                metadata_path,
                ..Default::default()
            });
            state.session.current_conference.directories = Some(Arc::new(directories));
            {
                let mut board = state.get_board().await;
                board.conferences[0] = state.session.current_conference.clone();
                board.config.file_transfer.promote_to_batch_transfers = false;
                board.config.paths.transfer_log = root.path().join("transfer.log");
            }
            if scenario != "pending" {
                state.session.tokens.clear();
            }
            state.session.batch_limit = 10;
            state.session.bytes_remaining = -1;
            state.session.is_local = true;
            if scenario == "denied" {
                state.session.user_command_level.cmd_d = "FALSE".parse().unwrap();
            }
            if scenario != "empty" {
                state.session.flagged_files.push(path.join("previous.txt"));
            }
            let (sender, mut picker) = tokio::sync::mpsc::channel::<LocalFilePickerRequest>(1);
            state.node_state.lock().await[state.node].as_mut().unwrap().local_file_picker = Some(sender);
            let mut io = DiskIO::new(root.path().to_str().unwrap(), None);
            let mut completed = false;
            let drive = async {
                let mut screen = VirtualScreen::new(icy_parser_core::AnsiParser::default());
                read_frame(&mut peer, &mut screen).await;
                peer.send(b"\r\r").await.unwrap();
                let rows = read_frame(&mut peer, &mut screen).await;
                assert_frame(&rows, language, scenario);
                assert!(rows[22].contains("D Download"), "{rows:?}");
                if scenario != "empty" {
                    peer.send(b"m").await.unwrap();
                    read_frame(&mut peer, &mut screen).await;
                }
                peer.send(b"\x1b[B\r").await.unwrap();
                let before = read_frame(&mut peer, &mut screen).await;
                assert!(before[4].contains("unmarked.txt"), "{before:?}");
                peer.send(b"d").await.unwrap();
                let rows = read_frame(&mut peer, &mut screen).await;
                match scenario {
                    "success" | "cancel" => {
                        assert!(!rows[1].contains("E1 @CLS@"), "download did not open: {rows:?}");
                        peer.send(b"Y\r\r").await.unwrap();
                        let request = picker.recv().await.expect("download did not open local picker");
                        assert_eq!(request.kind, LocalFilePickerKind::DownloadDirectory);
                        request.response.send((scenario == "success").then(|| destination.clone())).unwrap();
                        let rows = read_frame(&mut peer, &mut screen).await;
                        assert_frame(&rows, language, scenario);
                        assert!(rows[4].contains("unmarked.txt"), "{rows:?}");
                    }
                    "empty" => {
                        assert!(!rows[1].contains("E1 @CLS@"), "empty batch did not prompt: {rows:?}");
                        peer.send(b"\r").await.unwrap();
                        let rows = read_frame(&mut peer, &mut screen).await;
                        assert_frame(&rows, language, scenario);
                        assert!(rows[4].contains("unmarked.txt"), "{rows:?}");
                    }
                    "pending" => {
                        assert_frame(&rows, language, scenario);
                        assert!(rows[21].contains("caller arguments"), "{rows:?}");
                    }
                    "denied" => {
                        assert_frame(&rows, language, scenario);
                        assert!(rows[4].contains("unmarked.txt"), "{rows:?}");
                    }
                    _ => unreachable!(),
                }
                assert!(picker.try_recv().is_err(), "unexpected extra picker");
                for _ in 0..3 {
                    peer.send(b"\x1b").await.unwrap();
                    let rows = read_frame(&mut peer, &mut screen).await;
                    assert_frame(&rows, language, scenario);
                }
                completed = true;
                peer.send(b"\x1b").await.unwrap();
                std::future::pending::<()>().await;
            };
            let caller = root.path().join("browser.ppe");
            let result = tokio::time::timeout(Duration::from_secs(10), async {
                tokio::select! {
                    result = run(&caller, &executable, &mut io, &mut state) => result,
                    () = drive => unreachable!(),
                }
            })
            .await
            .unwrap_or_else(|_| panic!("{language}/{scenario}: download command timed out"))
            .unwrap();
            assert!(result && completed);
            if scenario == "success" {
                for name in ["marked.txt", "previous.txt"] {
                    assert_eq!(std::fs::read(destination.join(name)).unwrap(), std::fs::read(path.join(name)).unwrap());
                }
                assert_eq!(std::fs::read_dir(&destination).unwrap().count(), 2);
                assert!(state.session.flagged_files.is_empty());
                assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_downloads, 2);
            } else {
                assert_eq!(std::fs::read_dir(&destination).unwrap().count(), 0);
                assert_eq!(
                    state.session.flagged_files,
                    if scenario == "empty" {
                        vec![]
                    } else {
                        vec![path.join("previous.txt"), path.join("marked.txt")]
                    }
                );
                assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_downloads, 0);
            }
            assert_eq!(
                state.session.tokens.iter().map(String::as_str).collect::<Vec<_>>(),
                if scenario == "pending" { vec![language, "caller argument"] } else { vec![] }
            );
            assert_eq!(state.session.security_violations, i32::from(scenario == "denied"));
            assert_eq!(state.session.current_conference_number, 0);
            assert!(!state.session.request_logoff);
        }
    }
}

#[tokio::test]
async fn e1_file_browser_marks_without_downloading() {
    let source = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ppe/files/src/main.pps")).unwrap();
    let executable = super::compile(&source);
    for language in ["en", "de"] {
        for allowed in [true, false] {
            let root = tempfile::tempdir().unwrap();
            let (mut state, mut peer) = fixture(root.path(), language, "empty").await;
            let path = root.path().join("files");
            let metadata_path = root.path().join("index");
            std::fs::create_dir(&path).unwrap();
            let name = "@HANGUP@ @CLS@.txt";
            let file = path.join(name);
            std::fs::write(&file, b"test").unwrap();
            drop(dizbase::file_base::FileBase::open(&path, &metadata_path).unwrap());
            let mut directories = DirectoryList::default();
            directories.push(FileDirectory {
                name: "General".into(),
                path,
                metadata_path,
                download_security: if allowed { "TRUE" } else { "FALSE" }.parse().unwrap(),
                ..Default::default()
            });
            state.session.current_conference.directories = Some(Arc::new(directories));
            state.get_board().await.conferences[0] = state.session.current_conference.clone();
            let previous = root.path().join("previous.zip");
            state.session.flagged_files.push(previous.clone());
            state.session.batch_limit = 2;
            let mut io = DiskIO::new(root.path().to_str().unwrap(), None);
            let mut completed = false;
            let drive = async {
                let mut screen = VirtualScreen::new(icy_parser_core::AnsiParser::default());
                read_frame(&mut peer, &mut screen).await;
                peer.send(b"\r\r").await.unwrap();
                read_frame(&mut peer, &mut screen).await;
                for keys in [b"m".as_slice(), b"m", b"\rm"] {
                    peer.send(keys).await.unwrap();
                    let rows = read_frame(&mut peer, &mut screen).await;
                    assert_frame(&rows, language, "flag");
                    let expected = if allowed { "Marked: @HANGUP@ @CLS@.txt" } else { "Marking denied." };
                    assert_eq!(rows[21].trim(), expected, "{rows:?}");
                    assert!(rows[22].contains("M Mark"));
                }
                for _ in 0..3 {
                    peer.send(b"\x1b").await.unwrap();
                    let rows = read_frame(&mut peer, &mut screen).await;
                    assert_frame(&rows, language, "flag");
                }
                completed = true;
                peer.send(b"\x1b").await.unwrap();
                std::future::pending::<()>().await;
            };
            let caller = root.path().join("browser.ppe");
            let result = tokio::time::timeout(Duration::from_secs(10), async {
                tokio::select! {
                    result = run(&caller, &executable, &mut io, &mut state) => result,
                    () = drive => unreachable!(),
                }
            })
            .await
            .expect("marking browser timed out")
            .unwrap();
            assert!(result && completed);
            assert_eq!(state.session.flagged_files, if allowed { vec![previous, file] } else { vec![previous] });
            assert_eq!(
                state.session.tokens.iter().map(String::as_str).collect::<Vec<_>>(),
                [language, "caller argument"]
            );
            assert!(!state.session.request_logoff);
            assert_eq!(state.session.current_user.as_ref().unwrap().stats.num_downloads, 0);
        }
    }
}

#[tokio::test]
async fn a4_directory_flag_is_exact_silent_and_preserves_session() {
    let root = tempfile::tempdir().unwrap();
    let (mut state, mut peer) = fixture(root.path(), "en", "empty").await;
    let mut directories = DirectoryList::default();
    let mut paths = Vec::new();
    for number in 0..2 {
        let path = root.path().join(format!("area-{number}"));
        let metadata_path = root.path().join(format!("index-{number}"));
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("shared.txt"), b"test").unwrap();
        drop(dizbase::file_base::FileBase::open(&path, &metadata_path).unwrap());
        paths.push(path.join("shared.txt"));
        directories.push(FileDirectory {
            path,
            metadata_path,
            ..Default::default()
        });
    }
    state.session.current_conference.directories = Some(Arc::new(directories));
    state.get_board().await.conferences[0] = state.session.current_conference.clone();
    state.session.batch_limit = 2;
    let previous = root.path().join("already-flagged.zip");
    state.session.flagged_files.push(previous.clone());
    state.session.op_text = "caller text".into();
    let source = r#"
DIRECTORY directory = Board.Conferences[0].Directories[1]
PRINT directory.Flag("SHARED.TXT"), ":", Error.Last().OK
PRINT ":", directory.Flag("shared.txt"), ":", Error.Last().OK
PRINT ":", Board.Conferences[0].Directories[0].Flag("shared.txt"), ":", Error.Last().Code = ErrCode.Limit
PRINT ":", directory.Flag("*.txt"), ":", Error.Last().Code = ErrCode.Unavailable
PRINT ":", directory.Flag("../shared.txt"), ":", Error.Last().Code = ErrCode.Invalid
DIRECTORY invalid
PRINT ":", invalid.Flag("shared.txt"), ":", Error.Last().Code = ErrCode.Invalid
PRINT ":", directory.Flag("shared.txt"), ":", Error.Last().OK
EXIT
"#;
    let executable = super::compile(source);
    let mut io = DiskIO::new(root.path().to_str().unwrap(), None);
    assert!(run(&root.path().join("flag.ppe"), &executable, &mut io, &mut state).await.unwrap());
    assert_eq!(state.session.flagged_files, [previous, paths[1].clone()]);
    assert_eq!(state.session.tokens.iter().map(String::as_str).collect::<Vec<_>>(), ["en", "caller argument"]);
    assert_eq!(state.session.current_conference_number, 0);
    assert_eq!(state.session.op_text, "caller text");
    let mut screen = VirtualScreen::new(icy_parser_core::AnsiParser::default());
    let rows = read_frame(&mut peer, &mut screen).await;
    assert_eq!(rows[0].trim_end(), "1:1:1:1:0:1:0:1:0:1:0:1:1:1");
    assert!(rows[1..].iter().all(|row| row.trim().is_empty()), "{rows:?}");
}

#[tokio::test]
async fn e1_legacy_download_selection_is_conference_wide() {
    let root = tempfile::tempdir().unwrap();
    let (mut state, mut peer) = fixture(root.path(), "en", "empty").await;
    let mut directories = DirectoryList::default();
    let mut expected_files = Vec::new();
    for number in 0..2 {
        let path = root.path().join(format!("area-{number}"));
        let metadata_path = root.path().join(format!("index-{number}"));
        std::fs::create_dir_all(&path).unwrap();
        let mut base = dizbase::file_base::FileBase::open(&path, &metadata_path).unwrap();
        let file = path.join("shared.txt");
        std::fs::write(&file, format!("Area {number}\n")).unwrap();
        base.add_file(&file, Vec::new()).unwrap();
        expected_files.push(file);
        directories.push(FileDirectory {
            path,
            metadata_path,
            ..Default::default()
        });
    }
    state.session.current_conference.directories = Some(Arc::new(directories));
    state.get_board().await.conferences[0] = state.session.current_conference.clone();
    state.session.tokens.clear();
    state.session.batch_limit = 10;
    let caller_path = root.path().join("selection.ppe");
    let mut io = DiskIO::new(root.path().to_str().unwrap(), None);
    let executable = super::compile("FLAG \"shared.txt\"\nEXIT");
    assert!(run(&caller_path, &executable, &mut io, &mut state).await.unwrap());
    assert_eq!(state.session.flagged_files, expected_files);
    let mut screen = VirtualScreen::new(icy_parser_core::AnsiParser::default());
    read_frame(&mut peer, &mut screen).await;
    state.session.flagged_files.clear();
    let executable = super::compile(&format!("FLAG \"{}\"\nEXIT", expected_files[0].display()));
    assert!(run(&caller_path, &executable, &mut io, &mut state).await.unwrap());
    assert!(state.session.flagged_files.is_empty());
}

#[tokio::test]
async fn e1_directory_browser_call_ignores_language_token() {
    let source = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ppe/files/src/main.pps")).unwrap();
    let browser = super::compile(&source).to_buffer().unwrap();
    for language in ["en", "de"] {
        let root = tempfile::tempdir().unwrap();
        let (mut state, mut peer) = fixture(root.path(), language, "single").await;
        state.session.tokens.clear();
        let browser_path = root.path().join("files.ppe");
        std::fs::write(&browser_path, &browser).unwrap();
        let caller = super::compile(&format!("TOKENIZE \"{language}\"\nCALL \"{}\"\nEXIT", browser_path.display()));
        let mut io = DiskIO::new(root.path().to_str().unwrap(), None);
        let caller_path = root.path().join("browse.ppe");
        let mut rendered = false;
        let drive = async {
            let mut screen = VirtualScreen::new(icy_parser_core::AnsiParser::default());
            let rows = read_frame(&mut peer, &mut screen).await;
            assert_frame(&rows, language, "single");
            assert!(rows[4].starts_with(" >   1 General"), "{rows:?}");
            assert!(rows[20].trim_end().ends_with("1   1/1"), "{rows:?}");
            peer.send(b"\r").await.unwrap();
            let rows = read_frame(&mut peer, &mut screen).await;
            assert_frame(&rows, language, "single");
            assert!(rows[4].contains("General"), "{rows:?}");
            assert!(rows[6].trim_end().ends_with(": 1"), "{rows:?}");
            peer.send(b"\x1b").await.unwrap();
            let rows = read_frame(&mut peer, &mut screen).await;
            assert_frame(&rows, language, "single");
            assert!(rows[4].starts_with(" >   1 General"), "{rows:?}");
            rendered = true;
            peer.send(b"\x1b").await.unwrap();
            std::future::pending::<()>().await;
        };
        let result = tokio::time::timeout(Duration::from_secs(5), async {
            tokio::select! {
                result = run(&caller_path, &caller, &mut io, &mut state) => result,
                () = drive => unreachable!(),
            }
        })
        .await
        .expect("browser CALL timed out")
        .unwrap();
        assert!(result);
        assert!(rendered, "CALL did not open the {language} browser");
        assert!(!state.session.request_logoff);
    }
}

#[tokio::test]
async fn e1_file_browser_pages_search_and_descriptions() {
    let source = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ppe/files/src/main.pps")).unwrap();
    let executable = super::compile(&source);
    for language in ["en", "de"] {
        for scenario in ["files", "empty-index", "missing-index"] {
            let root = tempfile::tempdir().unwrap();
            let (mut state, mut peer) = fixture(root.path(), language, scenario).await;
            let mut io = DiskIO::new(root.path().to_str().unwrap(), None);
            let mut completed = false;
            let drive = async {
                let mut screen = VirtualScreen::new(icy_parser_core::AnsiParser::default());
                let rows = read_frame(&mut peer, &mut screen).await;
                assert_frame(&rows, language, scenario);
                peer.send(b"\r\r").await.unwrap();
                let rows = read_frame(&mut peer, &mut screen).await;
                assert_frame(&rows, language, scenario);
                if scenario == "files" {
                    assert!(rows[4].starts_with(" >file0000.txt"), "{rows:?}");
                    assert!(rows[4].contains("2147483648"), "{rows:?}");
                    assert!(rows[18].contains("file0014.txt"), "{rows:?}");
                    peer.send(b"\r").await.unwrap();
                    let rows = read_frame(&mut peer, &mut screen).await;
                    assert_frame(&rows, language, scenario);
                    assert!(rows[8].contains("@HANGUP@ @CLS@ Gr\u{fc}\u{df}e"), "{rows:?}");
                    peer.send(b"\x1b[F").await.unwrap();
                    let rows = read_frame(&mut peer, &mut screen).await;
                    assert_frame(&rows, language, scenario);
                    assert!(rows[8].contains("TAIL"), "{rows:?}");
                    peer.send(b"\x1b").await.unwrap();
                    read_frame(&mut peer, &mut screen).await;
                    peer.send(b"\x1b[6~").await.unwrap();
                    let rows = read_frame(&mut peer, &mut screen).await;
                    assert_frame(&rows, language, scenario);
                    assert!(rows[4].starts_with(" >file0015.txt"), "{rows:?}");
                    peer.send(b"\x1b[6~".repeat(15).as_slice()).await.unwrap();
                    let rows = read_frame(&mut peer, &mut screen).await;
                    assert_frame(&rows, language, scenario);
                    assert!(rows[4].starts_with(" >file0240.txt"), "{rows:?}");
                    peer.send(b"\x1b[5~").await.unwrap();
                    let rows = read_frame(&mut peer, &mut screen).await;
                    assert_frame(&rows, language, scenario);
                    assert!(rows[4].starts_with(" >file0225.txt"), "{rows:?}");
                    peer.send(b"fneedle\r").await.unwrap();
                    let rows = read_frame(&mut peer, &mut screen).await;
                    assert_frame(&rows, language, scenario);
                    assert!(rows[5].contains("Search continues"), "{rows:?}");
                    peer.send(b"\x1b[6~").await.unwrap();
                    let rows = read_frame(&mut peer, &mut screen).await;
                    assert_frame(&rows, language, scenario);
                    assert!(rows[4].starts_with(" >file1025.txt"), "{rows:?}");
                    assert!(!rows[20].contains('>'), "{rows:?}");
                    peer.send(b"fwrong").await.unwrap();
                    read_frame(&mut peer, &mut screen).await;
                    peer.send(b"\x1b").await.unwrap();
                    let rows = read_frame(&mut peer, &mut screen).await;
                    assert!(rows[21].trim_end().ends_with(": needle"), "{rows:?}");
                    peer.send(b"f\x08\x08\x08\x08\x08\x08absent\r").await.unwrap();
                    read_frame(&mut peer, &mut screen).await;
                    peer.send(b"\x1b[6~").await.unwrap();
                    let rows = read_frame(&mut peer, &mut screen).await;
                    assert_frame(&rows, language, scenario);
                    assert!(rows[5].contains("No files"), "{rows:?}");
                    peer.send(b"f\x08\x08\x08\x08\x08\x08\rr").await.unwrap();
                    let rows = read_frame(&mut peer, &mut screen).await;
                    assert_frame(&rows, language, scenario);
                    assert!(rows[4].starts_with(" >file0000.txt"), "{rows:?}");
                } else {
                    assert!(
                        rows[5].contains(if scenario == "empty-index" { "No files" } else { "File index unavailable" }),
                        "{rows:?}"
                    );
                    peer.send(b"\r\x1b[6~\x1b[5~").await.unwrap();
                    let rows = read_frame(&mut peer, &mut screen).await;
                    assert_frame(&rows, language, scenario);
                }
                for _ in 0..2 {
                    peer.send(b"\x1b").await.unwrap();
                    let rows = read_frame(&mut peer, &mut screen).await;
                    assert_frame(&rows, language, scenario);
                }
                assert!(screen.buffer.char_at(Position::new(1, 4)).ch == '>');
                completed = true;
                peer.send(b"\x1b").await.unwrap();
                std::future::pending::<()>().await;
            };
            let path = root.path().join("files.ppe");
            let result = tokio::time::timeout(Duration::from_secs(15), async {
                tokio::select! {
                    result = run(&path, &executable, &mut io, &mut state) => result,
                    () = drive => unreachable!(),
                }
            })
            .await
            .expect("file browser timed out")
            .unwrap();
            assert!(result && completed, "{language}/{scenario}");
            assert!(!state.session.request_logoff);
            assert_eq!(state.session.current_conference_number, 0);
            assert_eq!(
                state.session.tokens.iter().map(String::as_str).collect::<Vec<_>>(),
                [language, "caller argument"]
            );
        }
    }
}

#[tokio::test]
async fn e1_directory_browser_rendering_and_navigation() {
    let source = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ppe/files/src/main.pps")).unwrap();
    let executable = super::compile(&source);
    for language in ["en", "de"] {
        for scenario in ["empty", "denied", "browse"] {
            let root = tempfile::tempdir().unwrap();
            let (mut state, mut peer) = fixture(root.path(), language, scenario).await;
            let mut io = DiskIO::new(root.path().to_str().unwrap(), None);
            let mut visited = 0;
            let drive = async {
                let mut screen = VirtualScreen::new(icy_parser_core::AnsiParser::default());
                let rows = read_frame(&mut peer, &mut screen).await;
                assert_frame(&rows, language, scenario);
                let no_results = "No accessible directories found.";
                if scenario != "browse" {
                    assert!(rows[4].contains(no_results), "{language}/{scenario}: {rows:?}");
                    assert!(rows[20].trim_end().ends_with(": 0"), "{rows:?}");
                    peer.send(b"\r").await.unwrap();
                    let rows = read_frame(&mut peer, &mut screen).await;
                    assert_frame(&rows, language, scenario);
                    assert!(rows[4].contains(no_results), "{rows:?}");
                    visited = 1;
                    peer.send(b"\x1b").await.unwrap();
                    std::future::pending::<()>().await;
                }
                assert!(rows[4].starts_with(" >   2 Directory 000"), "{rows:?}");
                assert!(rows[18].contains("Directory 014"), "{rows:?}");
                assert!(rows[20].contains("1001   1/1001"), "{rows:?}");

                for (phase, keys) in [
                    b"\x1b[6~".as_slice(),
                    b"\r",
                    b"\x1b",
                    b"\x1b[F",
                    b"\x1b[H",
                    b"f",
                    b"999\r",
                    b"r",
                    b"f",
                    b"no such directory\r",
                    b"\r",
                    b"r",
                    b"f",
                    b"999x\x08\r",
                    b"f",
                    b"\x08\x08\x08000",
                    b"\x1b",
                    b"r",
                    b"\x1b[A\x1b[6~\x1b[5~",
                    b"f",
                    b"123456789012345678901234567890123456789012345678901234567890",
                    b"\x1b",
                ]
                .into_iter()
                .enumerate()
                {
                    peer.send(keys).await.unwrap();
                    let rows = read_frame(&mut peer, &mut screen).await;
                    assert_frame(&rows, language, scenario);
                    match phase {
                        0 | 2 => {
                            assert!(rows[18].starts_with(" >  17 Directory 015"), "{rows:?}");
                            assert!(rows[18].contains("Denied"), "{rows:?}");
                            assert!(rows[20].contains("16/1001"), "{rows:?}");
                        }
                        1 => {
                            assert!(rows[4].contains("Directory 015"), "{rows:?}");
                            assert!(rows[6].trim_end().ends_with(": 17"), "{rows:?}");
                            assert!(rows[8].contains("Download: Denied"), "{rows:?}");
                            assert!(rows[9].trim_end().ends_with(": Yes"), "{rows:?}");
                            assert!(rows[10].trim_end().ends_with(": Yes"), "{rows:?}");
                        }
                        3 => {
                            assert!(rows[18].contains("@HANGUP@ @CLS@ Gr\u{fc}\u{df}e"), "{rows:?}");
                            assert!(rows[20].contains("1001/1001"), "{rows:?}");
                        }
                        4 | 7 | 11 | 17 | 18 | 21 => {
                            assert!(rows[4].starts_with(" >   2 Directory 000"), "{rows:?}");
                            assert!(rows[20].contains("1001   1/1001"), "{rows:?}");
                        }
                        5 | 8 | 12 | 14 | 19 => {
                            assert!(rows[21].contains("Search"), "{rows:?}");
                            assert!(rows[22].contains("Esc Cancel"), "{rows:?}");
                        }
                        6 | 13 | 16 => {
                            assert!(rows[4].starts_with(" >1001 Directory 999"), "{rows:?}");
                            assert!(rows[5].trim().is_empty(), "{rows:?}");
                            assert!(rows[20].contains("1   1/1"), "{rows:?}");
                            assert!(rows[21].contains("999"), "{rows:?}");
                        }
                        9 | 10 => {
                            assert!(rows[4].contains(no_results), "{rows:?}");
                            assert!(rows[20].trim_end().ends_with(": 0"), "{rows:?}");
                        }
                        15 => {
                            assert!(rows[4].contains("Directory 999"), "{rows:?}");
                            assert!(rows[21].trim_end().ends_with(": 000"), "{rows:?}");
                        }
                        20 => {
                            let (_, query) = rows[21].split_once(": ").unwrap();
                            assert_eq!(query.trim_end(), "123456789012345678901234567890123456789012345678");
                        }
                        _ => unreachable!(),
                    }
                    visited += 1;
                }
                peer.send(b"\x1b").await.unwrap();
                std::future::pending::<()>().await;
            };
            let path = root.path().join("files.ppe");
            let result = tokio::time::timeout(Duration::from_secs(15), async {
                tokio::select! {
                    result = run(&path, &executable, &mut io, &mut state) => result,
                    () = drive => unreachable!(),
                }
            })
            .await
            .expect("directory browser timed out")
            .unwrap();
            assert!(result, "{language}/{scenario}: browser stopped unexpectedly");
            assert_eq!(visited, if scenario == "browse" { 22 } else { 1 }, "{language}/{scenario}");
            assert!(!state.session.request_logoff, "{language}/{scenario}");
            assert_eq!(state.session.current_conference_number, 0);
            assert_eq!(
                state.session.tokens.iter().map(String::as_str).collect::<Vec<_>>(),
                [language, "caller argument"]
            );
        }
    }
}
