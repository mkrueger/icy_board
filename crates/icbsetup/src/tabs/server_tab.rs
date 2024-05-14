use std::sync::Arc;
use std::sync::Mutex;

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::config_menu::EditMode;
use icy_board_tui::tab_page::TabPage;
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState},
    theme::THEME,
};
use ratatui::{
    layout::{Margin, Rect},
    widgets::{Block, BorderType, Borders, Clear, Padding, Widget},
    Frame,
};

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
                entry: vec![
                    ConfigEntry::Group("Terminal (TELNET)".to_string(), telnet),
                    ConfigEntry::Group("Terminal (SSH)".to_string(), ssh),
                    ConfigEntry::Group("Terminal (WEBSOCKET)".to_string(), websocket),
                    ConfigEntry::Group("Terminal (SECURE WEBSOCKET)".to_string(), secure_websocket),
                ],
            },
        }
    }

    fn write_back(&self, icy_board: &mut IcyBoard) {
        for entry in self.config.entry.iter() {
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
            _ => todo!(),
        }
    }
}

impl TabPage for ServerTab {
    fn title(&self) -> String {
        "Server".to_string()
    }

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
        if self.state.in_edit {
            self.set_cursor_position(frame);
        }
    }

    fn has_control(&self) -> bool {
        self.state.in_edit
    }

    fn set_cursor_position(&self, frame: &mut Frame) {
        self.config.get_item(self.state.selected).unwrap().text_field_state.set_cursor_position(frame);
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> ResultState {
        let res = self.config.handle_key_press(key, &mut self.state);
        if self.state.in_edit {
            self.write_back(&mut self.icy_board.lock().unwrap());
        }
        res
    }

    fn request_status(&self) -> ResultState {
        return ResultState {
            edit_mode: EditMode::None,
            status_line: if self.state.selected < self.config.entry.len() {
                "".to_string()
            } else {
                "".to_string()
            },
        };
    }
}
