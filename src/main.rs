use std::{
    collections::VecDeque,
    fs,
    io::stdout,
    path::PathBuf,
    process,
    sync::{Arc, Mutex},
};

use bbs::PcbBoardCommand;
use call_wait_screen::CallWaitMessage;
use clap::Parser;
use crossterm::{
    execute,
    style::{Attribute, Color, Print, SetAttribute, SetForegroundColor},
    terminal::Clear,
    ExecutableCommand,
};
use icy_board_engine::icy_board::{
    icb_config::IcbConfig, state::IcyBoardState, user_base::UserBase, PcbBoard,
};
use icy_engine_output::{IcyEngineOutput, Screen};
use icy_ppe::Res;
use relative_path::{PathExt, RelativePath, RelativePathBuf};
use semver::Version;
use tui::{print_exit_screen, Tui};
use walkdir::WalkDir;

use crate::call_wait_screen::restore_terminal;

pub mod bbs;
pub mod call_stat;
mod call_wait_screen;
mod icy_engine_output;
mod tui;

#[derive(clap::Parser)]
#[command(version="", about="PcbBoard BBS", long_about = None)]
struct Cli {
    /// PCBOARD.DAT file to run
    file: String,
}

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}
/// evlevlelvelvelv`

fn convert_pcb(pcb_file: &PcbBoard, output_directory: &PathBuf) -> Res<()> {
    start_action(format!(
        "Creating directory '{}'…",
        output_directory.display()
    ));
    check_result(fs::create_dir(output_directory));

    let o = output_directory.join("icbtext.toml");
    start_action(format!("Create ICBTEXT {}…", o.display()));
    check_result(pcb_file.display_text.save(&o));

    let o = output_directory.join("user_base.toml");
    start_action(format!("Create user base {}…", o.display()));
    let user_base = UserBase::import_pcboard(&pcb_file.users);
    check_result(user_base.save(&o));

    let o = output_directory.join("icbcfg.toml");
    start_action(format!("Create main configutation {}…", o.display()));
    let icb_cfg = IcbConfig::import_pcboard(&pcb_file.data);
    check_result(icb_cfg.save(&o));

    let help_loc = pcb_file.resolve_file(&pcb_file.data.path.help_loc);
    let help_loc = PathBuf::from(&help_loc);

    let o = output_directory.join("help");

    if help_loc.exists() {
        start_action(format!(
            "Copy help files from {} to {}…",
            help_loc.display(),
            o.display()
        ));
        println!();
        for entry in WalkDir::new(&help_loc) {
            let entry = entry.unwrap();
            if entry.path().is_dir() {
                continue;
            }
            let rel_path = entry.path().relative_to(&help_loc).unwrap();
            let lower_case = RelativePathBuf::from_path(rel_path.as_str().to_lowercase()).unwrap();
            let to = lower_case.to_logical_path(&o);
            if let Some(parent_dir) = to.parent() {
                if !parent_dir.exists() {
                    fs::create_dir(parent_dir).unwrap();
                }
            }
            start_action(format!("\t{} -> {}…", entry.path().display(), to.display()));
            check_result(fs::copy(entry.path(), to));
        }
    }

    Ok(())
}

fn start_action(format: String) {
    execute!(
        stdout(),
        SetAttribute(Attribute::Bold),
        Print(format),
        SetAttribute(Attribute::Reset)
    )
    .unwrap();
}

fn check_result<S, T: std::fmt::Display>(res: Result<S, T>) {
    match res {
        Ok(_) => {
            execute!(
                stdout(),
                SetAttribute(Attribute::Bold),
                SetForegroundColor(Color::Green),
                Print(" OK".to_string()),
                SetAttribute(Attribute::Reset),
                Print("\n")
            )
            .unwrap();
        }
        Err(e) => {
            execute!(
                stdout(),
                SetAttribute(Attribute::Bold),
                SetForegroundColor(Color::Red),
                Print(" Error:".to_string()),
                SetAttribute(Attribute::Reset),
                Print(format!(" {}\n", e))
            )
            .unwrap();
            process::exit(1);
        }
    }
}

fn main() {
    let _ = init_error_hooks();
    let arguments = Cli::parse();
    let output_directory = "icb";
    match PcbBoard::load(&arguments.file) {
        Ok(icy_board) => {
            convert_pcb(&icy_board, &PathBuf::from(output_directory)).unwrap();

            /*
            let board = Arc::new(Mutex::new(icy_board));
            loop {
                let mut app = CallWaitScreen::new(board.clone()).unwrap();
                match app.run() {
                    Ok(msg) => {
                        run_message(msg, board.clone());
                    }
                    Err(err) => {
                        restore_terminal().unwrap();
                        println!("Error: {}", err);
                    }
                }
            }*/
        }
        Err(e) => {
            restore_terminal().unwrap();
            println!("Error: {}", e);
        }
    }
}

fn run_message(msg: CallWaitMessage, board: Arc<Mutex<PcbBoard>>) {
    match msg {
        CallWaitMessage::User(_) | CallWaitMessage::Sysop(_) => {
            stdout()
                .execute(Clear(crossterm::terminal::ClearType::All))
                .unwrap();

            let screen = Arc::new(Mutex::new(Screen::new()));
            let input_buffer = Arc::new(Mutex::new(VecDeque::new()));
            let io = Arc::new(Mutex::new(IcyEngineOutput::new(
                screen.clone(),
                input_buffer.clone(),
            )));

            let mut state = IcyBoardState::new(board, io);
            state.session.is_sysop = true;
            state.set_current_user(0);
            let cmd = PcbBoardCommand::new(state);

            let mut tui = Tui::new(cmd, screen, input_buffer);
            if let Err(err) = tui.run() {
                restore_terminal().unwrap();
                println!("Error: {}", err);
                process::exit(1);
            }
        }
        CallWaitMessage::Exit(_) => {
            restore_terminal().unwrap();
            print_exit_screen();
            process::exit(0);
        }
    }
}

fn init_error_hooks() -> Res<()> {
    //let (panic, error) = HookBuilder::default().into_hooks();
    //let panic = panic.into_panic_hook();
    //let error = error.into_eyre_hook();
    /*color_eyre::eyre::set_hook(Box::new(move |e| {
        let _ = restore_terminal();
        error(e)
    }))?; */
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        eprintln!("{}", info);
    }));
    Ok(())
}
