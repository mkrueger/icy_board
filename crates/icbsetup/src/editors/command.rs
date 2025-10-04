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
        commands::{CommandAction, CommandList, CommandType},
        security_expr::SecurityExpression,
    },
};
use icy_board_tui::{
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue},
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

pub struct CommandsEditor<'a> {
    path: std::path::PathBuf,

    insert_table: InsertTable<'a>,
    command_list_orig: CommandList,
    command_list: Arc<Mutex<CommandList>>,

    edit_config_state: ConfigMenuState,
    edit_config: Option<ConfigMenu<(usize, Arc<Mutex<CommandList>>)>>,
    save_dialog: Option<SaveChangesDialog>,
}

impl<'a> CommandsEditor<'a> {
    pub(crate) fn new(path: &std::path::PathBuf) -> Res<Self> {
        let command_list_orig = if path.exists() { CommandList::load(&path)? } else { CommandList::default() };
        let command_list: Arc<Mutex<CommandList>> = Arc::new(Mutex::new(command_list_orig.clone()));
        let scroll_state = ScrollbarState::default().content_length(command_list_orig.len());
        let content_length = command_list_orig.len();
        let mnu2: Arc<Mutex<CommandList>> = command_list.clone();
        let insert_table = InsertTable {
            scroll_state,
            table_state: TableState::default().with_selected(0),
            headers: vec![
                "#".to_string(),
                get_text("command_editor_header_command"),
                get_text("command_editor_header_action"),
                get_text("command_editor_header_parameter"),
            ],
            get_content: Box::new(move |_table, i, j| {
                if let Ok(mnu2) = mnu2.lock() {
                    if *i < mnu2.commands.len() {
                        return match j {
                            0 => Line::from(format!("{})", i + 1)),
                            1 => Line::from(mnu2.commands[*i].keyword.clone()),
                            2 => {
                                if let Some(act) = mnu2.commands[*i].actions.get(0) {
                                    Line::from(act.command_type.to_string())
                                } else {
                                    Line::from("No Action")
                                }
                            }
                            3 => {
                                if let Some(act) = mnu2.commands[*i].actions.get(0) {
                                    Line::from(act.parameter.to_string())
                                } else {
                                    Line::from("No Action")
                                }
                            }
                            _ => Line::from("".to_string()),
                        };
                    }
                }
                return Line::from("".to_string());
            }),
            content_length,
        };

        Ok(Self {
            path: path.clone(),
            insert_table,
            command_list,
            command_list_orig,
            edit_config: None,
            edit_config_state: ConfigMenuState::default(),
            save_dialog: None,
        })
    }

    fn insert(&mut self) {
        self.command_list
            .lock()
            .unwrap()
            .commands
            .push(icy_board_engine::icy_board::commands::Command::default());
        self.insert_table.scroll_state = self.insert_table.scroll_state.content_length(self.command_list.lock().unwrap().commands.len());
        self.insert_table.content_length += 1;
    }

    fn remove(&mut self) {
        if let Some(selected) = self.insert_table.table_state.selected() {
            let len = if let Ok(menu) = self.command_list.lock() {
                menu.commands.len()
            } else {
                return;
            };

            if selected >= len {
                return;
            }
            self.command_list.lock().unwrap().commands.remove(selected);
            if len > 0 {
                self.insert_table.table_state.select(Some(selected.min(len - 1)))
            } else {
                self.insert_table.table_state.select(None)
            }
            self.insert_table.content_length -= 1;
        }
    }

    fn move_up(&mut self) {
        if let Some(selected) = self.insert_table.table_state.selected() {
            if selected > 0 {
                if let Ok(mut menu) = self.command_list.lock() {
                    menu.commands.swap(selected, selected - 1);
                }
                self.insert_table.table_state.select(Some(selected - 1));
            }
        }
    }

    fn move_down(&mut self) {
        if let Some(selected) = self.insert_table.table_state.selected() {
            let count = self.command_list.lock().unwrap().commands.len();
            if selected + 1 < count {
                if let Ok(mut menu) = self.command_list.lock() {
                    menu.commands.swap(selected, selected + 1);
                }
                self.insert_table.table_state.select(Some(selected + 1));
            }
        }
    }
}

impl<'a> Page for CommandsEditor<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());

        let block = Block::new()
            .title_alignment(Alignment::Center)
            .title(Title::from(
                Span::from(get_text("command_editor_title")).style(get_tui_theme().dialog_box_title),
            ))
            .style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_set(icy_board_tui::BORDER_SET)
            .title_bottom(Span::styled(get_text("icb_setup_key_conf_list_help"), get_tui_theme().key_binding));
        block.render(area, frame.buffer_mut());
        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        self.insert_table.render_table(frame, area);

        if let Some(edit_config) = &mut self.edit_config {
            let area = area.inner(Margin { vertical: 7, horizontal: 3 });
            Clear.render(area, frame.buffer_mut());
            let block = Block::new()
                .title_alignment(Alignment::Center)
                .title(Title::from(
                    Span::from(get_text("command_editor_editor")).style(get_tui_theme().dialog_box_title),
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
                    self.command_list.lock().unwrap().save(&self.path).unwrap();
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
                if self.command_list_orig == self.command_list.lock().unwrap().clone() {
                    return PageMessage::Close;
                }
                self.save_dialog = Some(SaveChangesDialog::new());
                return PageMessage::None;
            }
            _ => match key.code {
                KeyCode::PageUp => self.move_up(),
                KeyCode::PageDown => self.move_down(),

                KeyCode::Insert => {
                    self.insert();
                }
                KeyCode::Delete => {
                    self.remove();
                }

                KeyCode::Enter => {
                    self.edit_config_state = ConfigMenuState::default();
                    if let Some(selected_item) = self.insert_table.table_state.selected() {
                        let mut cmd: std::sync::MutexGuard<'_, CommandList> = self.command_list.lock().unwrap();
                        let Some(cur_prot) = cmd.get_mut(selected_item) else {
                            return PageMessage::None;
                        };
                        if cur_prot.actions.len() == 0 {
                            cur_prot.actions.push(CommandAction::default());
                        }
                        self.edit_config = Some(ConfigMenu {
                            obj: (selected_item, self.command_list.clone()),
                            entry: vec![
                                ConfigEntry::Item(
                                    ListItem::new(get_text("command_editor_keyword"), ListValue::Text(16, cur_prot.keyword.clone()))
                                        .with_label_width(16)
                                        .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<CommandList>>), value: String| {
                                            list.lock().unwrap()[*i].keyword = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(get_text("command_editor_help"), ListValue::Text(16, cur_prot.help.clone()))
                                        .with_label_width(16)
                                        .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<CommandList>>), value: String| {
                                            list.lock().unwrap()[*i].help = value;
                                        }),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("command_editor_security"),
                                        ListValue::Security(cur_prot.security.clone(), cur_prot.security.to_string()),
                                    )
                                    .with_label_width(16)
                                    .with_update_sec_value(
                                        &|(i, list): &(usize, Arc<Mutex<CommandList>>), value: SecurityExpression| {
                                            list.lock().unwrap()[*i].security = value;
                                        },
                                    ),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("command_editor_command_type"),
                                        ListValue::ComboBox(ComboBox {
                                            cur_value: ComboBoxValue::new(
                                                format!("{:?}", cur_prot.actions[0].command_type),
                                                format!("{:?}", cur_prot.actions[0].command_type),
                                            ),
                                            selected_item: 0,
                                            is_edit_open: false,
                                            first_item: 0,
                                            values: CommandType::iter()
                                                .map(|x| ComboBoxValue::new(format!("{:?}", x), format!("{:?}", x)))
                                                .collect::<Vec<ComboBoxValue>>(),
                                        }),
                                    )
                                    .with_label_width(16)
                                    .with_update_combobox_value(
                                        &|(i, list): &(usize, Arc<Mutex<CommandList>>), value: &ComboBox| {
                                            list.lock().unwrap()[*i].actions[0].command_type = CommandType::from_str(&value.cur_value.value).unwrap();
                                        },
                                    ),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(get_text("command_editor_parameter"), ListValue::Text(43, cur_prot.actions[0].parameter.clone()))
                                        .with_label_width(16)
                                        .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<CommandList>>), value: String| {
                                            list.lock().unwrap()[*i].actions[0].parameter = value;
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

pub fn edit_commands(_board: Arc<Mutex<IcyBoard>>, path: PathBuf) -> PageMessage {
    PageMessage::OpenSubPage(Box::new(CommandsEditor::new(&path).unwrap()))
}
