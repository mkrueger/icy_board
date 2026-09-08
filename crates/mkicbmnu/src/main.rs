use app::new_main_window;
use chrono::Local;
use clap::Parser;
use color_eyre::Result;
use icy_board_engine::{
    DEFAULT_ICYBOARD_FILE,
    icy_board::{IcyBoard, IcyBoardSerializer, menu::Menu},
};
use icy_board_tui::{print_error, term};
use semver::Version;
use std::{
    path::PathBuf,
    process::exit,
    sync::{Arc, Mutex},
};

mod app;
mod document;
mod validation;

mod tabs;
pub use tabs::*;

pub mod edit_command_dialog;

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}

/// Set by build.rs, empty when the binary was not built from a checkout.
const GIT_HASH: &str = env!("GIT_HASH");

#[derive(Parser)]
#[command(name = "mkicbmnu", disable_version_flag = true, about = icy_board_cli::text("mkicbmnu", "about"))]
struct Cli {
    #[arg(long = "create", short = 'c', help = icy_board_cli::text("mkicbmnu", "create"))]
    create: bool,

    #[arg(long = "board", short = 'b', value_name = "DIRECTORY_OR_CONFIG", help = icy_board_cli::text("mkicbmnu", "board"))]
    board: Option<PathBuf>,

    #[arg(long = "check", conflicts_with = "create", help = icy_board_cli::text("mkicbmnu", "check"))]
    check: bool,

    #[arg(long = "full-screen", short = 'f', help = icy_board_cli::text("mkicbmnu", "full-screen"))]
    full_screen: bool,

    #[arg(long = "version", help = icy_board_cli::text("mkicbmnu", "version"))]
    version: bool,

    #[arg(help = icy_board_cli::text("mkicbmnu", "file"))]
    file: Option<PathBuf>,
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    #[test]
    fn cli_defaults_and_options() {
        let cli = icy_board_cli::try_parse_from::<Cli, _, _>(["mkicbmnu"]).unwrap();
        assert!(!cli.create && !cli.full_screen && !cli.version && cli.file.is_none());
        let cli = icy_board_cli::try_parse_from::<Cli, _, _>(["mkicbmnu", "-c", "-f", "--version", "main.mnu"]).unwrap();
        assert!(cli.create && cli.full_screen && cli.version);
        assert_eq!(cli.file, Some(PathBuf::from("main.mnu")));
        assert!(icy_board_cli::try_parse_from::<Cli, _, _>(["mkicbmnu", "--create=true"]).is_err());
    }
}

fn main() -> Result<()> {
    let arguments = icy_board_cli::parse::<Cli>();
    if arguments.version {
        println!("{}", icy_board_cli::version_line("mkicbmnu", &*VERSION, GIT_HASH));
        return Ok(());
    }

    let Some(menu_file) = arguments.file.clone() else {
        eprintln!("{}", icy_board_cli::command::<Cli>().render_help());
        exit(1);
    };

    let file = menu_file.with_extension("mnu");
    if arguments.create && file.try_exists()? {
        print_error(format!("{}: {}", icy_board_tui::get_text("mnu_app_exists"), file.display()));
        exit(1);
    }
    if !file.exists() && !arguments.create {
        icy_board_tui::print_input_file_not_found("mkicbmnu", &file);
        exit(1);
    }

    let board_file = arguments
        .board
        .as_ref()
        .map(|path| if path.is_dir() { path.join(DEFAULT_ICYBOARD_FILE) } else { path.clone() })
        .or_else(|| find_icy_board(file.parent()));
    let Some(board_file) = board_file else {
        icy_board_tui::print_parent_board_config_not_found("mkicbmnu", &file);
        exit(1);
    };
    // Canonicalize the config so all assisted paths use one absolute BBS root,
    // independently of the editor's working directory or the menu location.
    let icy_board = match board_file
        .canonicalize()
        .map_err(|e| e.to_string())
        .and_then(|path| IcyBoard::load(&path).map_err(|e| e.to_string()))
    {
        Ok(icy_board) => icy_board,
        Err(err) => {
            print_error(format!("Can't load {}: {err}", board_file.display()));
            exit(1);
        }
    };

    let mnu = if arguments.create {
        Menu::default()
    } else {
        Menu::load(&file).map_err(|err| color_eyre::eyre::eyre!(err.to_string()))?
    };
    if arguments.check {
        let issues = validation::validate_menu(&icy_board, &mnu);
        for issue in &issues {
            println!(
                "{} {}: {}",
                issue.severity.label(),
                issue
                    .command
                    .map_or_else(|| icy_board_tui::get_text("mnu_check_menu"), |i| format!("#{}", i + 1)),
                issue.message
            );
        }
        if issues.is_empty() {
            println!("{}", icy_board_tui::get_text("mnu_check_ok"));
        }
        if issues.iter().any(|issue| issue.severity == validation::IssueSeverity::Error) {
            exit(1);
        }
        return Ok(());
    }

    icy_board_tui::theme::set_admin_theme(&icy_board.config.sysop.config_color_theme, &icy_board.config.sysop.config_color_configuration);

    let log_file = icy_board.file_name.with_extension("log");
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
        .chain(match fern::log_file(&log_file) {
            Ok(log) => log,
            Err(err) => {
                print_error(format!("Can't open log file {}: {err}", log_file.display()));
                exit(1);
            }
        })
        // Apply globally
        .apply()
        .unwrap_or_else(|err| {
            print_error(format!("Can't initialize logging: {err}"));
            exit(1);
        });

    let file = if file.exists() { file.canonicalize()? } else { std::path::absolute(file)? };
    let mut app = new_main_window(icy_board, Arc::new(Mutex::new(mnu)), arguments.full_screen, &file, arguments.create)?;
    let terminal = &mut term::init()?;
    let result = app.run(terminal);
    let restored = term::restore();
    result?;
    restored?;
    Ok(())
}

fn find_icy_board(parent: Option<&std::path::Path>) -> Option<PathBuf> {
    let directory = parent.filter(|p| !p.as_os_str().is_empty()).unwrap_or(std::path::Path::new("."));
    std::path::absolute(directory)
        .ok()?
        .ancestors()
        .map(|dir| dir.join(DEFAULT_ICYBOARD_FILE))
        .find(|path| path.is_file())
}
