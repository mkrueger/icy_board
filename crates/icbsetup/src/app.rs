use std::sync::{Arc, Mutex};

use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    app::{App, Mode, SaveChoice},
    help_view::HelpViewState,
};

use crate::tabs::{AboutTab, GeneralTab};

pub fn new_main_window(icy_board: Arc<Mutex<IcyBoard>>, full_screen: bool) -> App {
    let general_tab = GeneralTab::new(icy_board.clone());
    let date_format = icy_board.lock().unwrap().config.board.date_format.clone();
    App {
        full_screen,
        title: format!(" {}", icy_board_tui::get_text("app_icbsetup")),
        mode: Mode::default(),
        tab: 0,
        date_format,
        tabs: vec![Box::new(general_tab), Box::new(AboutTab::default())],
        status_line: String::new(),
        help_state: HelpViewState::new(),
        save: SaveChoice::default(),
        offers_quick_save: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use icy_board_tui::get_text;

    #[test]
    fn window_title_and_tabs_use_the_active_locale() {
        let board = IcyBoard {
            file_name: "icyboard.toml".into(),
            ..IcyBoard::default()
        };
        let app = new_main_window(Arc::new(Mutex::new(board)), false);
        assert_eq!(app.title, format!(" {}", get_text("app_icbsetup")));
        assert_eq!(app.tabs[0].title(), get_text("tui_tab_main"));
        assert_eq!(app.tabs[1].title(), get_text("tui_tab_about"));
    }
}
