use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::editors::EditorList;
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
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, ListItem, ListValue, ResultState, TextFlags},
    get_text, get_text_args,
    insert_table::{Column, InsertTable},
    tab_page::{Page, PageMessage},
};
use ratatui::{
    Frame,
    layout::{Margin, Rect},
    text::Line,
    widgets::{Clear, ScrollbarState, TableState, Widget},
};

pub struct DirsEditor<'a> {
    path: std::path::PathBuf,

    insert_table: InsertTable<'a>,
    dir_list: Arc<Mutex<DirectoryList>>,
    dir_list_orig: DirectoryList,

    detail: super::EditorDialog<(usize, Arc<Mutex<DirectoryList>>)>,
    save_changes: super::EditorSaveChanges,
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
            columns: vec![
                Column::new(get_text("dirs_table_name_header")).with_width(20),
                Column::new(get_text("dirs_table_path_header")),
            ],
            numbered: true,
            get_content: Box::new(move |_table, i, j| {
                if *i >= dl2.lock().unwrap().len() {
                    return Line::from("".to_string());
                }
                match j {
                    0 => Line::from(dl2.lock().unwrap()[*i].name.to_string()),
                    1 => Line::from(format!("{}", dl2.lock().unwrap()[*i].path.display())),
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
            detail: super::EditorDialog::default(),
            save_changes: super::EditorSaveChanges::default(),
        })
    }

    fn with_path_base(mut self, path_base: PathBuf) -> Self {
        self.detail.state.path_base = Some(path_base);
        self
    }
}

impl<'a> Page for DirsEditor<'a> {
    fn request_status(&self) -> ResultState {
        self.detail.status()
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        let conference_name = crate::tabs::conferences::get_cur_conference_name();
        let title = get_text_args("dirs_editor_title", HashMap::from([("conference".to_string(), conference_name)]));

        let block = super::list_editor_frame(title, "icb_setup_key_conf_list_help", self.detail.is_open() || self.save_changes.is_open());

        block.render(area, frame.buffer_mut());
        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        self.insert_table.render_list(frame, area);

        if self.detail.is_open() {
            let area = area.inner(Margin { vertical: 3, horizontal: 3 });
            self.detail.render(frame, area, get_text("dirs_edit_directory_title"), "");
        }
        self.save_changes.render(frame, area);
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if let Some(message) = self
            .save_changes
            .handle_key(key, || crate::editors::save_file(&self.path, || self.dir_list.lock().unwrap().save(&self.path)))
        {
            return message;
        }

        if let Some(message) = self.detail.handle_key(key) {
            return message;
        }

        match key.code {
            KeyCode::Esc => {
                return self.save_changes.request_close(self.dir_list_orig != *self.dir_list.lock().unwrap());
            }
            KeyCode::PageUp => self.insert_table.move_row(&mut self.dir_list.lock().unwrap(), -1),
            KeyCode::PageDown => self.insert_table.move_row(&mut self.dir_list.lock().unwrap(), 1),

            KeyCode::Insert => {
                self.insert_table.push_row(&mut *self.dir_list.lock().unwrap(), FileDirectory::default());
            }
            KeyCode::Delete => {
                self.insert_table.remove_row(&mut *self.dir_list.lock().unwrap());
            }

            KeyCode::Enter => {
                if let Some(selected_item) = self.insert_table.table_state.selected() {
                    let cmd = self.dir_list.lock().unwrap();
                    let Some(item) = cmd.get(selected_item) else {
                        return PageMessage::None;
                    };
                    self.detail.open(super::align_editor_labels(ConfigMenu {
                        obj: (selected_item, self.dir_list.clone()),

                        entry: vec![
                            ConfigEntry::Item(
                                ListItem::new(get_text("dirs_edit_name"), ListValue::Text(25, TextFlags::None, item.name.to_string()))
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
                                ListItem::new(get_text("dirs_edit_password"), ListValue::Text(12, TextFlags::None, item.password.to_string()))
                                    .with_label_width(16)
                                    .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<DirectoryList>>), value: String| {
                                        list.lock().unwrap()[*i].password = Password::PlainText(value);
                                    }),
                            ),
                            ConfigEntry::Item(
                                ListItem::new(
                                    get_text("dirs_edit_fido_tag"),
                                    ListValue::Text(32, TextFlags::None, item.ftn_area_tag.to_string()),
                                )
                                .with_status(get_text("dirs_edit_fido_tag-status"))
                                .with_help(get_text("dirs_edit_fido_tag-help"))
                                .with_label_width(16)
                                .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<DirectoryList>>), value: String| {
                                    list.lock().unwrap()[*i].ftn_area_tag = value.to_ascii_uppercase();
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
                                .with_label_width(16)
                                .with_update_combobox_value(
                                    &|(i, list): &(usize, Arc<Mutex<DirectoryList>>), value: &ComboBox| {
                                        let sort_order = match value.cur_value.value.as_str() {
                                            "NoSort" => SortOrder::NoSort,
                                            "FileDate" => SortOrder::FileDate,
                                            _ => SortOrder::FileName,
                                        };
                                        list.lock().unwrap()[*i].sort_order = sort_order;
                                    },
                                ),
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
                    }));
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

pub fn edit_dirs(board: (usize, Arc<Mutex<IcyBoard>>), path: PathBuf) -> PageMessage {
    let root = board.1.lock().unwrap().root_path.clone();
    PageMessage::OpenSubPage(Box::new(DirsEditor::new(&path).unwrap().with_path_base(root)))
}

#[cfg(test)]
mod path_browser_tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn nested_directory_browser_keeps_board_root_updates_immediately_and_esc_only_cancels_browser() {
        let root = tempfile::tempdir().unwrap();
        let selected = root.path().join("chosen.dat");
        std::fs::write(&selected, b"metadata").unwrap();
        // The configuration's parent must not be used as the browser's base.
        let config = root.path().join("configuration/conferences/directories.toml");
        let mut editor = DirsEditor::new(&config).unwrap().with_path_base(root.path().to_path_buf());
        editor.dir_list.lock().unwrap().push(FileDirectory::default());
        editor.insert_table.content_length = 1;
        editor.handle_key_press(KeyEvent::from(KeyCode::Enter));
        editor.detail.state.selected = 2; // Metadata path.
        assert_eq!(editor.detail.state.path_base.as_deref(), Some(root.path()));
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| editor.render(frame, Rect::new(0, 1, 80, 23))).unwrap();

        editor.handle_key_press(KeyEvent::from(KeyCode::F(4)));
        assert!(editor.detail.state.is_path_browser_open());
        terminal.draw(|frame| editor.render(frame, Rect::new(0, 1, 80, 23))).unwrap();
        editor.handle_key_press(KeyEvent::from(KeyCode::End));
        editor.handle_key_press(KeyEvent::from(KeyCode::Enter));
        assert!(!editor.detail.state.is_path_browser_open());
        let value = editor.dir_list.lock().unwrap()[0].metadata_path.clone();
        assert_eq!(root.path().join(&value), selected, "selection must update the callback before rendering");
        assert!(matches!(&editor.detail.menu.as_ref().unwrap().get_item(2).unwrap().value, ListValue::Path(path) if path == &value));

        editor.handle_key_press(KeyEvent::from(KeyCode::F(4)));
        editor.handle_key_press(KeyEvent::from(KeyCode::Esc));
        assert!(!editor.detail.state.is_path_browser_open());
        assert!(editor.detail.is_open());
        assert!(!editor.save_changes.is_open());
        assert_eq!(editor.dir_list.lock().unwrap()[0].metadata_path, value);
        editor.handle_key_press(KeyEvent::from(KeyCode::Esc));
        assert!(!editor.detail.is_open());
        editor.handle_key_press(KeyEvent::from(KeyCode::Enter));
        assert_eq!(editor.detail.state.path_base.as_deref(), Some(root.path()), "reopening must preserve the root");
    }
}
