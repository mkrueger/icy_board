use std::{
    io::stdout,
    path::{Path, PathBuf},
    process,
    sync::{Arc, Mutex},
};

use bbs::{await_telnet_connections, BBS};
use call_wait_screen::{CallWaitMessage, CallWaitScreen};
use chrono::Local;
use clap::{Parser, Subcommand};
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, Clear, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use icy_board_engine::icy_board::IcyBoard;
use icy_ppe::Res;
use import::{
    console_logger::{print_error, ConsoleLogger},
    PCBoardImporter,
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use semver::Version;
use tui::{print_exit_screen, Tui};

mod bbs;
mod call_wait_screen;
mod icy_engine_output;
mod import;
pub mod menu_runner;
mod node_monitoring_screen;
mod tui;

#[derive(clap::Parser)]
#[command(version="", about="PcbBoard BBS", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Import PCBDAT.DAT file to IcyBoard
    Import {
        /// PCBOARD.DAT file to import
        name: String,
        /// Output directory
        out: String,
    },

    Run {
        /// PCBOARD.DAT file to run
        file: String,
    },
}

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}
/// evlevlelvelvelv`

fn main() -> Res<()> {
    let arguments = Cli::parse();

    match &arguments.command {
        Commands::Import { name, out } => {
            let output = Box::<ConsoleLogger>::default();
            match PCBoardImporter::new(name, output, PathBuf::from(out)) {
                Ok(mut importer) => match importer.start_import() {
                    Ok(_) => {
                        println!("Imported successfully");
                    }
                    Err(e) => {
                        print_error(e.to_string());
                    }
                },
                Err(e) => {
                    print_error(e.to_string());
                }
            }
        }
        Commands::Run { file } => {
            start_icy_board(file)?;
        }
    }
    Ok(())
}

pub fn start_icy_board<P: AsRef<Path>>(config_file: &P) -> Res<()> {
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
        .chain(fern::log_file("output.log").unwrap())
        // Apply globally
        .apply()
        .unwrap();

    match IcyBoard::load(config_file) {
        Ok(icy_board) => {
            let mut bbs = Arc::new(Mutex::new(BBS::new(icy_board.config.num_nodes as usize)));
            let board = Arc::new(Mutex::new(icy_board));

            {
                let bbs = bbs.clone();
                let board = board.clone();
                std::thread::spawn(move || {
                    let _ = await_telnet_connections(board, bbs);
                });
            }
            let mut terminal = init_terminal()?;

            loop {
                let mut app = CallWaitScreen::new(&board)?;
                match app.run(&mut terminal, &board) {
                    Ok(msg) => {
                        run_message(msg, &mut terminal, &board, &mut bbs);
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

fn run_message(msg: CallWaitMessage, terminal: &mut Terminal<impl Backend>, board: &Arc<Mutex<IcyBoard>>, bbs: &mut Arc<Mutex<BBS>>) {
    match msg {
        CallWaitMessage::User(_busy) | CallWaitMessage::Sysop(_busy) => {
            stdout().execute(Clear(crossterm::terminal::ClearType::All)).unwrap();
            let mut tui = Tui::new(board, bbs);
            if let Err(err) = tui.run(&board) {
                restore_terminal().unwrap();
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
                Ok(_msg) => {
                    // TODO
                }
                Err(err) => {
                    restore_terminal().unwrap();
                    log::error!("while running node monitoring screen: {}", err.to_string());
                    print_error(err.to_string());
                }
            }
        }
    }
}
/*
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
*/

fn init_terminal() -> Res<Terminal<impl Backend>> {
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
