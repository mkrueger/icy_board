use std::{path::PathBuf, sync::Arc, thread};

use icy_board_engine::icy_board::{
    bbs::BBS,
    bulletins::{Bullettin, BullettinList},
    commands::CommandList,
    conferences::Conference,
    read_data_with_encoding_detection,
    user_base::User,
    xfer_protocols::SupportedProtocols,
    IcyBoard,
};
use icy_net::{channel::ChannelConnection, Connection, ConnectionType};

use crate::bbs::{handle_client, LoginOptions};

// !
#[test]
fn test_last_cmd() {
    let output = test_output("ABCDE\n!\n".to_string(), |_| {});
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mABCDE\n\n.\u{1b}[1;31mInvalid Entry!  Please try again, Sysop ...\n\n\u{1b}[33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0m!\n\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mABCDE");
}

#[test]
fn test_last_cmd_empty() {
    let output = test_output("ABCD\n!\n".to_string(), |_| {});
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mABCD\n\n.\u{1b}[1;31mInvalid Entry!  Please try again, Sysop ...\n\n\u{1b}[33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0m!\n\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0m");
}

// A
#[test]
fn test_abandon_empty_confs() {
    let output = test_output("A\n".to_string(), |_| {});
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mA\n\n.\u{1b}[1;31mSorry, Sysop, no Conferences are presently available!\n\n\u{1b}[32mPress (Enter) to continue? \u{1b}[0m");
}

// B
#[test]
fn test_blt_empty() {
    let output = test_output("B\n".to_string(), |_| {});
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mB\n\n.\u{1b}[1;31mSorry, Sysop, no Bulletins are presently available.\n\n\u{1b}[32mPress (Enter) to continue? \u{1b}[0m");
}

fn setup_conference(board: &mut IcyBoard) {
    let mut bulletins = BullettinList::default();
    bulletins.push(Bullettin {
        file: PathBuf::from("src/tests/main/blt1"),
        ..Default::default()
    });
    bulletins.push(Bullettin {
        file: PathBuf::from("src/tests/main/blt2"),
        ..Default::default()
    });

    board.conferences.push(Conference {
        name: "TESTCONF".to_string(),
        bulletins: Some(bulletins),
        blt_menu: PathBuf::from("src/tests/main/blt_menu"),
        ..Default::default()
    });
}

#[test]
fn test_blt() {
    let output = test_output("B\n".to_string(), |board| {
        setup_conference(board);
    });
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mB\n1) BLT1 2) BLT2\n\u{1b}[1;33m(H)elp, (1-2), Bulletin List Command? \u{1b}[0m");
}

// X
#[test]
fn test_extended_mode_toggle() {
    let output = test_output("x\nx\n".to_string(), |_| {});
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mX\n\n\u{1b}[1;37mExpert mode is now on, Sysop ...\n\n\u{1b}[33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mX\n\n\u{1b}[1;37mExpert mode is now off, Sysop ...\n\n\u{1b}[32mPress (Enter) to continue? \u{1b}[0m");
}

#[test]
fn test_extended_mode_on() {
    let output = test_output("x on\n".to_string(), |_| {});
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mX ON\n\n\u{1b}[1;37mExpert mode is now on, Sysop ...\n\n\u{1b}[33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0m");
}

#[test]
fn test_extended_mode_invalid() {
    let output = test_output("x invalid\n".to_string(), |_| {});
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mX INVALID\n\n\u{1b}[1;32mPress (Enter) to continue? \u{1b}[0m"
    );
}

#[test]
fn test_extended_mode_off() {
    let output = test_output("X OFF\n".to_string(), |_| {});
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mX OFF\n\n\u{1b}[1;37mExpert mode is now off, Sysop ...\n\n\u{1b}[32mPress (Enter) to continue? \u{1b}[0m"
    );
}

#[test]
fn test_extended_mode_cmd_file() {
    let output = test_output("x on\n".to_string(), |board| {
        board.config.paths.command_display_path = PathBuf::from("src/tests/cmd_files".to_string());
    });
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mX ON\nXCMDFILE\n\u{1b}[1;37mExpert mode is now on, Sysop ...\n\n\u{1b}[33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0m");
}

fn test_output<P: Fn(&mut IcyBoard)>(cmd: String, init_fn: P) -> String {
    let result = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let bbs: Arc<tokio::sync::Mutex<BBS>> = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
        let bbs2 = bbs.clone();
        let mut icy_board = icy_board_engine::icy_board::IcyBoard::new();

        icy_board.commands = CommandList::generate_pcboard_defaults();
        icy_board.protocols = SupportedProtocols::generate_pcboard_defaults();
        icy_board.default_display_text = icy_board_engine::icy_board::icb_text::DEFAULT_DISPLAY_TEXT.clone();
        icy_board.users.new_user(User {
            name: "SYSOP".to_string(),
            security_level: 255,
            ..Default::default()
        });
        icy_board.users.new_user(User {
            name: "TEST USER".to_string(),
            security_level: 10,
            ..Default::default()
        });
        init_fn(&mut icy_board);

        let board = Arc::new(tokio::sync::Mutex::new(icy_board));
        let board2 = board.clone();
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let node_state = bbs.lock().await.open_connections.clone();
        let node_state2 = node_state.clone();
        let (mut ui_connection, connection) = ChannelConnection::create_pair();

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

                    if let Err(err) = handle_client(bbs2, board2, node_state2, node, Box::new(connection), Some(options), &cmd).await {
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
