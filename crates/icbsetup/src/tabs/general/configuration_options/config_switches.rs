use std::sync::{Arc, Mutex};

use icy_board_engine::icy_board::{IcyBoard, icb_config::DisplayNewsBehavior};
use icy_board_tui::{
    cfg_entry_bool,
    config_menu::{ConfigEntry, ConfigMenu, ListItem, ListValue, ResultState, TextFlags},
    get_text,
    icbconfigmenu::ICBConfigMenuUI,
    tab_page::Page,
};

pub struct ConfigSwitches {
    menu: ICBConfigMenuUI,
}

impl ConfigSwitches {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock: std::sync::MutexGuard<'_, IcyBoard> = icy_board.lock().unwrap();
            let label_width = 34;
            let entry = vec![
                ConfigEntry::Separator,
                cfg_entry_bool!("default_graphics_at_login", label_width, switches, default_graphics_at_login, lock),
                cfg_entry_bool!("non_graphics", label_width, switches, non_graphics, lock),
                cfg_entry_bool!("exclude_local_calls_stats", label_width, switches, exclude_local_calls_stats, lock),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("display_news_behavior"),
                        ListValue::Text(1, TextFlags::None, lock.config.switches.display_news_behavior.to_pcb_char().to_string()),
                    )
                    .with_status(&get_text("display_news_behavior-status"))
                    .with_label_width(label_width)
                    .with_update_text_value(&|board, value| {
                        if let Some(value) = value.chars().next() {
                            board.lock().unwrap().config.switches.display_news_behavior = DisplayNewsBehavior::from_pcb_char(value);
                        }
                    }),
                ),
                cfg_entry_bool!("display_userinfo_at_login", label_width, switches, display_userinfo_at_login, lock),
                cfg_entry_bool!("force_intro_on_join", label_width, switches, force_intro_on_join, lock),
                cfg_entry_bool!("scan_new_blt", label_width, switches, scan_new_blt, lock),
                cfg_entry_bool!("capture_grp_chat_session", label_width, switches, capture_grp_chat_session, lock),
                cfg_entry_bool!("allow_handle_in_grpchat", label_width, switches, allow_handle_in_grpchat, lock),
            ];
            ConfigMenu { obj: icy_board.clone(), entry }
        };

        Self {
            menu: ICBConfigMenuUI::new(get_text("configuration_options_config_switches"), menu),
        }
    }
}

impl Page for ConfigSwitches {
    fn render(&mut self, frame: &mut ratatui::Frame, disp_area: ratatui::prelude::Rect) {
        self.menu.render(frame, disp_area)
    }
    fn request_status(&self) -> ResultState {
        self.menu.request_status()
    }
    fn handle_key_press(&mut self, key: crossterm::event::KeyEvent) -> icy_board_tui::tab_page::PageMessage {
        self.menu.handle_key_press(key)
    }
}
