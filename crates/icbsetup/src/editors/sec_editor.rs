use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    icy_board::{
        sec_levels::{SecurityLevel, SecurityLevelDefinitions},
        IcyBoardSerializer,
    },
    Res,
};
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue},
    insert_table::InsertTable,
    tab_page::Editor,
    theme::get_tui_theme,
};
use ratatui::{
    layout::{Alignment, Margin, Rect},
    text::{Line, Span},
    widgets::{block::Title, Block, BorderType, Borders, Clear, Padding, ScrollbarState, TableState, Widget},
    Frame,
};

pub struct SecurityLevelEditor<'a> {
    path: std::path::PathBuf,
    door_list: SecurityLevelDefinitions,

    insert_table: InsertTable<'a>,
    sec_levels: Arc<Mutex<Vec<SecurityLevel>>>,

    edit_config_state: ConfigMenuState,
    edit_config: Option<ConfigMenu>,
}

impl<'a> SecurityLevelEditor<'a> {
    pub(crate) fn new(path: &std::path::PathBuf) -> Res<Self> {
        let sec_levels = if path.exists() {
            SecurityLevelDefinitions::load(&path)?
        } else {
            let mut sec_levels = SecurityLevelDefinitions::default();
            sec_levels.levels = vec![
                SecurityLevel {
                    password: "".to_string(),
                    description: "Expired".to_string(),
                    security: 0,
                    uldl_ratio: 0,
                    uldl_kb_ratio: 0,
                    daily_file_limit: 0,
                    daily_file_kb_limit: 0,
                    file_limit: 0,
                    file_kb_limit: 0,
                    file_credit: 0,
                    file_kb_credit: 0,
                    time_per_day: 0,
                    calls_per_day: 0,
                    enforce_time_limit: true,
                    allow_alias: false,
                    enforce_read_mail: false,
                    is_demo_account: false,
                    is_enabled: true,
                },
                SecurityLevel {
                    password: "".to_string(),
                    description: "User".to_string(),
                    security: 10,
                    uldl_ratio: 0,
                    uldl_kb_ratio: 0,
                    daily_file_limit: 0,
                    daily_file_kb_limit: 0,
                    file_limit: 0,
                    file_kb_limit: 0,
                    file_credit: 0,
                    file_kb_credit: 0,
                    time_per_day: 0,
                    calls_per_day: 0,
                    enforce_time_limit: true,
                    allow_alias: true,
                    enforce_read_mail: false,
                    is_demo_account: false,
                    is_enabled: true,
                },
                SecurityLevel {
                    password: "".to_string(),
                    description: "Sysop".to_string(),
                    security: 100,
                    uldl_ratio: 0,
                    uldl_kb_ratio: 0,
                    daily_file_limit: 0,
                    daily_file_kb_limit: 0,
                    file_limit: 0,
                    file_kb_limit: 0,
                    file_credit: 0,
                    file_kb_credit: 0,
                    time_per_day: 0,
                    calls_per_day: 0,
                    enforce_time_limit: true,
                    allow_alias: true,
                    enforce_read_mail: false,
                    is_demo_account: false,
                    is_enabled: true,
                },
            ];
            sec_levels
        };
        let command_arc = Arc::new(Mutex::new(sec_levels.levels.clone()));
        let scroll_state = ScrollbarState::default().content_length(sec_levels.levels.len());
        let content_length = sec_levels.levels.len();
        let cmd2 = command_arc.clone();
        let insert_table = InsertTable {
            scroll_state,
            table_state: TableState::default().with_selected(0),
            headers: vec!["Security    ".to_string(), "Description".to_string(), "Time".to_string()],
            get_content: Box::new(move |_table, i, j| {
                if *i >= cmd2.lock().unwrap().len() {
                    return Line::from("".to_string());
                }
                match j {
                    0 => Line::from(format!("{}", cmd2.lock().unwrap()[*i].security)),
                    1 => Line::from(format!("{}", cmd2.lock().unwrap()[*i].description)),
                    2 => Line::from(format!("{}", cmd2.lock().unwrap()[*i].time_per_day)),
                    _ => Line::from("".to_string()),
                }
            }),
            content_length,
        };

        Ok(Self {
            path: path.clone(),
            door_list: sec_levels,
            insert_table,
            sec_levels: command_arc,
            edit_config: None,
            edit_config_state: ConfigMenuState::default(),
        })
    }

    fn display_insert_table(&mut self, frame: &mut Frame, area: &Rect) {
        let sel = self.insert_table.table_state.selected();
        self.insert_table.render_table(frame, *area);
        self.insert_table.table_state.select(sel);
    }

    fn move_up(&mut self) {
        if let Some(selected) = self.insert_table.table_state.selected() {
            if selected > 0 {
                let mut levels = self.sec_levels.lock().unwrap();
                levels.swap(selected, selected - 1);
                self.insert_table.table_state.select(Some(selected - 1));
            }
        }
    }

    fn move_down(&mut self) {
        if let Some(selected) = self.insert_table.table_state.selected() {
            if selected + 1 < self.sec_levels.lock().unwrap().len() {
                let mut levels = self.sec_levels.lock().unwrap();
                levels.swap(selected, selected + 1);
                self.insert_table.table_state.select(Some(selected + 1));
            }
        }
    }
}

impl<'a> Editor for SecurityLevelEditor<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        let block = Block::new()
            .title_alignment(Alignment::Center)
            .title(Title::from(Span::from(" Edit Security Levels ").style(get_tui_theme().content_box_title)))
            .style(get_tui_theme().content_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        block.render(area, frame.buffer_mut());
        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        self.display_insert_table(frame, &area);

        if let Some(edit_config) = &mut self.edit_config {
            let area = area.inner(Margin { vertical: 2, horizontal: 3 });
            Clear.render(area, frame.buffer_mut());
            let block = Block::new()
                .title_alignment(Alignment::Center)
                .title(Title::from(Span::from(" Edit Security Level ").style(get_tui_theme().content_box_title)))
                .style(get_tui_theme().content_box)
                .padding(Padding::new(2, 2, 1, 1))
                .borders(Borders::ALL)
                .border_type(BorderType::Double);
            //     let area =  footer.inner(&Margin { vertical: 15, horizontal: 5 });
            block.render(area, frame.buffer_mut());
            edit_config.render(area.inner(Margin { vertical: 1, horizontal: 1 }), frame, &mut self.edit_config_state);

            edit_config
                .get_item(self.edit_config_state.selected)
                .unwrap()
                .text_field_state
                .set_cursor_position(frame);
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> bool {
        if let Some(edit_config) = &mut self.edit_config {
            match key.code {
                KeyCode::Esc => {
                    let Some(selected_item) = self.insert_table.table_state.selected() else {
                        return true;
                    };
                    for item in edit_config.iter() {
                        match item.id.as_str() {
                            "security" => {
                                if let ListValue::U32(value, _, _) = item.value {
                                    self.sec_levels.lock().unwrap()[selected_item].security = value as u8;
                                }
                            }
                            "description" => {
                                if let ListValue::Text(_, value) = &item.value {
                                    self.sec_levels.lock().unwrap()[selected_item].description = value.to_string();
                                }
                            }
                            "password" => {
                                if let ListValue::Text(_, value) = &item.value {
                                    self.sec_levels.lock().unwrap()[selected_item].password = value.to_string();
                                }
                            }
                            "time_per_day" => {
                                if let ListValue::U32(value, _, _) = item.value {
                                    self.sec_levels.lock().unwrap()[selected_item].time_per_day = value as u32;
                                }
                            }
                            "daily_file_kb_limit" => {
                                if let ListValue::U32(value, _, _) = item.value {
                                    self.sec_levels.lock().unwrap()[selected_item].daily_file_kb_limit = value as u64;
                                }
                            }

                            "uldl_ratio" => {
                                if let ListValue::U32(value, _, _) = item.value {
                                    self.sec_levels.lock().unwrap()[selected_item].uldl_ratio = value as u32;
                                }
                            }
                            "uldl_kb_ratio" => {
                                if let ListValue::U32(value, _, _) = item.value {
                                    self.sec_levels.lock().unwrap()[selected_item].uldl_kb_ratio = value as u32;
                                }
                            }
                            "file_limit" => {
                                if let ListValue::U32(value, _, _) = item.value {
                                    self.sec_levels.lock().unwrap()[selected_item].file_limit = value as u64;
                                }
                            }
                            "file_kb_limit" => {
                                if let ListValue::U32(value, _, _) = item.value {
                                    self.sec_levels.lock().unwrap()[selected_item].file_kb_limit = value as u64;
                                }
                            }
                            "file_credit" => {
                                if let ListValue::U32(value, _, _) = item.value {
                                    self.sec_levels.lock().unwrap()[selected_item].file_credit = value as u64;
                                }
                            }
                            "file_kb_credit" => {
                                if let ListValue::U32(value, _, _) = item.value {
                                    self.sec_levels.lock().unwrap()[selected_item].file_kb_credit = value as u64;
                                }
                            }
                            "enforce_time_limit" => {
                                if let ListValue::Bool(value) = item.value {
                                    self.sec_levels.lock().unwrap()[selected_item].enforce_time_limit = value;
                                }
                            }
                            "allow_alias" => {
                                if let ListValue::Bool(value) = item.value {
                                    self.sec_levels.lock().unwrap()[selected_item].allow_alias = value;
                                }
                            }
                            "enforce_read_mail" => {
                                if let ListValue::Bool(value) = item.value {
                                    self.sec_levels.lock().unwrap()[selected_item].enforce_read_mail = value;
                                }
                            }
                            "is_demo_account" => {
                                if let ListValue::Bool(value) = item.value {
                                    self.sec_levels.lock().unwrap()[selected_item].is_demo_account = value;
                                }
                            }
                            "is_enabled" => {
                                if let ListValue::Bool(value) = item.value {
                                    self.sec_levels.lock().unwrap()[selected_item].is_enabled = value;
                                }
                            }
                            _ => {
                                panic!("Unknown item: {}", item.id);
                            }
                        }
                    }
                    self.edit_config = None;
                    return true;
                }
                _ => {
                    edit_config.handle_key_press(key, &mut self.edit_config_state);
                }
            }
            return true;
        }

        match key.code {
            KeyCode::Esc => {
                self.door_list.levels.clear();
                self.door_list.levels.append(&mut self.sec_levels.lock().unwrap());
                self.door_list.save(&self.path).unwrap();
                return false;
            }
            _ => match key.code {
                KeyCode::Char('1') => self.move_up(),
                KeyCode::Char('2') => self.move_down(),

                KeyCode::Insert => {
                    self.sec_levels.lock().unwrap().push(SecurityLevel {
                        description: "New Sec Level".to_string(),
                        password: "".to_string(),
                        security: 0,
                        uldl_ratio: 0,
                        uldl_kb_ratio: 0,
                        daily_file_limit: 0,
                        daily_file_kb_limit: 0,

                        file_limit: 0,
                        file_kb_limit: 0,
                        file_credit: 0,
                        file_kb_credit: 0,
                        time_per_day: 0,
                        calls_per_day: 0,
                        enforce_time_limit: true,
                        allow_alias: false,
                        enforce_read_mail: false,
                        is_demo_account: false,
                        is_enabled: true,
                    });
                    self.insert_table.content_length += 1;
                }
                KeyCode::Delete => {
                    if let Some(selected_item) = self.insert_table.table_state.selected() {
                        if selected_item < self.sec_levels.lock().unwrap().len() {
                            self.sec_levels.lock().unwrap().remove(selected_item);
                            self.insert_table.content_length -= 1;
                        }
                    }
                }

                KeyCode::Enter => {
                    self.edit_config_state = ConfigMenuState::default();

                    if let Some(selected_item) = self.insert_table.table_state.selected() {
                        let cmd = self.sec_levels.lock().unwrap();
                        let Some(action) = cmd.get(selected_item) else {
                            return true;
                        };
                        self.edit_config = Some(ConfigMenu {
                            entry: vec![
                                ConfigEntry::Item(
                                    ListItem::new("security", "Security".to_string(), ListValue::U32(action.security as u32, 0, 255)).with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new("description", "Description".to_string(), ListValue::Text(30, action.description.clone()))
                                        .with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new("password", "Password".to_string(), ListValue::Text(30, action.password.clone())).with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new("time_per_day", "Time".to_string(), ListValue::U32(action.time_per_day as u32, 0, u32::MAX))
                                        .with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        "daily_file_kb_limit",
                                        "Daily KBytes".to_string(),
                                        ListValue::U32(action.daily_file_kb_limit as u32, 0, u32::MAX),
                                    )
                                    .with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new("uldl_ratio", "File Ratio".to_string(), ListValue::U32(action.uldl_ratio as u32, 0, u32::MAX))
                                        .with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        "uldl_kb_ratio",
                                        "Byte Ratio".to_string(),
                                        ListValue::U32(action.uldl_kb_ratio as u32, 0, u32::MAX),
                                    )
                                    .with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new("file_limit", "File Limit".to_string(), ListValue::U32(action.file_limit as u32, 0, u32::MAX))
                                        .with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        "file_kb_limit",
                                        "KByte Limit".to_string(),
                                        ListValue::U32(action.file_kb_limit as u32, 0, u32::MAX),
                                    )
                                    .with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new("file_credit", "File Credit".to_string(), ListValue::U32(action.file_credit as u32, 0, u32::MAX))
                                        .with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        "file_kb_credit",
                                        "KByte Credit".to_string(),
                                        ListValue::U32(action.file_kb_credit as u32, 0, u32::MAX),
                                    )
                                    .with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        "enforce_time_limit",
                                        "Enforce Time Limit".to_string(),
                                        ListValue::Bool(action.enforce_time_limit),
                                    )
                                    .with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new("allow_alias", "Allow Alias".to_string(), ListValue::Bool(action.allow_alias)).with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new("enforce_read_mail", "Force Read Mail".to_string(), ListValue::Bool(action.enforce_read_mail))
                                        .with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new("is_demo_account", "Demo Account".to_string(), ListValue::Bool(action.is_demo_account)).with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new("is_enabled", "Enable Account".to_string(), ListValue::Bool(action.is_enabled)).with_label_width(16),
                                ),
                            ],
                        });
                    } else {
                        self.insert_table.handle_key_press(key).unwrap();
                    }
                }

                _ => {
                    self.insert_table.handle_key_press(key).unwrap();
                }
            },
        }
        true
    }
}
