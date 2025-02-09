use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    cfg_entry_u16, cfg_entry_u8,
    config_menu::{ConfigEntry, ConfigMenu, ResultState},
    get_text,
    icbconfigmenu::ICBConfigMenuUI,
    tab_page::{Page, PageMessage},
};
pub struct Limits {
    menu: ICBConfigMenuUI,
}

impl Limits {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_width = 43;
            let entry = vec![
                ConfigEntry::Separator,
                cfg_entry_u16!("keyboard_timeout", label_width, 0, 1000, limits, keyboard_timeout, lock),
                cfg_entry_u16!("max_number_upload_descr_lines", label_width, 0, 60, limits, max_number_upload_descr_lines, lock),
                cfg_entry_u16!("password_expire_days", label_width, 0, 10000, limits, password_expire_days, lock),
                cfg_entry_u16!("password_expire_warn_days", label_width, 0, 10000, limits, password_expire_warn_days, lock),
                cfg_entry_u8!("min_pwd_length", label_width, 0, 10000, limits, min_pwd_length, lock),
            ];
            ConfigMenu { obj: icy_board.clone(), entry }
        };

        Self {
            menu: ICBConfigMenuUI::new(get_text("configuration_options_limits"), menu),
        }
    }
}

impl Page for Limits {
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
