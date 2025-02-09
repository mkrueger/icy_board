use app::new_main_window;
use argh::FromArgs;
use chrono::Local;
use codepages::tables::write_with_bom;
use color_eyre::Result;
use create::IcyBoardCreator;
use icy_board_engine::icy_board::{read_with_encoding_detection, IcyBoard};
use icy_board_tui::{print_error, term};
use import::{console_logger::ConsoleLogger, PCBoardImporter};
use semver::Version;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{self, exit},
    sync::{Arc, Mutex},
};
use walkdir::WalkDir;

pub mod app;
mod create;
pub mod editors;
mod import;
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

    #[argh(subcommand)]
    command: Option<Commands>,

    #[argh(positional)]
    /// path/file name of the icyboard.toml configuration file
    file: Option<PathBuf>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum Commands {
    Import(Import),
    Create(Create),
    PPEConvert(PPEConvert),
}

#[derive(FromArgs, PartialEq, Debug)]
/// Import PCBDAT.DAT file to IcyBoard
#[argh(subcommand, name = "import")]
struct Import {
    /// PCBOARD.DAT file to import
    #[argh(positional)]
    name: PathBuf,

    /// output directory
    #[argh(positional)]
    out: PathBuf,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Creates a new IcyBoard configuration#[argh(subcommand, name = "scan")]
#[argh(subcommand, name = "create")]
struct Create {
    /// output directory
    #[argh(positional)]
    file: PathBuf,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Converts a path to UTF-8
#[argh(subcommand, name = "ppe-convert")]
struct PPEConvert {
    /// directory to convert
    #[argh(positional)]
    path: PathBuf,
}

fn main() -> Result<()> {
    let arguments: Cli = argh::from_env();

    match &arguments.command {
        Some(Commands::Import(Import { name, out })) => {
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
            return Ok(());
        }
        Some(Commands::Create(Create { file })) => {
            if file.exists() {
                print_error("Destination already exists".to_string());
                process::exit(1);
            }
            let mut creator = IcyBoardCreator::new(file);

            if let Err(err) = creator.create() {
                print_error(err.to_string());
                process::exit(1);
            }
            return Ok(());
        }
        Some(Commands::PPEConvert(PPEConvert { path })) => {
            println!("Converting PPE data files in {}", path.display());
            println!("Caution - this command is used for converting CP437 to UTF-8 in a directory.");

            if fs::metadata(path).is_err() {
                println!("Path does not exist");
                return Ok(());
            }

            if fs::metadata(path).unwrap().is_file() {
                println!("Converting file to utf-8...");
                convert_file(path.clone());
                return Ok(());
            }

            println!("Converting directories to lower case...");
            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                if !entry.path().is_dir() {
                    continue;
                }
                let lower_case = entry.path().to_string_lossy().to_string().to_lowercase();
                if lower_case == "." || lower_case == ".." {
                    continue;
                }
                println!("Rename directory {} to {}", entry.path().display(), lower_case);
                if fs::rename(entry.path(), lower_case).is_err() {
                    println!("Error renaming directory {}", entry.path().display());
                }
            }
            println!("Converting files...");
            let convert_ext = ["ANS", "PCB", "CFG", "DOC", "NFO", "ASC", "TXT", "PPX", "PPS", "PPD", "LST", "XXX"];
            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                if !entry.path().is_file() {
                    continue;
                }
                let lower_case = entry.path().to_string_lossy().to_string().to_lowercase();
                if lower_case == entry.path().to_string_lossy().to_string() {
                    continue;
                }
                if let Some(extension) = entry.path().extension() {
                    if convert_ext.contains(&extension.to_str().unwrap()) {
                        println!("Converting {} to utf8...", entry.path().display());
                        convert_file(entry.path().to_path_buf());
                    }
                }
                println!("Rename {} to {}", entry.path().display(), lower_case);
                if fs::rename(entry.path(), lower_case).is_err() {
                    println!("Error renaming {}", entry.path().display());
                }
            }
            return Ok(());
        }
        _ => {}
    }
    let Some(file) = icy_board_engine::lookup_icyboard_file(&arguments.file) else {
        print_error(icy_board_tui::get_text("error_file_or_path_not_found"));
        exit(1);
    };
    init_log(&file.parent().unwrap().join("icbsetup.log"));
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

fn init_log(path: &Path) {
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
        .chain(fern::log_file(path).unwrap())
        // Apply globally
        .apply()
        .unwrap();
}

fn convert_file(entry: PathBuf) {
    if let Ok(data) = read_with_encoding_detection(&entry) {
        if write_with_bom(&entry, &data).is_err() {
            println!("Error writing {}", entry.display());
        }
    }
}
