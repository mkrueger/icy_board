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
        language::{Language, SupportedLanguages},
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

pub struct LanguageListEditor<'a> {
    path: std::path::PathBuf,

    insert_table: InsertTable<'a>,
    lang_list_orig: SupportedLanguages,
    lang_list: Arc<Mutex<SupportedLanguages>>,

    detail: super::EditorDialog<(usize, Arc<Mutex<SupportedLanguages>>)>,
    save_changes: super::EditorSaveChanges,
}

impl<'a> LanguageListEditor<'a> {
    pub(crate) fn new(path: &std::path::PathBuf) -> Res<Self> {
        let lang_list_orig = if path.exists() {
            SupportedLanguages::load(&path)?
        } else {
            SupportedLanguages::default()
        };
        let lang_list = Arc::new(Mutex::new(lang_list_orig.clone()));
        let scroll_state = ScrollbarState::default().content_length(lang_list_orig.len());
        let content_length = lang_list_orig.len();
        let dl2 = lang_list.clone();
        let insert_table = InsertTable {
            scroll_state,
            table_state: TableState::default().with_selected(0),
            columns: vec![
                Column::new(get_text("lang_editor_header_language")).with_width(30),
                Column::new(get_text("lang_editor_header_ext")).with_width(10),
                Column::new(get_text("lang_editor_header_locale")).with_width(10),
                Column::new(get_text("lang_editor_header_yes")).with_width(6),
                Column::new(get_text("lang_editor_header_no")),
            ],
            numbered: true,
            get_content: Box::new(move |_table, i, j| {
                if *i >= dl2.lock().unwrap().len() {
                    return Line::from("".to_string());
                }
                match j {
                    0 => Line::from(dl2.lock().unwrap()[*i].description.to_string()),
                    1 => Line::from(dl2.lock().unwrap()[*i].extension.to_string()),
                    2 => Line::from(dl2.lock().unwrap()[*i].locale.to_string()),
                    3 => Line::from(format!("{}", dl2.lock().unwrap()[*i].yes_char)),
                    4 => Line::from(format!("{}", dl2.lock().unwrap()[*i].no_char)),
                    _ => Line::from("".to_string()),
                }
            }),
            content_length,
        };

        Ok(Self {
            path: path.clone(),
            insert_table,
            lang_list_orig,
            lang_list,
            detail: super::EditorDialog::default(),
            save_changes: super::EditorSaveChanges::default(),
        })
    }
}

impl<'a> Page for LanguageListEditor<'a> {
    fn request_status(&self) -> ResultState {
        self.detail.status()
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        let title = get_text("lang_editor_title");

        let block = super::list_editor_frame(
            title,
            get_text("icb_setup_key_conf_list_help"),
            self.detail.is_open() || self.save_changes.is_open(),
        );
        block.render(area, frame.buffer_mut());
        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        self.insert_table.render_list(frame, area);

        if self.detail.is_open() {
            let mut area = area.inner(Margin { vertical: 6, horizontal: 6 });
            area.height -= 2;
            self.detail.render(frame, area, get_text("lang_editor_edit_lang"), String::new());
        }

        self.save_changes.render(frame, area);
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if let Some(message) = self.save_changes.handle_key(key, || {
            crate::editors::save_file(&self.path, || self.lang_list.lock().unwrap().save(&self.path))
        }) {
            return message;
        }

        if let Some(message) = self.detail.handle_key(key) {
            return message;
        }

        match key.code {
            KeyCode::Esc => {
                return self.save_changes.request_close(self.lang_list_orig != *self.lang_list.lock().unwrap());
            }
            KeyCode::PageUp => self.insert_table.move_row(&mut self.lang_list.lock().unwrap(), -1),
            KeyCode::PageDown => self.insert_table.move_row(&mut self.lang_list.lock().unwrap(), 1),

            KeyCode::Insert => {
                self.insert_table.push_row(&mut *self.lang_list.lock().unwrap(), Language::default());
            }
            KeyCode::Delete => {
                self.insert_table.remove_row(&mut *self.lang_list.lock().unwrap());
            }

            KeyCode::Enter => {
                if let Some(selected_item) = self.insert_table.table_state.selected() {
                    let cmd = self.lang_list.lock().unwrap();
                    let Some(item) = cmd.get(selected_item) else {
                        return PageMessage::None;
                    };
                    self.detail.open(ConfigMenu {
                        obj: (selected_item, self.lang_list.clone()),
                        entry: vec![
                            ConfigEntry::Item(
                                ListItem::new(
                                    get_text("lang_editor_edit_lang_label"),
                                    ListValue::Text(25, TextFlags::None, item.description.to_string()),
                                )
                                .with_label_width(16)
                                .with_update_text_value(
                                    &|(i, list): &(usize, Arc<Mutex<SupportedLanguages>>), value: String| {
                                        list.lock().unwrap()[*i].description = value;
                                    },
                                ),
                            ),
                            ConfigEntry::Item(
                                ListItem::new(
                                    get_text("lang_editor_edit_extension"),
                                    ListValue::Text(25, TextFlags::None, item.extension.to_string()),
                                )
                                .with_label_width(16)
                                .with_update_text_value(
                                    &|(i, list): &(usize, Arc<Mutex<SupportedLanguages>>), value: String| {
                                        list.lock().unwrap()[*i].extension = value;
                                    },
                                ),
                            ),
                            ConfigEntry::Item(
                                ListItem::new(
                                    get_text("lang_editor_edit_locale"),
                                    ListValue::Text(25, TextFlags::None, item.locale.to_string()),
                                )
                                .with_label_width(16)
                                .with_update_text_value(
                                    &|(i, list): &(usize, Arc<Mutex<SupportedLanguages>>), value: String| {
                                        list.lock().unwrap()[*i].locale = value;
                                    },
                                ),
                            ),
                            ConfigEntry::Item(
                                ListItem::new(
                                    get_text("lang_editor_edit_yes_char"),
                                    ListValue::Text(1, TextFlags::None, item.yes_char.to_string()),
                                )
                                .with_label_width(16)
                                .with_update_text_value(
                                    &|(i, list): &(usize, Arc<Mutex<SupportedLanguages>>), value: String| {
                                        if let Some(c) = value.chars().next() {
                                            list.lock().unwrap()[*i].yes_char = c;
                                        }
                                    },
                                ),
                            ),
                            ConfigEntry::Item(
                                ListItem::new(
                                    get_text("lang_editor_edit_no_char"),
                                    ListValue::Text(1, TextFlags::None, item.no_char.to_string()),
                                )
                                .with_label_width(16)
                                .with_update_text_value(
                                    &|(i, list): &(usize, Arc<Mutex<SupportedLanguages>>), value: String| {
                                        if let Some(c) = value.chars().next() {
                                            list.lock().unwrap()[*i].no_char = c;
                                        }
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

pub fn edit_languages(_board: Arc<Mutex<IcyBoard>>, path: PathBuf) -> PageMessage {
    PageMessage::OpenSubPage(Box::new(LanguageListEditor::new(&path).unwrap()))
}
