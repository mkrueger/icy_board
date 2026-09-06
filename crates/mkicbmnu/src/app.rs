use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use icy_board_engine::icy_board::{IcyBoard, menu::Menu};
use icy_board_tui::{
    app::{App, Mode, SaveChoice},
    help_view::HelpViewState,
};

use crate::{AboutTab, CommandsTab, GeneralTab};

pub fn new_main_window(icy_board: IcyBoard, mnu: Arc<Mutex<Menu>>, full_screen: bool, path: &Path) -> App {
    let date_format = icy_board.config.board.date_format.clone();

    let icy_board = Arc::new(Mutex::new(icy_board));

    let general_tab = GeneralTab::new(mnu.clone());
    let command_tab = CommandsTab::new(icy_board, mnu.clone());
    App {
        full_screen,
        title: format!(
            " {}",
            icy_board_tui::get_text_args(
                "app_file_title",
                std::collections::HashMap::from([
                    ("application".to_string(), icy_board_tui::get_text("app_mkicbmnu")),
                    ("path".to_string(), path.display().to_string()),
                ]),
            )
        ),
        mode: Mode::default(),
        tab: 0,
        date_format,
        tabs: vec![Box::new(general_tab), Box::new(command_tab), Box::new(AboutTab::default())],
        status_line: String::new(),
        help_state: HelpViewState::new(),
        save: SaveChoice::default(),
        offers_quick_save: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use icy_board_tui::get_text;

    #[test]
    fn window_title_and_tabs_use_the_active_locale() {
        let app = new_main_window(IcyBoard::default(), Arc::new(Mutex::new(Menu::default())), false, Path::new("test.mnu"));
        assert_eq!(app.title, format!(" {} (test.mnu)", get_text("app_mkicbmnu")));
        assert_eq!(app.tabs[0].title(), get_text("tui_tab_general"));
        assert_eq!(app.tabs[1].title(), get_text("tui_tab_commands"));
        assert_eq!(app.tabs[2].title(), get_text("tui_tab_about"));
    }
}
