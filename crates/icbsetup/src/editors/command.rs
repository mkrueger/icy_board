use std::{
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
};

use crate::editors::EditorList;
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
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, ListItem, ListValue, ResultState, TextFlags},
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

pub struct CommandsEditor<'a> {
    path: std::path::PathBuf,

    insert_table: InsertTable<'a>,
    command_list_orig: CommandList,
    command_list: Arc<Mutex<CommandList>>,

    detail: super::EditorDialog<(usize, Arc<Mutex<CommandList>>)>,
    save_changes: super::EditorSaveChanges,
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
            columns: vec![
                Column::new(get_text("command_editor_header_command")).with_width(20),
                Column::new(get_text("command_editor_header_action")).with_width(24),
                Column::new(get_text("command_editor_header_parameter")),
            ],
            numbered: true,
            get_content: Box::new(move |_table, i, j| {
                if let Ok(mnu2) = mnu2.lock()
                    && *i < mnu2.commands.len()
                {
                    return match j {
                        0 => Line::from(mnu2.commands[*i].keyword.clone()),
                        1 => {
                            if let Some(act) = mnu2.commands[*i].actions.first() {
                                Line::from(act.command_type.to_string())
                            } else {
                                Line::from("No Action")
                            }
                        }
                        2 => {
                            if let Some(act) = mnu2.commands[*i].actions.first() {
                                Line::from(act.parameter.to_string())
                            } else {
                                Line::from("No Action")
                            }
                        }
                        _ => Line::from("".to_string()),
                    };
                }
                Line::from("".to_string())
            }),
            content_length,
        };

        Ok(Self {
            path: path.clone(),
            insert_table,
            command_list,
            command_list_orig,
            detail: super::EditorDialog::default(),
            save_changes: super::EditorSaveChanges::default(),
        })
    }
}

impl<'a> Page for CommandsEditor<'a> {
    fn request_status(&self) -> ResultState {
        self.detail.status()
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());

        let block = super::list_editor_frame(
            get_text("command_editor_title"),
            get_text("icb_setup_key_conf_list_help"),
            self.detail.is_open() || self.save_changes.is_open(),
        );
        block.render(area, frame.buffer_mut());
        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        self.insert_table.render_list(frame, area);

        if self.detail.is_open() {
            let area = area.inner(Margin { vertical: 7, horizontal: 3 });
            self.detail.render(frame, area, get_text("command_editor_editor"), String::new());
        }

        self.save_changes.render(frame, area);
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if let Some(message) = self.save_changes.handle_key(key, || {
            crate::editors::save_file(&self.path, || self.command_list.lock().unwrap().save(&self.path))
        }) {
            return message;
        }
        if let Some(message) = self.detail.handle_key(key) {
            return message;
        }

        match key.code {
            KeyCode::Esc => {
                return self.save_changes.request_close(self.command_list_orig != *self.command_list.lock().unwrap());
            }
            _ => match key.code {
                KeyCode::PageUp => self.insert_table.move_row(&mut self.command_list.lock().unwrap(), -1),
                KeyCode::PageDown => self.insert_table.move_row(&mut self.command_list.lock().unwrap(), 1),

                KeyCode::Insert => {
                    self.insert_table.push_row(
                        &mut *self.command_list.lock().unwrap(),
                        icy_board_engine::icy_board::commands::Command::default(),
                    );
                }
                KeyCode::Delete => {
                    if let Ok(mut commands) = self.command_list.lock() {
                        self.insert_table.remove_row(&mut *commands);
                    }
                }

                KeyCode::Enter => {
                    if let Some(selected_item) = self.insert_table.table_state.selected() {
                        let mut cmd: std::sync::MutexGuard<'_, CommandList> = self.command_list.lock().unwrap();
                        let Some(cur_prot) = cmd.get_mut(selected_item) else {
                            return PageMessage::None;
                        };
                        if cur_prot.actions.is_empty() {
                            cur_prot.actions.push(CommandAction::default());
                        }
                        self.detail.open(ConfigMenu {
                            obj: (selected_item, self.command_list.clone()),
                            entry: vec![
                                ConfigEntry::Item(
                                    ListItem::new(
                                        get_text("command_editor_keyword"),
                                        ListValue::Text(16, TextFlags::None, cur_prot.keyword.clone()),
                                    )
                                    .with_label_width(16)
                                    .with_update_text_value(
                                        &|(i, list): &(usize, Arc<Mutex<CommandList>>), value: String| {
                                            list.lock().unwrap()[*i].keyword = value;
                                        },
                                    ),
                                ),
                                ConfigEntry::Item(
                                    ListItem::new(get_text("command_editor_help"), ListValue::Text(16, TextFlags::None, cur_prot.help.clone()))
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
                                    ListItem::new(
                                        get_text("command_editor_parameter"),
                                        ListValue::Text(43, TextFlags::None, cur_prot.actions[0].parameter.clone()),
                                    )
                                    .with_label_width(16)
                                    .with_update_text_value(
                                        &|(i, list): &(usize, Arc<Mutex<CommandList>>), value: String| {
                                            list.lock().unwrap()[*i].actions[0].parameter = value;
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
            },
        }
        PageMessage::None
    }
}

pub fn edit_commands(_board: Arc<Mutex<IcyBoard>>, path: PathBuf) -> PageMessage {
    PageMessage::OpenSubPage(Box::new(CommandsEditor::new(&path).unwrap()))
}
