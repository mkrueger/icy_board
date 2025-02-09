use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    cfg_entry_bool, cfg_entry_u16,
    config_menu::{ConfigEntry, ConfigMenu, ResultState},
    get_text,
    icbconfigmenu::ICBConfigMenuUI,
    tab_page::{Page, PageMessage},
};

pub struct Messages {
    menu: ICBConfigMenuUI,
}

impl Messages {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_width = 40;
            let entry: Vec<ConfigEntry<Arc<Mutex<IcyBoard>>>> = vec![
                ConfigEntry::Separator,
                cfg_entry_u16!("max_msg_lines", label_width, 17, 400, message, max_msg_lines, lock),
                ConfigEntry::Separator,
                cfg_entry_bool!("disable_message_scan_prompt", label_width, message, disable_message_scan_prompt, lock),
                cfg_entry_bool!("allow_esc_codes", label_width, message, allow_esc_codes, lock),
                cfg_entry_bool!("allow_carbon_copy", label_width, message, allow_carbon_copy, lock),
                cfg_entry_bool!("validate_to_name", label_width, message, validate_to_name, lock),
                cfg_entry_bool!("default_quick_personal_scan", label_width, message, default_quick_personal_scan, lock),
                cfg_entry_bool!(
                    "default_scan_all_selected_confs_at_login",
                    label_width,
                    message,
                    default_scan_all_selected_confs_at_login,
                    lock
                ),
                cfg_entry_bool!("prompt_to_read_mail", label_width, message, prompt_to_read_mail, lock),
            ];
            ConfigMenu { obj: icy_board.clone(), entry }
        };

        Self {
            menu: ICBConfigMenuUI::new(get_text("configuration_options_messages"), menu),
        }
    }
}

impl Page for Messages {
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
