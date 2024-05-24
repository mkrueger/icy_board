use std::{
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    icy_board::{
        bulletins::{Bullettin, BullettinList},
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

pub struct BullettinsEditor<'a> {
    path: std::path::PathBuf,
    blt_list: BullettinList,

    insert_table: InsertTable<'a>,
    sec_levels: Arc<Mutex<Vec<Bullettin>>>,

    edit_config_state: ConfigMenuState,
    edit_config: Option<ConfigMenu>,
}

impl<'a> BullettinsEditor<'a> {
    pub(crate) fn new(path: &std::path::PathBuf) -> Res<Self> {
        let bullettins = if path.exists() {
            BullettinList::load(&path)?
        } else {
            let sec_levels = BullettinList::default();
            sec_levels
        };
        let command_arc = Arc::new(Mutex::new(bullettins.bullettins.clone()));
        let scroll_state = ScrollbarState::default().content_length(bullettins.bullettins.len());
        let content_length = bullettins.bullettins.len();
        let cmd2 = command_arc.clone();
        let insert_table = InsertTable {
            scroll_state,
            table_state: TableState::default().with_selected(0),
            headers: vec!["".to_string(), "Bullettin".to_string()],
            get_content: Box::new(move |_table, i, j| {
                if *i >= cmd2.lock().unwrap().len() {
                    return Line::from("".to_string());
                }
                match j {
                    0 => Line::from(format!("{})", *i + 1)),
                    1 => Line::from(format!("{}", cmd2.lock().unwrap()[*i].file.display())),
                    _ => Line::from("".to_string()),
                }
            }),
            content_length,
        };

        Ok(Self {
            path: path.clone(),
            blt_list: bullettins,
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

impl<'a> Editor for BullettinsEditor<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        let block = Block::new()
            .title(Title::from(Span::from(" Bullettins ").style(THEME.content_box_title)).alignment(Alignment::Center))
            .style(THEME.content_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        block.render(area, frame.buffer_mut());
        let area = area.inner(&Margin { horizontal: 1, vertical: 1 });
        self.display_insert_table(frame, &area);

        if let Some(edit_config) = &mut self.edit_config {
            let area = area.inner(&Margin { vertical: 2, horizontal: 3 });
            Clear.render(area, frame.buffer_mut());
            let block = Block::new()
                .title(Title::from(Span::from(" Edit Bullettin ").style(THEME.content_box_title)).alignment(Alignment::Center))
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
                            "path" => {
                                if let ListValue::Path(path) = &item.value {
                                    self.sec_levels.lock().unwrap()[selected_item].file = path.clone();
                                }
                            }
                            "security" => {
                                if let ListValue::Text(_, text) = &item.value {
                                    if let Ok(expr) = SecurityExpression::from_str(text) {
                                        self.sec_levels.lock().unwrap()[selected_item].required_security = expr;
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
                self.blt_list.bullettins.clear();
                self.blt_list.bullettins.append(&mut self.sec_levels.lock().unwrap());
                self.blt_list.save(&self.path).unwrap();
                return false;
            }
            _ => match key.code {
                KeyCode::Char('1') => self.move_up(),
                KeyCode::Char('2') => self.move_down(),

                KeyCode::Insert => {
                    self.sec_levels.lock().unwrap().push(Bullettin {
                        file: PathBuf::new(),
                        required_security: SecurityExpression::default(),
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
                                ConfigEntry::Item(ListItem::new("path", "Path".to_string(), ListValue::Path(action.file.clone())).with_label_width(16)),
                                ConfigEntry::Item(
                                    ListItem::new("security", "Security".to_string(), ListValue::Text(25, action.required_security.to_string()))
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
