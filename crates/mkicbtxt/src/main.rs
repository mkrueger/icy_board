use app::App;
use argh::FromArgs;
use color_eyre::Result;
use crossterm::{
    execute,
    style::{Attribute, Color, Print, SetAttribute, SetForegroundColor},
};
use icy_board_tui::{print_error, term};
use semver::Version;
use std::{io::stdout, path::PathBuf, process::exit};
mod app;
mod tabs;

use icy_board_engine::icy_board::{
    icb_text::{DEFAULT_DISPLAY_TEXT, IcbTextFile, IcbTextFormat},
    write_atomic,
};

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}

#[derive(FromArgs)]
/// ICBTEXT File Generator/Editor
struct Cli {
    /// create new ICBTEXT file
    #[argh(switch, short = 'c')]
    create: bool,

    /// record number to update with new text,
    #[argh(option, short = 'i')]
    update: Option<usize>,

    /// default is 80x25
    #[argh(switch, short = 'f')]
    full_screen: bool,

    /// convert PCBTEXT to ICBTEXT
    #[argh(switch)]
    convert: bool,

    /// overwrite a file that is already there
    #[argh(switch)]
    force: bool,

    /// file to edit/create
    #[argh(positional)]
    file: PathBuf,

    /// new text to update record
    #[argh(positional)]
    new_text: Option<String>,
}

fn main() -> Result<()> {
    let arguments: Cli = argh::from_env();

    let file = arguments.file;
    if !file.exists() && !arguments.create {
        print_error(icy_board_tui::get_text("error_file_or_path_not_found"));
        exit(1);
    }

    if arguments.create {
        if file.exists() && !arguments.force {
            print_error(format!("{} is already there. Pass --force to overwrite it.", file.display()));
            exit(1);
        }
        if file.exists() {
            create_backup(&file);
        }
        if let Err(err) = DEFAULT_DISPLAY_TEXT.save(&file) {
            print_error(format!("Can't create: {}", err));
            exit(1);
        }
        println!("File created: {}", file.display());
        return Ok(());
    }

    match IcbTextFile::load(&file) {
        Ok(mut icb_txt) => {
            if let Some(rec_num) = arguments.update {
                let Some(text) = arguments.new_text else {
                    print_error("New text is required for update".to_string());
                    exit(1);
                };
                if let Err(err) = icb_txt.update_record_number(rec_num, text) {
                    print_error(format!("{}", err));
                    exit(1);
                }
                save_file(&file, &icb_txt);
                execute!(
                    stdout(),
                    SetAttribute(Attribute::Bold),
                    SetForegroundColor(Color::White),
                    Print(format!("Record #{} has been upgraded in {}.\n", rec_num, file.display())),
                    SetAttribute(Attribute::Reset),
                )
                .ok();
                return Ok(());
            }
            if arguments.convert {
                let out_file = file.with_extension("toml");
                if out_file.exists() && !arguments.force {
                    print_error(format!("{} is already there. Pass --force to overwrite it.", out_file.display()));
                    exit(1);
                }
                if out_file.exists() {
                    create_backup(&out_file);
                }
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
                save_file(&file, &icb_txt);
            }
            Ok(())
        }
        Err(err) => {
            print_error(format!("{}", err));
            exit(1);
        }
    }
}

fn save_file(file: &PathBuf, icb_txt: &IcbTextFile) {
    create_backup(file);
    let res = match icb_txt.get_format() {
        IcbTextFormat::IcyBoard => icb_txt.save(file),
        IcbTextFormat::PCBoard => icb_txt.export_pcboard_format(file),
    };
    if let Err(err) = res {
        print_error(format!("Can't save: {}", err));
        exit(1);
    }
}

fn create_backup(file: &PathBuf) {
    if !file.is_file() {
        return;
    }
    let mut name = file.file_name().unwrap_or_default().to_os_string();
    name.push(".bak");
    let backup = file.with_file_name(name);
    match std::fs::read(file).and_then(|contents| write_atomic(&backup, &contents)) {
        Ok(()) => {}
        Err(err) => {
            print_error(format!("Can't create backup {}: {}", backup.display(), err));
            exit(1);
        }
    }
}
