use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    text_field::{TextField, TextfieldState},
    theme::THEME,
    TerminalType,
};
use ratatui::{
    layout::{Margin, Rect},
    style::Stylize,
    text::Text,
    widgets::{Block, BorderType, Borders, Clear, Padding, Widget},
    Frame,
};

use crate::app::ResultState;

use super::TabPage;

pub struct GeneralTab {
    state: ConfigMenuState,
    config: ConfigMenu,
}

pub enum ListValue {
    Text(u16, String),
    File(u16, String),
    U8(u8),
    Bool(bool),
    ValueList(String, Vec<String>),
}

pub struct ListItem {
    _id: String,
    title: String,
    status: String,
    text_field_state: TextfieldState,
    edit_pos: Arc<Mutex<(u16, u16)>>,
    value: ListValue,
}

impl ListItem {
    fn new(id: &str, title: String, value: ListValue) -> Self {
        Self {
            _id: id.to_string(),
            status: format!("{}", title),
            text_field_state: TextfieldState::default(),
            title,
            edit_pos: Arc::new(Mutex::new((0, 0))),
            value,
        }
    }

    fn with_status(mut self, status: &str) -> Self {
        self.status = status.to_string();
        self
    }

    fn render(&self, area: Rect, frame: &mut Frame) {
        self.edit_pos.lock().unwrap().0 = area.x;
        self.edit_pos.lock().unwrap().1 = area.y;

        match &self.value {
            ListValue::Text(edit_len, text) | ListValue::File(edit_len, text) => {
                let mut area = area;
                area.width = *edit_len;
                Text::from(text.clone()).style(THEME.value).render(area, frame.buffer_mut());
            }

            ListValue::U8(u8) => {
                Text::from(u8.to_string()).style(THEME.value).render(area, frame.buffer_mut());
            }
            ListValue::Bool(value) => {
                Text::from(if *value { "Yes" } else { "No" })
                    .style(THEME.value)
                    .render(area, frame.buffer_mut());
            }
            ListValue::ValueList(cur_value, _) => {
                Text::from(cur_value.clone()).style(THEME.value).render(area, frame.buffer_mut());
            }
        }
    }

    fn render_editor(&mut self, area: Rect, frame: &mut Frame) {
        match &self.value {
            ListValue::Text(_edit_len, text) | ListValue::File(_edit_len, text) => {
                let field = TextField::new().with_value(text.to_string());
                frame.render_stateful_widget(field, area, &mut self.text_field_state);
            }
            ListValue::U8(_value) => {
                self.render(area, frame);
            }
            ListValue::Bool(_value) => {
                self.render(area, frame);
            }
            ListValue::ValueList(_cur_value, _) => {
                self.render(area, frame);
            }
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> ResultState {
        match key {
            KeyEvent { code: KeyCode::Enter, .. } => {
                return ResultState {
                    in_edit_mode: false,
                    status_line: self.status.clone(),
                };
            }
            KeyEvent { code: KeyCode::Esc, .. } => {
                return ResultState {
                    in_edit_mode: false,
                    status_line: self.status.clone(),
                };
            }
            _ => {}
        }

        match &mut self.value {
            ListValue::File(_edit_len, text) | ListValue::Text(_edit_len, text) => {
                self.text_field_state.handle_input(key, text);
            }
            ListValue::Bool(_) | ListValue::U8(_) | ListValue::ValueList(_, _) => {}
        }
        ResultState {
            in_edit_mode: true,
            status_line: self.status.clone(),
        }
    }
}

pub enum ConfigEntry {
    Item(ListItem),
    Group(String, Vec<ConfigEntry>),
    Table(usize, Vec<ConfigEntry>),
    Separator,
}

pub struct ConfigMenu {
    items: Vec<ConfigEntry>,
}

#[derive(Default)]
pub struct ConfigMenuState {
    selected: usize,
    in_edit: bool,
    first_row: u16,
    area_height: u16,

    item_pos: HashMap<usize, u16>,
}

impl ConfigMenu {
    pub fn render(&mut self, area: Rect, frame: &mut Frame, state: &mut ConfigMenuState) {
        let max = 16; //self.items.iter().map(|item| item.title.len()).max().unwrap_or(0);

        let mut y = 0;
        let mut x = 0;
        let mut i = 0;

        state.area_height = area.height;

        Self::display_list(&mut i, &mut self.items, area, max, &mut y, &mut x, frame, state);
    }

    pub fn count(&self) -> usize {
        let mut len = 0;
        self.count_items(&self.items, &mut len);
        len
    }

    pub fn get_item(&self, i: usize) -> Option<&ListItem> {
        let mut len = 0;
        Self::get_item_internal(&self.items, &mut len, i)
    }

    pub fn get_item_internal<'a>(items: &'a Vec<ConfigEntry>, len: &mut usize, i: usize) -> Option<&'a ListItem> {
        for item in items.iter() {
            match item {
                ConfigEntry::Item(item) => {
                    if *len == i {
                        return Some(item);
                    }
                    *len += 1;
                }
                ConfigEntry::Group(_t, items) => {
                    let res = Self::get_item_internal(items, len, i);
                    if res.is_some() {
                        return res;
                    }
                }
                _ => {}
            }
        }

        None
    }

    pub fn get_item_mut(&mut self, i: usize) -> Option<&mut ListItem> {
        let mut len = 0;
        Self::get_item_internal_mut(&mut self.items, &mut len, i)
    }

    pub fn get_item_internal_mut<'a>(items: &'a mut Vec<ConfigEntry>, len: &mut usize, i: usize) -> Option<&'a mut ListItem> {
        for item in items.iter_mut() {
            match item {
                ConfigEntry::Item(item) => {
                    if *len == i {
                        return Some(item);
                    }
                    *len += 1;
                }
                ConfigEntry::Group(_t, items) => {
                    let res = Self::get_item_internal_mut(items, len, i);
                    if res.is_some() {
                        return res;
                    }
                }
                _ => {}
            }
        }

        None
    }

    fn count_items(&self, items: &Vec<ConfigEntry>, len: &mut usize) {
        for item in items.iter() {
            match item {
                ConfigEntry::Item(_) => {
                    *len += 1;
                }
                ConfigEntry::Group(_t, items) => {
                    self.count_items(items, len);
                }
                _ => {}
            }
        }
    }

    pub fn display_list(
        i: &mut usize,
        items: &mut Vec<ConfigEntry>,
        area: Rect,
        max: u16,
        y: &mut u16,
        x: &mut u16,
        frame: &mut Frame,
        state: &mut ConfigMenuState,
    ) {
        for item in items.iter_mut() {
            match item {
                ConfigEntry::Item(item) => {
                    if *y >= state.first_row && *y < area.height + state.first_row {
                        let left_area = Rect {
                            x: area.x + *x,
                            y: area.y + *y - state.first_row,
                            width: max as u16 + 2,
                            height: 1,
                        };

                        Text::from(format!(" {}:", item.title.clone()))
                            .alignment(ratatui::layout::Alignment::Right)
                            .style(if *i == state.selected && !state.in_edit {
                                THEME.selected_item
                            } else {
                                THEME.item
                            })
                            .render(left_area, frame.buffer_mut());

                        let right_area = Rect {
                            x: area.x + *x + max + 3,
                            y: area.y + *y - state.first_row,
                            width: area.width - (*x + max + 3) - 2,
                            height: 1,
                        };
                        if state.in_edit && *i == state.selected {
                            item.render_editor(right_area, frame);
                        } else {
                            item.render(right_area, frame);
                        }
                    }

                    state.item_pos.insert(*i, *y);
                    *y += 1;
                    *i += 1;
                }
                ConfigEntry::Group(title, items) => {
                    if *y >= state.first_row && *y < area.height + state.first_row {
                        let left_area = Rect {
                            x: area.x + *x,
                            y: area.y + *y - state.first_row,
                            width: area.width - *x - 1,
                            height: 1,
                        };
                        Text::from(format!(" {}", title.clone()))
                            .alignment(ratatui::layout::Alignment::Left)
                            .style(THEME.config_title.italic())
                            .render(left_area, frame.buffer_mut());
                    }
                    *y += 1;
                    Self::display_list(i, items, area, max, y, x, frame, state);
                }
                ConfigEntry::Table(_rows, _items) => {
                    todo!()
                }
                ConfigEntry::Separator => {
                    /*
                    let left_area = Rect {
                        x: area.x,
                        y: area.y + *y - state.top_row,
                        width: area.width - 2,
                        height: 1,
                    };

                    Text::from("-".repeat(area.width as usize))
                    .alignment(ratatui::layout::Alignment::Right)
                    .style(
                        THEME.item
                    )
                    .render(left_area, buf);*/
                    *y += 1;
                }
            }
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent, state: &ConfigMenuState) -> ResultState {
        let res = self.get_item_mut(state.selected).unwrap().handle_key_press(key);
        res
    }

    fn request_edit_mode(&self, _state: &ConfigMenuState) -> ResultState {
        ResultState {
            in_edit_mode: true,
            status_line: "".to_string(),
        }
    }
}

impl GeneralTab {
    pub fn new(icy_board: Arc<IcyBoard>) -> Self {
        let sysop_info = vec![
            ConfigEntry::Item(
                ListItem::new(
                    "sysop_name",
                    "Sysop's Name".to_string(),
                    ListValue::Text(25, icy_board.config.sysop.name.clone()),
                )
                .with_status("Enter the first name of the sysop."),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "local_pass",
                    "Local Password".to_string(),
                    ListValue::Text(25, icy_board.config.sysop.password.to_string().clone()),
                )
                .with_status("Call waiting screen password."),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "local_pass_exit",
                    "Require Password to Exit".to_string(),
                    ListValue::Bool(icy_board.config.sysop.require_password_to_exit),
                )
                .with_status("IcyBoard requires pw to exit the call waiting screen."),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "use_real_name",
                    "Use Real Name".to_string(),
                    ListValue::Bool(icy_board.config.sysop.use_real_name),
                )
                .with_status("Message to sysop with real name?"),
            ),
        ];

        let board_info = vec![
            ConfigEntry::Item(
                ListItem::new("board_name", "Board Name".to_string(), ListValue::Text(54, icy_board.config.board.name.clone()))
                    .with_status("Board name is shown on login to the caller."),
            ),
            ConfigEntry::Item(
                ListItem::new("location", "Location".to_string(), ListValue::Text(54, icy_board.config.board.location.clone()))
                    .with_status("Board location used in IEMSI"),
            ),
            ConfigEntry::Item(
                ListItem::new("operator", "Operator".to_string(), ListValue::Text(30, icy_board.config.board.operator.clone()))
                    .with_status("Board operator used in IEMSI"),
            ),
            ConfigEntry::Item(
                ListItem::new("notice", "Notice".to_string(), ListValue::Text(30, icy_board.config.board.notice.clone()))
                    .with_status("Board notice used in IEMSI"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "capabilities",
                    "Capabilities".to_string(),
                    ListValue::Text(30, icy_board.config.board.capabilities.clone()),
                )
                .with_status("Board capabilities used in IEMSI"),
            ),
            ConfigEntry::Separator,
            ConfigEntry::Item(
                ListItem::new(
                    "date_format",
                    "Date Format".to_string(),
                    ListValue::Text(25, icy_board.config.board.date_format.clone()),
                )
                .with_status("Default date format"),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "num_nodes",
                    "# Nodes".to_string(),
                    ListValue::Text(8, icy_board.config.board.num_nodes.to_string()),
                )
                .with_status("Numer of active nodes"),
            ),
        ];

        let new_user_info = vec![
            ConfigEntry::Item(ListItem::new(
                "sec_level",
                "Security Level".to_string(),
                ListValue::U8(icy_board.config.new_user_settings.sec_level),
            )),
            ConfigEntry::Item(ListItem::new(
                "new_user_groups",
                "Groups".to_string(),
                ListValue::Text(40, icy_board.config.new_user_settings.new_user_groups.clone()),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_city_or_state",
                "City or State".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_city_or_state),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_address",
                "Address".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_address),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_verification",
                "Verification".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_verification),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_bus_data_phone",
                "Bus Phone".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_bus_data_phone),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_voice_phone",
                "Home Phone".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_voice_phone),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_comment",
                "Comment".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_comment),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_clr_msg",
                "MsgClear".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_clr_msg),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_xfer_protocol",
                "Protocols".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_xfer_protocol),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_date_format",
                "Date Format".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_date_format),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_alias",
                "Alias".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_alias),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_gender",
                "Gender".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_gender),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_birthdate",
                "Birthdate".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_birthdate),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_email",
                "Email".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_email),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_web_address",
                "Web Address".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_web_address),
            )),
            ConfigEntry::Item(ListItem::new(
                "ask_use_short_descr",
                "Short File Descr".to_string(),
                ListValue::Bool(icy_board.config.new_user_settings.ask_use_short_descr),
            )),
        ];

        Self {
            state: ConfigMenuState::default(),
            config: ConfigMenu {
                items: vec![
                    ConfigEntry::Group("Sysop Information".to_string(), sysop_info),
                    ConfigEntry::Group("Board Information".to_string(), board_info),
                    ConfigEntry::Group("New User Settings".to_string(), new_user_info),
                ],
            },
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

impl TabPage for GeneralTab {
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
