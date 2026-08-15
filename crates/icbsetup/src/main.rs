use app::new_main_window;
use argh::FromArgs;
use chrono::Local;
use color_eyre::{Result, eyre::eyre};
use create::IcyBoardCreator;
use icy_board_engine::icy_board::{IcyBoard, lock::BoardLock, read_with_encoding_detection, write_atomic};
use icy_board_tui::{print_error, term};
use import::{PCBoardImporter, console_logger::ConsoleLogger};
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
    /// PCBOARD.DAT file or the directory of the PCBoard installation to import
    #[argh(positional)]
    name: PathBuf,

    /// output directory
    #[argh(positional)]
    out: PathBuf,

    /// map a dos path to a local one, may be repeated: --map 'D:\FILES=/mnt/files'
    #[argh(option)]
    map: Vec<String>,

    /// only report what would be imported and which paths can't be resolved
    #[argh(switch)]
    dry_run: bool,
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
        Some(Commands::Import(Import { name, out, map, dry_run })) => {
            let mut mappings = Vec::new();
            for mapping in map {
                let Some((dos_path, local_path)) = mapping.split_once('=') else {
                    print_error(format!("Invalid mapping '{}', expected 'C:\\PCB=/path/to/pcb'", mapping));
                    process::exit(1);
                };
                mappings.push((dos_path.to_string(), local_path.to_string()));
            }

            let output_directory = if *dry_run {
                std::env::temp_dir().join(format!("icbsetup-dry-run-{}", process::id()))
            } else {
                if out.exists() {
                    print_error(format!("Destination already exists: {}", out.display()));
                    process::exit(1);
                }
                PathBuf::from(out)
            };

            let output = Box::<ConsoleLogger>::default();
            match PCBoardImporter::new(name, output, output_directory.clone(), &mappings) {
                Ok(mut importer) => match importer.start_import() {
                    Ok(_) => {
                        if *dry_run {
                            let unresolved = importer.unresolved_paths();
                            println!("\nUnresolved paths ({}):", unresolved.len());
                            for path in unresolved {
                                println!("  {}", path);
                            }
                            let _ = fs::remove_dir_all(&output_directory);
                            println!("\nDry run - nothing was written.");
                            return Ok(());
                        }
                        // A board that doesn't load again is an import failure, no matter what got written.
                        let config = output_directory.join(icy_board_engine::DEFAULT_ICYBOARD_FILE);
                        match IcyBoard::load(&config) {
                            Ok(_) => println!("Imported successfully"),
                            Err(e) => {
                                print_error(format!("Imported board doesn't load: {}", e));
                                process::exit(1);
                            }
                        }
                    }
                    Err(e) => {
                        print_error(e.to_string());
                        let destination = importer.output_directory.join("importlog.txt");
                        fs::write(destination, &importer.logger.output)?;
                        if *dry_run {
                            let _ = fs::remove_dir_all(&output_directory);
                        }
                        process::exit(1);
                    }
                },
                Err(e) => {
                    print_error(e.to_string());
                    process::exit(1);
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
                print_error("Path does not exist".to_string());
                process::exit(1);
            }

            if path.is_file() {
                println!("Converting file to utf-8...");
                if let Err(err) = convert_file(path) {
                    print_error(err.to_string());
                    process::exit(1);
                }
                return Ok(());
            }
            if let Err(err) = convert_tree(path) {
                print_error(err.to_string());
                process::exit(1);
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
    let _board_lock = match BoardLock::acquire(file.parent().unwrap_or_else(|| Path::new("."))) {
        Ok(lock) => lock,
        Err(err) => {
            print_error(err);
            process::exit(1);
        }
    };
    match IcyBoard::load(&file) {
        Ok(icy_board) => {
            let terminal = &mut term::init()?;
            let icy_board = Arc::new(Mutex::new(icy_board));
            let mut app = new_main_window(icy_board.clone(), arguments.full_screen);
            app.run(terminal)?;

            if app.save
                && let Err(err) = icy_board.lock().unwrap().save()
            {
                term::restore()?;
                return Err(eyre!(err.to_string()));
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

fn convert_file(entry: &Path) -> Result<()> {
    let data = read_with_encoding_detection(&entry).map_err(|err| eyre!(err.to_string()))?;
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(data.as_bytes());
    write_atomic(entry, &bytes)?;
    Ok(())
}

fn convert_tree(root: &Path) -> Result<()> {
    const CONVERT_EXT: &[&str] = &["ANS", "PCB", "CFG", "DOC", "NFO", "ASC", "TXT", "PPX", "PPS", "PPD", "LST", "XXX"];
    let entries: Vec<_> = WalkDir::new(root).min_depth(1).into_iter().collect::<std::result::Result<_, _>>()?;

    println!("Converting files...");
    for entry in entries.iter().filter(|entry| entry.file_type().is_file()) {
        let path = entry.path();
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| CONVERT_EXT.contains(&ext.to_ascii_uppercase().as_str()))
        {
            println!("Converting {} to utf8...", path.display());
            convert_file(path)?;
        }
        rename_to_lowercase(path)?;
    }

    println!("Converting directories to lower case...");
    for entry in entries.iter().rev().filter(|entry| entry.file_type().is_dir()) {
        rename_to_lowercase(entry.path())?;
    }
    Ok(())
}

fn rename_to_lowercase(path: &Path) -> Result<()> {
    let Some(name) = path.file_name() else {
        return Ok(());
    };
    let lower = name.to_string_lossy().to_lowercase();
    if lower == name.to_string_lossy() {
        return Ok(());
    }
    let target = path.with_file_name(lower);
    if target.exists() {
        return Err(eyre!("Can't rename {}: {} already exists", path.display(), target.display()));
    }
    println!("Rename {} to {}", path.display(), target.display());
    fs::rename(path, target)?;
    Ok(())
}
