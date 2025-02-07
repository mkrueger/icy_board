use std::sync::{Arc, Mutex};

use crate::{cfg_entry_bool, cfg_entry_password, cfg_entry_text};
use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::{user_base::Password, IcyBoard};
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ResultState},
    get_text,
    tab_page::{Page, PageMessage},
};

use super::ICBConfigMenuUI;

pub struct SysopInformation {
    menu: ICBConfigMenuUI,
}

impl SysopInformation {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_width = 25;
            let sysop_info: Vec<icy_board_tui::config_menu::ConfigEntry<Arc<Mutex<IcyBoard>>>> = vec![
                ConfigEntry::Separator,
                cfg_entry_text!("sysop_name", label_width, 15, sysop, name, lock, 15),
                cfg_entry_password!("local_password", label_width, sysop, password, lock),
                cfg_entry_bool!("require_password_to_exit", label_width, sysop, require_password_to_exit, lock),
                cfg_entry_bool!("use_real_name", label_width, sysop, use_real_name, lock),
                ConfigEntry::Separator,
                cfg_entry_text!("sys_info_external_editor", label_width, 30, sysop, external_editor, lock),
                cfg_entry_text!("sys_info_theme", label_width, 30, sysop, config_color_theme, lock),
            ];
            ConfigMenu {
                obj: icy_board.clone(),
                entry: sysop_info,
            }
        };

        Self {
            menu: ICBConfigMenuUI::new(get_text("sysop_information_title"), menu),
        }
    }
}

impl Page for SysopInformation {
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
