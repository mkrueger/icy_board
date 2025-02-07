use std::sync::{Arc, Mutex};

use crate::{cfg_entry_bool, cfg_entry_dow, cfg_entry_path, cfg_entry_time};
use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ResultState},
    get_text,
    tab_page::{Page, PageMessage},
};

use super::ICBConfigMenuUI;

pub struct AccountingConfig {
    menu: ICBConfigMenuUI,
}

impl AccountingConfig {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let icy_board2 = icy_board.clone();
            let lock: std::sync::MutexGuard<'_, IcyBoard> = icy_board.lock().unwrap();
            let label_width = 37;
            let sysop_info: Vec<ConfigEntry<Arc<Mutex<IcyBoard>>>> = vec![
                ConfigEntry::Separator,
                cfg_entry_bool!("accounting_enabled", label_width, accounting, enabled, lock),
                cfg_entry_bool!("accounting_use_money", label_width, accounting, use_money, lock),
                cfg_entry_bool!("accounting_concurrent_tracking", label_width, accounting, concurrent_tracking, lock),
                cfg_entry_bool!("accounting_ignore_empty_sec_level", label_width, accounting, ignore_empty_sec_level, lock),
                cfg_entry_time!("accounting_peak_usage_start", label_width, accounting, peak_usage_start, lock),
                cfg_entry_time!("accounting_peak_usage_end", label_width, accounting, peak_usage_end, lock),
                cfg_entry_dow!("accounting_peak_days_of_week", label_width, accounting, peak_days_of_week, lock),
                ConfigEntry::Separator,
                cfg_entry_path!("accounting_peak_holiday_list_file", label_width, accounting, peak_holiday_list_file, lock),
                cfg_entry_path!(
                    "accounting_cfg_file",
                    label_width,
                    accounting,
                    cfg_file,
                    Box::new(crate::editors::accounting_rates::edit_account_config),
                    lock
                ),
                cfg_entry_path!("accounting_tracking_file", label_width, accounting, tracking_file, lock),
                cfg_entry_path!("accounting_info_file", label_width, accounting, info_file, lock),
                cfg_entry_path!("accounting_warning_file", label_width, accounting, warning_file, lock),
                cfg_entry_path!("accounting_logoff_file", label_width, accounting, logoff_file, lock),
            ];

            ConfigMenu {
                obj: icy_board2,
                entry: sysop_info,
            }
        };

        Self {
            menu: ICBConfigMenuUI::new(get_text("accounting_config_title"), menu),
        }
    }
}

impl Page for AccountingConfig {
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
