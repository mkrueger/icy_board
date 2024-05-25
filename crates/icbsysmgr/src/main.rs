use app::new_main_window;
use argh::FromArgs;
use chrono::Local;
use color_eyre::Result;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{get_text_args, print_error, term};
use semver::Version;
use std::{
    collections::HashMap,
    path::PathBuf,
    process::exit,
    sync::{Arc, Mutex},
};

pub mod app;
pub mod tabs;

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}

/// IcyBord Setup Utilitiy
#[derive(FromArgs)]
struct Cli {
    /// default is 80x25
    #[argh(switch, short = 'f')]
    full_screen: bool,

    #[argh(option)]
    /// file[.toml] to edit/create
    file: Option<PathBuf>,
}

fn main() -> Result<()> {
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
        .chain(fern::log_file("icbsetup.log").unwrap())
        // Apply globally
        .apply()
        .unwrap();

    let arguments: Cli = argh::from_env();

    let mut file = arguments.file.clone().unwrap_or(PathBuf::from("."));
    if file.is_dir() {
        file = file.join("icyboard.toml");
    }

    let file = file.with_extension("toml");
    if !file.exists() {
        let mut map: HashMap<String, String> = HashMap::new();
        map.insert("name".to_string(), file.display().to_string());
        print_error(get_text_args("file_not_found", map));
        exit(1);
    }

    match IcyBoard::load(&file) {
        Ok(icy_board) => {
            let terminal = &mut term::init()?;
            let icy_board = Arc::new(Mutex::new(icy_board));
            new_main_window(icy_board.clone(), arguments.full_screen).run(terminal)?;

            if let Err(err) = icy_board.lock().unwrap().save() {
                eprintln!("Error saving config: {}", err);
            }
            term::restore()?;
            Ok(())
        }
        Err(err) => {
            print_error(format!("Error loading main config file: {}", err));
            exit(1);
        }
    }
}
