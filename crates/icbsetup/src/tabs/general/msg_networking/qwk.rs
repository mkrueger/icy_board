use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    cfg_entry_path, cfg_entry_text,
    config_menu::{ConfigEntry, ConfigMenu, ResultState},
    get_text,
    icbconfigmenu::ICBConfigMenuUI,
    tab_page::{Page, PageMessage},
};

pub struct QwkSettings {
    menu: ICBConfigMenuUI,
}

impl QwkSettings {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_width = 14;
            let sysop_info: Vec<icy_board_tui::config_menu::ConfigEntry<Arc<Mutex<IcyBoard>>>> = vec![
                ConfigEntry::Separator,
                ConfigEntry::Label(get_text("qwk_bbs_label")),
                cfg_entry_text!("qwk_bbs_name", label_width, 25, qwk_settings, bbs_name, lock),
                cfg_entry_text!("qwk_bbs_city_and_state", label_width, 25, qwk_settings, bbs_city_and_state, lock),
                cfg_entry_text!("qwk_bbs_phone_number", label_width, 25, qwk_settings, bbs_phone_number, lock),
                cfg_entry_text!("qwk_bbs_sysop_name", label_width, 25, qwk_settings, bbs_sysop_name, lock),
                cfg_entry_text!("qwk_bbs_id", label_width, 25, qwk_settings, bbs_id, lock),
                ConfigEntry::Separator,
                ConfigEntry::Label(get_text("qwk_files_label")),
                cfg_entry_path!("qwk_welcome_screen", label_width, qwk_settings, welcome_screen, lock),
                cfg_entry_path!("qwk_goodbye_screen", label_width, qwk_settings, goodbye_screen, lock),
                cfg_entry_path!("qwk_news_sceen", label_width, qwk_settings, news_sceen, lock),
            ];
            ConfigMenu {
                obj: icy_board.clone(),
                entry: sysop_info,
            }
        };
        Self {
            menu: ICBConfigMenuUI::new(get_text("qwk_settings_title"), menu),
        }
    }
}

impl Page for QwkSettings {
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
