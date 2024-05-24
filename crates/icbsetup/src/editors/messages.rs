use std::{
    str::FromStr,
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    icy_board::{
        message_area::{AreaList, MessageArea},
        security_expr::SecurityExpression,
        IcyBoardSerializer,
    },
    Res,
};
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue},
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

pub struct MessageAreasEditor<'a> {
    path: std::path::PathBuf,

    insert_table: InsertTable<'a>,
    dir_list: Arc<Mutex<AreaList>>,

    edit_config_state: ConfigMenuState,
    edit_config: Option<ConfigMenu>,
}

impl<'a> MessageAreasEditor<'a> {
    pub(crate) fn new(path: &std::path::PathBuf) -> Res<Self> {
        let surveys = if path.exists() { AreaList::load(&path)? } else { AreaList::default() };
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
                    2 => Line::from(format!("{}", dl2.lock().unwrap()[*i].filename.display())),
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

impl<'a> Editor for MessageAreasEditor<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        let block = Block::new()
            .title(Title::from(Span::from(" Message Areas ").style(THEME.content_box_title)).alignment(Alignment::Center))
            .style(THEME.content_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        block.render(area, frame.buffer_mut());
        let area = area.inner(&Margin { horizontal: 1, vertical: 1 });
        self.display_insert_table(frame, &area);

        if let Some(edit_config) = &mut self.edit_config {
            let area = area.inner(&Margin { vertical: 3, horizontal: 3 });
            Clear.render(area, frame.buffer_mut());
            let block = Block::new()
                .title(Title::from(Span::from(" Message Area ").style(THEME.content_box_title)).alignment(Alignment::Center))
                .style(THEME.content_box)
                .padding(Padding::new(2, 2, 1, 1))
                .borders(Borders::ALL)
                .border_type(BorderType::Double);
            //     let area =  footer.inner(&Margin { vertical: 15, horizontal: 5 });
            block.render(area, frame.buffer_mut());
            edit_config.render(area.inner(&Margin { vertical: 1, horizontal: 1 }), frame, &mut self.edit_config_state);

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

                            "filename" => {
                                if let ListValue::Path(path) = &item.value {
                                    self.dir_list.lock().unwrap()[selected_item].filename = path.clone();
                                }
                            }

                            "is_readonly" => {
                                if let ListValue::Bool(value) = &item.value {
                                    self.dir_list.lock().unwrap()[selected_item].is_read_only = *value;
                                }
                            }

                            "allow_aliases" => {
                                if let ListValue::Bool(value) = &item.value {
                                    self.dir_list.lock().unwrap()[selected_item].allow_aliases = *value;
                                }
                            }

                            "req_level_to_list" => {
                                if let ListValue::Text(_, text) = &item.value {
                                    self.dir_list.lock().unwrap()[selected_item].req_level_to_list = SecurityExpression::from_str(text).unwrap();
                                }
                            }

                            "req_level_to_enter" => {
                                if let ListValue::Text(_, text) = &item.value {
                                    self.dir_list.lock().unwrap()[selected_item].req_level_to_enter = SecurityExpression::from_str(text).unwrap();
                                }
                            }

                            "req_level_to_save_attach" => {
                                if let ListValue::Text(_, text) = &item.value {
                                    self.dir_list.lock().unwrap()[selected_item].req_level_to_save_attach = SecurityExpression::from_str(text).unwrap();
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
                    self.dir_list.lock().unwrap().push(MessageArea::default());
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
                                ConfigEntry::Item(ListItem::new("filename", "File".to_string(), ListValue::Path(item.filename.clone())).with_label_width(16)),
                                ConfigEntry::Item(
                                    ListItem::new("is_readonly", "Is Read-Only".to_string(), ListValue::Bool(item.is_read_only)).with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new("allow_aliases", "Allow Aliases".to_string(), ListValue::Bool(item.allow_aliases)).with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        "req_level_to_list",
                                        "List Security".to_string(),
                                        ListValue::Text(25, item.req_level_to_list.to_string()),
                                    )
                                    .with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        "req_level_to_enter",
                                        "Enter Security".to_string(),
                                        ListValue::Text(25, item.req_level_to_enter.to_string()),
                                    )
                                    .with_label_width(16),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        "req_level_to_save_attach",
                                        "Attach Security".to_string(),
                                        ListValue::Text(25, item.req_level_to_save_attach.to_string()),
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
