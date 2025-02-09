use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    cfg_entry_bool, cfg_entry_text, cfg_entry_u8,
    config_menu::{ConfigEntry, ConfigMenu, ResultState},
    get_text,
    icbconfigmenu::ICBConfigMenuUI,
    tab_page::{Page, PageMessage},
};

pub struct NewUserOptions {
    menu: ICBConfigMenuUI,
}

impl NewUserOptions {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();

            let label_width = 22;
            let table_width_left = 16;
            let table_width_right = 18;

            let entry: Vec<ConfigEntry<Arc<Mutex<IcyBoard>>>> = vec![
                ConfigEntry::Separator,
                cfg_entry_u8!("new_user_security_level", label_width, 0, 255, new_user_settings, sec_level, lock),
                ConfigEntry::Separator,
                cfg_entry_bool!("allow_one_name_users", label_width, new_user_settings, allow_one_name_users, lock),
                cfg_entry_text!("new_user_groups", label_width, 30, new_user_settings, new_user_groups, lock),
                ConfigEntry::Separator,
                ConfigEntry::Label(get_text("new_user_options_ask_label")),
                ConfigEntry::Table(
                    2,
                    vec![
                        cfg_entry_bool!("ask_city_or_state", table_width_left, new_user_settings, ask_city_or_state, lock, 12),
                        cfg_entry_bool!("ask_address", table_width_right, new_user_settings, ask_address, lock),
                        cfg_entry_bool!("ask_verification", table_width_left, new_user_settings, ask_verification, lock),
                        cfg_entry_bool!("ask_bus_data_phone", table_width_right, new_user_settings, ask_business_phone, lock),
                        cfg_entry_bool!("ask_home_phone", table_width_left, new_user_settings, ask_home_phone, lock),
                        cfg_entry_bool!("ask_comment", table_width_right, new_user_settings, ask_comment, lock),
                        cfg_entry_bool!("ask_clr_msg", table_width_left, new_user_settings, ask_clr_msg, lock),
                        cfg_entry_bool!("ask_fse", table_width_right, new_user_settings, ask_fse, lock),
                        cfg_entry_bool!("ask_xfer_protocol", table_width_left, new_user_settings, ask_xfer_protocol, lock),
                        cfg_entry_bool!("ask_date_format", table_width_right, new_user_settings, ask_date_format, lock),
                        cfg_entry_bool!("ask_alias", table_width_left, new_user_settings, ask_alias, lock),
                        cfg_entry_bool!("ask_gender", table_width_right, new_user_settings, ask_gender, lock),
                        cfg_entry_bool!("ask_birthdate", table_width_left, new_user_settings, ask_birthdate, lock),
                        cfg_entry_bool!("ask_email", table_width_right, new_user_settings, ask_email, lock),
                        cfg_entry_bool!("ask_web_address", table_width_left, new_user_settings, ask_web_address, lock),
                        cfg_entry_bool!("ask_use_short_descr", table_width_right, new_user_settings, ask_use_short_descr, lock),
                    ],
                ),
            ];
            ConfigMenu { obj: icy_board.clone(), entry }
        };

        Self {
            menu: ICBConfigMenuUI::new(get_text("new_user_options_title"), menu),
        }
    }
}

impl Page for NewUserOptions {
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
