use std::sync::Arc;

use icy_board_engine::{
    icy_board::{bbs::BBS, commands::CommandList, state::IcyBoardState, user_base::User, xfer_protocols::SupportedProtocols},
    vm::TerminalTarget,
};
use icy_engine::TextPane;
use icy_net::{ConnectionType, channel::ChannelConnection};

async fn make_state() -> (IcyBoardState, ChannelConnection) {
    let bbs: Arc<tokio::sync::Mutex<BBS>> = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
    let mut icy_board = icy_board_engine::icy_board::IcyBoard::new();
    icy_board.commands = CommandList::new();
    icy_board.protocols = SupportedProtocols::generate_pcboard_defaults();
    icy_board.default_display_text = icy_board_engine::icy_board::icb_text::DEFAULT_DISPLAY_TEXT.clone();
    icy_board.users.new_user(User {
        name: "SYSOP".to_string(),
        security_level: 255,
        protocol: "Z".to_string(),
        ..Default::default()
    });

    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let node_state = bbs.lock().await.open_connections.clone();
    let (ui_connection, connection) = ChannelConnection::create_pair();

    let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(icy_board)), node_state, node, Box::new(connection)).await;
    state.session.cur_user_id = 0;
    // Returned so the peer end stays alive; dropping it would close the channel.
    (state, ui_connection)
}

/// Collects the characters on the first row whose cells carry the search highlight background.
fn highlighted_text(state: &IcyBoardState) -> String {
    let screen = state.display_screen();
    let width = screen.buffer.width();
    let mut s = String::new();
    for x in 0..width {
        let ch = screen.buffer.char_at((x, 0).into());
        if ch.attribute.background() == 7 {
            s.push(ch.ch);
        }
    }
    s
}

#[tokio::test]
async fn highlights_match_in_ascii_text() {
    let (mut state, _peer) = make_state().await;
    assert!(state.search_init("ACiD".to_string(), false));
    state.print_found_text(TerminalTarget::User, "Released by ACiD Productions").await.unwrap();
    assert_eq!(highlighted_text(&state), "ACiD");
}

#[tokio::test]
async fn highlights_match_after_multibyte_utf8() {
    // The box-drawing glyphs are multi-byte in UTF-8, so byte offsets from the
    // regex no longer line up with char indices - the bug this guards against.
    let (mut state, _peer) = make_state().await;
    assert!(state.search_init("ACiD".to_string(), false));
    state.print_found_text(TerminalTarget::User, "═══ Released by ACiD Productions").await.unwrap();
    assert_eq!(highlighted_text(&state), "ACiD");
}
