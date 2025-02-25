use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use icy_board_engine::icy_board::{IcyBoard, menu::Menu};
use icy_board_tui::{
    app::{App, Mode},
    help_view::HelpViewState,
};

use crate::{AboutTab, CommandsTab, GeneralTab};

pub fn new_main_window<'a>(icy_board: IcyBoard, mnu: Arc<Mutex<Menu>>, full_screen: bool, path: &Path) -> App<'a> {
    let date_format = icy_board.config.board.date_format.clone();

    let icy_board = Arc::new(Mutex::new(icy_board));

    let general_tab = GeneralTab::new(mnu.clone());
    let command_tab = CommandsTab::new(icy_board, mnu.clone());
    App {
        full_screen,
        title: format!(" MNU File Editor ({})", path.display()),
        mode: Mode::default(),
        tab: 0,
        date_format,
        tabs: vec![Box::new(general_tab), Box::new(command_tab), Box::new(AboutTab::default())],
        status_line: String::new(),
        help_state: HelpViewState::new(23),
        save: false,
    }
}
