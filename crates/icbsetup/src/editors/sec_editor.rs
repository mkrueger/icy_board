use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    Res,
    icy_board::{
        IcyBoard, IcyBoardSerializer,
        sec_levels::{SecurityLevel, SecurityLevelDefinitions},
    },
};
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, TextFlags},
    get_text,
    insert_table::InsertTable,
    save_changes_dialog::SaveChangesDialog,
    tab_page::{Page, PageMessage},
    theme::get_tui_theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, ScrollbarState, TableState, Widget, block::Title},
};

pub struct SecurityLevelEditor<'a> {
    path: std::path::PathBuf,

    insert_table: InsertTable<'a>,
    sec_levels_orig: SecurityLevelDefinitions,
    sec_levels: Arc<Mutex<SecurityLevelDefinitions>>,

    edit_config_state: ConfigMenuState,
    edit_config: Option<ConfigMenu<(usize, Arc<Mutex<SecurityLevelDefinitions>>)>>,
    save_dialog: Option<SaveChangesDialog>,
}

impl<'a> SecurityLevelEditor<'a> {
    pub(crate) fn new(path: &std::path::PathBuf) -> Res<Self> {
        let sec_levels_orig = if path.exists() {
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
        let sec_levels = Arc::new(Mutex::new(sec_levels_orig.clone()));
        let scroll_state = ScrollbarState::default().content_length(sec_levels_orig.levels.len());
        let content_length = sec_levels_orig.levels.len();
        let cmd2 = sec_levels.clone();

        let insert_table = InsertTable {
            scroll_state,
            table_state: TableState::default().with_selected(0),
            headers: vec![
                get_text("sec_level_header_security"),
                get_text("sec_level_header_description"),
                get_text("sec_level_header_time"),
            ],
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
            sec_levels_orig,
            insert_table,
            sec_levels,
            edit_config: None,
            edit_config_state: ConfigMenuState::default(),
            save_dialog: None,
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

impl<'a> Page for SecurityLevelEditor<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());

        let block = Block::new()
            .title_alignment(Alignment::Center)
            .title(Title::from(
                Span::from(get_text("sec_level_editor_title")).style(get_tui_theme().dialog_box_title),
            ))
            .style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_set(icy_board_tui::BORDER_SET)
            .title_bottom(Span::styled(get_text("icb_setup_key_conf_list_help"), get_tui_theme().key_binding));
        block.render(area, frame.buffer_mut());
        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        self.display_insert_table(frame, &area);

        if let Some(edit_config) = &mut self.edit_config {
            let mut area = area.inner(Margin { vertical: 2, horizontal: 3 });
            area.height += 1;
            Clear.render(area, frame.buffer_mut());
            let block = Block::new()
                .title_alignment(Alignment::Center)
                .title(Title::from(
                    Span::from(get_text("sec_level_editor_editor")).style(get_tui_theme().dialog_box_title),
                ))
                .style(get_tui_theme().dialog_box)
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
        if let Some(save_changes) = &self.save_dialog {
            save_changes.render(frame, area);
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if self.save_dialog.is_some() {
            let res = self.save_dialog.as_mut().unwrap().handle_key_press(key);
            return match res {
                icy_board_tui::save_changes_dialog::SaveChangesMessage::Cancel => {
                    self.save_dialog = None;
                    PageMessage::None
                }
                icy_board_tui::save_changes_dialog::SaveChangesMessage::Close => PageMessage::Close,
                icy_board_tui::save_changes_dialog::SaveChangesMessage::Save => {
                    if let Some(parent) = self.path.parent() {
                        if !parent.exists() {
                            std::fs::create_dir_all(parent).unwrap();
                        }
                    }
                    self.sec_levels.lock().unwrap().save(&self.path).unwrap();
                    PageMessage::Close
                }
                icy_board_tui::save_changes_dialog::SaveChangesMessage::None => PageMessage::None,
            };
        }
        if let Some(edit_config) = &mut self.edit_config {
            let res = edit_config.handle_key_press(key, &mut self.edit_config_state);
            if res.edit_msg == icy_board_tui::config_menu::EditMessage::Close {
                self.edit_config = None;
                return PageMessage::None;
            }
            return PageMessage::None;
        }

        match key.code {
            KeyCode::Esc => {
                if self.sec_levels_orig == self.sec_levels.lock().unwrap().clone() {
                    return PageMessage::Close;
                }
                self.save_dialog = Some(SaveChangesDialog::new());
                return PageMessage::None;
            }
            _ => match key.code {
                KeyCode::PageUp => self.move_up(),
                KeyCode::PageDown => self.move_down(),

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
                            return PageMessage::None;
                        };
                        self.edit_config = Some(ConfigMenu {
                            obj: (selected_item, self.sec_levels.clone()),
                            entry: vec![
                                ConfigEntry::Item(
                                    ListItem::new(get_text("sec_level_editor_security"), ListValue::U32(action.security as u32, 0, 255))
                                        .with_label_width(16)
                                        .with_update_u32_value(&|(i, list): &(usize, Arc<Mutex<SecurityLevelDefinitions>>), value: u32| {
                                            list.lock().unwrap()[*i].security = value as u8;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("sec_level_editor_description"),
                                        ListValue::Text(30, TextFlags::None, action.description.clone()),
                                    )
                                    .with_label_width(16)
                                    .with_update_text_value(
                                        &|(i, list): &(usize, Arc<Mutex<SecurityLevelDefinitions>>), value: String| {
                                            list.lock().unwrap()[*i].description = value;
                                        },
                                    ),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("sec_level_editor_password"),
                                        ListValue::Text(30, TextFlags::None, action.password.clone()),
                                    )
                                    .with_label_width(16)
                                    .with_update_text_value(
                                        &|(i, list): &(usize, Arc<Mutex<SecurityLevelDefinitions>>), value: String| {
                                            list.lock().unwrap()[*i].password = value;
                                        },
                                    ),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("sec_level_editor_time_per_day"),
                                        ListValue::U32(action.time_per_day as u32, 0, u32::MAX),
                                    )
                                    .with_label_width(16)
                                    .with_update_u32_value(
                                        &|(i, list): &(usize, Arc<Mutex<SecurityLevelDefinitions>>), value: u32| {
                                            list.lock().unwrap()[*i].time_per_day = value;
                                        },
                                    ),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("sec_level_editor_daily_bytes"),
                                        ListValue::U32(action.daily_file_kb_limit as u32, 0, u32::MAX),
                                    )
                                    .with_label_width(16)
                                    .with_update_u32_value(
                                        &|(i, list): &(usize, Arc<Mutex<SecurityLevelDefinitions>>), value: u32| {
                                            list.lock().unwrap()[*i].daily_file_kb_limit = value as u64;
                                        },
                                    ),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(get_text("sec_level_editor_file_ratio"), ListValue::U32(action.uldl_ratio as u32, 0, u32::MAX))
                                        .with_label_width(16)
                                        .with_update_u32_value(&|(i, list): &(usize, Arc<Mutex<SecurityLevelDefinitions>>), value: u32| {
                                            list.lock().unwrap()[*i].uldl_ratio = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("sec_level_editor_byte_ratio"),
                                        ListValue::U32(action.uldl_kb_ratio as u32, 0, u32::MAX),
                                    )
                                    .with_label_width(16)
                                    .with_update_u32_value(
                                        &|(i, list): &(usize, Arc<Mutex<SecurityLevelDefinitions>>), value: u32| {
                                            list.lock().unwrap()[*i].uldl_kb_ratio = value;
                                        },
                                    ),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(get_text("sec_level_editor_file_limit"), ListValue::U32(action.file_limit as u32, 0, u32::MAX))
                                        .with_label_width(16)
                                        .with_update_u32_value(&|(i, list): &(usize, Arc<Mutex<SecurityLevelDefinitions>>), value: u32| {
                                            list.lock().unwrap()[*i].file_limit = value as u64;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(get_text("sec_level_editor_kb_limit"), ListValue::U32(action.file_kb_limit as u32, 0, u32::MAX))
                                        .with_label_width(16)
                                        .with_update_u32_value(&|(i, list): &(usize, Arc<Mutex<SecurityLevelDefinitions>>), value: u32| {
                                            list.lock().unwrap()[*i].file_kb_limit = value as u64;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(get_text("sec_level_editor_file_credit"), ListValue::U32(action.file_credit as u32, 0, u32::MAX))
                                        .with_label_width(16)
                                        .with_update_u32_value(&|(i, list): &(usize, Arc<Mutex<SecurityLevelDefinitions>>), value: u32| {
                                            list.lock().unwrap()[*i].file_credit = value as u64;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("sec_level_editor_kb_credit"),
                                        ListValue::U32(action.file_kb_credit as u32, 0, u32::MAX),
                                    )
                                    .with_label_width(16)
                                    .with_update_u32_value(
                                        &|(i, list): &(usize, Arc<Mutex<SecurityLevelDefinitions>>), value: u32| {
                                            list.lock().unwrap()[*i].file_kb_credit = value as u64;
                                        },
                                    ),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(get_text("sec_level_editor_enforce_time"), ListValue::Bool(action.enforce_time_limit))
                                        .with_label_width(16)
                                        .with_update_bool_value(&|(i, list): &(usize, Arc<Mutex<SecurityLevelDefinitions>>), value: bool| {
                                            list.lock().unwrap()[*i].enforce_time_limit = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(get_text("sec_level_editor_allow_alias"), ListValue::Bool(action.allow_alias))
                                        .with_label_width(16)
                                        .with_update_bool_value(&|(i, list): &(usize, Arc<Mutex<SecurityLevelDefinitions>>), value: bool| {
                                            list.lock().unwrap()[*i].allow_alias = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(get_text("sec_level_force_read_mail"), ListValue::Bool(action.enforce_read_mail))
                                        .with_label_width(16)
                                        .with_update_bool_value(&|(i, list): &(usize, Arc<Mutex<SecurityLevelDefinitions>>), value: bool| {
                                            list.lock().unwrap()[*i].enforce_read_mail = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(get_text("sec_level_demo_acc"), ListValue::Bool(action.is_demo_account))
                                        .with_label_width(16)
                                        .with_update_bool_value(&|(i, list): &(usize, Arc<Mutex<SecurityLevelDefinitions>>), value: bool| {
                                            list.lock().unwrap()[*i].is_demo_account = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(get_text("sec_level_enable_acc"), ListValue::Bool(action.is_enabled))
                                        .with_label_width(16)
                                        .with_update_bool_value(&|(i, list): &(usize, Arc<Mutex<SecurityLevelDefinitions>>), value: bool| {
                                            list.lock().unwrap()[*i].is_enabled = value;
                                        }),
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
        PageMessage::None
    }
}

pub fn edit_sec(_board: Arc<Mutex<IcyBoard>>, path: PathBuf) -> PageMessage {
    PageMessage::OpenSubPage(Box::new(SecurityLevelEditor::new(&path).unwrap()))
}
