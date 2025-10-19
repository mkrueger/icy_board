use icy_board_engine::icy_board::{IcyBoard, bbs::BBS, commands::CommandList, state::IcyBoardState, user_base::User, xfer_protocols::SupportedProtocols};
use icy_net::{ConnectionType, channel::ChannelConnection};
use std::{path::PathBuf, sync::Arc};

#[tokio::test]
async fn test_display_file_with_error_false_returns_false() {
    let mut state = setup_test_state().await;

    let non_existent_file = PathBuf::from("/this/file/does/not/exist.txt");
    let result = state.display_file_with_error(&non_existent_file, false).await;

    assert!(result.is_ok(), "display_file_with_error should return Ok result");
    assert_eq!(result.unwrap(), false, "display_file_with_error should return false for non-existent file");
}

async fn setup_test_state() -> IcyBoardState {
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
    let (_, connection) = ChannelConnection::create_pair();

    IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(icy_board)), node_state, node, Box::new(connection)).await
}
