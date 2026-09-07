use app::new_main_window;
use chrono::{Local, Utc};
use clap::Parser;
use color_eyre::Result;
use icy_board_engine::icy_board::{
    IcyBoard,
    lock::BoardLock,
    user_maintenance::{self, UserSelection},
};
use icy_board_tui::{print_error, term, theme::set_admin_theme};
use semver::Version;
use std::{
    path::PathBuf,
    process::exit,
    sync::{Arc, Mutex},
};

pub mod app;
pub mod tabs;

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}

#[derive(Parser)]
#[command(name = "icbsm", disable_version_flag = true, about = icy_board_cli::text("icbsm", "about"))]
struct Cli {
    #[arg(long = "full-screen", short = 'f', help = icy_board_cli::text("icbsm", "full-screen"))]
    full_screen: bool,

    #[arg(long = "pack", help = icy_board_cli::text("icbsm", "pack"))]
    pack: bool,

    #[arg(long = "inactive-days", help = icy_board_cli::text("icbsm", "inactive-days"))]
    inactive_days: Option<u32>,

    #[arg(long = "never-logged-on", help = icy_board_cli::text("icbsm", "never-logged-on"))]
    never_logged_on: bool,

    #[arg(long = "no-delete-flagged", help = icy_board_cli::text("icbsm", "no-delete-flagged"))]
    no_delete_flagged: bool,

    #[arg(long = "keep-security", help = icy_board_cli::text("icbsm", "keep-security"))]
    keep_security: Option<u8>,

    #[arg(long = "pack-locked-out", help = icy_board_cli::text("icbsm", "pack-locked-out"))]
    pack_locked_out: bool,

    #[arg(long = "standardize-phones", help = icy_board_cli::text("icbsm", "standardize-phones"))]
    standardize_phones: bool,

    #[arg(long = "undo", help = icy_board_cli::text("icbsm", "undo"))]
    undo: bool,

    #[arg(long = "dry-run", help = icy_board_cli::text("icbsm", "dry-run"))]
    dry_run: bool,

    #[arg(long = "version", help = icy_board_cli::text("icbsm", "version"))]
    version: bool,

    #[arg(help = icy_board_cli::text("icbsm", "file"))]
    file: Option<PathBuf>,
}

impl Cli {
    fn is_batch(&self) -> bool {
        self.pack || self.standardize_phones || self.undo
    }
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    #[test]
    fn cli_defaults_and_options() {
        let cli = icy_board_cli::try_parse_from::<Cli, _, _>(["icbsm"]).unwrap();
        assert!(!cli.is_batch() && !cli.full_screen && !cli.version && !cli.dry_run);
        assert!(!cli.never_logged_on && !cli.no_delete_flagged && !cli.pack_locked_out);
        assert!(cli.inactive_days.is_none() && cli.keep_security.is_none() && cli.file.is_none());
        let cli = icy_board_cli::try_parse_from::<Cli, _, _>([
            "icbsm",
            "-f",
            "--pack",
            "--inactive-days",
            "30",
            "--never-logged-on",
            "--no-delete-flagged",
            "--keep-security",
            "100",
            "--pack-locked-out",
            "--standardize-phones",
            "--undo",
            "--dry-run",
            "--version",
            "board.toml",
        ])
        .unwrap();
        assert!(cli.full_screen && cli.pack && cli.never_logged_on && cli.no_delete_flagged && cli.pack_locked_out);
        assert!(cli.standardize_phones && cli.undo && cli.dry_run && cli.version && cli.is_batch());
        assert_eq!(cli.inactive_days, Some(30));
        assert_eq!(cli.keep_security, Some(100));
        assert_eq!(cli.file, Some(PathBuf::from("board.toml")));
        assert!(icy_board_cli::try_parse_from::<Cli, _, _>(["icbsm", "--pack=true"]).is_err());
    }
}

fn main() -> Result<()> {
    let arguments = icy_board_cli::parse::<Cli>();
    if arguments.version {
        println!("icbsm {}", *VERSION);
        return Ok(());
    }

    let file = match icy_board_engine::resolve_icyboard_file(&arguments.file) {
        Ok(file) => file,
        Err(icy_board_engine::IcyBoardFileLookupError::FileNotFound(path)) => {
            icy_board_tui::print_board_config_not_found("icbsm", &path);
            exit(1);
        }
    };

    // The log belongs to the board, not to wherever the tool was started from.
    let log_file = file.with_file_name("icbsm.log");
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
        .chain(fern::log_file(&log_file)?)
        // Apply globally
        .apply()
        .unwrap();

    match IcyBoard::load(&file) {
        Ok(mut icy_board) => {
            // No two tools may rewrite the same board data at the same time.
            let _lock = match BoardLock::acquire(&icy_board.root_path) {
                Ok(lock) => lock,
                Err(err) => {
                    print_error(format!("{}", err));
                    exit(1);
                }
            };

            if arguments.is_batch() {
                match run_batch(&arguments, &mut icy_board) {
                    Ok(()) => return Ok(()),
                    Err(err) => {
                        print_error(format!("{}", err));
                        exit(1);
                    }
                }
            }

            set_admin_theme(&icy_board.config.sysop.config_color_theme, &icy_board.config.sysop.config_color_configuration);
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

/// Runs one maintenance operation without a screen, for cron jobs and events.
fn run_batch(arguments: &Cli, icy_board: &mut IcyBoard) -> icy_board_engine::Res<()> {
    let users_file = icy_board.resolve_file(&icy_board.config.paths.user_file);

    if arguments.undo {
        if !user_maintenance::has_backup(&users_file) {
            println!("There is no backup to restore.");
            return Ok(());
        }
        if arguments.dry_run {
            println!("Would restore {}", user_maintenance::backup_path(&users_file).display());
            return Ok(());
        }
        user_maintenance::restore_backup(&users_file)?;
        println!("Restored {}", users_file.display());
        return Ok(());
    }

    let selection = UserSelection {
        inactive_days: arguments.inactive_days,
        never_logged_on: arguments.never_logged_on,
        delete_flagged: arguments.pack && !arguments.no_delete_flagged,
        disabled: arguments.pack,
        locked_out: arguments.pack && arguments.pack_locked_out,
        keep_security_at_least: if arguments.pack {
            arguments.keep_security.or(Some(100))
        } else {
            arguments.keep_security
        },
        keep_locked_out: !arguments.pack_locked_out,
        protect_first_record: arguments.pack,
        ..Default::default()
    };

    if arguments.dry_run {
        let selected = selection.select(&icy_board.users, Utc::now());
        println!("{} user(s) would be affected:", selected.len());
        for index in selected {
            println!("  {}", icy_board.users[index].get_name());
        }
        return Ok(());
    }

    user_maintenance::create_backup(&users_file)?;
    let report = if arguments.pack {
        user_maintenance::pack(&mut icy_board.users, &selection, Utc::now())
    } else {
        user_maintenance::standardize_phones(&mut icy_board.users, &selection, Utc::now())
    };
    icy_board.save_userbase()?;

    println!("{} of {} user(s) changed.", report.changed, report.matched);
    for name in &report.names {
        println!("  {}", name);
    }
    Ok(())
}
