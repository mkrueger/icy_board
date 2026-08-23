use std::{path::PathBuf, sync::Arc};

use icy_board_engine::icy_board::{
    IcyBoard,
    bbs::BBS,
    bulletins::{Bullettin, BullettinList},
    commands::CommandList,
    conferences::Conference,
    icb_config::DisplayNewsBehavior,
    message_area::{AreaList, MessageArea},
    state::{IcyBoardState, PPEExecute},
    user_base::User,
    xfer_protocols::SupportedProtocols,
};
use icy_board_engine::{
    compiler::{PPECompiler, workspace::Workspace},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection};

use crate::bbs::{LoginOptions, internal_handle_client};

mod cmd_3;
mod cmd_7;
mod cmd_a;
mod cmd_alias;
mod cmd_b;
mod cmd_bye;
mod cmd_c;
mod cmd_d;
mod cmd_e;
mod cmd_file_lists;
mod cmd_g;
mod cmd_j;
mod cmd_m;
mod cmd_o;
mod cmd_p;
mod cmd_q;
mod cmd_r;
mod cmd_t;
mod cmd_u;
mod cmd_users;
mod cmd_v;

mod cmd_w;
mod cmd_who;
mod cmd_x;
mod cmd_y;

mod display_file;
mod login_options;
mod statistics;
mod subscriptions;
mod sysop_security;
mod transfer_limits;

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
        path: fixture("main/blt1"),
        ..Default::default()
    });
    bulletins.push(Bullettin {
        path: fixture("main/blt2"),
        ..Default::default()
    });

    // R and Q need somewhere to open a message base; the reader creates it.
    board.conferences.push(Conference {
        name: "Main Board".to_string(),
        bulletins: Some(bulletins.clone()),
        blt_menu: fixture("main/blt_menu"),
        areas: Some(test_message_areas()),
        ..Default::default()
    });

    board.conferences.push(Conference {
        name: "TESTCONF".to_string(),
        bulletins: Some(bulletins),
        blt_menu: fixture("main/blt_menu"),
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

/// The same board, with one message long enough to fill more than a page.
pub fn setup_conference_with_a_long_message(board: &mut IcyBoard) {
    setup_conference(board);

    // A page length of zero is what turns the pause off, so the caller gets one.
    for user in board.users.iter_mut() {
        user.page_len = 24;
    }

    let path = board.conferences[0].areas.as_ref().unwrap()[0].path.clone();
    let mut base = jamjam::jam::JamMessageBase::create(path).unwrap();
    let body = (1..=60).map(|i| format!("Line {i}")).collect::<Vec<_>>().join("\r\n");
    base.write_message(
        &jamjam::jam::JamMessage::default()
            .with_from(bstr::BString::from("SYSOP"))
            .with_to(bstr::BString::from("ALL"))
            .with_subject(bstr::BString::from("A long one"))
            .with_date_time(chrono::Utc::now())
            .with_text(bstr::BString::from(body)),
    )
    .unwrap();
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

/// A scratch directory, unique per call, for whatever a test writes.
fn test_dir() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("icboard-test-{}-{id}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A file that ships with the tests. The board resolves a relative path against its
/// own directory, which is a scratch one here, so these have to name themselves.
pub fn fixture(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/tests").join(path)
}

/// A scratch message area, unique per test, that the reader is free to create
/// and fill.
fn test_message_areas() -> AreaList {
    AreaList::new(vec![MessageArea {
        name: "General".to_string(),
        path: test_dir().join("general"),
        ..Default::default()
    }])
}

pub fn test_output<P: Fn(&mut IcyBoard)>(cmd: String, init_fn: P) -> String {
    test_session_output(cmd, init_fn, true, None, true)
}

pub fn test_user_output<P: Fn(&mut IcyBoard)>(cmd: String, init_fn: P) -> String {
    test_session_output(cmd, init_fn, true, None, false)
}

pub fn test_login_output<P: Fn(&mut IcyBoard)>(cmd: String, init_fn: P) -> String {
    test_session_output(cmd, init_fn, false, None, true)
}

pub fn test_ppe_output<P: Fn(&mut IcyBoard)>(source: &str, init_fn: P) -> String {
    test_ppe_output_with_input(source, "", init_fn)
}

pub fn compile_test_ppe(source: &str) -> PathBuf {
    let dir = test_dir();
    let source_file = dir.join("direct.pps");
    let ppe_file = source_file.with_extension("ppe");
    std::fs::write(&source_file, source).unwrap();

    let mut workspace = Workspace::default();
    workspace.hard_coded_files = Some(vec![source_file.clone()]);
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(std::sync::Mutex::new(ErrorReporter::default()));
    let ast = parse_ast(source_file, errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&[&ast]);
    assert!(!errors.lock().unwrap().has_errors());
    std::fs::write(&ppe_file, compiler.create_executable().unwrap().to_buffer().unwrap()).unwrap();
    ppe_file
}

pub fn test_ppe_output_with_input<P: Fn(&mut IcyBoard)>(source: &str, input: &str, init_fn: P) -> String {
    let ppe_file = compile_test_ppe(source);

    test_session_output(
        input.to_string(),
        init_fn,
        false,
        Some(PPEExecute {
            ppe: ppe_file,
            user_name: None,
            password: None,
            args: Vec::new(),
        }),
        true,
    )
}

fn test_session_output<P: Fn(&mut IcyBoard)>(cmd: String, init_fn: P, login_sysop: bool, ppe: Option<PPEExecute>, stuff_input: bool) -> String {
    let result = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let bbs: Arc<tokio::sync::Mutex<BBS>> = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
        let mut icy_board = icy_board_engine::icy_board::IcyBoard::new();

        // A board writes where its paths point, and a fresh one points at plain names -
        // which would be the crate directory the tests are run from.
        let board_dir = test_dir();
        icy_board.root_path = board_dir.clone();
        icy_board.file_name = board_dir.join("icboard.toml");
        icy_board.config.paths.statistics_file = PathBuf::from("statistics.toml");
        icy_board.resolve_paths();

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

        let direct_ppe = ppe.is_some();
        let completion = icy_board_tui::get_text("run_ppe_completed").into_bytes();
        let node: usize = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let node_state: Arc<tokio::sync::Mutex<Vec<Option<icy_board_engine::icy_board::state::NodeState>>>> = bbs.lock().await.open_connections.clone();
        let (mut ui_connection, connection) = ChannelConnection::create_pair();

        let state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(icy_board)), node_state, node, Box::new(connection)).await;

        let result = Arc::new(tokio::sync::Mutex::new(Vec::new()));

        let res = result.clone();
        let user_input = (!stuff_input).then_some(cmd.clone());
        let _ = std::thread::Builder::new().name("Terminal update".to_string()).spawn(move || {
            tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                if let Some(input) = user_input {
                    ui_connection.send(input.as_bytes()).await.unwrap();
                }
                let mut buffer = [0; 1024];
                let mut exit_sent = false;
                loop {
                    let Ok(size) = ui_connection.read(&mut buffer).await else {
                        break;
                    };
                    if size == 0 {
                        break;
                    }
                    let should_exit = {
                        let mut output = res.lock().await;
                        output.extend(&buffer[0..size]);
                        direct_ppe && !exit_sent && output.windows(completion.len()).any(|window| window == completion)
                    };
                    if should_exit {
                        exit_sent = true;
                        ui_connection.send(b"x").await.unwrap();
                    }
                }
            });
        });

        std::thread::Builder::new()
            .name("Local mode handle".to_string())
            .spawn(move || {
                tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                    let options = LoginOptions { login_sysop, ppe, local: true };

                    let stuffed_chars = if stuff_input { cmd.as_str() } else { "" };
                    if let Err(err) = internal_handle_client(state, Some(options), stuffed_chars).await {
                        log::error!("Error running background client: {}", err);
                    }
                });
            })
            .unwrap();

        wait_for_the_board_to_go_quiet(&result).await;

        result.as_ref().lock().await.clone()
    });

    let result = String::from_utf8(result).expect("board output is not valid UTF-8");
    result.replace("\r\n", "\n")
}

/// A session does not end - it waits for input that never comes - so the answer is
/// complete once the board has stopped writing. Waiting for that rather than for a
/// fixed span keeps the tests honest on a busy machine.
async fn wait_for_the_board_to_go_quiet(output: &Arc<tokio::sync::Mutex<Vec<u8>>>) {
    const QUIET: std::time::Duration = std::time::Duration::from_secs(1);
    const FIRST_BYTE: std::time::Duration = std::time::Duration::from_secs(10);
    const NEVER_LONGER_THAN: std::time::Duration = std::time::Duration::from_secs(60);

    let start = std::time::Instant::now();
    let mut seen = 0;
    let mut last_change = start;

    loop {
        let len = output.lock().await.len();
        if len != seen {
            seen = len;
            last_change = std::time::Instant::now();
        } else if (seen > 0 && last_change.elapsed() >= QUIET) || (seen == 0 && start.elapsed() >= FIRST_BYTE) {
            return;
        }

        if start.elapsed() >= NEVER_LONGER_THAN {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
}
