use app::new_main_window;
use argh::FromArgs;
use chrono::Local;
use color_eyre::Result;
use icy_board_engine::{
    icy_board::{menu::Menu, IcyBoard, IcyBoardSerializer},
    Res, DEFAULT_ICYBOARD_FILE,
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

    /// file[.mnu] to edit/create (extension will always be .mnu)
    #[argh(positional)]
    file: PathBuf,
}

fn main() -> Result<()> {
    let arguments: Cli = argh::from_env();

    let file = arguments.file.with_extension("mnu");
    if !file.exists() && !arguments.create {
        print_error(icy_board_tui::get_text("error_file_or_path_not_found"));
        exit(1);
    }

    let Ok(icy_board) = load_icy_board(file.parent()) else {
        print_error(format!("{} not found", icy_board_engine::DEFAULT_ICYBOARD_FILE));
        exit(1);
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
        .chain(fern::log_file(&log_file).unwrap())
        // Apply globally
        .apply()
        .unwrap();

    if arguments.create {
        Menu::default().save(&file).unwrap();
    }

    match Menu::load(&file) {
        Ok(mnu) => {
            let terminal = &mut term::init()?;
            let mnu = Arc::new(Mutex::new(mnu));
            let mut app = new_main_window(icy_board, mnu.clone(), arguments.full_screen, &arguments.file);
            app.run(terminal)?;
            term::restore()?;
            if app.save {
                mnu.lock().unwrap().save(&file).unwrap();
            }
            Ok(())
        }
        Err(err) => {
            print_error(format!("{}", err));
            exit(1);
        }
    }
}

fn load_icy_board(parent: Option<&std::path::Path>) -> Res<IcyBoard> {
    let mut path = parent;
    while path.is_some() {
        let icb_path = path.unwrap();
        if icb_path.join(DEFAULT_ICYBOARD_FILE).exists() {
            return IcyBoard::load(&icb_path.join(DEFAULT_ICYBOARD_FILE));
        }
        path = icb_path.parent();
    }

    Err(std::io::Error::new(std::io::ErrorKind::NotFound, format!("{} not found", DEFAULT_ICYBOARD_FILE)).into())
}
