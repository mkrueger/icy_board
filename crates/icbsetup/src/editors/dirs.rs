use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    Res,
    icy_board::{
        IcyBoard, IcyBoardSerializer,
        file_directory::{DirectoryList, FileDirectory, SortDirection, SortOrder},
        security_expr::SecurityExpression,
        user_base::Password,
    },
};
use icy_board_tui::{
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue},
    get_text, get_text_args,
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

pub struct DirsEditor<'a> {
    path: std::path::PathBuf,

    insert_table: InsertTable<'a>,
    dir_list: Arc<Mutex<DirectoryList>>,
    dir_list_orig: DirectoryList,

    edit_config_state: ConfigMenuState,
    edit_config: Option<ConfigMenu<(usize, Arc<Mutex<DirectoryList>>)>>,
    save_dialog: Option<SaveChangesDialog>,
}

impl<'a> DirsEditor<'a> {
    pub(crate) fn new(path: &std::path::PathBuf) -> Res<Self> {
        let dir_list_orig = if path.exists() {
            DirectoryList::load(&path)?
        } else {
            DirectoryList::default()
        };
        let dir_list = Arc::new(Mutex::new(dir_list_orig.clone()));
        let scroll_state = ScrollbarState::default().content_length(dir_list_orig.len());
        let content_length = dir_list_orig.len();
        let dl2 = dir_list.clone();
        let insert_table = InsertTable {
            scroll_state,
            table_state: TableState::default().with_selected(0),
            headers: vec![
                "".to_string(),
                format!("{:<20}", get_text("dirs_table_name_header")),
                get_text("dirs_table_path_header"),
            ],
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
            dir_list_orig,
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

impl<'a> Page for DirsEditor<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        let conference_name = crate::tabs::conferences::get_cur_conference_name();
        let title = get_text_args("dirs_editor_title", HashMap::from([("conference".to_string(), conference_name)]));

        let block = Block::new()
            .title_alignment(Alignment::Center)
            .title(Title::from(Span::from(title).style(get_tui_theme().dialog_box_title)))
            .style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_set(icy_board_tui::BORDER_SET)
            .title_bottom(Span::styled(get_text("icb_setup_key_conf_list_help"), get_tui_theme().key_binding));

        block.render(area, frame.buffer_mut());
        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        self.display_insert_table(frame, &area);

        if let Some(edit_config) = &mut self.edit_config {
            let area = area.inner(Margin { vertical: 5, horizontal: 3 });
            Clear.render(area, frame.buffer_mut());
            let block = Block::new()
                .title_alignment(Alignment::Center)
                .title(Title::from(
                    Span::from(get_text("dirs_edit_directory_title")).style(get_tui_theme().dialog_box_title),
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
            if let Some(save_changes) = &self.save_dialog {
                save_changes.render(frame, area);
            }
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
                    self.dir_list.lock().unwrap().save(&self.path).unwrap();
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
                if self.dir_list_orig == self.dir_list.lock().unwrap().clone() {
                    return PageMessage::Close;
                }
                self.save_dialog = Some(SaveChangesDialog::new());
                return PageMessage::None;
            }
            KeyCode::PageUp => self.move_up(),
            KeyCode::PageDown => self.move_down(),

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
                        return PageMessage::None;
                    };
                    self.edit_config = Some(ConfigMenu {
                        obj: (selected_item, self.dir_list.clone()),

                        entry: vec![
                            ConfigEntry::Item(
                                ListItem::new(get_text("dirs_edit_name"), ListValue::Text(25, item.name.to_string()))
                                    .with_label_width(16)
                                    .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<DirectoryList>>), value: String| {
                                        list.lock().unwrap()[*i].name = value;
                                    }),
                            ),
                            ConfigEntry::Item(
                                ListItem::new(get_text("dirs_edit_path"), ListValue::Path(item.path.clone()))
                                    .with_label_width(16)
                                    .with_update_path_value(&|(i, list): &(usize, Arc<Mutex<DirectoryList>>), value: PathBuf| {
                                        list.lock().unwrap()[*i].path = value;
                                    }),
                            ),
                            ConfigEntry::Item(
                                ListItem::new(get_text("dirs_metadata_path"), ListValue::Path(item.metadata_path.clone()))
                                    .with_label_width(16)
                                    .with_update_path_value(&|(i, list): &(usize, Arc<Mutex<DirectoryList>>), value: PathBuf| {
                                        list.lock().unwrap()[*i].metadata_path = value;
                                    }),
                            ),
                            ConfigEntry::Item(
                                ListItem::new(get_text("dirs_edit_password"), ListValue::Text(12, item.password.to_string()))
                                    .with_label_width(16)
                                    .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<DirectoryList>>), value: String| {
                                        list.lock().unwrap()[*i].password = Password::PlainText(value);
                                    }),
                            ),
                            ConfigEntry::Item(
                                ListItem::new(
                                    get_text("dirs_edit_sort"),
                                    ListValue::ComboBox(ComboBox {
                                        cur_value: ComboBoxValue::new(format!("{:?}", item.sort_order), format!("{:?}", item.sort_order)),
                                        selected_item: 0,
                                        is_edit_open: false,
                                        first_item: 0,
                                        values: SortOrder::iter()
                                            .map(|x| ComboBoxValue::new(format!("{:?}", x), format!("{:?}", x)))
                                            .collect::<Vec<ComboBoxValue>>(),
                                    }),
                                )
                                .with_label_width(16),
                            ),
                            ConfigEntry::Item(
                                ListItem::new(get_text("dirs_edit_sort_asc"), ListValue::Bool(item.sort_direction == SortDirection::Ascending))
                                    .with_label_width(16)
                                    .with_update_bool_value(&|(i, list): &(usize, Arc<Mutex<DirectoryList>>), value: bool| {
                                        list.lock().unwrap()[*i].sort_direction = if value { SortDirection::Ascending } else { SortDirection::Descending };
                                    }),
                            ),
                            ConfigEntry::Item(
                                ListItem::new(get_text("dirs_edit_has_new_files"), ListValue::Bool(item.has_new_files))
                                    .with_label_width(16)
                                    .with_update_bool_value(&|(i, list): &(usize, Arc<Mutex<DirectoryList>>), value: bool| {
                                        list.lock().unwrap()[*i].sort_direction = if value { SortDirection::Ascending } else { SortDirection::Descending };
                                    }),
                            ),
                            ConfigEntry::Item(
                                ListItem::new(get_text("dirs_edit_is_free"), ListValue::Bool(item.is_free))
                                    .with_label_width(16)
                                    .with_update_bool_value(&|(i, list): &(usize, Arc<Mutex<DirectoryList>>), value: bool| {
                                        list.lock().unwrap()[*i].is_free = value;
                                    }),
                            ),
                            ConfigEntry::Item(
                                ListItem::new(
                                    get_text("dirs_edit_list_sec"),
                                    ListValue::Security(item.list_security.clone(), item.list_security.to_string()),
                                )
                                .with_label_width(16)
                                .with_update_sec_value(
                                    &|(i, list): &(usize, Arc<Mutex<DirectoryList>>), value: SecurityExpression| {
                                        list.lock().unwrap()[*i].list_security = value;
                                    },
                                ),
                            ),
                            ConfigEntry::Item(
                                ListItem::new(
                                    get_text("dirs_download_sec"),
                                    ListValue::Security(item.download_security.clone(), item.download_security.to_string()),
                                )
                                .with_label_width(16)
                                .with_update_sec_value(
                                    &|(i, list): &(usize, Arc<Mutex<DirectoryList>>), value: SecurityExpression| {
                                        list.lock().unwrap()[*i].download_security = value;
                                    },
                                ),
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
        }
        PageMessage::None
    }
}

pub fn edit_dirs(_board: (usize, Arc<Mutex<IcyBoard>>), path: PathBuf) -> PageMessage {
    PageMessage::OpenSubPage(Box::new(DirsEditor::new(&path).unwrap()))
}
