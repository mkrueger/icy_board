use app::new_main_window;
use argh::FromArgs;
use chrono::Local;
use color_eyre::{Result, eyre::eyre};
use create::IcyBoardCreator;
use icy_board_engine::icy_board::{
    IcyBoard,
    lock::BoardLock,
    path_check::{PathKind, PathProblem, PathReport},
    read_with_encoding_detection, write_atomic,
};
use icy_board_tui::{app::SaveChoice, print_error, term};
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

    /// print the version and exit
    #[argh(switch)]
    version: bool,

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
    Check(Check),
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

#[derive(FromArgs, PartialEq, Debug)]
/// Reports every path in the configuration that doesn't lead where it says
#[argh(subcommand, name = "check")]
struct Check {
    /// offer to create the directories that are missing
    #[argh(switch)]
    create_dirs: bool,

    /// path/file name of the icyboard.toml configuration file
    #[argh(positional)]
    file: Option<PathBuf>,
}

fn main() -> Result<()> {
    let arguments: Cli = argh::from_env();
    if arguments.version {
        println!("icbsetup {}", *VERSION);
        return Ok(());
    }

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
        Some(Commands::Check(Check { file, create_dirs })) => {
            let Some(config) = icy_board_engine::lookup_icyboard_file(file) else {
                print_error(icy_board_tui::get_text("error_file_or_path_not_found"));
                process::exit(1);
            };
            let board = match IcyBoard::load(&config) {
                Ok(board) => board,
                Err(err) => {
                    print_error(format!("Error loading main config file: {}", err));
                    process::exit(1);
                }
            };
            if report_paths(&board, *create_dirs) == 0 {
                return Ok(());
            }
            process::exit(1);
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
            term::restore()?;

            if app.save.writes() {
                if let Err(err) = icy_board.lock().unwrap().save() {
                    return Err(eyre!(err.to_string()));
                }
            }
            // PCBSetup left its editor for a plain screen to report on the paths. See writefile() in DATAWRIT.C.
            if app.save == SaveChoice::Save {
                println!("Checking directories while saving files...");
                if report_paths(&icy_board.lock().unwrap(), true) > 0 {
                    print!("press any key to continue...");
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                    let mut line = String::new();
                    let _ = std::io::BufRead::read_line(&mut std::io::stdin().lock(), &mut line);
                }
            }
            Ok(())
        }
        Err(err) => {
            print_error(format!("Error loading main config file: {}", err));
            exit(1);
        }
    }
}

/// Reports on the paths and, when asked, offers to make the missing
/// directories, the way PCBSetup did after a full save. See checkexistence()
/// in CHKEXIST.C. Answers how many paths need attention.
fn report_paths(board: &IcyBoard, offer_to_create: bool) -> usize {
    let reports = board.check_paths();
    if reports.is_empty() {
        println!("All paths lead where they say.");
        return 0;
    }

    let mut create_the_rest = false;
    for report in &reports {
        println!("{}", report);
        if !offer_to_create || !offers_to_create(report) {
            continue;
        }
        if !create_the_rest {
            match ask_to_create(&report.resolved, is_inside(&board.root_path, &report.resolved)) {
                Answer::No => continue,
                Answer::Stop => break,
                Answer::AllOfThem => create_the_rest = true,
                Answer::Yes => {}
            }
        }
        match fs::create_dir_all(&report.resolved) {
            Ok(()) => println!("  created {}", report.resolved.display()),
            Err(err) => println!("  {} could not be created: {}", report.resolved.display(), err),
        }
    }

    println!("\n{} path(s) need attention.", reports.len());
    reports.len()
}

fn offers_to_create(report: &PathReport) -> bool {
    report.kind == PathKind::Directory && report.problem == PathProblem::Missing
}

fn is_inside(root: &Path, path: &Path) -> bool {
    path.starts_with(root)
}

enum Answer {
    Yes,
    No,
    AllOfThem,
    Stop,
}

/// A path outside the board is not offered a default, because that is what a
/// mistyped absolute path looks like.
fn ask_to_create(path: &Path, inside_the_board: bool) -> Answer {
    let prompt = if inside_the_board {
        "  create it now (Y,n,a=all,q=stop asking)? "
    } else {
        "  this is outside the board - create it now (y,N,a=all,q=stop asking)? "
    };
    print!("{prompt}");
    let _ = std::io::Write::flush(&mut std::io::stdout());

    let mut answer = String::new();
    if std::io::BufRead::read_line(&mut std::io::stdin().lock(), &mut answer).is_err() {
        return Answer::Stop;
    }
    match answer.trim().to_ascii_lowercase().as_str() {
        "y" => Answer::Yes,
        "n" => Answer::No,
        "a" => Answer::AllOfThem,
        "q" => Answer::Stop,
        "" if inside_the_board => Answer::Yes,
        "" => Answer::No,
        _ => {
            println!("  {} left alone", path.display());
            Answer::No
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
