use std::sync::{Arc, Mutex};

use crate::{cfg_entry_path, tabs::ICBConfigMenuUI};
use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ResultState},
    get_text,
    tab_page::{Page, PageMessage},
};

pub struct DisplayFiles {
    menu: ICBConfigMenuUI,
}

impl DisplayFiles {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_width = 33;
            let entry = vec![
                ConfigEntry::Separator,
                cfg_entry_path!("paths_welcome", label_width, paths, welcome, lock),
                cfg_entry_path!("paths_newuser", label_width, paths, newuser, lock),
                cfg_entry_path!("paths_closed", label_width, paths, closed, lock),
                cfg_entry_path!("paths_expire_warning", label_width, paths, expire_warning, lock),
                cfg_entry_path!("paths_expired", label_width, paths, expired, lock),
                cfg_entry_path!("paths_conf_join_menu", label_width, paths, conf_join_menu, lock),
                cfg_entry_path!("paths_conf_chat_intro_file", label_width, paths, chat_intro_file, lock),
                cfg_entry_path!("paths_conf_chat_menu", label_width, paths, chat_menu, lock),
                cfg_entry_path!("paths_conf_chat_actions_menu", label_width, paths, chat_actions_menu, lock),
                cfg_entry_path!("paths_no_ansi", label_width, paths, no_ansi, lock),
            ];
            ConfigMenu { obj: icy_board.clone(), entry }
        };

        Self {
            menu: ICBConfigMenuUI::new(get_text("file_locations_display_files"), menu),
        }
    }
}

impl Page for DisplayFiles {
    fn render(&mut self, frame: &mut ratatui::Frame, disp_area: ratatui::prelude::Rect) {
        self.menu.render(frame, disp_area)
    }
    fn request_status(&self) -> ResultState {
        self.menu.request_status()
    }
    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        self.menu.handle_key_press(key)
    }
}
