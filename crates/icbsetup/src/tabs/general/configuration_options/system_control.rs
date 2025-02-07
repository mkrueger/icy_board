use std::sync::{Arc, Mutex};

use crate::{cfg_entry_bool, tabs::ICBConfigMenuUI};
use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ResultState},
    get_text,
    tab_page::{Page, PageMessage},
};

pub struct SystemControl {
    menu: ICBConfigMenuUI,
}

impl SystemControl {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_width = 31;
            let entry = vec![
                ConfigEntry::Separator,
                cfg_entry_bool!("disable_ns_logon", label_width, system_control, disable_ns_logon, lock),
                cfg_entry_bool!("is_multi_lingual", label_width, system_control, is_multi_lingual, lock),
                cfg_entry_bool!("allow_alias_change", label_width, system_control, allow_alias_change, lock),
                cfg_entry_bool!("is_closed_board", label_width, system_control, is_closed_board, lock),
                cfg_entry_bool!("enforce_daily_time_limit", label_width, system_control, enforce_daily_time_limit, lock),
                cfg_entry_bool!(
                    "allow_password_failure_comment",
                    label_width,
                    system_control,
                    allow_password_failure_comment,
                    lock
                ),
                cfg_entry_bool!("guard_logoff", label_width, system_control, guard_logoff, lock),
            ];
            ConfigMenu { obj: icy_board.clone(), entry }
        };

        Self {
            menu: ICBConfigMenuUI::new(get_text("system_control_title"), menu),
        }
    }
}

impl Page for SystemControl {
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
