use app::App;
use argh::FromArgs;
use color_eyre::Result;
use icy_board_tui::{get_text_args, print_error, term};
use semver::Version;
use std::{collections::HashMap, path::PathBuf, process::exit};
mod app;
mod tabs;

use icy_board_engine::icy_board::icb_text::{IcbTextFile, IcbTextFormat, DEFAULT_DISPLAY_TEXT};

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}

#[derive(FromArgs)]
/// ICBTEXT File Generator/Editor
struct Cli {
    /// create new ICBTEXT file
    #[argh(switch, short = 'c')]
    create: bool,

    /// import PCBTEXT file
    #[argh(switch, short = 'i')]
    import: bool,

    /// default is 80x25
    #[argh(switch, short = 'f')]
    full_screen: bool,

    /// file to edit/create
    #[argh(positional)]
    file: PathBuf,
}

fn main() -> Result<()> {
    let arguments: Cli = argh::from_env();

    let file = arguments.file;
    if !file.exists() && !arguments.create {
        let mut map = HashMap::new();
        map.insert("name".to_string(), file.display().to_string());
        print_error(get_text_args("file_not_found", map));
        exit(1);
    }

    if arguments.create {
        DEFAULT_DISPLAY_TEXT.save(&file).unwrap();
        println!("File created: {}", file.display());
        return Ok(());
    }

    match IcbTextFile::load(&file) {
        Ok(mut icb_txt) => {
            if arguments.import {
                let out_file = file.with_extension("toml");
                if let Err(err) = icb_txt.save(&out_file) {
                    print_error(format!("Can't save: {}", err));
                    exit(1);
                }
                println!("File imported to: {}", out_file.display());
                return Ok(());
            }

            let terminal = &mut term::init()?;
            let mut app = App::new(&mut icb_txt, file.clone(), arguments.full_screen);
            app.run(terminal)?;
            term::restore()?;
            if app.save {
                let res = match icb_txt.get_format() {
                    IcbTextFormat::IcyBoard => icb_txt.save(&file),
                    IcbTextFormat::PCBoard => icb_txt.export_pcboard_format(&file),
                };
                if let Err(err) = res {
                    print_error(format!("Can't save: {}", err));
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
