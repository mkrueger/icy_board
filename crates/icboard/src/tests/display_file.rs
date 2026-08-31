use icy_board_engine::icy_board::{IcyBoard, bbs::BBS, commands::CommandList, state::IcyBoardState, user_base::User, xfer_protocols::SupportedProtocols};
use icy_engine::{AttributedChar, EditableScreen, FileFormat, Position, SaveOptions, TextScreen};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection};
use std::{path::PathBuf, sync::Arc};

#[tokio::test]
async fn test_display_file_with_error_false_returns_false() {
    let mut state = setup_test_state().await;

    let non_existent_file = PathBuf::from("/this/file/does/not/exist.txt");
    let result = state.display_file_with_error(&non_existent_file, false).await;

    assert!(result.is_ok(), "display_file_with_error should return Ok result");
    assert!(!result.unwrap(), "display_file_with_error should return false for non-existent file");
}

#[tokio::test]
async fn icy_draw_file_is_rendered_as_ansi() {
    let (mut state, mut connection) = setup_test_state_with_connection().await;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("test.icy");
    let mut screen = TextScreen::new((4, 1));
    screen.set_char(Position::default(), AttributedChar::new('I', Default::default()));
    let data = FileFormat::IcyDraw.to_bytes(&screen.buffer, &SaveOptions::icy_draw()).unwrap();
    std::fs::write(&path, data).unwrap();

    assert!(state.display_file(&path).await.unwrap());
    let output = drain_connection(&mut connection).await;

    assert!(output.contains(&b'I'));
}

#[tokio::test]
async fn icy_animation_is_displayed_and_restores_the_cursor() {
    let (mut state, mut connection) = setup_test_state_with_connection().await;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("test.icyanim");
    std::fs::write(
        &path,
        r#"
local buffer = new_buffer(4, 1)
buffer:set_char(0, 0, "A")
set_delay(0)
next_frame(buffer)
"#,
    )
    .unwrap();

    assert!(state.display_file(&path).await.unwrap());
    let output = drain_connection(&mut connection).await;

    assert!(output.windows(6).any(|bytes| bytes == b"\x1b[?25l"));
    assert!(output.contains(&b'A'));
    assert!(output.windows(6).any(|bytes| bytes == b"\x1b[?25h"));
}

#[tokio::test]
async fn failing_icy_animation_restores_the_cursor() {
    let (mut state, mut connection) = setup_test_state_with_connection().await;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("broken.icyanim");
    std::fs::write(&path, "error(\"broken animation\")").unwrap();

    let error = state.display_file(&path).await.unwrap_err();
    let output = drain_connection(&mut connection).await;

    assert!(error.to_string().contains("broken animation"));
    assert!(output.windows(6).any(|bytes| bytes == b"\x1b[?25h"));
}

async fn drain_connection(connection: &mut ChannelConnection) -> Vec<u8> {
    let mut output = Vec::new();
    let mut buffer = [0; 4096];
    loop {
        let size = connection.try_read(&mut buffer).await.unwrap();
        if size == 0 {
            return output;
        }
        output.extend_from_slice(&buffer[..size]);
    }
}

async fn setup_test_state() -> IcyBoardState {
    setup_test_state_with_connection().await.0
}

async fn setup_test_state_with_connection() -> (IcyBoardState, ChannelConnection) {
    let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
    let mut icy_board = IcyBoard::new();

    icy_board.commands = CommandList::new();
    icy_board.protocols = SupportedProtocols::generate_pcboard_defaults();
    icy_board.default_display_text = icy_board_engine::icy_board::icb_text::DEFAULT_DISPLAY_TEXT.clone();

    icy_board.users.new_user(User {
        name: "SYSOP".to_string(),
        security_level: 255,
        protocol: "Z".to_string(),
        ..Default::default()
    });

    icy_board.users.new_user(User {
        name: "TEST USER".to_string(),
        security_level: 10,
        protocol: "Z".to_string(),
        ..Default::default()
    });

    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let node_state: Arc<tokio::sync::Mutex<Vec<Option<icy_board_engine::icy_board::state::NodeState>>>> = bbs.lock().await.open_connections.clone();
    let (ui_connection, connection) = ChannelConnection::create_pair();

    (
        IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(icy_board)), node_state, node, Box::new(connection)).await,
        ui_connection,
    )
}
