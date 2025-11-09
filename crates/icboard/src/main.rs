use std::{
    fmt::Display,
    io::stdout,
    path::PathBuf,
    process::{self, Command, exit},
    sync::Arc,
};

use argh::FromArgs;
use bbs::await_telnet_connections;
use call_wait_screen::{CallWaitMessage, CallWaitScreen};
use chrono::Local;
use crossterm::{
    ExecutableCommand, execute,
    style::{Attribute, Print, SetAttribute, SetForegroundColor},
    terminal::Clear,
};
use icy_board_engine::{
    Res,
    icy_board::{IcyBoard, bbs::BBS, state::PPEExecute},
};

use node_monitoring_screen::NodeMonitoringScreenMessage;
use ratatui::{Terminal, backend::Backend};
use semver::Version;
use system_statistics_screen::{SystemStatisticsScreen, SystemStatisticsScreenMessage};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tui::{Tui, print_exit_screen};

use crate::bbs::await_securewebsocket_connections;

pub mod bbs;
mod call_wait_screen;
pub mod menu_runner;
mod node_monitoring_screen;
mod system_statistics_screen;
mod terminal_thread;
mod tui;

#[cfg(test)]
mod tests;

static mut SHOW_TOTAL_STATS: bool = true;

#[derive(FromArgs)]
/// IcyBoard BBS
struct Cli {
    /// default is 80x25
    #[argh(switch, short = 'f')]
    full_screen: bool,

    #[argh(switch)]
    /// login locally to icy board
    localon: bool,

    #[argh(option)]
    /// execute PPE file
    ppe: Option<PathBuf>,

    #[argh(option)]
    /// run PPE with user login: "first;last;PWRD:password;PPE:file.ppe;param1;param2;..."
    runppe: Option<String>,

    #[argh(option)]
    /// stuffed key chars
    key: Option<String>,

    /// path/file name of the icyboard.toml configuration file
    #[argh(positional)]
    file: Option<PathBuf>,
}

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}
/// evlevlelvelvelv`

#[tokio::main]
async fn main() -> Res<()> {
    let arguments: Cli = argh::from_env();
    let Some(file) = icy_board_engine::lookup_icyboard_file(&arguments.file) else {
        print_error(icy_board_tui::get_text("error_file_or_path_not_found"));
        exit(1);
    };

    start_icy_board(&arguments, file).await?;
    Ok(())
}

async fn start_icy_board(arguments: &Cli, file: PathBuf) -> Res<()> {
    let stuffed = arguments.key.clone().unwrap_or_default();
    let config_file = file.with_extension("toml");
    let log_file = config_file.with_extension("log");
    fern::Dispatch::new()
        // Perform allocation-free log formatting
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                message
            ))
        })
        // Add blanket level filter -
        .level(log::LevelFilter::Info)
        // - and per-module overrides
        .level_for("hyper", log::LevelFilter::Info)
        // Output to stdout, files, and other Dispatch configurations
        .chain(fern::log_file(&log_file).unwrap())
        // Apply globally
        .apply()
        .unwrap();
    match IcyBoard::load(&config_file) {
        Ok(mut icy_board) => {
            icy_board.resolve_paths();
            let mut bbs = Arc::new(Mutex::new(BBS::new(icy_board.config.board.num_nodes as usize)));
            let mut board: Arc<Mutex<IcyBoard>> = Arc::new(tokio::sync::Mutex::new(icy_board));
            if arguments.localon || arguments.ppe.is_some() {
                let mut terminal = init_terminal()?;
                let cmd = if let Some(ppe) = &arguments.ppe {
                    CallWaitMessage::RunPPE(ppe.clone(), None, None, None)
                } else {
                    CallWaitMessage::User(false)
                };
                run_message(cmd, &mut terminal, &board, &mut bbs, arguments.full_screen, stuffed).await?;
                restore_terminal()?;
                return Ok(());
            }

            // Handle /runppe parameter
            if let Some(runppe_params) = &arguments.runppe {
                match handle_runppe(runppe_params).await {
                    Ok(cmd) => {
                        let mut terminal = init_terminal()?;
                        run_message(cmd, &mut terminal, &board, &mut bbs, arguments.full_screen, stuffed).await?;
                        restore_terminal()?;
                    }
                    Err(err) => {
                        print_error(err.to_string());
                        exit(99);
                    }
                }
                exit(0);
            }

            let mut connection_token = CancellationToken::new();
            start_connections(&bbs, &board, connection_token.clone()).await;
            let mut app = CallWaitScreen::new(&board).await?;
            let mut terminal = init_terminal()?;
            loop {
                terminal.clear()?;
                app.reset(&board).await;
                match app.run(&mut terminal, &board, arguments.full_screen).await {
                    Ok(msg) => match run_message(msg, &mut terminal, &mut board, &mut bbs, arguments.full_screen, String::new()).await {
                        Ok(reload) => {
                            if reload {
                                icy_board = IcyBoard::load(&config_file)?;
                                icy_board.resolve_paths();

                                bbs = Arc::new(Mutex::new(BBS::new(icy_board.config.board.num_nodes as usize)));
                                board = Arc::new(tokio::sync::Mutex::new(icy_board));
                                app = CallWaitScreen::new(&board).await?;
                                connection_token.cancel();
                                connection_token = CancellationToken::new();
                                start_connections(&bbs, &board, connection_token.clone()).await;
                                continue;
                            }
                        }
                        Err(err) => {
                            restore_terminal()?;
                            log::error!("while processing call wait screen message: {}", err.to_string());
                            print_error(err.to_string());
                            return Err(err);
                        }
                    },
                    Err(err) => {
                        restore_terminal()?;
                        log::error!("while running call wait screen: {}", err.to_string());
                        print_error(err.to_string());
                        return Err(err);
                    }
                }
            }
        }
        Err(err) => {
            log::error!("while loading icy board configuration: {}", err.to_string());
            print_error(err.to_string());
            return Err(err);
        }
    }
}

async fn start_connections(bbs: &Arc<Mutex<BBS>>, board: &Arc<Mutex<IcyBoard>>, token: CancellationToken) {
    let telnet_connection: icy_board_engine::icy_board::login_server::Telnet = board.lock().await.config.login_server.telnet.clone();
    if telnet_connection.is_enabled {
        let bbs = bbs.clone();
        let board: Arc<Mutex<IcyBoard>> = board.clone();
        let token = token.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = await_telnet_connections(telnet_connection, board, bbs) => {
                },
                _ = token.cancelled() => {
                }
            }
        });
    }

    let ssh_connection = board.lock().await.config.login_server.ssh.clone();
    if ssh_connection.is_enabled {
        let bbs: Arc<Mutex<BBS>> = bbs.clone();
        let board = board.clone();
        let token = token.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = bbs::ssh::await_ssh_connections(ssh_connection, board, bbs) => {
                },
                _ = token.cancelled() => {
                }
            }
        });
    }
    /*
    let websocket_connection = board.lock().await.config.login_server.websocket.clone();
    if websocket_connection.is_enabled {
        let bbs = bbs.clone();
        let board = board.clone();
        std::thread::Builder::new()
            .name("Websocket connect".to_string())
            .spawn(move || {
                tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                    let _ = await_websocket_connections(websocket_connection, board, bbs).await;
                });
            })
            .unwrap();
    }*/
    let secure_websocket_connection = board.lock().await.config.login_server.secure_websocket.clone();
    if secure_websocket_connection.is_enabled {
        let bbs = bbs.clone();
        let board = board.clone();
        let token = token.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = await_securewebsocket_connections(secure_websocket_connection, board, bbs) => {
                },
                _ = token.cancelled() => {
                }
            }
        });
    }
}

async fn run_message(
    msg: CallWaitMessage,
    terminal: &mut Terminal<impl Backend>,
    board: &Arc<tokio::sync::Mutex<IcyBoard>>,
    bbs: &mut Arc<Mutex<BBS>>,
    full_screen: bool,
    stuffed_chars: String,
) -> Res<bool> {
    match msg {
        CallWaitMessage::User(_busy) => {
            stdout().execute(Clear(crossterm::terminal::ClearType::All)).unwrap();
            match Tui::local_mode(board, bbs, false, None, stuffed_chars).await {
                Ok(mut tui) => {
                    if let Err(err) = tui.run(bbs, &board).await {
                        restore_terminal()?;
                        log::error!("while running board in local mode: {}", err.to_string());
                        println!("Error: {}", err);
                        process::exit(1);
                    }
                }
                Err(err) => {
                    restore_terminal()?;
                    log::error!("while initializing board in local mode: {}", err.to_string());
                    println!("Error: {}", err);
                    process::exit(1);
                }
            }
        }
        CallWaitMessage::RunPPE(ppe, name_opt, pw_opt, params_opt) => {
            stdout().execute(Clear(crossterm::terminal::ClearType::All)).unwrap();
            match Tui::local_mode(
                board,
                bbs,
                false,
                Some(PPEExecute {
                    ppe,
                    user_name: name_opt,
                    password: pw_opt,
                    args: params_opt.unwrap_or_default(),
                }),
                stuffed_chars,
            )
            .await
            {
                Ok(mut tui) => {
                    if let Err(err) = tui.run(bbs, &board).await {
                        restore_terminal()?;
                        log::error!("while running board in local mode: {}", err.to_string());
                        println!("Error: {}", err);
                        process::exit(1);
                    }
                }
                Err(err) => {
                    restore_terminal()?;
                    log::error!("while initializing board in local mode: {}", err.to_string());
                    println!("Error: {}", err);
                    process::exit(1);
                }
            }
        }
        CallWaitMessage::Sysop(_busy) => {
            stdout().execute(Clear(crossterm::terminal::ClearType::All)).unwrap();
            match Tui::local_mode(board, bbs, true, None, stuffed_chars).await {
                Ok(mut tui) => {
                    if let Err(err) = tui.run(bbs, &board).await {
                        restore_terminal()?;
                        log::error!("while running board in local mode: {}", err.to_string());
                        println!("Error: {}", err);
                        process::exit(1);
                    }
                }
                Err(err) => {
                    restore_terminal()?;
                    log::error!("while initializing board in local mode: {}", err.to_string());
                    println!("Error: {}", err);
                    process::exit(1);
                }
            }
        }
        CallWaitMessage::Exit(_busy) => {
            restore_terminal().unwrap();
            print_exit_screen();
            process::exit(0);
        }
        CallWaitMessage::Monitor => {
            let mut app = node_monitoring_screen::NodeMonitoringScreen::new(&board).await;
            match app.run(terminal, &board, bbs, full_screen).await {
                Ok(msg) => {
                    if let NodeMonitoringScreenMessage::EnterNode(node) = msg {
                        let mut tui = Tui::sysop_mode(bbs, node).await?;
                        if let Err(err) = tui.run(bbs, &board).await {
                            restore_terminal()?;
                            log::error!("while running board in local mode: {}", err.to_string());
                            println!("Error: {}", err);
                            process::exit(1);
                        }
                    }
                }
                Err(err) => {
                    restore_terminal()?;
                    log::error!("while running node monitoring screen: {}", err.to_string());
                    print_error(err.to_string());
                    process::exit(1);
                }
            }
        }
        CallWaitMessage::ToggleCallLog => {
            let config = &mut board.lock().await.config;
            config.options.call_log = !config.options.call_log;
        }
        CallWaitMessage::TogglePageBell => {
            let config = &mut board.lock().await.config;
            config.options.page_bell = !config.options.page_bell;
        }
        CallWaitMessage::ToggleAlarm => {
            let config = &mut board.lock().await.config;
            config.options.alarm = !config.options.alarm;
        }
        CallWaitMessage::SystemManager => {
            let path = std::env::current_exe().unwrap().with_file_name("icbsysmgr");
            let mut cmd = Command::new(path)
                .arg(format!("{}", board.lock().await.file_name.display()))
                .spawn()
                .expect("icbsysmgr command failed to start");
            cmd.wait().expect("icbsysmgr command failed to run");
            return Ok(true);
        }
        CallWaitMessage::Setup => {
            let path = std::env::current_exe().unwrap().with_file_name("icbsetup");
            let mut cmd = Command::new(path)
                .arg(format!("{}", board.lock().await.file_name.display()))
                .spawn()
                .expect("icbsysmgr command failed to start");
            cmd.wait().expect("icbsysmgr command failed to run");
            return Ok(true);
        }
        CallWaitMessage::IcbText => {
            let icbtxt_path = board.lock().await.config.paths.icbtext.clone();
            let icbtxt_path = board.lock().await.resolve_file(&icbtxt_path);

            let path = std::env::current_exe().unwrap().with_file_name("mkicbtxt");
            let mut cmd = Command::new(path)
                .arg(format!("{}", icbtxt_path.display()))
                .spawn()
                .expect("icbsysmgr command failed to start");
            cmd.wait().expect("icbsysmgr command failed to run");
            return Ok(true);
        }
        CallWaitMessage::ToggleStatistics => unsafe {
            SHOW_TOTAL_STATS = !SHOW_TOTAL_STATS;
        },
        CallWaitMessage::ShowStatistics => {
            let mut app = SystemStatisticsScreen::new(&board).await;
            match app.run(terminal, full_screen).await {
                Ok(msg) => {
                    if msg == SystemStatisticsScreenMessage::Reset {
                        let mut board = board.lock().await;
                        board.statistics = Default::default();
                        board.save_statistics().expect("failed to save statistics.");
                    }
                    // just exit
                }
                Err(err) => {
                    restore_terminal()?;
                    log::error!("while running system statistics screen: {}", err.to_string());
                    print_error(err.to_string());
                    process::exit(1);
                }
            }
        }
    }
    Ok(false)
}

fn init_terminal() -> Res<Terminal<impl Backend>> {
    color_eyre::install()?;
    Ok(ratatui::init())
}

pub fn restore_terminal() -> Res<()> {
    ratatui::restore();
    Ok(())
}

pub fn print_error<A: Display>(error: A) {
    execute!(
        stdout(),
        SetAttribute(Attribute::Bold),
        SetForegroundColor(crossterm::style::Color::Red),
        //Print(gettext("error_cmd_line_label")),
        Print("error:"),
        Print(" "),
        SetAttribute(Attribute::Reset),
        SetAttribute(Attribute::Bold),
        Print(error),
        Print("\n"),
        SetAttribute(Attribute::Reset)
    )
    .unwrap();
}

async fn handle_runppe(params: &str) -> Res<CallWaitMessage> {
    // Parse semicolon-separated parameters
    let parts: Vec<&str> = params.split(';').collect();

    if parts.len() < 4 {
        return Err("Insufficient parameters. Format: first;last;PWRD:password;PPE:file.ppe".into());
    }

    let mut first_name = String::new();
    let mut last_name = String::new();
    let password;
    let ppe_file;
    let mut ppe_params = Vec::new();
    let mut idx = 0;

    // Parse user name (might be 2 or 3 parts for Jr./Sr./III etc.)
    while idx < parts.len() && !parts[idx].to_uppercase().starts_with("PWRD:") {
        if first_name.is_empty() {
            first_name = parts[idx].to_string();
        } else if last_name.is_empty() {
            last_name = parts[idx].to_string();
        } else {
            // Handle suffixes like Jr., Sr., III
            last_name.push(' ');
            last_name.push_str(parts[idx]);
        }
        idx += 1;
    }

    // Parse password
    if idx >= parts.len() || !parts[idx].to_uppercase().starts_with("PWRD:") {
        return Err("Error in Password - missing PWRD: prefix".into());
    }
    password = parts[idx][5..].to_string();
    idx += 1;

    // Parse PPE file
    if idx >= parts.len() || !parts[idx].to_uppercase().starts_with("PPE:") {
        return Err("PPE Name is missing - missing PPE: prefix".into());
    }
    ppe_file = PathBuf::from(&parts[idx][4..]);
    idx += 1;

    // Remaining parts are PPE parameters
    while idx < parts.len() {
        ppe_params.push(parts[idx].to_string());
        idx += 1;
    }

    // Validate PPE file exists
    if !ppe_file.exists() {
        return Err(format!("PPE Name is missing - file not found: {}", ppe_file.display()).into());
    }

    let name = if last_name.is_empty() {
        first_name
    } else {
        format!("{} {}", first_name, last_name)
    };

    Ok(CallWaitMessage::RunPPE(ppe_file, Some(name), Some(password), Some(ppe_params)))
}
