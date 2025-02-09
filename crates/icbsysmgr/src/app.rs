use std::sync::{Arc, Mutex};

use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    app::{App, Mode},
    help_view::HelpViewState,
};

use crate::tabs::{AboutTab, GeneralTab};

pub fn new_main_window<'a>(icy_board: Arc<Mutex<IcyBoard>>, full_screen: bool) -> App<'a> {
    let date_format = icy_board.lock().unwrap().config.board.date_format.clone();
    let general_tab = GeneralTab::new(icy_board.clone());
    App {
        full_screen,
        title: format!(" IcyBoard System Manager"),
        mode: Mode::default(),
        tab: 0,
        date_format,
        tabs: vec![Box::new(general_tab), Box::new(AboutTab::default())],
        status_line: String::new(),
        help_state: HelpViewState::new(23),
        save: false,
    }
}
