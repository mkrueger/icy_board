use app::new_main_window;
use argh::FromArgs;
use chrono::{Local, Utc};
use color_eyre::Result;
use icy_board_engine::icy_board::{
    IcyBoard,
    lock::BoardLock,
    user_maintenance::{self, UserSelection},
};
use icy_board_tui::{print_error, term};
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

/// IcyBoard System Manager
#[derive(FromArgs)]
struct Cli {
    /// default is 80x25
    #[argh(switch, short = 'f')]
    full_screen: bool,

    /// remove users instead of starting the editor, see the criteria below
    #[argh(switch)]
    pack: bool,

    /// pack users that have not called in that many days
    #[argh(option)]
    inactive_days: Option<u32>,

    /// pack users that never logged on
    #[argh(switch)]
    never_logged_on: bool,

    /// do not pack users marked for deletion
    #[argh(switch)]
    no_delete_flagged: bool,

    /// keep users at or above that security level
    #[argh(option)]
    keep_security: Option<u8>,

    /// pack users that are locked out instead of keeping them
    #[argh(switch)]
    pack_locked_out: bool,

    /// rewrite all phone numbers in one format
    #[argh(switch)]
    standardize_phones: bool,

    /// put the user file back the way it was before the last run
    #[argh(switch)]
    undo: bool,

    /// report what would happen and write nothing
    #[argh(switch)]
    dry_run: bool,

    #[argh(positional)]
    /// path/file name of the icyboard.toml configuration file
    file: Option<PathBuf>,
}

impl Cli {
    fn is_batch(&self) -> bool {
        self.pack || self.standardize_phones || self.undo
    }
}

fn main() -> Result<()> {
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
        .chain(fern::log_file("icbsm.log").unwrap())
        // Apply globally
        .apply()
        .unwrap();

    let arguments: Cli = argh::from_env();

    let Some(file) = icy_board_engine::lookup_icyboard_file(&arguments.file) else {
        print_error(icy_board_tui::get_text("error_file_or_path_not_found"));
        exit(1);
    };

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
        keep_security_at_least: arguments.keep_security,
        keep_locked_out: !arguments.pack_locked_out,
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
