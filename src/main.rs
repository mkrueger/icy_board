use std::{
    collections::VecDeque,
    io::stdout,
    process,
    sync::{Arc, Mutex},
};

use bbs::IcyBoardCommand;
use call_wait_screen::{CallWaitMessage, CallWaitScreen};
use clap::Parser;
use crossterm::{terminal::Clear, ExecutableCommand};
use icy_board_engine::icy_board::{state::IcyBoardState, IcyBoard};
use icy_engine_output::{IcyEngineOutput, Screen};
use icy_ppe::Res;
use semver::Version;
use tui::{print_exit_screen, Tui};

use crate::call_wait_screen::restore_terminal;

pub mod bbs;
pub mod call_stat;
mod call_wait_screen;
mod icy_engine_output;
mod tui;

#[derive(clap::Parser)]
#[command(version="", about="IcyBoard BBS", long_about = None)]
struct Cli {
    /// PCBOARD.DAT file to run
    file: String,
}

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}

fn main() {
    let _ = init_error_hooks();
    let arguments = Cli::parse();

    match IcyBoard::load(&arguments.file) {
        Ok(icy_board) => {
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
            }
        }
        Err(e) => {
            restore_terminal().unwrap();
            println!("Error: {}", e);
        }
    }
}

fn run_message(msg: CallWaitMessage, board: Arc<Mutex<IcyBoard>>) {
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
            let cmd = IcyBoardCommand::new(state);

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
