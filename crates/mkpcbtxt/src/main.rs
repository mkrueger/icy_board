use app::App;
use clap::Parser;
use color_eyre::Result;
use icy_board_tui::{get_text_args, print_error, term};
use semver::Version;
use std::{collections::HashMap, path::PathBuf, process::exit};
mod app;
mod tabs;

use icy_board_engine::icy_board::icb_text::IcbTextFile;

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}

/// PCBText File Generator/Editor
#[derive(clap::Parser)]
#[command(version="", about="IcyBoard menu utility", long_about = None)]
struct Cli {
    /// create menu file
    #[arg(long, short)]
    create: bool,

    /// Default is 80x25
    #[arg(long, short)]
    full_screen: bool,

    /// file to edit/create
    file: PathBuf,
}

fn main() -> Result<()> {
    let arguments = Cli::parse();

    let file = arguments.file;
    if !file.exists() && !arguments.create {
        let mut map = HashMap::new();
        map.insert("name".to_string(), file.display().to_string());
        print_error(get_text_args("file_not_found", map));
        exit(1);
    }

    if arguments.create {
        IcbTextFile::default().save(&file).unwrap();
    }

    match IcbTextFile::load(&file) {
        Ok(icb_txt) => {
            let terminal = &mut term::init()?;
            App::new(icb_txt, file, arguments.full_screen).run(terminal)?;
            term::restore()?;
            Ok(())
        }
        Err(err) => {
            print_error(format!("{}", err));
            exit(1);
        }
    }
}
