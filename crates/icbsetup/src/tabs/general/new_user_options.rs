use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState},
    get_text,
    tab_page::{Page, PageMessage},
    theme::get_tui_theme,
    BORDER_SET,
};
use ratatui::{
    layout::{Margin, Rect},
    text::Line,
    widgets::{Block, Borders, Padding, Widget},
};

use crate::{cfg_entry_bool, cfg_entry_sec_level, cfg_entry_text};

pub struct NewUserOptions {
    pub state: ConfigMenuState,
    menu: ConfigMenu<Arc<Mutex<IcyBoard>>>,
}

impl NewUserOptions {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();

            let label_width = 22;
            let table_width = 20;

            let entry = vec![
                cfg_entry_sec_level!("new_user_sec", label_width, new_user_settings, sec_level, lock),
                cfg_entry_bool!("allow_one_name_users", label_width, new_user_settings, allow_one_name_users, lock),
                cfg_entry_text!("new_user_groups", 40, label_width, new_user_settings, new_user_groups, lock),
                ConfigEntry::Table(
                    2,
                    vec![
                        cfg_entry_bool!("ask_city_or_state", table_width, new_user_settings, ask_city_or_state, lock),
                        cfg_entry_bool!("ask_address", table_width, new_user_settings, ask_address, lock),
                        cfg_entry_bool!("ask_verification", table_width, new_user_settings, ask_verification, lock),
                        cfg_entry_bool!("ask_bus_data_phone", table_width, new_user_settings, ask_business_phone, lock),
                        cfg_entry_bool!("ask_home_phone", table_width, new_user_settings, ask_home_phone, lock),
                        cfg_entry_bool!("ask_comment", table_width, new_user_settings, ask_comment, lock),
                        cfg_entry_bool!("ask_clr_msg", table_width, new_user_settings, ask_clr_msg, lock),
                        cfg_entry_bool!("ask_fse", table_width, new_user_settings, ask_fse, lock),
                        cfg_entry_bool!("ask_xfer_protocol", table_width, new_user_settings, ask_xfer_protocol, lock),
                        cfg_entry_bool!("ask_date_format", table_width, new_user_settings, ask_date_format, lock),
                        cfg_entry_bool!("ask_alias", table_width, new_user_settings, ask_alias, lock),
                        cfg_entry_bool!("ask_gender", table_width, new_user_settings, ask_gender, lock),
                        cfg_entry_bool!("ask_birthdate", table_width, new_user_settings, ask_birthdate, lock),
                        cfg_entry_bool!("ask_email", table_width, new_user_settings, ask_email, lock),
                        cfg_entry_bool!("ask_web_address", table_width, new_user_settings, ask_web_address, lock),
                        cfg_entry_bool!("ask_use_short_descr", table_width, new_user_settings, ask_use_short_descr, lock),
                    ],
                ),
            ];
            ConfigMenu { obj: icy_board.clone(), entry }
        };

        Self {
            state: ConfigMenuState::default(),
            menu,
        }
    }
}

impl Page for NewUserOptions {
    fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        let block: Block<'_> = Block::new()
            .style(get_tui_theme().background)
            .padding(Padding::new(2, 2, 1 + 4, 0))
            .borders(Borders::ALL)
            .border_set(BORDER_SET)
            .border_style(get_tui_theme().content_box);
        block.render(area, frame.buffer_mut());

        let val = get_text("new_user_options_title");
        let width = val.len() as u16;
        Line::raw(val).style(get_tui_theme().menu_title).render(
            Rect {
                x: area.x + 1 + area.width.saturating_sub(width) / 2,
                y: area.y + 1,
                width,
                height: 1,
            },
            frame.buffer_mut(),
        );

        frame.buffer_mut().set_string(
            area.x + 1,
            area.y + 2,
            "─".repeat((area.width as usize).saturating_sub(2)),
            get_tui_theme().content_box,
        );

        let area = area.inner(Margin { vertical: 4, horizontal: 1 });
        self.menu.render(area, frame, &mut self.state);
    }

    fn request_status(&self) -> ResultState {
        ResultState {
            edit_mode: icy_board_tui::config_menu::EditMode::None,
            status_line: self.menu.current_status_line(&self.state),
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        let res = self.menu.handle_key_press(key, &mut self.state);

        PageMessage::ResultState(res)
    }
}
