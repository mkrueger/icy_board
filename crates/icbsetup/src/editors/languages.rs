use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    Res,
    icy_board::{
        IcyBoard, IcyBoardSerializer,
        language::{Language, SupportedLanguages},
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

pub struct LanguageListEditor<'a> {
    path: std::path::PathBuf,

    insert_table: InsertTable<'a>,
    lang_list_orig: SupportedLanguages,
    lang_list: Arc<Mutex<SupportedLanguages>>,

    edit_config_state: ConfigMenuState,
    edit_config: Option<ConfigMenu<(usize, Arc<Mutex<SupportedLanguages>>)>>,
    save_dialog: Option<SaveChangesDialog>,
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
            headers: vec![
                "".to_string(),
                get_text("lang_editor_header_language"),
                get_text("lang_editor_header_ext"),
                get_text("lang_editor_header_locale"),
                get_text("lang_editor_header_yes"),
                get_text("lang_editor_header_no"),
            ],
            get_content: Box::new(move |_table, i, j| {
                if *i >= dl2.lock().unwrap().len() {
                    return Line::from("".to_string());
                }
                match j {
                    0 => Line::from(format!("{})", *i + 1)),
                    1 => Line::from(format!("{}", dl2.lock().unwrap()[*i].description)),
                    2 => Line::from(format!("{}", dl2.lock().unwrap()[*i].extension)),
                    3 => Line::from(format!("{}", dl2.lock().unwrap()[*i].locale)),
                    4 => Line::from(format!("{}", dl2.lock().unwrap()[*i].yes_char)),
                    5 => Line::from(format!("{}", dl2.lock().unwrap()[*i].no_char)),
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
                let mut levels = self.lang_list.lock().unwrap();
                levels.swap(selected, selected - 1);
                self.insert_table.table_state.select(Some(selected - 1));
            }
        }
    }

    fn move_down(&mut self) {
        if let Some(selected) = self.insert_table.table_state.selected() {
            if selected + 1 < self.lang_list.lock().unwrap().len() {
                let mut levels = self.lang_list.lock().unwrap();
                levels.swap(selected, selected + 1);
                self.insert_table.table_state.select(Some(selected + 1));
            }
        }
    }
}

impl<'a> Page for LanguageListEditor<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        let title = get_text("lang_editor_title");

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
            let mut area = area.inner(Margin { vertical: 6, horizontal: 6 });
            area.height -= 2;
            Clear.render(area, frame.buffer_mut());
            let block = Block::new()
                .title_alignment(Alignment::Center)
                .title(Title::from(
                    Span::from(get_text("lang_editor_edit_lang")).style(get_tui_theme().dialog_box_title),
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
                    self.lang_list.lock().unwrap().save(&self.path).unwrap();
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
                if self.lang_list_orig == self.lang_list.lock().unwrap().clone() {
                    return PageMessage::Close;
                }
                self.save_dialog = Some(SaveChangesDialog::new());
                return PageMessage::None;
            }
            KeyCode::PageUp => self.move_up(),
            KeyCode::PageDown => self.move_down(),

            KeyCode::Insert => {
                self.lang_list.lock().unwrap().push(Language::default());
                self.insert_table.content_length += 1;
            }
            KeyCode::Delete => {
                if let Some(selected_item) = self.insert_table.table_state.selected() {
                    if selected_item < self.lang_list.lock().unwrap().len() {
                        self.lang_list.lock().unwrap().remove(selected_item);
                        self.insert_table.content_length -= 1;
                    }
                }
            }

            KeyCode::Enter => {
                self.edit_config_state = ConfigMenuState::default();

                if let Some(selected_item) = self.insert_table.table_state.selected() {
                    let cmd = self.lang_list.lock().unwrap();
                    let Some(item) = cmd.get(selected_item) else {
                        return PageMessage::None;
                    };
                    self.edit_config = Some(ConfigMenu {
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
