use std::{
    fmt::Display,
    io::stdout,
    path::{Path, PathBuf},
    process,
    sync::{Arc, Mutex},
};

use argh::FromArgs;
use bbs::{await_telnet_connections, BBS};
use call_wait_screen::{CallWaitMessage, CallWaitScreen};
use chrono::Local;
use crossterm::{
    execute,
    style::{Attribute, Print, SetAttribute, SetForegroundColor},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use icy_board_engine::{icy_board::IcyBoard, Res};

use node_monitoring_screen::NodeMonitoringScreenMessage;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use semver::Version;
use tui::{print_exit_screen, Tui};

use crate::bbs::{await_securewebsocket_connections, await_ssh_connections, await_websocket_connections};

mod bbs;
mod call_wait_screen;
mod icy_engine_output;
pub mod menu_runner;
pub mod mods;
mod node_monitoring_screen;
mod tui;

#[derive(FromArgs)]
/// IcyBoard BBS
struct Cli {
    #[argh(option)]
    /// path/file name of the icyboard.toml configuration file
    file: Option<PathBuf>,

    #[argh(switch)]
    /// login locally to icy board
    localon: bool,

    #[argh(option)]
    /// execute PPE file
    ppe: Option<PathBuf>,
    /*

    */
}
/*

*/
lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}
/// evlevlelvelvelv`

fn main() -> Res<()> {
    let arguments: Cli = argh::from_env();
    let file = arguments.file.clone().unwrap_or(PathBuf::from("."));

    start_icy_board(&arguments, &file)?;

    Ok(())
}

fn start_icy_board<P: AsRef<Path>>(arguments: &Cli, config_file: &P) -> Res<()> {
    let mut file = config_file.as_ref().to_path_buf();
    if file.is_dir() {
        file = file.join("icyboard.toml");
    }

    let config_file = file.with_extension("toml");
    match IcyBoard::load(&config_file) {
        Ok(icy_board) => {
            let log_file = icy_board.resolve_file(&icy_board.config.paths.log_file);
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

            let mut bbs = Arc::new(Mutex::new(BBS::new(icy_board.config.board.num_nodes as usize)));
            let board = Arc::new(Mutex::new(icy_board));
            if arguments.localon || arguments.ppe.is_some() {
                let mut terminal = init_terminal()?;
                let cmd = if let Some(ppe) = &arguments.ppe {
                    CallWaitMessage::RunPPE(ppe.clone())
                } else {
                    CallWaitMessage::User(false)
                };
                run_message(cmd, &mut terminal, &board, &mut bbs)?;
                restore_terminal()?;
                return Ok(());
            }
            {
                let telnet_connection = board.lock().unwrap().config.login_server.telnet.clone();
                if telnet_connection.is_enabled {
                    let bbs = bbs.clone();
                    let board = board.clone();
                    std::thread::spawn(move || {
                        tokio::runtime::Builder::new_multi_thread()
                            .worker_threads(4)
                            .enable_all()
                            .build()
                            .unwrap()
                            .block_on(async {
                                await_telnet_connections(telnet_connection, board, bbs).await;
                            });
                    });
                }

                let ssh_connection = board.lock().unwrap().config.login_server.ssh.clone();
                if ssh_connection.is_enabled {
                    let bbs = bbs.clone();
                    let board = board.clone();
                    std::thread::spawn(move || {
                        let _ = await_ssh_connections(ssh_connection, board, bbs);
                    });
                }

                let websocket_connection = board.lock().unwrap().config.login_server.websocket.clone();
                if websocket_connection.is_enabled {
                    let bbs = bbs.clone();
                    let board = board.clone();
                    std::thread::spawn(move || {
                        let _ = await_websocket_connections(websocket_connection, board, bbs);
                    });
                }

                let secure_websocket_connection = board.lock().unwrap().config.login_server.secure_websocket.clone();
                if secure_websocket_connection.is_enabled {
                    let bbs = bbs.clone();
                    let board = board.clone();
                    std::thread::spawn(move || {
                        let _ = await_securewebsocket_connections(secure_websocket_connection, board, bbs);
                    });
                }
            }

            loop {
                let mut app = CallWaitScreen::new(&board)?;
                let mut terminal = init_terminal()?;
                match app.run(&mut terminal, &board) {
                    Ok(msg) => {
                        if let Err(err) = run_message(msg, &mut terminal, &board, &mut bbs) {
                            restore_terminal()?;
                            log::error!("while processing call wait screen message: {}", err.to_string());
                            print_error(err.to_string());
                        }
                    }
                    Err(err) => {
                        restore_terminal()?;
                        log::error!("while running call wait screen: {}", err.to_string());
                        print_error(err.to_string());
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

fn run_message(msg: CallWaitMessage, terminal: &mut Terminal<impl Backend>, board: &Arc<Mutex<IcyBoard>>, bbs: &mut Arc<Mutex<BBS>>) -> Res<()> {
    match msg {
        CallWaitMessage::User(_busy) => {
            stdout().execute(Clear(crossterm::terminal::ClearType::All)).unwrap();
            let mut tui = Tui::local_mode(board, bbs, false, None);
            if let Err(err) = tui.run(&board) {
                restore_terminal()?;
                log::error!("while running board in local mode: {}", err.to_string());
                println!("Error: {}", err);
                process::exit(1);
            }
        }
        CallWaitMessage::RunPPE(ppe) => {
            stdout().execute(Clear(crossterm::terminal::ClearType::All)).unwrap();
            let mut tui = Tui::local_mode(board, bbs, true, Some(ppe));
            if let Err(err) = tui.run(&board) {
                restore_terminal()?;
                log::error!("while running board in local mode: {}", err.to_string());
                println!("Error: {}", err);
                process::exit(1);
            }
        }
        CallWaitMessage::Sysop(_busy) => {
            stdout().execute(Clear(crossterm::terminal::ClearType::All)).unwrap();
            let mut tui = Tui::local_mode(board, bbs, true, None);
            if let Err(err) = tui.run(&board) {
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
            let mut app = node_monitoring_screen::NodeMonitoringScreen::new(&board);
            match app.run(terminal, &board, bbs) {
                Ok(msg) => {
                    if let NodeMonitoringScreenMessage::EnterNode(node) = msg {
                        let mut tui = Tui::sysop_mode(bbs, node)?;
                        if let Err(err) = tui.run(&board) {
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
                }
            }
        }
    }
    Ok(())
}

fn init_error_hooks() -> Res<()> {
    //let (panic, error) = HookBuilder::default().into_hooks();
    //let panic = panic.into_panic_hook();
    //let error = error.into_eyre_hook();
    /*color_eyre::eyre::set_hook(Box::new(move |e| {
        let _ = restore_terminal();
        error(e)
    }))?; */
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        eprintln!("{}", info);
    }));
    Ok(())
}

fn init_terminal() -> Res<Terminal<impl Backend>> {
    init_error_hooks()?;
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

pub fn restore_terminal() -> Res<()> {
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
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
