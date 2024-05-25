use std::sync::{Arc, Mutex};

use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    app::{App, Mode},
    help_view::HelpViewState,
};

use crate::tabs::{AboutTab, GroupsTab, UsersTab};

pub fn new_main_window<'a>(icy_board: Arc<Mutex<IcyBoard>>, full_screen: bool) -> App<'a> {
    let date_format = icy_board.lock().unwrap().config.board.date_format.clone();
    let users_tab = UsersTab::new(icy_board.clone());
    let groups_tab = GroupsTab::new(icy_board.clone());
    App {
        full_screen,
        title: format!(" IcyBoard System Manager"),
        mode: Mode::default(),
        tab: 0,
        date_format,
        tabs: vec![Box::new(users_tab), Box::new(groups_tab), Box::new(AboutTab::default())],
        status_line: String::new(),
        help_state: HelpViewState::new(23),
        open_editor: None,
        get_editor: Box::new(|_id, _path| Ok(None)),
        save: false,
    }
}
