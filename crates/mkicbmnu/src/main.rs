use app::new_main_window;
use argh::FromArgs;
use chrono::Local;
use color_eyre::Result;
use icy_board_engine::{
    DEFAULT_ICYBOARD_FILE,
    icy_board::{IcyBoard, IcyBoardSerializer, menu::Menu},
};
use icy_board_tui::{print_error, term};
use semver::Version;
use std::{
    path::PathBuf,
    process::exit,
    sync::{Arc, Mutex},
};

mod app;

mod tabs;
pub use tabs::*;

pub mod edit_command_dialog;

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}

/// IcyBoard menu utility
#[derive(FromArgs)]
struct Cli {
    /// create menu file
    #[argh(switch, short = 'c')]
    create: bool,

    /// default is 80x25
    #[argh(switch, short = 'f')]
    full_screen: bool,

    /// print the version and exit
    #[argh(switch)]
    version: bool,

    /// file[.mnu] to edit/create (extension will always be .mnu)
    #[argh(positional)]
    file: Option<PathBuf>,
}

fn main() -> Result<()> {
    let arguments: Cli = argh::from_env();
    if arguments.version {
        println!("mkicbmnu {}", *VERSION);
        return Ok(());
    }

    let Some(menu_file) = arguments.file.clone() else {
        if let Err(err) = Cli::from_args(&["mkicbmnu"], &["--help"]) {
            eprintln!("{}", err.output);
        }
        exit(1);
    };

    let file = menu_file.with_extension("mnu");
    if !file.exists() && !arguments.create {
        icy_board_tui::print_input_file_not_found("mkicbmnu", &file);
        exit(1);
    }

    let Some(board_file) = find_icy_board(file.parent()) else {
        icy_board_tui::print_parent_board_config_not_found("mkicbmnu", &file);
        exit(1);
    };
    let icy_board = match IcyBoard::load(&board_file) {
        Ok(icy_board) => icy_board,
        Err(err) => {
            print_error(format!("Can't load {}: {err}", board_file.display()));
            exit(1);
        }
    };

    let log_file = icy_board.file_name.with_extension("log");
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
        .chain(match fern::log_file(&log_file) {
            Ok(log) => log,
            Err(err) => {
                print_error(format!("Can't open log file {}: {err}", log_file.display()));
                exit(1);
            }
        })
        // Apply globally
        .apply()
        .unwrap_or_else(|err| {
            print_error(format!("Can't initialize logging: {err}"));
            exit(1);
        });

    if arguments.create {
        if let Err(err) = Menu::default().save(&file) {
            print_error(format!("Can't create {}: {err}", file.display()));
            exit(1);
        }
    }

    match Menu::load(&file) {
        Ok(mnu) => {
            let terminal = &mut term::init()?;
            let mnu = Arc::new(Mutex::new(mnu));
            let mut app = new_main_window(icy_board, mnu.clone(), arguments.full_screen, &menu_file);
            app.run(terminal)?;
            term::restore()?;
            if app.save.writes() {
                if let Err(err) = mnu.lock().unwrap().save(&file) {
                    print_error(format!("Can't save {}: {err}", file.display()));
                    exit(1);
                }
            }
            Ok(())
        }
        Err(err) => {
            print_error(format!("{}", err));
            exit(1);
        }
    }
}

fn find_icy_board(parent: Option<&std::path::Path>) -> Option<PathBuf> {
    let mut path = parent;
    while path.is_some() {
        let icb_path = path.unwrap();
        let board_file = icb_path.join(DEFAULT_ICYBOARD_FILE);
        if board_file.exists() {
            return Some(board_file);
        }
        path = icb_path.parent();
    }

    None
}
