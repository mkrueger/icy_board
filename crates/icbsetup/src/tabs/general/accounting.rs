use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ListItem, ListValue, ResultState},
    get_text,
    icbconfigmenu::ICBConfigMenuUI,
    tab_page::{Page, PageMessage},
};

pub struct AccountingConfig {
    menu: ICBConfigMenuUI,
}

impl AccountingConfig {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let icy_board2 = icy_board.clone();
            let lock: std::sync::MutexGuard<'_, IcyBoard> = icy_board.lock().unwrap();
            let label_width = 38;
            // All controls in this section are consumed by the accounting runtime.
            macro_rules! accounting_item {
                ($key:literal, $field:ident, $kind:ident $(, $text:expr)?) => {
                    ListItem::new(get_text($key), ListValue::$kind(lock.config.accounting.$field.clone() $(, $text)?))
                        .with_label_width(label_width)
                        .with_status(get_text(concat!($key, "-status")))
                        .with_help(get_text(concat!($key, "-help")))
                        .with_update_value(Box::new(|board: &Arc<Mutex<IcyBoard>>, value: &ListValue| {
                            let ListValue::$kind(val, ..) = value else { return; };
                            board.lock().unwrap().config.accounting.$field = val.clone();
                        }))
                };
            }
            let sysop_info: Vec<ConfigEntry<Arc<Mutex<IcyBoard>>>> = vec![
                ConfigEntry::Separator,
                ConfigEntry::Item(accounting_item!("accounting_enabled", enabled, Bool)),
                ConfigEntry::Item(accounting_item!("accounting_use_money", use_money, Bool)),
                ConfigEntry::Item(accounting_item!("accounting_concurrent_tracking", concurrent_tracking, Bool)),
                ConfigEntry::Item(accounting_item!("accounting_ignore_empty_sec_level", ignore_empty_sec_level, Bool)),
                ConfigEntry::Item(accounting_item!(
                    "accounting_peak_usage_start",
                    peak_usage_start,
                    Time,
                    lock.config.accounting.peak_usage_start.to_string()
                )),
                ConfigEntry::Item(accounting_item!(
                    "accounting_peak_usage_end",
                    peak_usage_end,
                    Time,
                    lock.config.accounting.peak_usage_end.to_string()
                )),
                ConfigEntry::Item(accounting_item!(
                    "accounting_peak_days_of_week",
                    peak_days_of_week,
                    DoW,
                    lock.config.accounting.peak_days_of_week.to_string()
                )),
                ConfigEntry::Separator,
                ConfigEntry::Item(accounting_item!("accounting_peak_holiday_list_file", peak_holiday_list_file, Path)),
                ConfigEntry::Item(
                    accounting_item!("accounting_cfg_file", cfg_file, Path).with_path_editor(Box::new(crate::editors::accounting_rates::edit_account_config)),
                ),
                ConfigEntry::Item(accounting_item!("accounting_tracking_file", tracking_file, Path)),
                ConfigEntry::Item(accounting_item!("accounting_info_file", info_file, Path)),
                ConfigEntry::Item(accounting_item!("accounting_warning_file", warning_file, Path)),
                ConfigEntry::Item(accounting_item!("accounting_logoff_file", logoff_file, Path)),
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
