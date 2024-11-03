use std::{
    str::FromStr,
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    icy_board::{
        file_directory::{DirectoryList, FileDirectory, SortDirection, SortOrder},
        security_expr::SecurityExpression,
        user_base::Password,
        IcyBoardSerializer,
    },
    Res,
};
use icy_board_tui::{
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue},
    insert_table::InsertTable,
    tab_page::Editor,
    theme::THEME,
};
use ratatui::{
    layout::{Alignment, Margin, Rect},
    text::{Line, Span},
    widgets::{block::Title, Block, BorderType, Borders, Clear, Padding, ScrollbarState, TableState, Widget},
    Frame,
};

pub struct DirsEditor<'a> {
    path: std::path::PathBuf,

    insert_table: InsertTable<'a>,
    dir_list: Arc<Mutex<DirectoryList>>,

    edit_config_state: ConfigMenuState,
    edit_config: Option<ConfigMenu>,
}

impl<'a> DirsEditor<'a> {
    pub(crate) fn new(path: &std::path::PathBuf) -> Res<Self> {
        let surveys = if path.exists() {
            DirectoryList::load(&path)?
        } else {
            DirectoryList::default()
        };
        let dir_list = Arc::new(Mutex::new(surveys.clone()));
        let scroll_state = ScrollbarState::default().content_length(surveys.len());
        let content_length = surveys.len();
        let dl2 = dir_list.clone();
        let insert_table = InsertTable {
            scroll_state,
            table_state: TableState::default().with_selected(0),
            headers: vec!["".to_string(), "Name            ".to_string(), "Path".to_string()],
            get_content: Box::new(move |_table, i, j| {
                if *i >= dl2.lock().unwrap().len() {
                    return Line::from("".to_string());
                }
                match j {
                    0 => Line::from(format!("{})", *i + 1)),
                    1 => Line::from(format!("{}", dl2.lock().unwrap()[*i].name)),
                    2 => Line::from(format!("{}", dl2.lock().unwrap()[*i].path.display())),
                    _ => Line::from("".to_string()),
                }
            }),
            content_length,
        };

        Ok(Self {
            path: path.clone(),
            insert_table,
            dir_list,
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
                let mut levels = self.dir_list.lock().unwrap();
                levels.swap(selected, selected - 1);
                self.insert_table.table_state.select(Some(selected - 1));
            }
        }
    }

    fn move_down(&mut self) {
        if let Some(selected) = self.insert_table.table_state.selected() {
            if selected + 1 < self.dir_list.lock().unwrap().len() {
                let mut levels = self.dir_list.lock().unwrap();
                levels.swap(selected, selected + 1);
                self.insert_table.table_state.select(Some(selected + 1));
            }
        }
    }
}

impl<'a> Editor for DirsEditor<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        let block = Block::new()
            .title_alignment(Alignment::Center)
            .title(Title::from(Span::from(" File Directories ").style(THEME.content_box_title)))
            .style(THEME.content_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        block.render(area, frame.buffer_mut());
        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        self.display_insert_table(frame, &area);

        if let Some(edit_config) = &mut self.edit_config {
            let area = area.inner(Margin { vertical: 3, horizontal: 3 });
            Clear.render(area, frame.buffer_mut());
            let block = Block::new()
                .title_alignment(Alignment::Center)
                .title(Title::from(Span::from(" Edit Directory ").style(THEME.content_box_title)))
                .style(THEME.content_box)
                .padding(Padding::new(2, 2, 1, 1))
                .borders(Borders::ALL)
                .border_type(BorderType::Double);
            //     let area =  footer.inner(&Margin { vertical: 15, horizontal: 5 });
            block.render(area, frame.buffer_mut());
            edit_config.render(area.inner(Margin { vertical: 1, horizontal: 1 }), frame, &mut self.edit_config_state);

            if self.edit_config_state.in_edit {
                edit_config
                    .get_item(self.edit_config_state.selected)
                    .unwrap()
                    .text_field_state
                    .set_cursor_position(frame);
            }
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
                            "name" => {
                                if let ListValue::Text(_, text) = &item.value {
                                    self.dir_list.lock().unwrap()[selected_item].name = text.to_string();
                                }
                            }

                            "path" => {
                                if let ListValue::Path(path) = &item.value {
                                    self.dir_list.lock().unwrap()[selected_item].path = path.clone();
                                }
                            }

                            "file_base" => {
                                if let ListValue::Path(path) = &item.value {
                                    self.dir_list.lock().unwrap()[selected_item].file_base = path.clone();
                                }
                            }

                            "password" => {
                                if let ListValue::Text(_, text) = &item.value {
                                    self.dir_list.lock().unwrap()[selected_item].password = Password::PlainText(text.to_string());
                                }
                            }

                            "sort_order" => {
                                if let ListValue::ComboBox(combo) = &item.value {
                                    let value = &combo.cur_value.value;
                                    if let Ok(sort_order) = SortOrder::from_str(value) {
                                        self.dir_list.lock().unwrap()[selected_item].sort_order = sort_order;
                                    }
                                }
                            }

                            "sort_direction" => {
                                if let ListValue::Bool(value) = &item.value {
                                    self.dir_list.lock().unwrap()[selected_item].sort_direction =
                                        if *value { SortDirection::Ascending } else { SortDirection::Descending };
                                }
                            }

                            "has_new_files" => {
                                if let ListValue::Bool(value) = &item.value {
                                    self.dir_list.lock().unwrap()[selected_item].has_new_files = *value;
                                }
                            }

                            "is_readonly" => {
                                if let ListValue::Bool(value) = &item.value {
                                    self.dir_list.lock().unwrap()[selected_item].is_readonly = *value;
                                }
                            }

                            "is_free" => {
                                if let ListValue::Bool(value) = &item.value {
                                    self.dir_list.lock().unwrap()[selected_item].is_free = *value;
                                }
                            }

                            "allow_ul_pwd" => {
                                if let ListValue::Bool(value) = &item.value {
                                    self.dir_list.lock().unwrap()[selected_item].allow_ul_pwd = *value;
                                }
                            }

                            "list_security" => {
                                if let ListValue::Text(_, text) = &item.value {
                                    if let Ok(expr) = SecurityExpression::from_str(text) {
                                        self.dir_list.lock().unwrap()[selected_item].list_security = expr;
                                    }
                                }
                            }

                            "download_security" => {
                                if let ListValue::Text(_, text) = &item.value {
                                    if let Ok(expr) = SecurityExpression::from_str(text) {
                                        self.dir_list.lock().unwrap()[selected_item].download_security = expr;
                                    }
                                }
                            }

                            "upload_security" => {
                                if let ListValue::Text(_, text) = &item.value {
                                    if let Ok(expr) = SecurityExpression::from_str(text) {
                                        self.dir_list.lock().unwrap()[selected_item].upload_security = expr;
                                    }
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
                self.dir_list.lock().unwrap().save(&self.path).unwrap();
                return false;
            }
            _ => match key.code {
                KeyCode::Char('1') => self.move_up(),
                KeyCode::Char('2') => self.move_down(),

                KeyCode::Insert => {
                    self.dir_list.lock().unwrap().push(FileDirectory::default());
                    self.insert_table.content_length += 1;
                }
                KeyCode::Delete => {
                    if let Some(selected_item) = self.insert_table.table_state.selected() {
                        if selected_item < self.dir_list.lock().unwrap().len() {
                            self.dir_list.lock().unwrap().remove(selected_item);
                            self.insert_table.content_length -= 1;
                        }
                    }
                }

                KeyCode::Enter => {
                    self.edit_config_state = ConfigMenuState::default();

                    if let Some(selected_item) = self.insert_table.table_state.selected() {
                        let cmd = self.dir_list.lock().unwrap();
                        let Some(item) = cmd.get(selected_item) else {
                            return true;
                        };
                        self.edit_config = Some(ConfigMenu {
                            entry: vec![
                                ConfigEntry::Item(ListItem::new("name", "Name".to_string(), ListValue::Text(25, item.name.to_string())).with_label_width(16)),
                                ConfigEntry::Item(ListItem::new("path", "Path".to_string(), ListValue::Path(item.path.clone())).with_label_width(16)),
                                ConfigEntry::Item(
                                    ListItem::new("file_base", "File Base".to_string(), ListValue::Path(item.file_base.clone())).with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new("password", "Password".to_string(), ListValue::Text(25, item.password.to_string())).with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        "sort_order",
                                        "Sort".to_string(),
                                        ListValue::ComboBox(ComboBox {
                                            cur_value: ComboBoxValue::new(format!("{:?}", item.sort_order), format!("{:?}", item.sort_order)),
                                            first: 0,
                                            scroll_state: ScrollbarState::default(),
                                            values: SortOrder::iter()
                                                .map(|x| ComboBoxValue::new(format!("{:?}", x), format!("{:?}", x)))
                                                .collect::<Vec<ComboBoxValue>>(),
                                        }),
                                    )
                                    .with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        "sort_direction",
                                        "Sort ascending".to_string(),
                                        ListValue::Bool(item.sort_direction == SortDirection::Ascending),
                                    )
                                    .with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new("has_new_files", "Has New Files".to_string(), ListValue::Bool(item.has_new_files)).with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new("is_readonly", "Is Read-Only".to_string(), ListValue::Bool(item.is_readonly)).with_label_width(16),
                                ),
                                ConfigEntry::Item(ListItem::new("is_free", "Is Free".to_string(), ListValue::Bool(item.is_free)).with_label_width(16)),
                                ConfigEntry::Item(
                                    ListItem::new("allow_ul_pwd", "Allow Upload Password".to_string(), ListValue::Bool(item.allow_ul_pwd)).with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        "list_security",
                                        "List Security".to_string(),
                                        ListValue::Text(25, item.list_security.to_string()),
                                    )
                                    .with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        "download_security",
                                        "Download Security".to_string(),
                                        ListValue::Text(25, item.download_security.to_string()),
                                    )
                                    .with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        "upload_security",
                                        "Upload Security".to_string(),
                                        ListValue::Text(25, item.upload_security.to_string()),
                                    )
                                    .with_label_width(16),
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
