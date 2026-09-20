// See icy_board_tui's crate-level allow for the rationale (ratatui config-menu
// callback/closure types are inherently complex; not worth type-aliasing).
#![allow(clippy::type_complexity)]
use app::new_main_window;
use chrono::Local;
use clap::{Args, Parser, Subcommand};
use color_eyre::{Result, eyre::eyre};
use create::IcyBoardCreator;
use icy_board_engine::icy_board::{
    IcyBoard,
    lock::BoardLock,
    path_check::{PathKind, PathProblem, PathReport},
    read_with_encoding_detection, write_atomic,
};
use icy_board_tui::{app::SaveChoice, print_error, term, theme::set_admin_theme};
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
mod dos_console;
pub mod editors;
pub mod genhelp;
mod import;
pub mod tabs;

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}

/// Set by build.rs, empty when the binary was not built from a checkout.
const GIT_HASH: &str = env!("GIT_HASH");

#[derive(Parser)]
#[command(name = "icbsetup", disable_version_flag = true, subcommand_precedence_over_arg = true, about = icy_board_cli::text("icbsetup", "about"))]
struct Cli {
    #[arg(long = "full-screen", short = 'f', help = icy_board_cli::text("icbsetup", "full-screen"))]
    full_screen: bool,

    #[arg(long = "version", help = icy_board_cli::text("icbsetup", "version"))]
    version: bool,

    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(help = icy_board_cli::text("icbsetup", "file"))]
    file: Option<PathBuf>,
}

#[derive(Subcommand, PartialEq, Debug)]
enum Commands {
    #[command(name = "genhelp", about = icy_board_cli::text("icbsetup", "genhelp-about"))]
    GenHelp(genhelp::GenHelp),
    #[command(name = "import", about = icy_board_cli::text("icbsetup", "import-about"))]
    Import(Import),
    #[command(name = "create", about = icy_board_cli::text("icbsetup", "create-about"))]
    Create(Create),
    #[command(name = "ppe-convert", about = icy_board_cli::text("icbsetup", "ppe-convert-about"))]
    PPEConvert(PPEConvert),
    #[command(name = "check", about = icy_board_cli::text("icbsetup", "check-about"))]
    Check(Check),
    #[command(name = "dos-image", about = icy_board_cli::text("icbsetup", "dos-image-about"))]
    DosImage(DosImage),
    #[command(name = "dos-copy", about = icy_board_cli::text("icbsetup", "dos-copy-about"))]
    DosCopy(DosCopy),
    #[command(name = "dos-fossil", about = icy_board_cli::text("icbsetup", "dos-fossil-about"))]
    DosFossil(DosFossil),
    #[command(name = "dos-console", about = icy_board_cli::text("icbsetup", "dos-console-about"))]
    DosConsole(dos_console::DosConsole),
}

#[derive(Args, PartialEq, Debug)]
struct Import {
    #[arg(help = icy_board_cli::text("icbsetup", "import-name"))]
    name: PathBuf,

    #[arg(help = icy_board_cli::text("icbsetup", "output-directory"))]
    out: PathBuf,

    #[arg(long = "map", help = icy_board_cli::text("icbsetup", "map"))]
    map: Vec<String>,

    #[arg(long = "dry-run", help = icy_board_cli::text("icbsetup", "dry-run"))]
    dry_run: bool,
}

#[derive(Args, PartialEq, Debug)]
struct Create {
    #[arg(help = icy_board_cli::text("icbsetup", "output-directory"))]
    file: PathBuf,
}

#[derive(Args, PartialEq, Debug)]
struct PPEConvert {
    #[arg(help = icy_board_cli::text("icbsetup", "ppe-convert-path"))]
    path: PathBuf,
}

#[derive(Args, PartialEq, Debug)]
struct Check {
    #[arg(long = "create-dirs", help = icy_board_cli::text("icbsetup", "create-dirs"))]
    create_dirs: bool,

    #[arg(help = icy_board_cli::text("icbsetup", "file"))]
    file: Option<PathBuf>,
}

#[derive(Args, PartialEq, Debug)]
struct DosImage {
    #[arg(help = icy_board_cli::text("icbsetup", "dos-image-directory"))]
    directory: PathBuf,
}

#[derive(Args, PartialEq, Debug)]
struct DosCopy {
    #[arg(help = icy_board_cli::text("icbsetup", "dos-copy-image"))]
    image: PathBuf,
    #[arg(help = icy_board_cli::text("icbsetup", "dos-copy-source"))]
    source: PathBuf,
    #[arg(help = icy_board_cli::text("icbsetup", "dos-copy-destination"))]
    destination: String,
}

#[derive(Args, PartialEq, Debug)]
struct DosFossil {
    #[arg(help = icy_board_cli::text("icbsetup", "dos-image-directory"))]
    directory: PathBuf,
    #[arg(help = icy_board_cli::text("icbsetup", "dos-fossil-archive"))]
    archive: PathBuf,
    #[arg(long, help = icy_board_cli::text("icbsetup", "dos-fossil-door"))]
    door: Option<String>,
    #[arg(long, required = true, help = icy_board_cli::text("icbsetup", "dos-fossil-license"))]
    accept_license: bool,
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    fn parse(args: &[&str]) -> Cli {
        icy_board_cli::try_parse_from::<Cli, _, _>(std::iter::once("icbsetup").chain(args.iter().copied())).unwrap()
    }

    #[test]
    fn cli_defaults_and_positional_file() {
        let cli = parse(&[]);
        assert!(!cli.full_screen && !cli.version);
        assert!(cli.file.is_none() && cli.command.is_none());
        let cli = parse(&["-f", "--version", "board.toml"]);
        assert!(cli.full_screen && cli.version && cli.command.is_none());
        assert_eq!(cli.file, Some(PathBuf::from("board.toml")));
        assert_eq!(parse(&["--", "check"]).file, Some(PathBuf::from("check")));
    }

    #[test]
    fn cli_subcommands_take_precedence_over_the_optional_file() {
        let cli = parse(&["check"]);
        assert!(cli.file.is_none());
        assert_eq!(
            cli.command,
            Some(Commands::Check(Check {
                create_dirs: false,
                file: None
            }))
        );
        assert_eq!(
            parse(&["check", "--create-dirs", "board.toml"]).command,
            Some(Commands::Check(Check {
                create_dirs: true,
                file: Some("board.toml".into())
            }))
        );
        assert_eq!(parse(&["create", "board"]).command, Some(Commands::Create(Create { file: "board".into() })));
        assert_eq!(
            parse(&["ppe-convert", "scripts"]).command,
            Some(Commands::PPEConvert(PPEConvert { path: "scripts".into() }))
        );
        assert_eq!(
            parse(&["dos-image", "board"]).command,
            Some(Commands::DosImage(DosImage { directory: "board".into() }))
        );
        assert_eq!(
            parse(&["dos-copy", "disk.img", "host.exe", "DOORS/HOST.EXE"]).command,
            Some(Commands::DosCopy(DosCopy {
                image: "disk.img".into(),
                source: "host.exe".into(),
                destination: "DOORS/HOST.EXE".into()
            }))
        );
        assert!(icy_board_cli::try_parse_from::<Cli, _, _>(["icbsetup", "ppe-convert"]).is_err());
    }

    #[test]
    fn cli_dos_fossil_requires_license_acknowledgement() {
        assert!(icy_board_cli::try_parse_from::<Cli, _, _>(["icbsetup", "dos-fossil", "board", "x00.zip"]).is_err());
        assert_eq!(
            parse(&["dos-fossil", "board", "x00.zip", "--accept-license"]).command,
            Some(Commands::DosFossil(DosFossil {
                directory: "board".into(),
                archive: "x00.zip".into(),
                door: None,
                accept_license: true,
            }))
        );
        assert_eq!(
            parse(&["dos-fossil", "board", "x00.zip", "--door", "LORD", "--accept-license"]).command,
            Some(Commands::DosFossil(DosFossil {
                directory: "board".into(),
                archive: "x00.zip".into(),
                door: Some("LORD".into()),
                accept_license: true,
            }))
        );
    }

    #[test]
    fn cli_import_defaults_and_repeated_maps() {
        assert_eq!(
            parse(&["import", "PCBOARD.DAT", "board"]).command,
            Some(Commands::Import(Import {
                name: "PCBOARD.DAT".into(),
                out: "board".into(),
                map: vec![],
                dry_run: false
            }))
        );
        assert_eq!(
            parse(&["import", "PCBOARD.DAT", "board", "--map", "C:=/pcb", "--map", "D:=/files", "--dry-run"]).command,
            Some(Commands::Import(Import {
                name: "PCBOARD.DAT".into(),
                out: "board".into(),
                map: vec!["C:=/pcb".into(), "D:=/files".into()],
                dry_run: true
            }))
        );
    }
}

fn main() -> Result<()> {
    let arguments = icy_board_cli::parse::<Cli>();
    if arguments.version {
        println!("{}", icy_board_cli::version_line("icbsetup", &*VERSION, GIT_HASH));
        return Ok(());
    }

    match &arguments.command {
        Some(Commands::GenHelp(command)) => {
            if let Err(error) = genhelp::run(command) {
                print_error(error.to_string());
                process::exit(1);
            }
            return Ok(());
        }
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
            let config = match icy_board_engine::resolve_icyboard_file(file) {
                Ok(config) => config,
                Err(icy_board_engine::IcyBoardFileLookupError::FileNotFound(path)) => {
                    icy_board_tui::print_board_config_not_found("icbsetup check", &path);
                    process::exit(1);
                }
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
        Some(Commands::DosImage(DosImage { directory })) => {
            prepare_dos_image(directory)?;
            return Ok(());
        }
        Some(Commands::DosFossil(command)) => {
            install_dos_fossil(command)?;
            return Ok(());
        }
        Some(Commands::DosConsole(command)) => {
            dos_console::run(command)?;
            return Ok(());
        }
        Some(Commands::DosCopy(DosCopy { image, source, destination })) => {
            icy_board_engine::icy_board::doors::dos::copy_file_into_image(image, source, destination).map_err(|error| eyre!(error.to_string()))?;
            println!("Copied {} to {} in {}", source.display(), destination, image.display());
            return Ok(());
        }
        _ => {}
    }
    let file = match icy_board_engine::resolve_icyboard_file(&arguments.file) {
        Ok(file) => file,
        Err(icy_board_engine::IcyBoardFileLookupError::FileNotFound(path)) => {
            icy_board_tui::print_board_config_not_found("icbsetup", &path);
            exit(1);
        }
    };
    init_log(&file.parent().unwrap().join("icbsetup.log"))?;
    let _board_lock = match BoardLock::acquire(file.parent().unwrap_or_else(|| Path::new("."))) {
        Ok(lock) => lock,
        Err(err) => {
            print_error(err);
            process::exit(1);
        }
    };
    match IcyBoard::load(&file) {
        Ok(icy_board) => {
            set_admin_theme(&icy_board.config.sysop.config_color_theme, &icy_board.config.sysop.config_color_configuration);
            let terminal = &mut term::init()?;
            let icy_board = Arc::new(Mutex::new(icy_board));
            let mut app = new_main_window(icy_board.clone(), arguments.full_screen);
            app.run(terminal)?;
            term::restore()?;

            if app.save.writes()
                && let Err(err) = icy_board.lock().unwrap().save()
            {
                return Err(eyre!(err.to_string()));
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

fn init_log(path: &Path) -> Result<()> {
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
        .chain(fern::log_file(path).map_err(|err| eyre!("Can't open log file {}: {err}", path.display()))?)
        // Apply globally
        .apply()
        .map_err(|err| eyre!("Can't initialize logging: {err}"))?;
    Ok(())
}

fn prepare_dos_image(board_directory: &Path) -> Result<()> {
    let destination = board_directory.join("assets/dos");
    println!("{}: {}", icy_board_cli::text("icbsetup", "dos-preparing"), destination.display());
    icy_board_engine::icy_board::doors::dos::prepare_dos_assets(&destination).map_err(|error| eyre!(error.to_string()))?;
    println!("{}: {}", icy_board_cli::text("icbsetup", "dos-ready"), destination.display());
    Ok(())
}

fn install_dos_fossil(command: &DosFossil) -> Result<()> {
    use icy_board_engine::icy_board::doors::dos;

    let _board_lock = BoardLock::acquire(&command.directory).map_err(|error| eyre!(error.to_string()))?;
    let assets = command.directory.join("assets/dos");
    let image = if let Some(door) = &command.door {
        let image = assets.join("doors").join(dos::image_file_name(door));
        if !image.is_file() {
            return Err(eyre!("{}: {}", icy_board_cli::text("icbsetup", "dos-fossil-missing-door"), image.display()));
        }
        image
    } else {
        prepare_dos_image(&command.directory)?;
        assets.join("freedos.img")
    };
    let backup = dos::install_x00_fossil(&image, &command.archive).map_err(|error| eyre!(error.to_string()))?;
    println!("{}: {}", icy_board_cli::text("icbsetup", "dos-fossil-installed"), image.display());
    println!("{}: {}", icy_board_cli::text("icbsetup", "dos-fossil-backup"), backup.display());
    Ok(())
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
