use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::{IcyBoard, icb_config::PasswordStorageMethod};
use icy_board_tui::{
    cfg_entry_bool,
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, ListItem, ListValue, ResultState},
    get_text,
    icbconfigmenu::ICBConfigMenuUI,
    tab_page::{Page, PageMessage},
};

pub struct SystemControl {
    menu: ICBConfigMenuUI,
}

fn password_storage_method_text(method: PasswordStorageMethod) -> String {
    match method {
        PasswordStorageMethod::Argon2 => icy_board_tui::get_text("password_storage_method_argon2"),
        PasswordStorageMethod::PlainText => icy_board_tui::get_text("password_storage_method_plain_text"),
    }
}

impl SystemControl {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_width = 31;
            let cur_method = lock.config.system_control.password_storage_method;

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
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("password_storage_method"),
                        ListValue::ComboBox(ComboBox {
                            cur_value: ComboBoxValue::new(password_storage_method_text(cur_method), format!("{:?}", cur_method)),
                            selected_item: 0,
                            is_edit_open: false,
                            first_item: 0,
                            values: vec![
                                ComboBoxValue::new(password_storage_method_text(PasswordStorageMethod::Argon2), "Argon2"),
                                ComboBoxValue::new(password_storage_method_text(PasswordStorageMethod::PlainText), "PlainText"),
                            ],
                        }),
                    )
                    .with_label_width(label_width)
                    .with_update_combobox_value(&|board: &Arc<Mutex<IcyBoard>>, combo: &ComboBox| {
                        let mut b = board.lock().unwrap();
                        b.config.system_control.password_storage_method = if combo.cur_value.value == "PlainText" {
                            PasswordStorageMethod::PlainText
                        } else {
                            PasswordStorageMethod::Argon2
                        };
                    })
                    .with_help(get_text("password_storage_method_help")),
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
