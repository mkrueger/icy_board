use std::{
    fs,
    io::stdout,
    path::{Path, PathBuf},
    process,
    sync::{Arc, Mutex},
};

use bbs::{await_telnet_connections, BBS};
use call_wait_screen::{CallWaitMessage, CallWaitScreen};
use chrono::Local;
use clap::{Parser, Subcommand};
use create::IcyBoardCreator;
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, Clear, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use icy_board_engine::{icy_board::IcyBoard, Res};
use import::{
    console_logger::{print_error, ConsoleLogger},
    PCBoardImporter,
};
use node_monitoring_screen::NodeMonitoringScreenMessage;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use semver::Version;
use tui::{print_exit_screen, Tui};

mod bbs;
mod call_wait_screen;
mod create;
mod icy_engine_output;
mod import;
pub mod menu_runner;
pub mod mods;
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
        name: PathBuf,
        /// Output directory
        out: PathBuf,
    },
    /// Creates a new IcyBoard configuration
    Create { destination: PathBuf },
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
                        let destination = importer.output_directory.join("importlog.txt");
                        fs::write(destination, &importer.logger.output)?;
                    }
                },
                Err(e) => {
                    print_error(e.to_string());
                }
            }
        }
        Commands::Create { destination } => {
            if destination.exists() {
                print_error("Destination already exists".to_string());
                process::exit(1);
            }
            let mut creator = IcyBoardCreator::new(destination);

            if let Err(err) = creator.create() {
                print_error(err.to_string());
                process::exit(1);
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
            let mut bbs = Arc::new(Mutex::new(BBS::new(icy_board.config.board.num_nodes as usize)));
            let board = Arc::new(Mutex::new(icy_board));

            {
                let bbs = bbs.clone();
                let board = board.clone();
                std::thread::spawn(move || {
                    let _ = await_telnet_connections(board, bbs);
                });
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
            let mut tui = Tui::local_mode(board, bbs, false);
            if let Err(err) = tui.run(&board) {
                restore_terminal()?;
                log::error!("while running board in local mode: {}", err.to_string());
                println!("Error: {}", err);
                process::exit(1);
            }
        }
        CallWaitMessage::Sysop(_busy) => {
            stdout().execute(Clear(crossterm::terminal::ClearType::All)).unwrap();
            let mut tui = Tui::local_mode(board, bbs, true);
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
