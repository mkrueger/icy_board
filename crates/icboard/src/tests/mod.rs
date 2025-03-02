use std::{path::PathBuf, sync::Arc, thread};

use icy_board_engine::icy_board::{
    IcyBoard,
    bbs::BBS,
    bulletins::{Bullettin, BullettinList},
    commands::CommandList,
    conferences::Conference,
    read_data_with_encoding_detection,
    state::IcyBoardState,
    user_base::User,
    xfer_protocols::SupportedProtocols,
};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection};

use crate::bbs::{LoginOptions, internal_handle_client};

mod cmd_a;
mod cmd_alias;
mod cmd_b;
mod cmd_bye;
mod cmd_c;
mod cmd_g;
mod cmd_j;
mod cmd_m;
mod cmd_p;
mod cmd_t;

mod cmd_x;

// !
#[test]
fn test_last_cmd() {
    let output = test_output("ABCDE\n!\n".to_string(), |_| {});
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mABCDE\n\n.\u{1b}[1;31mInvalid Entry!  Please try again, Sysop ...\n\n\u{1b}[33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0m!\n\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mABCDE"
    );
}

#[test]
fn test_last_cmd_empty() {
    let output = test_output("ABCD\n!\n".to_string(), |_| {});
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mABCD\n\n.\u{1b}[1;31mInvalid Entry!  Please try again, Sysop ...\n\n\u{1b}[33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0m!\n\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0m"
    );
}

pub fn setup_conference(board: &mut IcyBoard) {
    let mut bulletins = BullettinList::default();
    bulletins.push(Bullettin {
        path: PathBuf::from("src/tests/main/blt1"),
        ..Default::default()
    });
    bulletins.push(Bullettin {
        path: PathBuf::from("src/tests/main/blt2"),
        ..Default::default()
    });

    board.conferences.push(Conference {
        name: "Main Board".to_string(),
        bulletins: Some(bulletins.clone()),
        blt_menu: PathBuf::from("src/tests/main/blt_menu"),
        ..Default::default()
    });

    board.conferences.push(Conference {
        name: "TESTCONF".to_string(),
        bulletins: Some(bulletins),
        blt_menu: PathBuf::from("src/tests/main/blt_menu"),
        ..Default::default()
    });
}

pub fn test_output<P: Fn(&mut IcyBoard)>(cmd: String, init_fn: P) -> String {
    let result = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
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
        icy_board.users.new_user(User {
            name: "TEST USER".to_string(),
            security_level: 10,
            protocol: "Z".to_string(),
            ..Default::default()
        });

        init_fn(&mut icy_board);

        let node: usize = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let node_state: Arc<tokio::sync::Mutex<Vec<Option<icy_board_engine::icy_board::state::NodeState>>>> = bbs.lock().await.open_connections.clone();
        let (mut ui_connection, connection) = ChannelConnection::create_pair();

        let state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(icy_board)), node_state, node, Box::new(connection)).await;

        let result = Arc::new(tokio::sync::Mutex::new(Vec::new()));

        let res = result.clone();
        let _ = std::thread::Builder::new().name("Terminal update".to_string()).spawn(move || {
            tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                let mut buffer = [0; 1024];
                loop {
                    let Ok(size) = ui_connection.read(&mut buffer).await else {
                        break;
                    };
                    if size == 0 {
                        break;
                    }
                    res.lock().await.extend(&buffer[0..size]);
                }
            });
        });

        std::thread::Builder::new()
            .name("Local mode handle".to_string())
            .spawn(move || {
                tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                    let options = LoginOptions {
                        login_sysop: true,
                        ppe: None,
                        local: true,
                    };

                    if let Err(err) = internal_handle_client(state, Some(options), &cmd).await {
                        log::error!("Error running backround client: {}", err);
                    }
                });
            })
            .unwrap();

        thread::sleep(std::time::Duration::from_millis(150));
        let x = result.as_ref().lock().await.clone();
        x
    });

    let result = read_data_with_encoding_detection(&result).unwrap();
    result.replace("\r\n", "\n")
}
