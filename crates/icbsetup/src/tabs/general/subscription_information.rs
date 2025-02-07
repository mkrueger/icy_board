use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ResultState},
    get_text,
    tab_page::{Page, PageMessage},
};

use crate::{cfg_entry_bool, cfg_entry_u32, cfg_entry_u8};

use super::ICBConfigMenuUI;

pub struct SubscriptionInformation {
    menu: ICBConfigMenuUI,
}

impl SubscriptionInformation {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_width = 36;
            let sysop_info = vec![
                ConfigEntry::Separator,
                cfg_entry_bool!("subscription_is_enabled", label_width, subscription_info, is_enabled, lock),
                cfg_entry_u32!("subscription_length", label_width, 0, 1000, subscription_info, subscription_length, lock),
                cfg_entry_u8!("default_expired_level", label_width, 0, 255, subscription_info, default_expired_level, lock),
                cfg_entry_u32!("warning_days", label_width, 0, 365, subscription_info, warning_days, lock),
            ];
            ConfigMenu {
                obj: icy_board.clone(),
                entry: sysop_info,
            }
        };

        Self {
            menu: ICBConfigMenuUI::new(get_text("subscription_information_title"), menu),
        }
    }
}

impl Page for SubscriptionInformation {
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
