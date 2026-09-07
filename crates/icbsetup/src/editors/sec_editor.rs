use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::editors::EditorList;
use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    Res,
    icy_board::{
        IcyBoard, IcyBoardSerializer,
        sec_levels::{SecurityLevel, SecurityLevelDefinitions},
    },
};
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ListItem, ListValue, ResultState, TextFlags},
    get_text,
    insert_table::{Column, InsertTable},
    tab_page::{Page, PageMessage},
};
use ratatui::{
    Frame,
    layout::{Margin, Rect},
    text::Line,
    widgets::{Clear, ScrollbarState, TableState, Widget},
};

pub struct SecurityLevelEditor<'a> {
    path: std::path::PathBuf,

    insert_table: InsertTable<'a>,
    sec_levels_orig: SecurityLevelDefinitions,
    sec_levels: Arc<Mutex<SecurityLevelDefinitions>>,

    detail: super::EditorDialog<(usize, Arc<Mutex<SecurityLevelDefinitions>>)>,
    save_changes: super::EditorSaveChanges,
}

impl<'a> SecurityLevelEditor<'a> {
    pub(crate) fn new(path: &std::path::PathBuf) -> Res<Self> {
        let sec_levels_orig = if path.exists() {
            SecurityLevelDefinitions::load(&path)?
        } else {
            SecurityLevelDefinitions {
                levels: vec![
                    SecurityLevel {
                        password: "".to_string(),
                        description: "Expired".to_string(),
                        security: 0,
                        base_baud_rate: 0,
                        batch_limit: 0,
                        uldl_ratio_tenths: 0,
                        uldl_kb_ratio_tenths: 0,
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
                        base_baud_rate: 0,
                        batch_limit: 0,
                        uldl_ratio_tenths: 0,
                        uldl_kb_ratio_tenths: 0,
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
                        base_baud_rate: 0,
                        batch_limit: 0,
                        uldl_ratio_tenths: 0,
                        uldl_kb_ratio_tenths: 0,
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
                ],
            }
        };
        let sec_levels = Arc::new(Mutex::new(sec_levels_orig.clone()));
        let scroll_state = ScrollbarState::default().content_length(sec_levels_orig.levels.len());
        let content_length = sec_levels_orig.levels.len();
        let cmd2 = sec_levels.clone();

        let insert_table = InsertTable {
            scroll_state,
            table_state: TableState::default().with_selected(0),
            columns: vec![
                Column::new(get_text("sec_level_header_security")).with_width(12),
                Column::new(get_text("sec_level_header_description")).with_width(40),
                Column::new(get_text("sec_level_header_time")),
            ],
            numbered: true,
            get_content: Box::new(move |_table, i, j| {
                if *i >= cmd2.lock().unwrap().len() {
                    return Line::from("".to_string());
                }
                match j {
                    0 => Line::from(format!("{}", cmd2.lock().unwrap()[*i].security)),
                    1 => Line::from(cmd2.lock().unwrap()[*i].description.to_string()),
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
            detail: super::EditorDialog::default(),
            save_changes: super::EditorSaveChanges::default(),
        })
    }
}

impl<'a> Page for SecurityLevelEditor<'a> {
    fn request_status(&self) -> ResultState {
        self.detail.status()
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());

        let block = super::list_editor_frame(
            get_text("sec_level_editor_title"),
            get_text("icb_setup_key_conf_list_help"),
            self.detail.is_open() || self.save_changes.is_open(),
        );
        block.render(area, frame.buffer_mut());
        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        self.insert_table.render_list(frame, area);

        if self.detail.is_open() {
            let mut area = area.inner(Margin { vertical: 2, horizontal: 3 });
            area.height += 1;
            self.detail.render(frame, area, get_text("sec_level_editor_editor"), String::new());
        }
        self.save_changes.render(frame, area);
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if let Some(message) = self.save_changes.handle_key(key, || {
            crate::editors::save_file(&self.path, || self.sec_levels.lock().unwrap().save(&self.path))
        }) {
            return message;
        }
        if let Some(message) = self.detail.handle_key(key) {
            return message;
        }

        match key.code {
            KeyCode::Esc => {
                return self.save_changes.request_close(self.sec_levels_orig != *self.sec_levels.lock().unwrap());
            }
            _ => match key.code {
                KeyCode::PageUp => self.insert_table.move_row(&mut self.sec_levels.lock().unwrap(), -1),
                KeyCode::PageDown => self.insert_table.move_row(&mut self.sec_levels.lock().unwrap(), 1),

                KeyCode::Insert => {
                    self.insert_table.push_row(
                        &mut *self.sec_levels.lock().unwrap(),
                        SecurityLevel {
                            description: "New Sec Level".to_string(),
                            password: "".to_string(),
                            security: 0,
                            base_baud_rate: 0,
                            batch_limit: 0,
                            uldl_ratio_tenths: 0,
                            uldl_kb_ratio_tenths: 0,
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
                    );
                }
                KeyCode::Delete => {
                    self.insert_table.remove_row(&mut *self.sec_levels.lock().unwrap());
                }

                KeyCode::Enter => {
                    if let Some(selected_item) = self.insert_table.table_state.selected() {
                        let cmd = self.sec_levels.lock().unwrap();
                        let Some(action) = cmd.get(selected_item) else {
                            return PageMessage::None;
                        };
                        self.detail.open(super::align_editor_labels(ConfigMenu {
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
                                    ListItem::new(get_text("sec_level_editor_time_per_day"), ListValue::U32(action.time_per_day, 0, u32::MAX))
                                        .with_label_width(16)
                                        .with_update_u32_value(&|(i, list): &(usize, Arc<Mutex<SecurityLevelDefinitions>>), value: u32| {
                                            list.lock().unwrap()[*i].time_per_day = value;
                                        }),
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
                                    ListItem::new(get_text("sec_level_editor_file_ratio"), ListValue::U32(action.uldl_ratio_tenths, 0, u32::MAX))
                                        .with_label_width(16)
                                        .with_update_u32_value(&|(i, list): &(usize, Arc<Mutex<SecurityLevelDefinitions>>), value: u32| {
                                            list.lock().unwrap()[*i].uldl_ratio_tenths = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("sec_level_editor_byte_ratio"),
                                        ListValue::U32(action.uldl_kb_ratio_tenths, 0, u32::MAX),
                                    )
                                    .with_label_width(16)
                                    .with_update_u32_value(
                                        &|(i, list): &(usize, Arc<Mutex<SecurityLevelDefinitions>>), value: u32| {
                                            list.lock().unwrap()[*i].uldl_kb_ratio_tenths = value;
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
                        }));
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
