//! A board running one node answers further callers instead of dropping the line.
use std::{sync::Arc, time::Duration};

use icy_board_engine::icy_board::{IcyBoard, bbs::BBS};
use icy_net::ConnectionType;
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
    sync::Mutex,
    time::timeout,
};

const DEADLINE: Duration = Duration::from_secs(10);

/// Everything the refused caller received until the board closed the line.
async fn refused_caller_transcript(occupied: bool, maintenance: bool) -> String {
    let board = Arc::new(Mutex::new(IcyBoard::default()));
    let bbs = Arc::new(Mutex::new(BBS::new(1)));
    if occupied {
        bbs.lock().await.try_create_new_node(ConnectionType::Telnet).await.unwrap();
    }
    bbs.lock().await.operator_maintenance = maintenance;

    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    let listening = tokio::spawn(crate::bbs::serve_telnet_connections(listener, board, bbs));

    let received = timeout(DEADLINE, async {
        let mut stream = TcpStream::connect(address).await.unwrap();
        let mut bytes = Vec::new();
        // The board closes the line after the notice, so this ends at EOF.
        stream.read_to_end(&mut bytes).await.unwrap();
        bytes
    })
    .await
    .expect("the refused caller was neither answered nor disconnected");

    listening.abort();
    // Telnet option negotiation is interleaved; keep only the printable notice.
    received
        .into_iter()
        .filter(|byte| byte.is_ascii_graphic() || *byte == b' ')
        .map(char::from)
        .collect()
}

#[tokio::test]
async fn a_caller_that_finds_every_node_taken_is_told_the_board_is_busy() {
    let transcript = refused_caller_transcript(true, false).await;

    assert!(transcript.contains("ALL NODES ARE BUSY AT THIS TIME PLEASE TRY LATER"), "got {transcript:?}");
}

#[tokio::test]
async fn a_board_closed_for_maintenance_still_drops_the_line_without_a_notice() {
    let transcript = refused_caller_transcript(false, true).await;

    assert!(transcript.is_empty(), "got {transcript:?}");
}
