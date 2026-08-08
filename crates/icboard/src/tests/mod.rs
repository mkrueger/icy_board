use std::{path::PathBuf, sync::Arc, thread};

use icy_board_engine::icy_board::{
    IcyBoard,
    bbs::BBS,
    bulletins::{Bullettin, BullettinList},
    commands::CommandList,
    conferences::Conference,
    icb_config::DisplayNewsBehavior,
    message_area::{AreaList, MessageArea},
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
mod cmd_e;
mod cmd_g;
mod cmd_j;
mod cmd_m;
mod cmd_o;
mod cmd_p;
mod cmd_r;
mod cmd_t;

mod cmd_w;
mod cmd_x;

mod display_file;

// !
#[test]
fn test_last_cmd() {
    let output = test_output("ABCDE\n!\n".to_string(), |_| {});
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mABCDE\n\n\u{7}\u{1b}[1;31mInvalid Entry!  Please try again, Sysop ...\n\n\u{1b}[33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0m!\n\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mABCDE"
    );
}

#[test]
fn test_last_cmd_empty() {
    let output = test_output("ABCD\n!\n".to_string(), |_| {});
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mABCD\n\n\u{7}\u{1b}[1;31mInvalid Entry!  Please try again, Sysop ...\n\n\u{1b}[33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0m!\n\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0m"
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

    // R and Q need somewhere to open a message base; the reader creates it.
    board.conferences.push(Conference {
        name: "Main Board".to_string(),
        bulletins: Some(bulletins.clone()),
        blt_menu: PathBuf::from("src/tests/main/blt_menu"),
        areas: Some(test_message_areas()),
        ..Default::default()
    });

    board.conferences.push(Conference {
        name: "TESTCONF".to_string(),
        bulletins: Some(bulletins),
        blt_menu: PathBuf::from("src/tests/main/blt_menu"),
        areas: Some(test_message_areas()),
        ..Default::default()
    });
}

/// Three messages in the main board's area, so the reader has something to show,
/// kill or move.
pub fn setup_conference_with_messages(board: &mut IcyBoard) {
    setup_conference(board);

    let path = board.conferences[0].areas.as_ref().unwrap()[0].path.clone();
    let mut base = jamjam::jam::JamMessageBase::create(path).unwrap();
    for i in 1..=3 {
        // Only the middle message carries BANANA, so a text search has
        // something to narrow down to.
        let marker = if i == 2 { " BANANA" } else { "" };
        base.write_message(
            &jamjam::jam::JamMessage::default()
                .with_from(bstr::BString::from("SYSOP"))
                .with_to(bstr::BString::from("ALL"))
                .with_subject(bstr::BString::from(format!("Subject {i}")))
                .with_date_time(chrono::Utc::now())
                .with_text(bstr::BString::from(format!("Body of message {i}{marker}"))),
        )
        .unwrap();
    }
    base.write_jhr_header().unwrap();
}

/// The same board, but the second conference carries two message areas so a
/// move into it has to ask which one.
pub fn setup_conference_with_two_areas(board: &mut IcyBoard) {
    setup_conference_with_messages(board);

    let mut areas = board.conferences[1].areas.clone().unwrap_or_default();
    let dir = areas[0].path.parent().unwrap().to_path_buf();
    areas.push(MessageArea {
        name: "Second".to_string(),
        path: dir.join("second"),
        ..Default::default()
    });
    board.conferences[1].areas = Some(areas);
}

/// A scratch message area, unique per test, that the reader is free to create
/// and fill.
fn test_message_areas() -> AreaList {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("icboard-test-{}-{id}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    AreaList::new(vec![MessageArea {
        name: "General".to_string(),
        path: dir.join("general"),
        ..Default::default()
    }])
}

pub fn test_output<P: Fn(&mut IcyBoard)>(cmd: String, init_fn: P) -> String {
    let result = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let bbs: Arc<tokio::sync::Mutex<BBS>> = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
        let mut icy_board = icy_board_engine::icy_board::IcyBoard::new();
        icy_board.config.switches.display_news_behavior = DisplayNewsBehavior::Never;
        icy_board.config.switches.scan_new_blt = false;
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

        for conference in icy_board.conferences.iter_mut() {
            if conference.areas.is_none() {
                conference.areas = Some(test_message_areas());
            }
        }

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
                        log::error!("Error running background client: {}", err);
                    }
                });
            })
            .unwrap();

        thread::sleep(std::time::Duration::from_millis(150));
        let x = result.as_ref().lock().await.clone();
        x
    });

    let result = String::from_utf8(result).expect("board output is not valid UTF-8");
    result.replace("\r\n", "\n")
}
