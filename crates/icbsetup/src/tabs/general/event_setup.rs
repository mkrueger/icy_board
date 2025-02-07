use std::sync::{Arc, Mutex};

use crate::{cfg_entry_bool, cfg_entry_path, cfg_entry_u16};
use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ResultState},
    get_text,
    tab_page::{Page, PageMessage},
};

use super::ICBConfigMenuUI;

pub struct EventSetup {
    menu: ICBConfigMenuUI,
}

impl EventSetup {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let icy_board2 = icy_board.clone();
            let lock: std::sync::MutexGuard<'_, IcyBoard> = icy_board.lock().unwrap();
            let label_width = 37;
            let sysop_info: Vec<ConfigEntry<Arc<Mutex<IcyBoard>>>> = vec![
                ConfigEntry::Separator,
                cfg_entry_bool!("event_enabled", label_width, event, enabled, lock),
                cfg_entry_path!("event_dat_path", label_width, event, event_dat_path, lock),
                ConfigEntry::Separator,
                ConfigEntry::Label(get_text("event_enabled_for_expedited_label")),
                ConfigEntry::Separator,
                cfg_entry_u16!("event_suspend_minutes", label_width, 0, 99, event, suspend_minutes, lock),
                cfg_entry_bool!("event_disallow_uploads", label_width, event, disallow_uploads, lock),
                cfg_entry_u16!("event_minutes_uploads_disallowed", label_width, 0, 99, event, minutes_uploads_disallowed, lock),
            ];

            ConfigMenu {
                obj: icy_board2,
                entry: sysop_info,
            }
        };

        Self {
            menu: ICBConfigMenuUI::new(get_text("event_setup_title"), menu),
        }
    }
}

impl Page for EventSetup {
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
