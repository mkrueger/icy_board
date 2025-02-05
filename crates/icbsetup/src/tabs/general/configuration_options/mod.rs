use std::sync::{Arc, Mutex};

use config_switches::ConfigSwitches;
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
mod colors;
mod config_switches;
mod file_transfer;
mod function_keys;
mod limits;
mod messages;
mod system_control;

pub struct ConfigurationOptions {
    pub page: IcbSetupMenuUI,
    icy_board: Arc<Mutex<IcyBoard>>,
}

impl ConfigurationOptions {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        Self {
            page: IcbSetupMenuUI::new(SelectMenu::new(vec![
                MenuItem::new(0, 'A', get_text("configuration_options_messages")),
                MenuItem::new(1, 'B', get_text("configuration_options_file_transfer")),
                MenuItem::new(2, 'C', get_text("configuration_options_system_control")),
                MenuItem::new(3, 'D', get_text("configuration_options_config_switches")),
                MenuItem::new(4, 'E', get_text("configuration_options_limits")),
                MenuItem::new(5, 'F', get_text("configuration_options_colors")),
                MenuItem::new(6, 'G', get_text("configuration_options_func_keys")),
            ]))
            .with_center_title(get_text("configuration_options_title")),
            icy_board,
        }
    }
}

impl Page for ConfigurationOptions {
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
                0 => PageMessage::OpenSubPage(Box::new(messages::Messages::new(self.icy_board.clone()))),
                1 => PageMessage::OpenSubPage(Box::new(file_transfer::FileTransfers::new(self.icy_board.clone()))),
                2 => PageMessage::OpenSubPage(Box::new(system_control::SystemControl::new(self.icy_board.clone()))),
                3 => PageMessage::OpenSubPage(Box::new(ConfigSwitches::new(self.icy_board.clone()))),
                4 => PageMessage::OpenSubPage(Box::new(limits::Limits::new(self.icy_board.clone()))),
                5 => PageMessage::OpenSubPage(Box::new(colors::ColorOptions::new(self.icy_board.clone()))),
                6 => PageMessage::OpenSubPage(Box::new(function_keys::FunctionKeys::new(self.icy_board.clone()))),
                _ => PageMessage::None,
            };
        }
        PageMessage::None
    }
}
