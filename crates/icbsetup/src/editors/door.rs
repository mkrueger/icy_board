use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    icy_board::{
        doors::{BBSLink, Door, DoorList, DoorServerAccount, DoorType},
        security_expr::SecurityExpression,
        IcyBoardSerializer,
    },
    Res,
};
use icy_board_tui::{
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue},
    insert_table::InsertTable,
    tab_page::Editor,
    theme::get_tui_theme,
};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    text::{Line, Span},
    widgets::{block::Title, Block, BorderType, Borders, Clear, Padding, ScrollbarState, TableState, Widget},
    Frame,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditCommandMode {
    Config,
    Table,
}

pub struct DoorEditor<'a> {
    path: std::path::PathBuf,
    door_list: DoorList,
    menu: ConfigMenu,
    menu_state: ConfigMenuState,
    mode: EditCommandMode,

    insert_table: InsertTable<'a>,
    command: Arc<Mutex<Vec<Door>>>,

    edit_config_state: ConfigMenuState,
    edit_config: Option<ConfigMenu>,
}

impl<'a> DoorEditor<'a> {
    pub(crate) fn new(path: &std::path::PathBuf) -> Res<Self> {
        let mut door_list = if path.exists() {
            DoorList::load(&path)?
        } else {
            let mut door_list = DoorList::default();
            door_list.accounts.push(DoorServerAccount::BBSLink(BBSLink::default()));
            door_list
        };

        if door_list.accounts.is_empty() {
            door_list.accounts.push(DoorServerAccount::BBSLink(BBSLink::default()));
        }

        let DoorServerAccount::BBSLink(bbs_link) = &door_list.accounts[0];
        let l = 16;
        let items = vec![ConfigEntry::Group(
            "BBSLink credentials".to_string(),
            vec![
                ConfigEntry::Item(
                    ListItem::new("system_code", "System Code".to_string(), ListValue::Text(25, bbs_link.system_code.clone())).with_label_width(l),
                ),
                ConfigEntry::Item(ListItem::new("auth_code", "Auth Code".to_string(), ListValue::Text(25, bbs_link.auth_code.clone())).with_label_width(l)),
                ConfigEntry::Item(ListItem::new("sheme_code", "Scheme Code".to_string(), ListValue::Text(25, bbs_link.sheme_code.clone())).with_label_width(l)),
            ],
        )];

        let menu = ConfigMenu { entry: items };
        let command_arc = Arc::new(Mutex::new(door_list.doors.clone()));
        let scroll_state = ScrollbarState::default().content_length(door_list.doors.len());
        let content_length = door_list.doors.len();
        let cmd2 = command_arc.clone();
        let insert_table = InsertTable {
            scroll_state,
            table_state: TableState::default().with_selected(0),
            headers: vec!["Door    ".to_string(), "Description".to_string(), "Type".to_string()],
            get_content: Box::new(move |_table, i, j| {
                if *i >= cmd2.lock().unwrap().len() {
                    return Line::from("".to_string());
                }
                match j {
                    0 => Line::from(format!("{}", cmd2.lock().unwrap()[*i].name)),
                    1 => Line::from(format!("{}", cmd2.lock().unwrap()[*i].description)),
                    2 => Line::from(format!("{}", cmd2.lock().unwrap()[*i].door_type)),
                    _ => Line::from("".to_string()),
                }
            }),
            content_length,
        };

        Ok(Self {
            path: path.clone(),
            door_list,
            menu,
            menu_state: ConfigMenuState::default(),
            insert_table,
            command: command_arc,
            mode: EditCommandMode::Config,
            edit_config: None,
            edit_config_state: ConfigMenuState::default(),
        })
    }

    fn display_insert_table(&mut self, frame: &mut Frame, area: &Rect) {
        let sel = self.insert_table.table_state.selected();
        if self.mode == EditCommandMode::Config {
            self.insert_table.table_state.select(None);
        }
        self.insert_table.render_table(frame, *area);
        self.insert_table.table_state.select(sel);
    }
}

impl<'a> Editor for DoorEditor<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        let block = Block::new()
            .title_alignment(Alignment::Center)
            .title(Title::from(Span::from(" Edit Doors ").style(get_tui_theme().content_box_title)))
            .style(get_tui_theme().content_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        block.render(area, frame.buffer_mut());

        let vertical = Layout::vertical([Constraint::Length(6), Constraint::Fill(1)]);
        let [menu_area, table_area] = vertical.areas(area.inner(Margin { vertical: 1, horizontal: 1 }));
        let sel = self.menu_state.selected;
        if self.mode == EditCommandMode::Table {
            self.menu_state.selected = usize::MAX;
        }
        self.menu.render(menu_area, frame, &mut self.menu_state);
        self.menu_state.selected = sel;

        self.display_insert_table(frame, &table_area);

        self.menu
            .get_item(self.menu_state.selected)
            .unwrap()
            .text_field_state
            .set_cursor_position(frame);

        if let Some(edit_config) = &mut self.edit_config {
            let area = area.inner(Margin { vertical: 8, horizontal: 3 });
            Clear.render(area, frame.buffer_mut());
            let block = Block::new()
                .title_alignment(Alignment::Center)
                .title(Title::from(Span::from(" Edit Door ").style(get_tui_theme().content_box_title)))
                .style(get_tui_theme().content_box)
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
                                if let ListValue::Text(_, ref value) = item.value {
                                    self.command.lock().unwrap()[selected_item].name = value.clone();
                                }
                            }
                            "description" => {
                                if let ListValue::Text(_, ref value) = item.value {
                                    self.command.lock().unwrap()[selected_item].description = value.clone();
                                }
                            }
                            "password" => {
                                if let ListValue::Text(_, ref value) = item.value {
                                    self.command.lock().unwrap()[selected_item].password = value.clone();
                                }
                            }
                            "path" => {
                                if let ListValue::Text(_, ref value) = item.value {
                                    self.command.lock().unwrap()[selected_item].path = value.clone();
                                }
                            }
                            "use_shell_execute" => {
                                if let ListValue::Bool(ref value) = item.value {
                                    self.command.lock().unwrap()[selected_item].use_shell_execute = *value;
                                }
                            }

                            "door_type" => {
                                if let ListValue::ComboBox(ref value) = item.value {
                                    let value = value.cur_value.value.parse::<DoorType>().unwrap();
                                    self.command.lock().unwrap()[selected_item].door_type = value;
                                }
                            }
                            _ => {}
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
                for item in self.menu.iter() {
                    if let ListValue::Text(_, value) = &item.value {
                        match item.id.as_str() {
                            "system_code" => {
                                let DoorServerAccount::BBSLink(bbs_link) = &mut self.door_list.accounts[0];
                                bbs_link.system_code = value.clone();
                            }
                            "auth_code" => {
                                let DoorServerAccount::BBSLink(bbs_link) = &mut self.door_list.accounts[0];
                                bbs_link.auth_code = value.clone();
                            }
                            "sheme_code" => {
                                let DoorServerAccount::BBSLink(bbs_link) = &mut self.door_list.accounts[0];
                                bbs_link.sheme_code = value.clone();
                            }
                            _ => {}
                        }
                    }
                }
                self.door_list.doors.clear();
                self.door_list.doors.append(&mut self.command.lock().unwrap());
                self.door_list.save(&self.path).unwrap();
                return false;
            }

            KeyCode::Tab => {
                if self.mode == EditCommandMode::Config {
                    self.mode = EditCommandMode::Table;
                } else if self.mode == EditCommandMode::Table {
                    self.mode = EditCommandMode::Config;
                }
            }

            _ => match self.mode {
                EditCommandMode::Table => match key.code {
                    KeyCode::Insert => {
                        self.command.lock().unwrap().push(Door {
                            name: format!("door{}", self.door_list.len() + 1),
                            description: "".to_string(),
                            password: "".to_string(),
                            securiy_level: SecurityExpression::default(),
                            use_shell_execute: false,
                            door_type: DoorType::BBSlink,
                            path: "".to_string(),
                            drop_file: Default::default(),
                        });
                        self.insert_table.content_length += 1;
                    }
                    KeyCode::Delete => {
                        if let Some(selected_item) = self.insert_table.table_state.selected() {
                            if selected_item < self.command.lock().unwrap().len() {
                                self.command.lock().unwrap().remove(selected_item);
                                self.insert_table.content_length -= 1;
                            }
                        }
                    }

                    KeyCode::Enter => {
                        self.edit_config_state = ConfigMenuState::default();

                        if let Some(selected_item) = self.insert_table.table_state.selected() {
                            let cmd = self.command.lock().unwrap();
                            let Some(action) = cmd.get(selected_item) else {
                                return true;
                            };
                            self.edit_config = Some(ConfigMenu {
                                entry: vec![
                                    ConfigEntry::Item(ListItem::new("name", "Name".to_string(), ListValue::Text(30, action.name.clone())).with_label_width(16)),
                                    ConfigEntry::Item(
                                        ListItem::new("description", "Description".to_string(), ListValue::Text(30, action.description.clone()))
                                            .with_label_width(16),
                                    ),
                                    ConfigEntry::Item(
                                        ListItem::new("password", "Password".to_string(), ListValue::Text(30, action.password.clone())).with_label_width(16),
                                    ),
                                    ConfigEntry::Item(ListItem::new("path", "Path".to_string(), ListValue::Text(30, action.path.clone())).with_label_width(16)),
                                    ConfigEntry::Item(
                                        ListItem::new(
                                            "door_type",
                                            "Door Type".to_string(),
                                            ListValue::ComboBox(ComboBox {
                                                cur_value: ComboBoxValue::new(format!("{}", action.door_type), format!("{}", action.door_type)),
                                                first: 0,
                                                scroll_state: ScrollbarState::default(),
                                                values: DoorType::iter()
                                                    .map(|x| ComboBoxValue::new(format!("{}", x), format!("{}", x)))
                                                    .collect::<Vec<ComboBoxValue>>(),
                                            }),
                                        )
                                        .with_label_width(16),
                                    ),
                                    ConfigEntry::Item(
                                        ListItem::new("use_shell_execute", "Use Shell Execute".to_string(), ListValue::Bool(action.use_shell_execute))
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
                EditCommandMode::Config => {
                    self.menu.handle_key_press(key, &mut self.menu_state);
                }
            },
        }
        true
    }
}
