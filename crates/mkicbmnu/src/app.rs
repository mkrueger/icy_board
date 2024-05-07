use std::sync::{Arc, Mutex};

use icy_board_engine::icy_board::menu::Menu;
use icy_board_tui::{
    app::{App, Mode},
    help_view::HelpViewState,
};

use crate::{AboutTab, CommandsTab, GeneralTab};

pub fn new_main_window<'a>(mnu: Arc<Mutex<Menu>>, full_screen: bool) -> App<'a> {
    let general_tab = GeneralTab::new(mnu.clone());
    let command_tab = CommandsTab::new(mnu.clone());
    App {
        full_screen,
        mode: Mode::default(),
        tab: 0,
        date_format: "%m/%d/%y".to_string(),
        tabs: vec![Box::new(general_tab), Box::new(command_tab), Box::new(AboutTab::default())],
        status_line: String::new(),
        help_state: HelpViewState::new(23),
        open_editor: None,
        get_editor: Box::new(|_id, _path| Ok(None)),
    }
}
