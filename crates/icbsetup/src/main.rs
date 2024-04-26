use app::App;
use clap::Parser;
use color_eyre::Result;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{get_text_args, print_error, term};
use semver::Version;
use std::{collections::HashMap, path::PathBuf, process::exit};

pub mod app;
pub mod help_view;
pub mod tabs;

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}

/// IcyBord Setup Utilitiy
#[derive(clap::Parser)]
#[command(version="", about="IcyBoard setup utilitiy", long_about = None)]
struct Cli {
    /// Default is 80x25
    #[arg(long, short)]
    full_screen: bool,

    /// file[.toml] to edit/create
    file: PathBuf,
}

fn main() -> Result<()> {
    let arguments = Cli::parse();

    let mut file = arguments.file.clone();
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
            App::new(icy_board, file, arguments.full_screen).run(terminal)?;
            term::restore()?;
            Ok(())
        }
        Err(err) => {
            print_error(format!("Error loading main config file: {}", err));
            exit(1);
        }
    }
}
