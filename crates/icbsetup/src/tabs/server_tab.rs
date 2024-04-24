use std::sync::Arc;
use std::sync::Mutex;

use crossterm::event::{KeyCode, KeyEvent};
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
    icy_board: Arc<Mutex<IcyBoard>>,
}
impl ServerTab {
    pub fn new(lock: Arc<Mutex<IcyBoard>>) -> Self {
        let icy_board: std::sync::MutexGuard<'_, IcyBoard> = lock.lock().unwrap();
        let telnet = vec![
            ConfigEntry::Item(ListItem::new(
                "telnet_is_enabled",
                "Enabled".to_string(),
                ListValue::Bool(icy_board.config.login_server.telnet.is_enabled),
            )),
            ConfigEntry::Item(ListItem::new(
                "telnet_port",
                "Port".to_string(),
                ListValue::U32(icy_board.config.login_server.telnet.port as u32, 1, u16::MAX as u32),
            )),
            ConfigEntry::Item(ListItem::new(
                "telnet_address",
                "Address".to_string(),
                ListValue::Text(60, icy_board.config.login_server.telnet.address.clone()),
            )),
            ConfigEntry::Item(ListItem::new(
                "telnet_display_file",
                "Display File".to_string(),
                ListValue::Path(icy_board.config.login_server.telnet.display_file.clone()),
            )),
        ];

        let ssh = vec![
            ConfigEntry::Item(ListItem::new(
                "ssh_is_enabled",
                "Enabled".to_string(),
                ListValue::Bool(icy_board.config.login_server.ssh.is_enabled),
            )),
            ConfigEntry::Item(ListItem::new(
                "ssh_port",
                "Port".to_string(),
                ListValue::U32(icy_board.config.login_server.ssh.port as u32, 1, u16::MAX as u32),
            )),
            ConfigEntry::Item(ListItem::new(
                "ssh_address",
                "Address".to_string(),
                ListValue::Text(60, icy_board.config.login_server.ssh.address.clone()),
            )),
            ConfigEntry::Item(ListItem::new(
                "ssh_display_file",
                "Display File".to_string(),
                ListValue::Path(icy_board.config.login_server.ssh.display_file.clone()),
            )),
        ];

        let websocket = vec![
            ConfigEntry::Item(ListItem::new(
                "websocket_is_enabled",
                "Enabled".to_string(),
                ListValue::Bool(icy_board.config.login_server.websocket.is_enabled),
            )),
            ConfigEntry::Item(ListItem::new(
                "websocket_port",
                "Port".to_string(),
                ListValue::U32(icy_board.config.login_server.websocket.port as u32, 1, u16::MAX as u32),
            )),
            ConfigEntry::Item(ListItem::new(
                "websocket_address",
                "Address".to_string(),
                ListValue::Text(60, icy_board.config.login_server.websocket.address.clone()),
            )),
            ConfigEntry::Item(ListItem::new(
                "websocket_display_file",
                "Display File".to_string(),
                ListValue::Path(icy_board.config.login_server.websocket.display_file.clone()),
            )),
        ];

        let secure_websocket = vec![
            ConfigEntry::Item(ListItem::new(
                "secure_websocket_is_enabled",
                "Enabled".to_string(),
                ListValue::Bool(icy_board.config.login_server.secure_websocket.is_enabled),
            )),
            ConfigEntry::Item(ListItem::new(
                "secure_websocket_port",
                "Port".to_string(),
                ListValue::U32(icy_board.config.login_server.secure_websocket.port as u32, 1, u16::MAX as u32),
            )),
            ConfigEntry::Item(ListItem::new(
                "secure_websocket_address",
                "Address".to_string(),
                ListValue::Text(60, icy_board.config.login_server.secure_websocket.address.clone()),
            )),
            ConfigEntry::Item(ListItem::new(
                "secure_websocket_display_file",
                "Display File".to_string(),
                ListValue::Path(icy_board.config.login_server.secure_websocket.display_file.clone()),
            )),
        ];

        Self {
            state: ConfigMenuState::default(),
            icy_board: lock.clone(),
            config: ConfigMenu {
                items: vec![
                    ConfigEntry::Group("Terminal (TELNET)".to_string(), telnet),
                    ConfigEntry::Group("Terminal (SSH)".to_string(), ssh),
                    ConfigEntry::Group("Terminal (WEBSOCKET)".to_string(), websocket),
                    ConfigEntry::Group("Terminal (SECURE WEBSOCKET)".to_string(), secure_websocket),
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
                "telnet_address" => icy_board.config.login_server.telnet.address = text.clone(),
                "ssh_address" => icy_board.config.login_server.ssh.address = text.clone(),
                "websocket_address" => icy_board.config.login_server.websocket.address = text.clone(),
                "secure_websocket_address" => icy_board.config.login_server.secure_websocket.address = text.clone(),
                _ => panic!("Unknown id: {}", item.id),
            },
            ListValue::Path(path) => match item.id.as_str() {
                "telnet_display_file" => icy_board.config.login_server.telnet.display_file = path.clone(),
                "ssh_display_file" => icy_board.config.login_server.ssh.display_file = path.clone(),
                "websocket_display_file" => icy_board.config.login_server.websocket.display_file = path.clone(),
                "secure_websocket_display_file" => icy_board.config.login_server.secure_websocket.display_file = path.clone(),
                _ => panic!("Unknown id: {}", item.id),
            },
            ListValue::U32(i, _, _) => match item.id.as_str() {
                "telnet_port" => icy_board.config.login_server.telnet.port = *i as u16,
                "ssh_port" => icy_board.config.login_server.ssh.port = *i as u16,
                "websocket_port" => icy_board.config.login_server.websocket.port = *i as u16,
                "secure_websocket_port" => icy_board.config.login_server.secure_websocket.port = *i as u16,
                _ => panic!("Unknown id: {}", item.id),
            },
            ListValue::Bool(b) => match item.id.as_str() {
                "telnet_is_enabled" => icy_board.config.login_server.telnet.is_enabled = *b,
                "ssh_is_enabled" => icy_board.config.login_server.ssh.is_enabled = *b,
                "websocket_is_enabled" => icy_board.config.login_server.websocket.is_enabled = *b,
                "secure_websocket_is_enabled" => icy_board.config.login_server.secure_websocket.is_enabled = *b,
                _ => panic!("Unknown id: {}", item.id),
            },
            ListValue::Color(_c) => match item.id.as_str() {
                _ => panic!("Unknown id: {}", item.id),
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
        let area = area.inner(&Margin { horizontal: 2, vertical: 1 });

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
