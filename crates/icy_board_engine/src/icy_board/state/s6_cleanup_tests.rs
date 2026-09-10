use super::*;
use crate::{
    compiler::{PPECompiler, workspace::Workspace},
    icy_board::bbs::BBS,
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
};

const MARKER: &[u8] = b"[s6-ready]";

struct FailingOutput {
    connection: ChannelConnection,
    armed: bool,
    failures: usize,
    attempts: Arc<std::sync::Mutex<Vec<Vec<u8>>>>,
}

#[async_trait::async_trait]
impl Connection for FailingOutput {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Channel
    }

    async fn read(&mut self, buffer: &mut [u8]) -> icy_net::Result<usize> {
        self.connection.read(buffer).await
    }

    async fn try_read(&mut self, buffer: &mut [u8]) -> icy_net::Result<usize> {
        self.connection.try_read(buffer).await
    }

    async fn send(&mut self, buffer: &[u8]) -> icy_net::Result<()> {
        self.attempts.lock().unwrap().push(buffer.to_vec());
        if self.armed {
            self.failures += 1;
            return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, format!("s6 send failure {}", self.failures)).into());
        }
        self.armed = self.attempts.lock().unwrap().concat().windows(MARKER.len()).any(|part| part == MARKER);
        self.connection.send(buffer).await
    }
}

fn compile(source: &str) -> Executable {
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(std::sync::Mutex::new(ErrorReporter::default()));
    let mut workspace = Workspace::default();
    workspace.package.runtime = Some(400);
    workspace.set_default_language_version(Some(400));
    let ast = parse_ast(PathBuf::from("cleanup.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&[&ast]);
    let errors = errors.lock().unwrap();
    assert!(
        errors.errors.is_empty(),
        "{:?}",
        errors.errors.iter().map(|entry| entry.error.to_string()).collect::<Vec<_>>()
    );
    compiler.create_executable().unwrap()
}

#[tokio::test]
async fn s6_read_eof_stops_execution_and_releases_terminal_modes() {
    for timeout in ["100", "-1"] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("eof.ppe");
        let executable = compile(&format!(
            r#"
ON ERROR GOTO Handler
Terminal.Gfx.Init(GfxBackend.Sixel, TRUE)
Terminal.Input.MouseOn(MouseMode(0))
Terminal.Input.KeyboardOn(TRUE)
Terminal.BeginUpdate()
Terminal.Margins.SetVertical(1, 20)
Terminal.Input.Wait({timeout})
PRINT "unreachable"
EXIT
:Handler
PRINT "handler-unreachable"
"#
        ));
        std::fs::write(&path, executable.to_buffer().unwrap()).unwrap();
        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (mut peer, connection) = ChannelConnection::create_pair();
        peer.shutdown().await.unwrap();
        let mut state = IcyBoardState::new(bbs, Arc::new(Mutex::new(IcyBoard::new())), nodes, node, Box::new(connection)).await;
        state.session.disp_options.grapics_mode = GraphicsMode::Graphics;
        let result = tokio::time::timeout(Duration::from_secs(2), state.run_ppe(&path, None)).await.unwrap();
        assert!(state.session.request_logoff);
        assert!(!result.unwrap());
        assert_eq!(state.session.op_text, icy_net::NetError::ConnectionClosed.to_string());
        assert_eq!(state.ppe_nesting, 0);
        assert!(!state.ppl_mouse.is_enabled());
        assert!(!state.ppl_keys.is_enabled());
        assert!(state.ppl_graphics.is_none());
        assert_eq!(state.ppl_terminal.take_update_depth(), 0);
        assert!(!state.ppl_terminal.take_margins_changed());
        let mut output = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            let count = peer.try_read(&mut buffer).await.unwrap();
            if count == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..count]);
        }
        assert!(!String::from_utf8_lossy(&output).contains("unreachable"));
        assert!(output.windows(b"\x1b[?2026h".len()).any(|part| part == b"\x1b[?2026h"));
        assert!(output.windows(b"\x1b[?2026l".len()).any(|part| part == b"\x1b[?2026l"));
        assert!(output.windows(b"\x1b[?25h".len()).any(|part| part == b"\x1b[?25h"));
    }
}

#[tokio::test]
async fn s6_loader_failure_survives_diagnostic_send_failure() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("invalid.ppe");
    for exists in [false, true] {
        if exists {
            std::fs::write(&path, b"invalid PPE").unwrap();
        }
        let expected = Executable::read_file(&path, false).err().unwrap().to_string();
        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (_peer, connection) = ChannelConnection::create_pair();
        let attempts = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut board = IcyBoard::new();
        board.default_display_text = crate::icy_board::icb_text::DEFAULT_DISPLAY_TEXT.clone();
        let mut state = IcyBoardState::new(
            bbs,
            Arc::new(Mutex::new(board)),
            nodes,
            node,
            Box::new(FailingOutput {
                connection,
                armed: true,
                failures: 0,
                attempts: attempts.clone(),
            }),
        )
        .await;
        state.session.tokens.extend(["unused-parameter".to_string()]);
        let error = state.run_ppe(&path, None).await.unwrap_err();
        assert_eq!(error.to_string(), expected);
        assert_eq!(state.session.op_text, expected);
        assert!(state.session.tokens.is_empty());
        assert_eq!(state.ppe_nesting, 0);
        assert!(!attempts.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn s6_cleanup_failure_does_not_hide_primary_vm_error() {
    for restore_color in [false, true] {
        let error = crate::vm::VMError::PushPopStackEmpty.to_string();
        assert_failed_output_cleanup("INTEGER unused\nPOP unused", restore_color, Err(&error), &error).await;
    }
}

#[tokio::test]
async fn s6_cleanup_preserves_first_send_failure() {
    for restore_color in [false, true] {
        assert_failed_output_cleanup("PRINT \".\"", restore_color, Err("s6 send failure 1"), "s6 send failure 1").await;
    }
}

#[tokio::test]
async fn s6_cleanup_exit_and_stop_release_resources_despite_send_failures() {
    for (ending, keep_answers) in [("EXIT", true), ("STOP", false)] {
        assert_failed_output_cleanup(ending, false, Ok(keep_answers), "").await;
        assert_failed_output_cleanup(ending, true, Err("s6 send failure 7"), "").await;
    }
}

async fn assert_failed_output_cleanup(ending: &str, restore_color: bool, expected: Result<bool, &str>, diagnostic: &str) {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("tone.wav"), b"RIFFxxxxWAVEfmt ").unwrap();
    let path = directory.path().join("cleanup.ppe");
    let executable = compile(&format!(
        r#"
ON ERROR GOTO Handler
Terminal.Gfx.Init(GfxBackend.Sixel, TRUE)
SURFACE picture = Surface.New(2, 2)
AUDIO tone = Audio.Load("tone.wav")
tone.Play(TRUE)
Terminal.Input.MouseOn(MouseMode(0))
Terminal.Input.KeyboardOn(TRUE)
Terminal.BeginUpdate()
Terminal.Margins.SetVertical(1, 20)
COLOR 12
PRINT "[s6-ready]"
{ending}
PRINT "unreachable"
EXIT
:Handler
PRINT "handler-unreachable"
"#
    ));
    std::fs::write(&path, executable.to_buffer().unwrap()).unwrap();
    let bbs = Arc::new(Mutex::new(BBS::new(1)));
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let (mut peer, connection) = ChannelConnection::create_pair();
    peer.send(b"\x1b[=7;100;1n\x1b[=7;101;1;2;1n").await.unwrap();
    let attempts = Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut board = IcyBoard::new();
    board.default_display_text = crate::icy_board::icb_text::DEFAULT_DISPLAY_TEXT.clone();
    board.root_path = directory.path().to_path_buf();
    let mut state = IcyBoardState::new(
        bbs,
        Arc::new(Mutex::new(board)),
        nodes,
        node,
        Box::new(FailingOutput {
            connection,
            armed: false,
            failures: 0,
            attempts: attempts.clone(),
        }),
    )
    .await;
    state.session.disp_options.grapics_mode = GraphicsMode::Graphics;
    state.session.tokens.extend(["unused-parameter".to_string()]);
    let result = tokio::time::timeout(Duration::from_secs(10), state.run_ppe_with_color_restore(&path, None, restore_color))
        .await
        .unwrap();
    assert_eq!(state.session.op_text, diagnostic, "{ending}, restore_color={restore_color}");
    assert_eq!(
        result.map_err(|error| error.to_string()),
        expected.map_err(str::to_owned),
        "{ending}, restore_color={restore_color}"
    );
    assert_eq!(state.ppe_nesting, 0);
    assert!(!state.ppl_mouse.is_enabled());
    assert!(!state.ppl_keys.is_enabled());
    assert!(state.ppl_graphics.is_none());
    assert!(state.ppl_audio.iter().all(Option::is_none));
    assert!(!state.sound_active.iter().any(|active| *active));
    assert_eq!(state.sound_volume, [100; 14]);
    assert_eq!(state.ppl_terminal.take_update_depth(), 0);
    assert!(!state.ppl_terminal.take_margins_changed());
    assert!(state.ppl_event_keys.poll().is_none());
    assert!(state.session.tokens.is_empty());
    let attempts = attempts.lock().unwrap().concat();
    assert!(!String::from_utf8_lossy(&attempts).contains("unreachable"));
    assert!(attempts.windows(b"\x1b[?2026h".len()).any(|part| part == b"\x1b[?2026h"));
    let cleanup = attempts.windows(MARKER.len()).position(|part| part == MARKER).unwrap() + MARKER.len();
    let cleanup = if ending == "PRINT \".\"" {
        assert_eq!(attempts[cleanup], b'.');
        cleanup + 1
    } else {
        cleanup
    };
    let expected = [
        ppl_mouse::MOUSE_OFF_SEQUENCE,
        b"\x1b\x1b\x1b[=2l\x1b[=1l\x1b[?1070h\x1b[?80h\x1b[?7h\x1b[?25h\x1b_SyncTERM:A;Flush;C=2;O=0\x1b\\",
    ]
    .concat();
    assert!(attempts[cleanup..].starts_with(&expected), "cleanup attempts: {:?}", &attempts[cleanup..]);
}
