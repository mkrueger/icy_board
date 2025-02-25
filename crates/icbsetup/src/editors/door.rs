use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    Res,
    icy_board::{
        IcyBoard, IcyBoardSerializer,
        doors::{BBSLink, Door, DoorList, DoorServerAccount, DoorType},
        security_expr::SecurityExpression,
    },
};
use icy_board_tui::{
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue},
    get_text, get_text_args,
    insert_table::InsertTable,
    save_changes_dialog::SaveChangesDialog,
    tab_page::{Page, PageMessage},
    theme::get_tui_theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, ScrollbarState, TableState, Widget, block::Title},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditCommandMode {
    Config,
    Table,
}

pub struct DoorEditor<'a> {
    path: std::path::PathBuf,
    door_list_orig: DoorList,
    door_list: Arc<Mutex<DoorList>>,

    menu: ConfigMenu<u32>,
    menu_state: ConfigMenuState,
    mode: EditCommandMode,

    insert_table: InsertTable<'a>,
    edit_config_state: ConfigMenuState,
    edit_config: Option<ConfigMenu<(usize, Arc<Mutex<DoorList>>)>>,
    save_dialog: Option<SaveChangesDialog>,
}

impl<'a> DoorEditor<'a> {
    pub(crate) fn new(path: &std::path::PathBuf) -> Res<Self> {
        let mut door_list_orig = if path.exists() {
            DoorList::load(&path)?
        } else {
            let mut door_list = DoorList::default();
            door_list.accounts.push(DoorServerAccount::BBSLink(BBSLink::default()));
            door_list
        };

        if door_list_orig.accounts.is_empty() {
            door_list_orig.accounts.push(DoorServerAccount::BBSLink(BBSLink::default()));
        }

        let DoorServerAccount::BBSLink(bbs_link) = &door_list_orig.accounts[0];
        let l = 16;
        let items = vec![ConfigEntry::Group(
            "BBSLink credentials".to_string(),
            vec![
                ConfigEntry::Item(ListItem::new("System Code".to_string(), ListValue::Text(25, bbs_link.system_code.clone())).with_label_width(l)),
                ConfigEntry::Item(ListItem::new("Auth Code".to_string(), ListValue::Text(25, bbs_link.auth_code.clone())).with_label_width(l)),
                ConfigEntry::Item(ListItem::new("Scheme Code".to_string(), ListValue::Text(25, bbs_link.sheme_code.clone())).with_label_width(l)),
            ],
        )];

        let menu = ConfigMenu { obj: 0, entry: items };
        let door_list = Arc::new(Mutex::new(door_list_orig.clone()));
        let scroll_state = ScrollbarState::default().content_length(door_list_orig.doors.len());
        let content_length = door_list_orig.doors.len();
        let cmd2 = door_list.clone();

        let insert_table = InsertTable {
            scroll_state,
            table_state: TableState::default().with_selected(0),
            headers: vec![
                format!("{:<15}", get_text("doors_editor_header_door")),
                format!("{:<33}", get_text("doors_editor_header_description")),
                get_text("doors_editor_header_type"),
            ],
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
            door_list_orig,
            menu,
            menu_state: ConfigMenuState::default(),
            insert_table,
            mode: EditCommandMode::Config,
            edit_config: None,
            edit_config_state: ConfigMenuState::default(),
            save_dialog: None,
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

impl<'a> Page for DoorEditor<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        let conference_name = crate::tabs::conferences::get_cur_conference_name();
        let title = get_text_args("doors_editor_title", HashMap::from([("conference".to_string(), conference_name)]));

        let block = Block::new()
            .title_alignment(Alignment::Center)
            .title(Title::from(Span::from(title).style(get_tui_theme().dialog_box_title)))
            .style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_set(icy_board_tui::BORDER_SET)
            .title_bottom(Span::styled(
                if self.mode == EditCommandMode::Config {
                    get_text("doors_editor_key_help")
                } else {
                    get_text("doors_editor_key_help_door")
                },
                get_tui_theme().key_binding,
            ));

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
            let area = area.inner(Margin { vertical: 7, horizontal: 3 });
            Clear.render(area, frame.buffer_mut());
            let block = Block::new()
                .title_alignment(Alignment::Center)
                .title(Title::from(
                    Span::from(get_text("doors_editor_edit_title")).style(get_tui_theme().dialog_box_title),
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
                    self.door_list.lock().unwrap().save(&self.path).unwrap();
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
                if self.door_list_orig == self.door_list.lock().unwrap().clone() {
                    return PageMessage::Close;
                }
                self.save_dialog = Some(SaveChangesDialog::new());
                return PageMessage::None;
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
                        let name = format!("door{}", self.door_list.lock().unwrap().len() + 1);
                        if let Ok(lock) = &mut self.door_list.lock() {
                            lock.push(Door {
                                name,
                                description: "".to_string(),
                                password: "".to_string(),
                                securiy_level: SecurityExpression::default(),
                                use_shell_execute: false,
                                door_type: DoorType::BBSlink,
                                path: "".to_string(),
                                drop_file: Default::default(),
                            });
                        }
                        self.insert_table.content_length += 1;
                    }
                    KeyCode::Delete => {
                        if let Some(selected_item) = self.insert_table.table_state.selected() {
                            if selected_item < self.door_list.lock().unwrap().len() {
                                self.door_list.lock().unwrap().remove(selected_item);
                                self.insert_table.content_length -= 1;
                            }
                        }
                    }

                    KeyCode::Enter => {
                        self.edit_config_state = ConfigMenuState::default();

                        if let Some(selected_item) = self.insert_table.table_state.selected() {
                            let cmd = self.door_list.lock().unwrap();
                            let Some(action) = cmd.get(selected_item) else {
                                return PageMessage::None;
                            };
                            self.edit_config = Some(ConfigMenu {
                                obj: (selected_item, self.door_list.clone()),
                                entry: vec![
                                    ConfigEntry::Item(
                                        ListItem::new(get_text("door_editor_name"), ListValue::Text(30, action.name.clone()))
                                            .with_label_width(16)
                                            .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<DoorList>>), value: String| {
                                                list.lock().unwrap()[*i].name = value;
                                            }),
                                    ),
                                    ConfigEntry::Item(
                                        ListItem::new(get_text("door_editor_description"), ListValue::Text(30, action.description.clone()))
                                            .with_label_width(16)
                                            .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<DoorList>>), value: String| {
                                                list.lock().unwrap()[*i].description = value;
                                            }),
                                    ),
                                    ConfigEntry::Item(
                                        ListItem::new(get_text("door_editor_password"), ListValue::Text(30, action.password.clone()))
                                            .with_label_width(16)
                                            .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<DoorList>>), value: String| {
                                                list.lock().unwrap()[*i].password = value;
                                            }),
                                    ),
                                    ConfigEntry::Item(
                                        ListItem::new(get_text("door_editor_path"), ListValue::Text(30, action.path.clone()))
                                            .with_label_width(16)
                                            .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<DoorList>>), value: String| {
                                                list.lock().unwrap()[*i].path = value;
                                            }),
                                    ),
                                    ConfigEntry::Item(
                                        ListItem::new(
                                            get_text("door_editor_door_type"),
                                            ListValue::ComboBox(ComboBox {
                                                cur_value: ComboBoxValue::new(format!("{}", action.door_type), format!("{}", action.door_type)),
                                                first: 0,
                                                scroll_state: ScrollbarState::default(),
                                                values: DoorType::iter()
                                                    .map(|x| ComboBoxValue::new(format!("{}", x), format!("{}", x)))
                                                    .collect::<Vec<ComboBoxValue>>(),
                                            }),
                                        )
                                        .with_label_width(16)
                                        .with_update_combobox_value(
                                            &|(i, list): &(usize, Arc<Mutex<DoorList>>), value: &ComboBox| {
                                                if value.cur_value.value == "BBSlink" {
                                                    list.lock().unwrap()[*i].door_type = DoorType::BBSlink;
                                                } else {
                                                    list.lock().unwrap()[*i].door_type = DoorType::Local;
                                                }
                                            },
                                        ),
                                    ),
                                    ConfigEntry::Item(
                                        ListItem::new(get_text("door_editor_use_shell_execute"), ListValue::Bool(action.use_shell_execute))
                                            .with_label_width(16)
                                            .with_update_bool_value(&|(i, list): &(usize, Arc<Mutex<DoorList>>), value: bool| {
                                                list.lock().unwrap()[*i].use_shell_execute = value;
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
                EditCommandMode::Config => {
                    self.menu.handle_key_press(key, &mut self.menu_state);
                }
            },
        }
        PageMessage::None
    }
}

pub fn edit_doors(_board: (usize, Arc<Mutex<IcyBoard>>), path: PathBuf) -> PageMessage {
    PageMessage::OpenSubPage(Box::new(DoorEditor::new(&path).unwrap()))
}
