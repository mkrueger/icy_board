use std::{
    io::stdout,
    path::{Path, PathBuf},
    process,
    sync::{Arc, Mutex},
};

use call_wait_screen::{CallWaitMessage, CallWaitScreen};
use chrono::Local;
use clap::{Parser, Subcommand};
use crossterm::{terminal::Clear, ExecutableCommand};
use icy_board_engine::icy_board::IcyBoard;
use icy_engine_output::Screen;
use import::{
    console_logger::{print_error, ConsoleLogger},
    PCBoardImporter,
};
use semver::Version;
use tui::{print_exit_screen, Tui};

use crate::call_wait_screen::restore_terminal;

mod bbs;
mod call_wait_screen;
mod icy_engine_output;
mod import;
pub mod menu_runner;
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
                        log::error!("while running call wait screen: {}", err.to_string());
                        print_error(err.to_string());
                    }
                }
            }
        }
        Err(err) => {
            log::error!("while loading icy board configuration: {}", err.to_string());
            print_error(err.to_string());
        }
    }
}

fn run_message(msg: CallWaitMessage, board: Arc<Mutex<IcyBoard>>) {
    match msg {
        CallWaitMessage::User(busy) | CallWaitMessage::Sysop(busy) => {
            println!("Login {}...", busy);

            stdout().execute(Clear(crossterm::terminal::ClearType::All)).unwrap();

            let mut tui = Tui::new(board);
            if let Err(err) = tui.run() {
                restore_terminal().unwrap();
                log::error!("while running board in local mode: {}", err.to_string());
                println!("Error: {}", err);
                process::exit(1);
            }
        }
        CallWaitMessage::Exit(busy) => {
            println!("Exiting {}...", busy);
            restore_terminal().unwrap();
            print_exit_screen();
            process::exit(0);
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
