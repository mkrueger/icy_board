use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::ResultState,
    get_text,
    select_menu::{MenuItem, SelectMenu},
    tab_page::{Page, PageMessage},
};
use ratatui::{layout::Rect, Frame};

use super::IcbSetupMenuUI;
mod sysop_commands;
mod sysop_functions;
mod user_commands;

pub struct SecurityLevelOptions {
    pub page: IcbSetupMenuUI,
    icy_board: Arc<Mutex<IcyBoard>>,
}

impl SecurityLevelOptions {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        Self {
            page: IcbSetupMenuUI::new(SelectMenu::new(vec![
                MenuItem::new(0, 'A', get_text("sec_level_menu_sysop_funcs")),
                MenuItem::new(1, 'B', get_text("sec_level_menu_sysop_commands")),
                MenuItem::new(2, 'C', get_text("sec_level_menu_user_commands")),
            ]))
            .with_center_title(get_text("sec_level_menu_title")),
            icy_board,
        }
    }
}

impl Page for SecurityLevelOptions {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.page.render(frame, area);
    }

    fn request_status(&self) -> ResultState {
        self.page.request_status()
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        let (_state, opt) = self.page.handle_key_press(key);
        if let Some(selected) = opt {
            return match selected {
                0 => PageMessage::OpenSubPage(Box::new(sysop_functions::SysopFunctions::new(self.icy_board.clone()))),
                1 => PageMessage::OpenSubPage(Box::new(sysop_commands::SysopCommands::new(self.icy_board.clone()))),
                2 => PageMessage::OpenSubPage(Box::new(user_commands::UserCommands::new(self.icy_board.clone()))),
                _ => PageMessage::None,
            };
        }
        PageMessage::None
    }
}
