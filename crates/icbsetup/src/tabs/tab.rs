use std::sync::Arc;
use std::sync::Mutex;

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::icy_board::user_base::Password;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState},
    theme::THEME,
    TerminalType,
};
use ratatui::{
    layout::{Margin, Rect},
    widgets::{Block, BorderType, Borders, Clear, Padding, Widget},
    Frame,
};

use super::TabPage;

pub struct ServerTab {
    pub state: ConfigMenuState,
    config: ConfigMenu,
    icy_board: Arc<Mutex<IcyBoard>>
}
impl ServerTab {
    pub fn new(lock: Arc<Mutex<IcyBoard>>) -> Self {
        let icy_board = lock.lock().unwrap();
        let sysop_info = vec![
        ];

      

        Self {
            state: ConfigMenuState::default(),
            icy_board: lock.clone(),
            config: ConfigMenu {
                items: vec![
                    ConfigEntry::Group("Sysop Information".to_string(), sysop_info),
                ],
            },
        }
    }

    fn write_back(&self, icy_board: &mut IcyBoard) {
        for entry in self.config.items.iter() {
            self.visit_item(&entry, icy_board);
        }
    }

    fn visit_item(&self, entry: &ConfigEntry, icy_board: &mut IcyBoard) {
        match entry {
            ConfigEntry::Group(_grp, entries) => {
                for e in entries {
                    self.visit_item(&e, icy_board);
                }
            }
            ConfigEntry::Separator => {}
            ConfigEntry::Item(item) => self.write_item(&item, icy_board),
            ConfigEntry::Table(_, _) => todo!(),
        }
    }

    fn write_item(&self, item: &ListItem, icy_board: &mut IcyBoard) {
        match &item.value {
            ListValue::Text(_, text) => match item.id.as_str() {
                "board_name" => icy_board.config.board.name = text.clone(),
                "location" => icy_board.config.board.location = text.clone(),
                "operator" => icy_board.config.board.operator = text.clone(),
                "notice" => icy_board.config.board.notice = text.clone(),
                "capabilities" => icy_board.config.board.capabilities = text.clone(),
                "date_format" => icy_board.config.board.date_format = text.clone(),

                "sysop_name" => icy_board.config.sysop.name = text.clone(),
                "local_pass" => icy_board.config.sysop.password = Password::PlainText(text.clone()),
                "new_user_groups" => icy_board.config.new_user_settings.new_user_groups = text.clone(),
                "f1" => icy_board.config.func_keys[0] = text.clone(),
                "f2" => icy_board.config.func_keys[1] = text.clone(),
                "f3" => icy_board.config.func_keys[2] = text.clone(),
                "f4" => icy_board.config.func_keys[3] = text.clone(),
                "f5" => icy_board.config.func_keys[4] = text.clone(),
                "f6" => icy_board.config.func_keys[5] = text.clone(),
                "f7" => icy_board.config.func_keys[6] = text.clone(),
                "f8" => icy_board.config.func_keys[7] = text.clone(),
                "f9" => icy_board.config.func_keys[8] = text.clone(),
                "f10" => icy_board.config.func_keys[9] = text.clone(),
                _=> panic!("Unknown id: {}", item.id)
            },
            ListValue::Path(_, _path) => match item.id.as_str() {
                _ => panic!("Unknown id: {}", item.id)
            },
            ListValue::U32(i, _, _) =>  match item.id.as_str() {
                "sec_level" => icy_board.config.new_user_settings.sec_level = *i as u8,
                "num_nodes" => icy_board.config.board.num_nodes = *i as u16,
                "max_lines" => icy_board.config.options.max_msg_lines = *i as u16,
                "keyboard_timeout" => icy_board.config.options.keyboard_timeout = *i as u16,
                "upload_descr_lines" => icy_board.config.options.upload_descr_lines = *i as u8,


                _ => panic!("Unknown id: {}", item.id)
            },
            ListValue::Bool(b) =>  match item.id.as_str() {
                "local_pass_exit" => icy_board.config.sysop.require_password_to_exit = *b,
                "use_real_name" => icy_board.config.sysop.use_real_name = *b,
                "ask_city_or_state" => icy_board.config.new_user_settings.ask_city_or_state = *b,
                "ask_address" => icy_board.config.new_user_settings.ask_address = *b,
                "ask_verification" => icy_board.config.new_user_settings.ask_verification = *b,
                "ask_bus_data_phone" => icy_board.config.new_user_settings.ask_bus_data_phone = *b,
                "ask_voice_phone" => icy_board.config.new_user_settings.ask_voice_phone = *b,
                "ask_comment" => icy_board.config.new_user_settings.ask_comment = *b,
                "ask_clr_msg" => icy_board.config.new_user_settings.ask_clr_msg = *b,
                "ask_xfer_protocol" => icy_board.config.new_user_settings.ask_xfer_protocol = *b,
                "ask_date_format" => icy_board.config.new_user_settings.ask_date_format = *b,
                "ask_alias" => icy_board.config.new_user_settings.ask_alias = *b,
                "ask_gender" => icy_board.config.new_user_settings.ask_gender = *b,
                "ask_birthdate" => icy_board.config.new_user_settings.ask_birthdate = *b,
                "ask_email" => icy_board.config.new_user_settings.ask_email = *b,
                "ask_web_address" => icy_board.config.new_user_settings.ask_web_address = *b,
                "ask_use_short_descr" => icy_board.config.new_user_settings.ask_use_short_descr = *b,
                "scan_all_mail_at_login" => icy_board.config.options.scan_all_mail_at_login = *b,
                "prompt_to_read_mail" => icy_board.config.options.prompt_to_read_mail = *b,
                "check_files_uploaded" => icy_board.config.options.check_files_uploaded = *b,
                "display_uploader" => icy_board.config.options.display_uploader = *b,
                "exclude_local_calls" => icy_board.config.options.exclude_local_calls = *b,

                _ => panic!("Unknown id: {}", item.id)
            },
            ListValue::Color(c) =>  match item.id.as_str() {
                "msg_hdr_date" => icy_board.config.color_configuration.msg_hdr_date = c.clone(),
                "msg_hdr_to" => icy_board.config.color_configuration.msg_hdr_to = c.clone(),
                "msg_hdr_from" => icy_board.config.color_configuration.msg_hdr_from = c.clone(),
                "msg_hdr_subj" => icy_board.config.color_configuration.msg_hdr_subj = c.clone(),
                "msg_hdr_read" => icy_board.config.color_configuration.msg_hdr_read = c.clone(),
                "msg_hdr_conf" => icy_board.config.color_configuration.msg_hdr_conf = c.clone(),
                "file_head" => icy_board.config.color_configuration.file_head = c.clone(),
                "file_name" => icy_board.config.color_configuration.file_name = c.clone(),
                "file_size" => icy_board.config.color_configuration.file_size = c.clone(),
                "file_date" => icy_board.config.color_configuration.file_date = c.clone(),
                "file_description" => icy_board.config.color_configuration.file_description = c.clone(),
                "file_description_low" => icy_board.config.color_configuration.file_description_low = c.clone(),
                "file_text" => icy_board.config.color_configuration.file_text = c.clone(),
                "file_deleted" => icy_board.config.color_configuration.file_deleted = c.clone(),

                _ => panic!("Unknown id: {}", item.id)
            },
            ListValue::ValueList(_, _) => todo!(),
    
        }

    }
    
    fn prev(&mut self) {
        if self.state.selected > 0 {
            self.state.selected -= 1;

            if let Some(y) = self.state.item_pos.get(&self.state.selected) {
                if *y < self.state.first_row {
                    self.state.first_row = *y;
                    if self.state.first_row == 1 {
                        self.state.first_row = 0;
                    }
                }
            }
        }
    }

    fn next(&mut self) {
        let count = self.config.count();
        if self.state.selected < count - 1 {
            self.state.selected += 1;
            if let Some(y) = self.state.item_pos.get(&self.state.selected) {
                if *y >= self.state.area_height {
                    self.state.first_row = *y - self.state.area_height + 1;
                }
            }
        }
    }
    
    
}

impl TabPage for ServerTab {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let area = area.inner(&Margin { horizontal: 2, vertical: 2 });

        Clear.render(area, frame.buffer_mut());

        let block = Block::new()
            .style(THEME.content_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        block.render(area, frame.buffer_mut());

        let area = area.inner(&Margin { vertical: 1, horizontal: 1 });
        self.config.render(area, frame, &mut self.state);
    }

    fn request_edit_mode(&mut self, _t: &mut TerminalType, _fs: bool) -> ResultState {
        self.state.in_edit = true;
        self.config.request_edit_mode(&self.state)
    }

    fn set_cursor_position(&self, frame: &mut Frame) {
        self.config.get_item(self.state.selected).unwrap().text_field_state.set_cursor_position(frame);
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> ResultState {
        if !self.state.in_edit {
            match key.code {
                KeyCode::Char('k') | KeyCode::Up => self.prev(),
                KeyCode::Char('j') | KeyCode::Down => self.next(),
                _ => {}
            }
            return ResultState {
                in_edit_mode: false,
                status_line: self.config.get_item(self.state.selected).unwrap().status.clone(),
            };
        } else {
            let res = self.config.handle_key_press(key, &self.state);
            self.write_back(&mut self.icy_board.lock().unwrap());
            self.state.in_edit = res.in_edit_mode;
            res
        }
    }

    fn request_status(&self) -> ResultState {
        return ResultState {
            in_edit_mode: self.state.in_edit,
            status_line: if self.state.selected < self.config.items.len() {
                "".to_string()
            } else {
                "".to_string()
            },
        };
    }
}
