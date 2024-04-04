use std::{
    collections::VecDeque,
    io::stdout,
    path::{Path, PathBuf},
    process,
    sync::{Arc, Mutex},
};

use bbs::PcbBoardCommand;
use call_wait_screen::{CallWaitMessage, CallWaitScreen};
use chrono::Local;
use clap::{Parser, Subcommand};
use crossterm::{terminal::Clear, ExecutableCommand};
use icy_board_engine::icy_board::{
    pcboard_data::PcbBoardData, state::IcyBoardState, IcyBoard, PcbBoard,
};
use icy_engine_output::{IcyEngineOutput, Screen};
use icy_ppe::Res;
use import::convert_pcb;
use semver::Version;
use tui::{print_exit_screen, Tui};

use crate::call_wait_screen::restore_terminal;

pub mod bbs;
mod call_wait_screen;
mod icy_engine_output;
mod import;
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

fn main() {
    let _ = init_error_hooks();
    let arguments = Cli::parse();
    match &arguments.command {
        Commands::Import { name, out } => match PcbBoard::load(name) {
            Ok(mut icy_board) => {
                convert_pcb(&mut icy_board, &PathBuf::from(out)).unwrap();
            }
            Err(e) => {
                restore_terminal().unwrap();
                println!("Error: {}", e);
            }
        },
        Commands::Run { file } => {
            start_icy_board(file);
        }
    }
}

pub fn start_icy_board<P: AsRef<Path>>(config_file: &P) {
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
            let board = Arc::new(Mutex::new(icy_board));
            loop {
                let mut app = CallWaitScreen::new(board.clone()).unwrap();
                match app.run() {
                    Ok(msg) => {
                        run_message(msg, board.clone());
                    }
                    Err(err) => {
                        restore_terminal().unwrap();
                        println!("Error: {}", err);
                    }
                }
            }
        }
        Err(e) => {
            restore_terminal().unwrap();
            println!("Error: {}", e);
        }
    }
}

fn run_message(msg: CallWaitMessage, board: Arc<Mutex<IcyBoard>>) {
    match msg {
        CallWaitMessage::User(_) | CallWaitMessage::Sysop(_) => {
            stdout()
                .execute(Clear(crossterm::terminal::ClearType::All))
                .unwrap();

            let screen = Arc::new(Mutex::new(Screen::new()));
            let input_buffer = Arc::new(Mutex::new(VecDeque::new()));
            let io = Arc::new(Mutex::new(IcyEngineOutput::new(
                screen.clone(),
                input_buffer.clone(),
            )));

            let mut state = IcyBoardState::new(board, io);
            state.session.is_sysop = true;
            state.set_current_user(0);
            let cmd = PcbBoardCommand::new(state);

            let mut tui = Tui::new(cmd, screen, input_buffer);
            if let Err(err) = tui.run() {
                restore_terminal().unwrap();
                println!("Error: {}", err);
                process::exit(1);
            }
        }
        CallWaitMessage::Exit(_) => {
            restore_terminal().unwrap();
            print_exit_screen();
            process::exit(0);
        }
    }
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
