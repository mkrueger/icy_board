use super::{compile_errors, compile_errors_with_runtime, run_ppl, run_ppl_with_files};

const A6_SETTINGS_TYPE: &str = r#"
TYPE Settings
    INTEGER PageRows
    STRING Search
    BOOLEAN Descending
ENDTYPE
"#;

fn a6_save_settings_source(path: &std::path::Path, temporary: &std::path::Path, label: &str, rows: i32) -> String {
    format!(
        r#"
{A6_SETTINGS_TYPE}
Settings candidate = Settings {{ PageRows = {rows}, Search = "{label}", Descending = TRUE }}
FOPEN 3, "{lock_path}", O_RW, S_DB
IF FERR(3) GOTO Failed
FCREATE 1, "{temporary}", O_WR, S_DB
IF FERR(1) GOTO Failed
FPUTLN 1, "PPESETTINGS 1"
IF FERR(1) GOTO Failed
FPUTREC 1, candidate
IF FERR(1) GOTO Failed
FFLUSH 1
IF FERR(1) GOTO Failed
FCLOSE 1
IF FERR(1) GOTO Failed
RENAME "{temporary}", "{path}"
IF !Error.Last().OK GOTO Failed
FCLOSE 3
PRINTLN "saved"
EXIT
:Failed
PRINTLN "failed:", Error.Last().Kind = ErrKind.File
EXIT
"#,
        temporary = temporary.display(),
        lock_path = path.with_extension("lock").display(),
        path = path.display()
    )
}

#[test]
fn a6_settings_survive_new_ppe_runs_and_failed_staged_saves() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("settings-user-1.dat");
    let other_user = root.path().join("settings-user-2.dat");
    let temporary = root.path().join("settings-user-1.node-1.tmp");
    let other_temporary = root.path().join("settings-user-2.node-2.tmp");
    for label in ["Search", "Suche"] {
        assert_eq!(run_ppl(&a6_save_settings_source(&path, &temporary, label, 20)), "saved\n");
        let saved = std::fs::read(&path).unwrap();
        let load = format!(
            r#"
{A6_SETTINGS_TYPE}
Settings current
STRING header
FOPEN 1, "{}", O_RD, S_DN
FGET 1, header
IF header <> "PPESETTINGS 1" GOTO Failed
FGETREC 1, current
IF FERR(1) GOTO Failed
FCLOSE 1
PRINTLN current.PageRows, ":", current.Search, ":", current.Descending
EXIT
:Failed
PRINTLN "failed"
"#,
            path.display()
        );
        assert_eq!(run_ppl(&load), format!("20:{label}:1\n"));
        assert_eq!(run_ppl(&a6_save_settings_source(&other_user, &other_temporary, "other", 10)), "saved\n");
        assert_eq!(std::fs::read(&path).unwrap(), saved);

        std::fs::create_dir(&temporary).unwrap();
        assert_eq!(run_ppl(&a6_save_settings_source(&path, &temporary, "changed", 30)), "failed:1\n");
        assert_eq!(std::fs::read(&path).unwrap(), saved);
        std::fs::remove_dir(&temporary).unwrap();

        std::fs::write(&temporary, b"staged").unwrap();
        let write_failure = a6_save_settings_source(&path, &temporary, "changed", 30)
            .replace("FCREATE 1,", "FOPEN 1,")
            .replace("O_WR, S_DB", "O_RD, S_DB");
        assert_eq!(run_ppl(&write_failure), "failed:1\n");
        assert_eq!(std::fs::read(&path).unwrap(), saved);

        let record_write_failure = a6_save_settings_source(&path, &temporary, "changed", 30).replace(
            "FPUTREC 1, candidate",
            &format!("FCLOSE 1\nFOPEN 1, \"{}\", O_RD, S_DB\nFPUTREC 1, candidate", temporary.display()),
        );
        assert_eq!(run_ppl(&record_write_failure), "failed:1\n");
        assert_eq!(std::fs::read(&path).unwrap(), saved);

        let rename_failure =
            a6_save_settings_source(&path, &temporary, "changed", 30).replace("RENAME ", &format!("DELETE \"{}\"\nRENAME ", temporary.display()));
        assert_eq!(run_ppl(&rename_failure), "failed:1\n");
        assert_eq!(std::fs::read(&path).unwrap(), saved);
        assert_eq!(run_ppl(&load), format!("20:{label}:1\n"));

        let stopped = a6_save_settings_source(&path, &temporary, "changed", 30).replace("RENAME ", "STOP\nRENAME ");
        assert_eq!(run_ppl(&stopped), "");
        assert_eq!(std::fs::read(&path).unwrap(), saved);
        assert!(temporary.exists());
        assert_eq!(run_ppl(&a6_save_settings_source(&path, &temporary, label, 20)), "saved\n");
        assert!(!temporary.exists());
    }
}

#[test]
fn a6_settings_load_checks_version_and_preserves_defaults_on_failure() {
    for label in ["Search", "Suche"] {
        for (contents, status, expected) in [
            (None, "missing", format!("15:{label}:0")),
            (Some(b"PPESETTINGS 1\n20\nfiles\n1\n".as_slice()), "loaded", "20:files:1".to_owned()),
            (Some(b"PPESETTINGS 2\nnot-a-v1-record\n".as_slice()), "version", format!("15:{label}:0")),
            (Some(b"PPESETTINGS 1\n20\nfiles\ninvalid\n".as_slice()), "invalid", format!("15:{label}:0")),
            (Some(b"PPESETTINGS 1\n20\n".as_slice()), "invalid", format!("15:{label}:0")),
        ] {
            let source = format!(
                r#"
{A6_SETTINGS_TYPE}
Settings current = Settings {{ PageRows = 15, Search = "{label}", Descending = FALSE }}
Settings candidate = current
STRING status = "missing"
STRING header
FOPEN 1, "settings.dat", O_RD, S_DN
IF !FERR(1) THEN
    FGET 1, header
    IF header = "PPESETTINGS 1" THEN
        FGETREC 1, candidate
        IF FERR(1) THEN
            status = "invalid"
        ELSE
            current = candidate
            status = "loaded"
        ENDIF
    ELSE
        status = "version"
    ENDIF
    FCLOSE 1
ENDIF
PRINTLN status
PRINTLN current.PageRows, ":", current.Search, ":", current.Descending
"#
            );
            let files = contents.map(|contents| vec![("settings.dat", contents)]).unwrap_or_default();
            assert_eq!(run_ppl_with_files(&source, &files), format!("{status}\n{expected}\n"), "{label}/{status}");
        }
    }
}

#[tokio::test]
async fn a6_settings_two_nodes_lock_the_entire_update() {
    use crate::{
        icy_board::{IcyBoard, bbs::BBS, state::IcyBoardState, user_base::User},
        vm::{DiskIO, run},
    };
    use icy_net::{Connection, ConnectionType, channel::ChannelConnection};
    use std::{sync::Arc, time::Duration};

    async fn read_line(peer: &mut ChannelConnection) -> String {
        let mut output = Vec::new();
        while !output.contains(&b'\n') {
            let mut buffer = [0; 256];
            let count = peer.read(&mut buffer).await.unwrap();
            assert_ne!(count, 0);
            output.extend_from_slice(&buffer[..count]);
        }
        String::from_utf8(output).unwrap().replace('\r', "")
    }

    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("settings-user-1.dat");
    let lock_path = root.path().join("settings-user-1.lock");
    std::fs::write(&path, b"PPESETTINGS 1\n20\nshared\n1\n").unwrap();
    let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(2)));
    let mut board = IcyBoard::new();
    board.root_path = root.path().into();
    board.default_display_text = crate::icy_board::icb_text::DEFAULT_DISPLAY_TEXT.clone();
    board.users.new_user(User {
        name: "SETTINGS".into(),
        ..Default::default()
    });
    let user = board.users[0].clone();
    let board = Arc::new(tokio::sync::Mutex::new(board));
    let mut sessions = Vec::new();
    for node_index in 0..2 {
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (peer, connection) = ChannelConnection::create_pair();
        let mut state = IcyBoardState::new(bbs.clone(), board.clone(), nodes, node, Box::new(connection)).await;
        state.session.current_user = Some(user.clone());
        state.session.cur_user_id = 0;
        state.session.term_caps.is_utf8 = true;
        state.session.time_limit = 30;
        state.session.page_len = 0;
        let temporary = root.path().join(format!("settings-user-1.node-{node_index}.tmp"));
        let source = format!(
            r#"
{A6_SETTINGS_TYPE}
STARTDISP FNS
TERMINPUT input = Terminal.Input
Settings current
STRING header
EVENT gate
IF {node_index} = 1 THEN
    PRINTLN "ready"
    gate = input.Wait(-1)
ENDIF
FOPEN 3, "{lock_path}", O_RW, S_DB
BOOLEAN blocked = FERR(3)
PRINTLN "locked:", !blocked, ":", Error.Last().OK
IF blocked THEN
    PRINTLN "conflict:", Error.Last().Kind = ErrKind.File, ":", Error.Last().Code = ErrCode.IO
    gate = input.Wait(-1)
    FOPEN 3, "{lock_path}", O_RW, S_DB
    IF FERR(3) GOTO Failed
ENDIF
gate = input.Wait(-1)
FOPEN 1, "{path}", O_RD, S_DN
FGET 1, header
FGETREC 1, current
IF FERR(1) GOTO Failed
FCLOSE 1
current.PageRows = current.PageRows + 1
PRINTLN "read:", current.PageRows
gate = input.Wait(-1)
FCREATE 2, "{temporary}", O_WR, S_DB
IF FERR(2) GOTO Failed
FPUTLN 2, "PPESETTINGS 1"
FPUTREC 2, current
IF FERR(2) GOTO Failed
FFLUSH 2
IF FERR(2) GOTO Failed
FCLOSE 2
RENAME "{temporary}", "{path}"
BOOLEAN saved = Error.Last().OK
FCLOSE 3
PRINTLN "saved:", saved
input.Release()
EXIT
:Failed
PRINTLN "failed"
EXIT
"#,
            path = path.display(),
            lock_path = lock_path.display(),
            temporary = temporary.display()
        );
        sessions.push((state, peer, super::compile(&source), DiskIO::new(root.path().to_str().unwrap(), None)));
    }
    let (mut second, mut second_peer, second_ppe, mut second_io) = sessions.pop().unwrap();
    let (mut first, mut first_peer, first_ppe, mut first_io) = sessions.pop().unwrap();
    let ppe_path = root.path().join("settings.ppe");
    let first_run = run(&ppe_path, &first_ppe, &mut first_io, &mut first);
    let second_run = run(&ppe_path, &second_ppe, &mut second_io, &mut second);
    let interleave = async {
        assert_eq!(read_line(&mut first_peer).await, "locked:1:1\n");
        assert_eq!(read_line(&mut second_peer).await, "ready\n");
        second_peer.send(b"o").await.unwrap();
        let mut conflict = read_line(&mut second_peer).await;
        while !conflict.contains("conflict:1:1\n") {
            conflict.push_str(&read_line(&mut second_peer).await);
        }
        assert_eq!(conflict, "locked:0:0\nconflict:1:1\n");
        first_peer.send(b"r").await.unwrap();
        assert_eq!(read_line(&mut first_peer).await, "read:21\n");
        first_peer.send(b"w").await.unwrap();
        assert_eq!(read_line(&mut first_peer).await, "saved:1\n");
        second_peer.send(b"rr").await.unwrap();
        assert_eq!(read_line(&mut second_peer).await, "read:22\n");
        second_peer.send(b"w").await.unwrap();
        assert_eq!(read_line(&mut second_peer).await, "saved:1\n");
    };
    let (first_result, second_result, ()) = tokio::time::timeout(Duration::from_secs(5), async { tokio::join!(first_run, second_run, interleave) })
        .await
        .unwrap();
    assert!(first_result.unwrap());
    assert!(second_result.unwrap());
    let saved = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        saved.trim_start_matches('\u{feff}'),
        "PPESETTINGS 1\n22\nshared\n1\n",
        "the retried update must read the first node's committed value"
    );
    assert!(lock_path.exists());
}

#[tokio::test]
async fn a6_settings_lock_is_released_on_every_ppe_exit() {
    use crate::{
        icy_board::{IcyBoard, bbs::BBS, state::IcyBoardState, user_base::User},
        vm::{DiskIO, PCBoardIO, run},
    };
    use icy_net::{Connection, ConnectionType, channel::ChannelConnection};
    use std::{sync::Arc, time::Duration};

    for (case, ending) in [
        ("exit", "EXIT"),
        ("stop", "STOP"),
        ("error", "FSEEK 1, 0, 99"),
        ("disconnect", "EXIT"),
        ("cancel", "EXIT"),
    ] {
        let root = tempfile::tempdir().unwrap();
        let lock_path = root.path().join("settings.lock");
        std::fs::write(&lock_path, b"held\n").unwrap();
        let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
        let mut board = IcyBoard::new();
        board.root_path = root.path().into();
        board.users.new_user(User {
            name: "SETTINGS".into(),
            ..Default::default()
        });
        let user = board.users[0].clone();
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (mut peer, connection) = ChannelConnection::create_pair();
        let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(board)), nodes, node, Box::new(connection)).await;
        state.session.current_user = Some(user);
        state.session.time_limit = 30;
        state.session.page_len = 0;
        let executable = super::compile(&format!(
            r#"
STARTDISP FNS
FOPEN 1, "{}", O_RW, S_DB
STRING line
FGET 1, line
TERMINPUT input = Terminal.Input
PRINTLN line
EVENT gate = input.Wait(-1)
{ending}
"#,
            lock_path.display()
        ));
        let mut io = DiskIO::new(root.path().to_str().unwrap(), None);
        let mut contender = DiskIO::new(root.path().to_str().unwrap(), None);
        let ppe_path = root.path().join("settings.ppe");
        let mut execution = Box::pin(run(&ppe_path, &executable, &mut io, &mut state));
        let drive = async {
            let mut output = Vec::new();
            while !output.contains(&b'\n') {
                let mut buffer = [0; 128];
                let count = peer.read(&mut buffer).await.unwrap();
                assert_ne!(count, 0, "{case}");
                output.extend_from_slice(&buffer[..count]);
            }
            assert_eq!(String::from_utf8(output).unwrap().replace('\r', ""), "held\n", "{case}");
            contender.fopen(1, lock_path.to_str().unwrap(), 2, 3).unwrap();
            assert!(contender.ferr(1), "cached FGET released the lock: {case}");
            if case == "disconnect" {
                peer.shutdown().await.unwrap();
            } else if case != "cancel" {
                peer.send(b"x").await.unwrap();
            }
        };
        let result = tokio::time::timeout(Duration::from_secs(5), async {
            if case == "cancel" {
                tokio::select! {
                    result = &mut execution => panic!("PPE ended before cancellation: {result:?}"),
                    () = drive => None,
                }
            } else {
                let (result, ()) = tokio::join!(&mut execution, drive);
                Some(result)
            }
        })
        .await
        .unwrap();
        drop(execution);
        assert!(!io.is_open(1), "{case}");
        contender.fopen(1, lock_path.to_str().unwrap(), 2, 3).unwrap();
        assert!(!contender.ferr(1), "lock leaked after {case}");
        match case {
            "error" => assert!(result.unwrap().is_err()),
            "exit" => assert!(result.unwrap().unwrap()),
            "stop" => assert!(!result.unwrap().unwrap()),
            _ => {}
        }
    }
}

#[test]
fn a6_settings_fappend_passes_its_share_mode_to_disk_io() {
    let output = run_ppl(
        r#"
FAPPEND 1, "settings.lock", O_WR, S_DB
PRINTLN !FERR(1)
FOPEN 2, "settings.lock", O_RD, S_DN
PRINTLN FERR(2), ":", Error.Last().Kind = ErrKind.File
FCLOSE 1
FOPEN 2, "settings.lock", O_RD, S_DN
PRINTLN !FERR(2)
EXIT
"#,
    );
    assert_eq!(output, "1\n1:1\n1\n");
}

#[test]
fn record_io_requires_records_and_runtime_400() {
    for statement in ["FGETREC 1, value", "FPUTREC 1, value", "FREADREC 1, value", "FWRITEREC 1, value"] {
        let errors = compile_errors(&format!("INTEGER value\n{statement}"));
        assert!(
            errors.iter().any(|error| error.contains("expects user-defined record")),
            "{statement}: {errors:?}"
        );
    }

    let source = "TYPE Item\n INTEGER Value\nENDTYPE\nItem value\nFPUTREC 1, value";
    let errors = compile_errors_with_runtime(source, 340);
    assert!(errors.iter().any(|error| error.contains("FPutRec needs runtime 400")), "{errors:?}");
}

#[test]
fn a_line_record_round_trips_and_leaves_documentation_unread() {
    let output = run_ppl(
        r#"
        TYPE Profile
            STRING Name
            INTEGER Age
            BIGSTR Note
        ENDTYPE

        Profile source
        source.Name = "Alice\\Admin"
        source.Age = 42
        source.Note = "first" + Chr(10) + "second"

        FCREATE 1, "profile.txt", O_WR, S_DN
        FPUTREC 1, source
        FPUTLN 1, "This text documents the record."
        FCLOSE 1

        Profile target
        STRING documentation
        FOPEN 1, "profile.txt", O_RD, S_DN
        FGETREC 1, target
        FGET 1, documentation
        FCLOSE 1

        PRINTLN target = source
        PRINTLN "[", target.Name, "] [", target.Note, "]"
        PRINTLN documentation
        "#,
    );

    assert_eq!(output, "1\n[Alice\\\\Admin] [first\nsecond]\nThis text documents the record.\n");
}

#[test]
fn binary_records_round_trip_nested_records_arrays_and_multiple_frames() {
    let output = run_ppl(
        r#"
        TYPE Point
            INTEGER X, Y
        ENDTYPE
        TYPE Packet
            STRING Name
            Point Position
            INTEGER Values(1)
        ENDTYPE

        Packet first
        first.Name = "one"
        first.Position.X = 10
        first.Position.Y = 20
        first.Values(0) = 30
        first.Values(1) = 40

        Packet second = first
        second.Name = "two"
        second.Position.X = 50

        FCREATE 1, "packets.bin", O_WR, S_DN
        FWRITEREC 1, first
        FWRITEREC 1, second
        FCLOSE 1

        Packet actualFirst
        Packet actualSecond
        FOPEN 1, "packets.bin", O_RD, S_DN
        FREADREC 1, actualFirst
        FREADREC 1, actualSecond
        FCLOSE 1

        PRINTLN actualFirst = first
        PRINTLN actualSecond = second
        PRINTLN actualSecond.Name, " ", actualSecond.Position.X, " ", actualSecond.Values(1)
        "#,
    );

    assert_eq!(output, "1\n1\ntwo 50 40\n");
}

#[test]
fn a_malformed_line_record_leaves_the_destination_unchanged() {
    let output = run_ppl_with_files(
        r#"
        TYPE Pair
            INTEGER First, Second
        ENDTYPE
        Pair value = Pair { First = 10, Second = 20 }

        FOPEN 1, "broken.txt", O_RD, S_DN
        FGETREC 1, value
        PRINTLN value.First, " ", value.Second
        PRINTLN FERR(1), " ", Error.Last().Code = ErrCode.Format
        FCLOSE 1
        "#,
        &[("broken.txt", b"not-a-number\n20\n")],
    );

    assert_eq!(output, "10 20\n1 1\n");
}

#[test]
fn a_truncated_binary_record_leaves_the_destination_unchanged() {
    let output = run_ppl_with_files(
        r#"
        TYPE Pair
            INTEGER First, Second
        ENDTYPE
        Pair value = Pair { First = 10, Second = 20 }

        FOPEN 1, "broken.bin", O_RD, S_DN
        FREADREC 1, value
        PRINTLN value.First, " ", value.Second
        PRINTLN FERR(1), " ", Error.Last().Code = ErrCode.Format
        FCLOSE 1
        "#,
        &[("broken.bin", &[8, 0, 0, 0, 1, 2, 3, 4])],
    );

    assert_eq!(output, "10 20\n1 1\n");
}

#[test]
fn every_supported_scalar_type_round_trips_through_both_codecs() {
    let output = run_ppl(
        r#"
        TYPE Scalars
            BOOLEAN BoolValue
            UNSIGNED UnsignedValue
            DATE DateValue
            EDATE EDateValue
            INTEGER IntegerValue
            MONEY MoneyValue
            FLOAT FloatValue
            STRING StringValue
            TIME TimeValue
            BYTE ByteValue
            WORD WordValue
            SBYTE SByteValue
            SWORD SWordValue
            BIGSTR BigValue
            DOUBLE DoubleValue
            DDATE DDateValue
            LONG LongValue
            ULONG ULongValue
        ENDTYPE

        Scalars source
        source.BoolValue = TRUE
        source.UnsignedValue = 4000000000
        source.DateValue = Date()
        source.EDateValue = Date()
        source.IntegerValue = -123456
        source.MoneyValue = 123.45
        source.FloatValue = 1.25
        source.StringValue = "text"
        source.TimeValue = Time()
        source.ByteValue = 250
        source.WordValue = 60000
        source.SByteValue = -100
        source.SWordValue = -30000
        source.BigValue = "big" + Chr(0) + "value"
        source.DoubleValue = 1.23456789
        source.DDateValue = 20260828
        source.LongValue = ToLong("-5000000000")
        source.ULongValue = ToULong("10000000000")

        FCREATE 1, "scalars.txt", O_WR, S_DN
        FPUTREC 1, source
        FCLOSE 1
        Scalars fromText
        FOPEN 1, "scalars.txt", O_RD, S_DN
        FGETREC 1, fromText
        FCLOSE 1

        FCREATE 1, "scalars.bin", O_WR, S_DN
        FWRITEREC 1, source
        FCLOSE 1
        Scalars fromBinary
        FOPEN 1, "scalars.bin", O_RD, S_DN
        FREADREC 1, fromBinary
        FCLOSE 1

        PRINTLN fromText = source
        PRINTLN fromBinary = source
        "#,
    );

    assert_eq!(output, "1\n1\n");
}

#[test]
fn a_record_format_failure_enters_on_error() {
    let output = run_ppl_with_files(
        r#"
        TYPE Item
            INTEGER Value
        ENDTYPE
        Item item
        FOPEN 1, "broken.txt", O_RD, S_DN
        ON ERROR GOTO Failed
        FGETREC 1, item
        PRINTLN "not handled"
        EXIT
        :Failed
        PRINTLN Error.Last().Kind = ErrKind.File, " ", Error.Last().Code = ErrCode.Format
        "#,
        &[("broken.txt", b"wrong\n")],
    );

    assert_eq!(output, "1 1\n");
}
