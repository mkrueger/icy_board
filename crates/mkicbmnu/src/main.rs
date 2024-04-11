use app::App;
use clap::Parser;
use color_eyre::Result;
use icy_board_engine::icy_board::{menu::Menu, IcyBoardSerializer};
use icy_board_tui::{get_text_args, print_error, term};
use semver::Version;
use std::{collections::HashMap, path::PathBuf, process::exit};

mod app;

mod tabs;
pub use tabs::*;

pub mod edit_command_dialog;

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}

/// Make IcyBord Menus
#[derive(clap::Parser)]
#[command(version="", about="IcyBoard menu utility", long_about = None)]
struct Cli {
    /// create menu file
    #[arg(long, short)]
    create: bool,

    /// Default is 80x25
    #[arg(long, short)]
    full_screen: bool,

    /// file[.mnu] to edit/create (extension will always be .mnu)
    file: PathBuf,
}

fn main() -> Result<()> {
    let arguments = Cli::parse();

    let file = arguments.file.with_extension("mnu");
    if !file.exists() && !arguments.create {
        let mut map = HashMap::new();
        map.insert("name".to_string(), file.display().to_string());
        print_error(get_text_args("file_not_found", map));
        exit(1);
    }

    if arguments.create {
        Menu::default().save(&file).unwrap();
    }

    match Menu::load(&file) {
        Ok(mut mnu) => {
            mnu.title = "Title Menu".to_string();
            mnu.display_file = PathBuf::from("file.txt");
            mnu.help_file = PathBuf::from("help.txt");
            mnu.prompt = "Enter selection: ".to_string();
            let terminal = &mut term::init()?;
            App::new(mnu, arguments.full_screen).run(terminal)?;
            term::restore()?;
            Ok(())
        }
        Err(err) => {
            print_error(format!("{}", err));
            exit(1);
        }
    }
}
