use std::{
    fmt::Display,
    io::stdout,
    path::{Path, PathBuf},
    process::{self, Command},
    sync::Arc,
};

use argh::FromArgs;
use bbs::await_telnet_connections;
use call_wait_screen::{CallWaitMessage, CallWaitScreen};
use chrono::Local;
use crossterm::{
    execute,
    style::{Attribute, Print, SetAttribute, SetForegroundColor},
    terminal::Clear,
    ExecutableCommand,
};
use icy_board_engine::{
    icy_board::{bbs::BBS, IcyBoard},
    Res,
};

use node_monitoring_screen::NodeMonitoringScreenMessage;
use ratatui::{backend::Backend, Terminal};
use semver::Version;
use tokio::sync::Mutex;
use tui::{print_exit_screen, Tui};

use crate::bbs::{await_securewebsocket_connections, await_ssh_connections, await_websocket_connections};

mod bbs;
mod call_wait_screen;
mod icy_engine_output;
pub mod menu_runner;
pub mod mods;
mod node_monitoring_screen;
mod terminal_thread;
mod tui;

#[derive(FromArgs)]
/// IcyBoard BBS
struct Cli {
    /// default is 80x25
    #[argh(switch, short = 'f')]
    full_screen: bool,

    #[argh(option)]
    /// path/file name of the icyboard.toml configuration file
    file: Option<PathBuf>,

    #[argh(switch)]
    /// login locally to icy board
    localon: bool,

    #[argh(option)]
    /// execute PPE file
    ppe: Option<PathBuf>,
}

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}
/// evlevlelvelvelv`

#[tokio::main]
async fn main() -> Res<()> {
    let arguments: Cli = argh::from_env();
    let file = arguments.file.clone().unwrap_or(PathBuf::from("."));
    start_icy_board(&arguments, &file).await?;
    Ok(())
}

async fn start_icy_board<P: AsRef<Path>>(arguments: &Cli, config_file: &P) -> Res<()> {
    let mut file = config_file.as_ref().to_path_buf();
    if file.is_dir() {
        file = file.join("icyboard.toml");
    }

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
        Ok(icy_board) => {
            let mut bbs = Arc::new(Mutex::new(BBS::new(icy_board.config.board.num_nodes as usize)));
            let board = Arc::new(tokio::sync::Mutex::new(icy_board));
            if arguments.localon || arguments.ppe.is_some() {
                let mut terminal = init_terminal()?;
                let cmd = if let Some(ppe) = &arguments.ppe {
                    CallWaitMessage::RunPPE(ppe.clone())
                } else {
                    CallWaitMessage::User(false)
                };
                run_message(cmd, &mut terminal, &board, &mut bbs, arguments.full_screen).await?;
                restore_terminal()?;
                return Ok(());
            }
            {
                let telnet_connection = board.lock().await.config.login_server.telnet.clone();
                if telnet_connection.is_enabled {
                    let bbs = bbs.clone();
                    let board = board.clone();
                    std::thread::Builder::new()
                        .name("Telnet connect".to_string())
                        .spawn(move || {
                            tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                                let _ = await_telnet_connections(telnet_connection, board, bbs).await;
                            });
                        })
                        .unwrap();
                }

                let ssh_connection = board.lock().await.config.login_server.ssh.clone();
                if ssh_connection.is_enabled {
                    let bbs = bbs.clone();
                    let board = board.clone();
                    std::thread::Builder::new()
                        .name("SSH connect".to_string())
                        .spawn(move || {
                            let _ = await_ssh_connections(ssh_connection, board, bbs);
                        })
                        .unwrap();
                }

                let websocket_connection = board.lock().await.config.login_server.websocket.clone();
                if websocket_connection.is_enabled {
                    let bbs = bbs.clone();
                    let board = board.clone();
                    std::thread::Builder::new()
                        .name("Websocket connect".to_string())
                        .spawn(move || {
                            let _ = await_websocket_connections(websocket_connection, board, bbs);
                        })
                        .unwrap();
                }

                let secure_websocket_connection = board.lock().await.config.login_server.secure_websocket.clone();
                if secure_websocket_connection.is_enabled {
                    let bbs = bbs.clone();
                    let board = board.clone();
                    std::thread::Builder::new()
                        .name("Secure Websocket connect".to_string())
                        .spawn(move || {
                            let _ = await_securewebsocket_connections(secure_websocket_connection, board, bbs);
                        })
                        .unwrap();
                }
            }

            let mut app = CallWaitScreen::new(&board).await?;
            let mut terminal = init_terminal()?;
            loop {
                terminal.clear()?;
                app.reset(&board).await;
                match app.run(&mut terminal, &board, arguments.full_screen).await {
                    Ok(msg) => {
                        if let Err(err) = run_message(msg, &mut terminal, &board, &mut bbs, arguments.full_screen).await {
                            restore_terminal()?;
                            log::error!("while processing call wait screen message: {}", err.to_string());
                            print_error(err.to_string());
                            return Err(err);
                        }
                    }
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

async fn run_message(
    msg: CallWaitMessage,
    terminal: &mut Terminal<impl Backend>,
    board: &Arc<tokio::sync::Mutex<IcyBoard>>,
    bbs: &mut Arc<Mutex<BBS>>,
    full_screen: bool,
) -> Res<()> {
    match msg {
        CallWaitMessage::User(_busy) => {
            stdout().execute(Clear(crossterm::terminal::ClearType::All)).unwrap();
            let mut tui = Tui::local_mode(board, bbs, false, None).await;
            if let Err(err) = tui.run(bbs, &board).await {
                restore_terminal()?;
                log::error!("while running board in local mode: {}", err.to_string());
                println!("Error: {}", err);
                process::exit(1);
            }
        }
        CallWaitMessage::RunPPE(ppe) => {
            stdout().execute(Clear(crossterm::terminal::ClearType::All)).unwrap();
            let mut tui = Tui::local_mode(board, bbs, true, Some(ppe)).await;
            if let Err(err) = tui.run(bbs, &board).await {
                restore_terminal()?;
                log::error!("while running board in local mode: {}", err.to_string());
                println!("Error: {}", err);
                process::exit(1);
            }
        }
        CallWaitMessage::Sysop(_busy) => {
            stdout().execute(Clear(crossterm::terminal::ClearType::All)).unwrap();
            let mut tui: Tui = Tui::local_mode(board, bbs, true, None).await;
            if let Err(err) = tui.run(bbs, &board).await {
                restore_terminal()?;
                log::error!("while running board in local mode: {}", err.to_string());
                println!("Error: {}", err);
                process::exit(1);
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
                .arg("--file")
                .arg(format!("{}", board.lock().await.file_name.display()))
                .spawn()
                .expect("icbsysmgr command failed to start");
            cmd.wait().expect("icbsysmgr command failed to run");
        }
        CallWaitMessage::Setup => {
            let path = std::env::current_exe().unwrap().with_file_name("icbsetup");
            let mut cmd = Command::new(path)
                .arg("--file")
                .arg(format!("{}", board.lock().await.file_name.display()))
                .spawn()
                .expect("icbsysmgr command failed to start");
            cmd.wait().expect("icbsysmgr command failed to run");
        }
    }
    Ok(())
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
