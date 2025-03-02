use std::{
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    Res,
    icy_board::{
        IcyBoard, IcyBoardSerializer,
        bulletins::{Bullettin, BullettinList},
        security_expr::SecurityExpression,
    },
};
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue},
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

pub struct BullettinsEditor<'a> {
    path: std::path::PathBuf,
    blt_list: BullettinList,
    orig_blt_list: BullettinList,

    insert_table: InsertTable<'a>,
    sec_levels: Arc<Mutex<Vec<Bullettin>>>,

    edit_config_state: ConfigMenuState,
    edit_config: Option<ConfigMenu<(usize, Arc<Mutex<Vec<Bullettin>>>)>>,

    save_dialog: Option<SaveChangesDialog>,
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
                    1 => Line::from(format!("{}", cmd2.lock().unwrap()[*i].path.display())),
                    _ => Line::from("".to_string()),
                }
            }),
            content_length,
        };

        Ok(Self {
            path: path.clone(),
            blt_list: bullettins.clone(),
            orig_blt_list: bullettins,
            insert_table,
            sec_levels: command_arc,
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

impl<'a> Page for BullettinsEditor<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        let block = Block::new()
            .title_alignment(Alignment::Center)
            .title(Title::from(Span::from(" Bullettins ").style(get_tui_theme().dialog_box_title)))
            .style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_set(icy_board_tui::BORDER_SET)
            .title_bottom(Span::styled(get_text("icb_setup_key_conf_list_help"), get_tui_theme().key_binding));
        block.render(area, frame.buffer_mut());
        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        self.display_insert_table(frame, &area);

        if let Some(edit_config) = &mut self.edit_config {
            let area = area.inner(Margin { vertical: 8, horizontal: 3 });
            Clear.render(area, frame.buffer_mut());
            let block = Block::new()
                .title_alignment(Alignment::Center)
                .title(Title::from(Span::from(" Edit Bullettin ").style(get_tui_theme().dialog_box_title)))
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
                    self.blt_list.save(&self.path).unwrap();
                    PageMessage::Close
                }
                icy_board_tui::save_changes_dialog::SaveChangesMessage::None => PageMessage::None,
            };
        }

        if let Some(edit_config) = &mut self.edit_config {
            match key.code {
                KeyCode::Esc => {
                    self.edit_config = None;
                    return PageMessage::None;
                }
                _ => {
                    edit_config.handle_key_press(key, &mut self.edit_config_state);
                }
            }
            return PageMessage::None;
        }

        match key.code {
            KeyCode::Esc => {
                self.blt_list.bullettins.clear();
                self.blt_list.bullettins.append(&mut self.sec_levels.lock().unwrap().clone());
                if self.blt_list == self.orig_blt_list {
                    return PageMessage::Close;
                }
                self.save_dialog = Some(SaveChangesDialog::new());
                return PageMessage::None;
            }
            _ => match key.code {
                KeyCode::PageUp => self.move_up(),
                KeyCode::PageDown => self.move_down(),

                KeyCode::Insert => {
                    self.sec_levels.lock().unwrap().push(Bullettin {
                        path: PathBuf::new(),
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
                            return PageMessage::None;
                        };
                        self.edit_config = Some(ConfigMenu {
                            obj: (selected_item, self.sec_levels.clone()),
                            entry: vec![
                                ConfigEntry::Item(
                                    ListItem::new("Path".to_string(), ListValue::Path(action.path.clone()))
                                        .with_label_width(16)
                                        .with_update_path_value(&|board: &(usize, Arc<Mutex<Vec<Bullettin>>>), value: PathBuf| {
                                            board.1.lock().unwrap()[board.0].path = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new("Security".to_string(), ListValue::Text(25, action.required_security.to_string()))
                                        .with_label_width(16)
                                        .with_update_text_value(&|board: &(usize, Arc<Mutex<Vec<Bullettin>>>), value: String| {
                                            if let Ok(expr) = SecurityExpression::from_str(&value) {
                                                board.1.lock().unwrap()[board.0].required_security = expr;
                                            }
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

pub fn edit_bulletins(_board: (usize, Arc<Mutex<IcyBoard>>), path: PathBuf) -> PageMessage {
    PageMessage::OpenSubPage(Box::new(BullettinsEditor::new(&path).unwrap()))
}
