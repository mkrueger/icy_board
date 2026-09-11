use super::{compile_errors_with_runtime, run_ppl};

#[tokio::test]
async fn a5_telnet_resize_wakes_ppe_and_preserves_input_and_idle_time() {
    use crate::{
        icy_board::{
            IcyBoard,
            bbs::BBS,
            state::{IcyBoardState, virtual_screen::VirtualScreen},
            user_base::User,
        },
        vm::{DiskIO, run},
    };
    use icy_engine::{Position, TextPane};
    use icy_net::{ConnectionType, connection::telnet::TelnetConnection};
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    };

    for language in ["en", "de"] {
        for (with_keys, disconnect) in [(false, false), (true, false), (false, true)] {
            let root = tempfile::tempdir().unwrap();
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let mut peer = TcpStream::connect(listener.local_addr().unwrap()).await.unwrap();
            let (socket, _) = listener.accept().await.unwrap();
            let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
            let mut board = IcyBoard::new();
            board.root_path = root.path().into();
            board.default_display_text = crate::icy_board::icb_text::DEFAULT_DISPLAY_TEXT.clone();
            board.users.new_user(User {
                name: "RESIZE".into(),
                ..Default::default()
            });
            let user = board.users[0].clone();
            let node = bbs.lock().await.create_new_node(ConnectionType::Telnet).await;
            let nodes = bbs.lock().await.open_connections.clone();
            let mut state = IcyBoardState::new(
                bbs,
                Arc::new(tokio::sync::Mutex::new(board)),
                nodes,
                node,
                Box::new(TelnetConnection::accept(socket).unwrap()),
            )
            .await;
            state.session.current_user = Some(user);
            state.session.cur_user_id = 0;
            state.session.time_limit = 30;
            state.session.page_len = 0;
            state.session.tokens.push_back("caller argument".into());
            let idle_since = Instant::now() - Duration::from_secs(30);
            state.session.keyboard_timer_started = idle_since;
            let label = if language == "de" { "Groesse" } else { "Size" };
            let keys = if with_keys {
                r#"STRING letters = ""
BOOLEAN sawResize = FALSE
FOR frame = 1 TO 3
    EVENT mixed = input.Wait(-1)
    IF mixed.Kind = EventKind.Key letters += mixed.Text
    IF mixed.Kind = EventKind.Resize sawResize = TRUE
NEXT
PRINT letters, ":", sawResize, ":", Terminal.Info.Columns, "x", Terminal.Info.Rows"#
            } else if disconnect {
                "EVENT closed = input.Wait(-1)"
            } else {
                ""
            };
            let source = format!(
                r#"
STARTDISP FNS
TERMINPUT input = Terminal.Input
TERMINFO previous = Terminal.Info
PRINT "READY"
INTEGER frame
FOR frame = 1 TO 3
    EVENT resized
    IF frame = 2 THEN
        resized = input.Wait(1000)
    ELSE
        resized = input.Wait(-1)
    ENDIF
    TERMINFO current = Terminal.Info
    CLS
    ANSIPOS 1, 1
    PRINT "{label}: ", current.Columns, "x", current.Rows
    ANSIPOS 1, 2
    PRINT previous.Columns, "x", previous.Rows, ":", resized.Kind = EventKind.Resize
    ANSIPOS current.Columns, current.Rows - 1
    PRINT "X"
    ANSIPOS 1, 3
    PRINT "FRAME", frame
NEXT
EVENT duplicate = input.Wait(200)
ANSIPOS 1, 4
PRINT "QUIET:", duplicate.Kind = EventKind.None
{keys}
PRINT "DONE"
input.Release()
EXIT
"#
            );
            let executable = super::compile(&source);
            let mut io = DiskIO::new(root.path().to_str().unwrap(), None);
            let path = root.path().join("resize.ppe");
            let drive = async {
                async fn receive(peer: &mut TcpStream, marker: &[u8]) -> Vec<u8> {
                    let mut output = Vec::new();
                    while !output.windows(marker.len()).any(|part| part == marker) {
                        let mut buffer = [0; 4096];
                        let read = peer.read(&mut buffer).await.unwrap();
                        assert_ne!(read, 0);
                        output.extend_from_slice(&buffer[..read]);
                    }
                    output
                }
                receive(&mut peer, b"READY").await;
                let mut screen = VirtualScreen::new(icy_parser_core::AnsiParser::default());
                for (frame, reported_width, reported_height) in [(1, 132u16, 43u16), (2, 65535, 65535), (3, 80, 25)] {
                    let mut naws = vec![255, 250, 31];
                    for byte in reported_width.to_be_bytes().into_iter().chain(reported_height.to_be_bytes()) {
                        naws.push(byte);
                        if byte == 255 {
                            naws.push(byte);
                        }
                    }
                    naws.extend([255, 240]);
                    peer.write_all(&naws).await.unwrap();
                    let output = receive(&mut peer, format!("FRAME{frame}").as_bytes()).await;
                    let (width, height) = (reported_width.min(132), reported_height.min(60));
                    screen.write_bytes(format!("\x1b[8;{height};{width}t").as_bytes());
                    screen.write_bytes(&output);
                    let line = |row| {
                        (0..i32::from(width))
                            .map(|column| screen.buffer.char_at(Position::new(column, row)).ch)
                            .collect::<String>()
                    };
                    assert_eq!(line(0).trim_end(), format!("{label}: {width}x{height}"));
                    assert_eq!(line(1).trim_end(), "80x25:1");
                    assert_eq!(screen.buffer.char_at(Position::new(i32::from(width) - 1, i32::from(height) - 2)).ch, 'X');
                }
                peer.write_all(&[255, 250, 31, 0, 80, 0, 25, 255, 240, 255, 250, 31, 0, 0, 0, 25, 255, 240])
                    .await
                    .unwrap();
                let mut output = receive(&mut peer, b"QUIET:1").await;
                if disconnect {
                    peer.shutdown().await.unwrap();
                    return;
                }
                if with_keys {
                    peer.write_all(&[b'a', 255, 250, 31, 0, 100, 0, 40, 255, 240, b'b']).await.unwrap();
                }
                if !output.windows(4).any(|part| part == b"DONE") {
                    output.extend(receive(&mut peer, b"DONE").await);
                }
                if with_keys {
                    assert!(output.windows(b"ab:1:100x40DONE".len()).any(|part| part == b"ab:1:100x40DONE"), "{output:?}");
                }
            };
            let (result, ()) = tokio::time::timeout(Duration::from_secs(5), async {
                tokio::join!(run(&path, &executable, &mut io, &mut state), drive)
            })
            .await
            .expect("Telnet resize stalled");
            if disconnect {
                assert!(result.is_err(), "Wait must fail on disconnect: {result:?}");
            } else {
                assert!(result.unwrap());
            }
            let expected_size = if with_keys { (100, 40) } else { (80, 25) };
            assert_eq!(state.session.term_caps.term_size, expected_size);
            assert_eq!(state.session.term_caps.reported_term_size, expected_size);
            assert_eq!(state.session.tokens.iter().map(String::as_str).collect::<Vec<_>>(), ["caller argument"]);
            assert_eq!(state.session.request_logoff, disconnect);
            if with_keys {
                assert!(state.session.keyboard_timer_started > idle_since);
            } else {
                assert_eq!(state.session.keyboard_timer_started, idle_since);
            }
        }
    }
}

#[test]
fn a5_logical_resize_events_coalesce_and_release_discards_pending() {
    let output = run_ppl(
        r#"
TERMINPUT input = Terminal.Input
PRINT input.Poll().Kind = EventKind.None
PRINT CHR(27), "[8;43;100t", CHR(27), "[8;50;132t"
EVENT resized = input.Wait(-1)
PRINT ":", resized.Kind = EventKind.Resize, ":", Terminal.Info.Columns, "x", Terminal.Info.Rows
PRINT ":", resized.Code, ":", resized.Text = "", ":", resized.Channel
PRINT ":", input.Poll().Kind = EventKind.None
PRINT CHR(27), "[8;50;132t"
PRINT ":", input.Wait(1).Kind = EventKind.None
PRINT CHR(27), "[8;25;80t"
input.Release()
PRINT ":", input.Poll().Kind = EventKind.None
EXIT
"#,
    );
    assert_eq!(output, "1\x1b[8;43;100t\x1b[8;50;132t:1:132x50:0:1:-1:1\x1b[8;50;132t:1\x1b[8;25;80t:1");
}

#[test]
fn a5_terminal_info_tracks_logical_resize_with_stable_snapshots() {
    use crate::icy_board::state::virtual_screen::VirtualScreen;
    use icy_engine::{Position, Size, TextPane};

    for (initial, resized) in [((80, 25), (132, 43)), ((132, 43), (80, 25))] {
        for label in ["Size", "Groesse"] {
            let (initial_columns, initial_rows) = initial;
            let (columns, rows) = resized;
            let source = format!(
                r#"
STARTDISP FNS
PRINT CHR(27), "[8;{initial_rows};{initial_columns}t"
TERMINFO previous = Terminal.Info
PRINT CHR(27), "[8;{rows};{columns}t"
TERMINFO current = Terminal.Info
CLS
ANSIPOS 1, 1
PRINT "{label}: ", current.Columns, "x", current.Rows
ANSIPOS 1, 2
PRINT previous.Columns, "x", previous.Rows
ANSIPOS 1, 3
EVENT pending = Terminal.Input.Poll()
PRINT pending.Kind = EventKind.Resize
ANSIPOS current.Columns, current.Rows - 1
PRINT "X"
ANSIPOS 1, 4
EXIT
"#
            );
            let output = run_ppl(&source);
            let mut screen = VirtualScreen::new(icy_parser_core::AnsiParser::default());
            screen.write_bytes(output.as_bytes());
            assert_eq!(screen.buffer.buffer.terminal_state.size(), Size::new(columns, rows), "{output:?}");
            let line = |row| {
                (0..columns)
                    .map(|column| screen.buffer.char_at(Position::new(column, row)).ch)
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            };
            assert_eq!(line(0), format!("{label}: {columns}x{rows}"));
            assert_eq!(line(1), format!("{initial_columns}x{initial_rows}"));
            assert_eq!(line(2), "1");
            assert_eq!(screen.buffer.char_at(Position::new(columns - 1, rows - 2)).ch, 'X');
            assert!(line(rows - 1).is_empty());
            assert_eq!(screen.buffer.caret.position(), Position::new(0, 3));
        }
    }
}

#[test]
fn terminal_info_requires_runtime_400() {
    for runtime in [330, 340] {
        let errors = compile_errors_with_runtime("TERMINFO info = Terminal.Info", runtime);
        assert!(
            errors.iter().any(|error| error.contains("Terminal needs runtime 400")),
            "runtime {runtime}: {errors:?}"
        );
    }
    assert!(compile_errors_with_runtime("TERMINFO info = Terminal.Info", 400).is_empty());
}

#[test]
fn terminal_info_returns_the_cached_local_snapshot() {
    let output = run_ppl(
        r#"
        TERMINFO info = Terminal.Info
        PrintLn info.Program
        PrintLn info.DeviceAttrs = ""
        PrintLn info.Columns, "x", info.Rows
        PrintLn info.Utf8, " ", info.RipVersion = "", " ", info.CTermLevel
        PrintLn info.Sixel, " ", info.Jxl, " ", info.InlineGraphics, " ", info.Audio, " ", info.PhysicalKeys, " ", info.SynchronizedOutput, " ", info.TerminalMacros
        PrintLn info.CellWidth, "x", info.CellHeight, " ", info.ScreenWidth, "x", info.ScreenHeight
        "#,
    );

    assert_eq!(output, "Unknown\n1\n80x25\n1 1 0\n0 0 0 0 0 0 0\n8x16 0x0\n");
}
