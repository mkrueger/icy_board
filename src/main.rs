use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use call_wait_screen::{CallWaitScreen, IcyBoardCommand};
use clap::Parser;
use icy_board_engine::icy_board::{state::IcyBoardState, IcyBoard};
use icy_engine_output::{IcyEngineOutput, Screen};
use semver::Version;
use tui::Tui;

use crate::call_wait_screen::restore_terminal;

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
    let arguments = Cli::parse();

    match IcyBoard::load(&arguments.file) {
        Ok(icy_board) => {
            let board = Arc::new(Mutex::new(icy_board));
            let mut app = CallWaitScreen::new(board.clone()).unwrap();
            if let Err(err) = app.run() {
                println!("Error: {}", err);
            }
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
            }
        }
        Err(e) => {
            restore_terminal().unwrap();
            println!("Error: {}", e);
        }
    }
}
