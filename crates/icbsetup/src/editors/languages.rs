use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    icy_board::{
        language::{Language, SupportedLanguages},
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

pub struct LanguageListEditor<'a> {
    path: std::path::PathBuf,

    insert_table: InsertTable<'a>,
    dir_list: Arc<Mutex<SupportedLanguages>>,

    edit_config_state: ConfigMenuState,
    edit_config: Option<ConfigMenu>,
}

impl<'a> LanguageListEditor<'a> {
    pub(crate) fn new(path: &std::path::PathBuf) -> Res<Self> {
        let surveys = if path.exists() {
            SupportedLanguages::load(&path)?
        } else {
            SupportedLanguages::default()
        };
        let dir_list = Arc::new(Mutex::new(surveys.clone()));
        let scroll_state = ScrollbarState::default().content_length(surveys.len());
        let content_length = surveys.len();
        let dl2 = dir_list.clone();
        let insert_table = InsertTable {
            scroll_state,
            table_state: TableState::default().with_selected(0),
            headers: vec![
                "".to_string(),
                "Language            ".to_string(),
                "Extension".to_string(),
                "Locale".to_string(),
                "Yes".to_string(),
                "No".to_string(),
            ],
            get_content: Box::new(move |_table, i, j| {
                if *i >= dl2.lock().unwrap().len() {
                    return Line::from("".to_string());
                }
                match j {
                    0 => Line::from(format!("{})", *i + 1)),
                    1 => Line::from(format!("{}", dl2.lock().unwrap().languages[*i].description)),
                    2 => Line::from(format!("{}", dl2.lock().unwrap().languages[*i].extension)),
                    3 => Line::from(format!("{}", dl2.lock().unwrap().languages[*i].locale)),
                    4 => Line::from(format!("{}", dl2.lock().unwrap().languages[*i].yes_char)),
                    5 => Line::from(format!("{}", dl2.lock().unwrap().languages[*i].no_char)),
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
                levels.languages.swap(selected, selected - 1);
                self.insert_table.table_state.select(Some(selected - 1));
            }
        }
    }

    fn move_down(&mut self) {
        if let Some(selected) = self.insert_table.table_state.selected() {
            if selected + 1 < self.dir_list.lock().unwrap().len() {
                let mut levels = self.dir_list.lock().unwrap();
                levels.languages.swap(selected, selected + 1);
                self.insert_table.table_state.select(Some(selected + 1));
            }
        }
    }
}

impl<'a> Editor for LanguageListEditor<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        let block = Block::new()
            .title_alignment(Alignment::Center)
            .title(Title::from(Span::from(" Languages ").style(get_tui_theme().content_box_title)))
            .style(get_tui_theme().content_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        block.render(area, frame.buffer_mut());
        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        self.display_insert_table(frame, &area);

        if let Some(edit_config) = &mut self.edit_config {
            let area = area.inner(Margin { vertical: 8, horizontal: 6 });
            Clear.render(area, frame.buffer_mut());
            let block = Block::new()
                .title_alignment(Alignment::Center)
                .title(Title::from(Span::from(" Edit Language ").style(get_tui_theme().content_box_title)))
                .style(get_tui_theme().content_box)
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
                            "description" => {
                                if let ListValue::Text(_, text) = &item.value {
                                    self.dir_list.lock().unwrap().languages[selected_item].description = text.to_string();
                                }
                            }

                            "extension" => {
                                if let ListValue::Text(_, text) = &item.value {
                                    self.dir_list.lock().unwrap().languages[selected_item].extension = text.to_string();
                                }
                            }

                            "locale" => {
                                if let ListValue::Text(_, text) = &item.value {
                                    self.dir_list.lock().unwrap().languages[selected_item].locale = text.to_string();
                                }
                            }

                            "yes_char" => {
                                if let ListValue::Text(_, text) = &item.value {
                                    self.dir_list.lock().unwrap().languages[selected_item].yes_char = text.chars().next().unwrap_or('Y');
                                }
                            }

                            "no_char" => {
                                if let ListValue::Text(_, text) = &item.value {
                                    self.dir_list.lock().unwrap().languages[selected_item].no_char = text.chars().next().unwrap_or('N');
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
                    self.dir_list.lock().unwrap().languages.push(Language::default());
                    self.insert_table.content_length += 1;
                }
                KeyCode::Delete => {
                    if let Some(selected_item) = self.insert_table.table_state.selected() {
                        if selected_item < self.dir_list.lock().unwrap().len() {
                            self.dir_list.lock().unwrap().languages.remove(selected_item);
                            self.insert_table.content_length -= 1;
                        }
                    }
                }

                KeyCode::Enter => {
                    self.edit_config_state = ConfigMenuState::default();

                    if let Some(selected_item) = self.insert_table.table_state.selected() {
                        let cmd = self.dir_list.lock().unwrap();
                        let Some(item) = cmd.languages.get(selected_item) else {
                            return true;
                        };
                        self.edit_config = Some(ConfigMenu {
                            entry: vec![
                                ConfigEntry::Item(
                                    ListItem::new("description", "Language".to_string(), ListValue::Text(25, item.description.to_string()))
                                        .with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new("extension", "Extension".to_string(), ListValue::Text(25, item.extension.to_string())).with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new("locale", "Locale".to_string(), ListValue::Text(25, item.locale.to_string())).with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new("yes_char", "Yes Char".to_string(), ListValue::Text(25, item.yes_char.to_string())).with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new("no_char", "No Char".to_string(), ListValue::Text(25, item.no_char.to_string())).with_label_width(16),
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
