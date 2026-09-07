use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::editors::EditorList;
use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    Res,
    icy_board::{
        IcyBoard, IcyBoardSerializer,
        doors::{BBSLink, Door, DoorList, DoorServerAccount, DoorType, DropFile},
        security_expr::SecurityExpression,
    },
};
use icy_board_tui::{
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState, TextFlags},
    get_text, get_text_args,
    insert_table::{Column, InsertTable},
    tab_page::{Page, PageMessage},
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    text::Line,
    widgets::{Clear, ScrollbarState, TableState, Widget},
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

    menu: ConfigMenu<Arc<Mutex<DoorList>>>,
    menu_state: ConfigMenuState,
    mode: EditCommandMode,

    insert_table: InsertTable<'a>,
    detail: super::EditorDialog<(usize, Arc<Mutex<DoorList>>)>,
    save_changes: super::EditorSaveChanges,
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
        let l = 22;
        let items = vec![ConfigEntry::Group(
            get_text("doors_editor_bbslink_credentials"),
            vec![
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("doors_editor_system_code"),
                        ListValue::Text(25, TextFlags::None, bbs_link.system_code.clone()),
                    )
                    .with_label_width(l)
                    .with_update_text_value(&|list: &Arc<Mutex<DoorList>>, value: String| {
                        let DoorServerAccount::BBSLink(bbs_link) = &mut list.lock().unwrap().accounts[0];
                        bbs_link.system_code = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("doors_editor_auth_code"),
                        ListValue::Text(25, TextFlags::None, bbs_link.auth_code.clone()),
                    )
                    .with_label_width(l)
                    .with_update_text_value(&|list: &Arc<Mutex<DoorList>>, value: String| {
                        let DoorServerAccount::BBSLink(bbs_link) = &mut list.lock().unwrap().accounts[0];
                        bbs_link.auth_code = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("doors_editor_scheme_code"),
                        ListValue::Text(25, TextFlags::None, bbs_link.sheme_code.clone()),
                    )
                    .with_label_width(l)
                    .with_update_text_value(&|list: &Arc<Mutex<DoorList>>, value: String| {
                        let DoorServerAccount::BBSLink(bbs_link) = &mut list.lock().unwrap().accounts[0];
                        bbs_link.sheme_code = value;
                    }),
                ),
            ],
        )];

        let door_list = Arc::new(Mutex::new(door_list_orig.clone()));
        let menu = ConfigMenu {
            obj: door_list.clone(),
            entry: items,
        };
        let scroll_state = ScrollbarState::default().content_length(door_list_orig.doors.len());
        let content_length = door_list_orig.doors.len();
        let cmd2 = door_list.clone();

        let insert_table = InsertTable {
            scroll_state,
            table_state: TableState::default().with_selected(0),
            columns: vec![
                Column::new(get_text("doors_editor_header_door")).with_width(15),
                Column::new(get_text("doors_editor_header_description")).with_width(33),
                Column::new(get_text("doors_editor_header_type")),
            ],
            numbered: true,
            get_content: Box::new(move |_table, i, j| {
                if *i >= cmd2.lock().unwrap().len() {
                    return Line::from("".to_string());
                }
                match j {
                    0 => Line::from(cmd2.lock().unwrap()[*i].name.to_string()),
                    1 => Line::from(cmd2.lock().unwrap()[*i].description.to_string()),
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
            detail: super::EditorDialog::default(),
            save_changes: super::EditorSaveChanges::default(),
        })
    }

    fn display_insert_table(&mut self, frame: &mut Frame, area: &Rect) {
        let len = self.door_list.lock().unwrap().len();
        let selected = self.insert_table.table_state.selected();
        // Only the credentials form is focused, so the list hides its marker but keeps the row.
        let focused = if self.mode == EditCommandMode::Config { None } else { selected };
        self.insert_table.sync_rows(len, focused);
        self.insert_table.render_list(frame, *area);
        self.insert_table.sync_rows(len, selected);
    }

    fn add_door(&mut self) {
        let mut door_list = self.door_list.lock().unwrap();
        let selected = door_list.len();
        door_list.push(Door {
            name: format!("door{}", selected + 1),
            number: 0,
            valid: false,
            description: String::new(),
            password: String::new(),
            securiy_level: SecurityExpression::default(),
            use_shell_execute: false,
            door_type: DoorType::Local,
            path: String::new(),
            drop_file: Default::default(),
            dos_command: String::new(),
            dos_memory_mb: 64,
            dos_max_runtime_seconds: icy_board_engine::icy_board::doors::DEFAULT_DOS_MAX_RUNTIME_SECONDS,
            charge_per_use: 0.0,
            charge_per_minute: 0.0,
        });
        self.insert_table.sync_rows(door_list.len(), Some(selected));
        self.mode = EditCommandMode::Table;
    }
}

impl<'a> Page for DoorEditor<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        let conference_name = crate::tabs::conferences::get_cur_conference_name();
        let title = get_text_args("doors_editor_title", HashMap::from([("conference".to_string(), conference_name)]));

        let modal = self.detail.is_open() || self.save_changes.is_open() || self.menu_state.is_path_browser_open();
        let block = super::list_editor_frame(
            title,
            if self.mode == EditCommandMode::Config {
                get_text("doors_editor_key_help")
            } else {
                get_text("doors_editor_key_help_door")
            },
            modal,
        );

        block.render(area, frame.buffer_mut());

        let vertical = Layout::vertical([Constraint::Length(6), Constraint::Fill(1)]);
        let [menu_area, table_area] = vertical.areas(area.inner(Margin { vertical: 1, horizontal: 1 }));
        let sel = self.menu_state.selected;
        if self.mode == EditCommandMode::Table {
            self.menu_state.selected = usize::MAX;
        }
        super::render_config_form(frame, menu_area, &mut self.menu, &mut self.menu_state);
        self.menu_state.selected = sel;

        self.display_insert_table(frame, &table_area);

        self.detail.render(
            frame,
            area.inner(Margin { vertical: 4, horizontal: 3 }),
            get_text("doors_editor_edit_title"),
            String::new(),
        );
        self.save_changes.render(frame, area);
    }

    fn request_status(&self) -> ResultState {
        if self.detail.is_open() {
            self.detail.status()
        } else if self.mode == EditCommandMode::Config {
            ResultState::status_line(self.menu.current_status_line(&self.menu_state))
        } else {
            ResultState::default()
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if let Some(message) = self.save_changes.handle_key(key, || {
            super::save_file(&self.path, || {
                let list = self.door_list.lock().unwrap();
                for door in list.iter() {
                    super::accounting_rates::validate_activity_rates(door.charge_per_use, door.charge_per_minute)?;
                }
                list.save(&self.path)
            })
        }) {
            return message;
        }

        if let Some(message) = self.detail.handle_key(key) {
            return message;
        }

        if self.menu_state.is_path_browser_open() {
            return PageMessage::ResultState(self.menu.handle_key_press(key, &mut self.menu_state));
        }

        match key.code {
            KeyCode::Esc => {
                return self.save_changes.request_close(self.door_list_orig != *self.door_list.lock().unwrap());
            }

            KeyCode::Tab => {
                if self.mode == EditCommandMode::Config {
                    self.mode = EditCommandMode::Table;
                } else if self.mode == EditCommandMode::Table {
                    self.mode = EditCommandMode::Config;
                }
            }

            KeyCode::F(2) => self.add_door(),

            _ => match self.mode {
                EditCommandMode::Table => match key.code {
                    KeyCode::Insert => self.add_door(),
                    KeyCode::Delete => {
                        self.insert_table.remove_row(&mut *self.door_list.lock().unwrap());
                    }

                    KeyCode::Enter => {
                        if let Some(selected_item) = self.insert_table.table_state.selected() {
                            let cmd = self.door_list.lock().unwrap();
                            let Some(action) = cmd.get(selected_item) else {
                                return PageMessage::None;
                            };
                            self.detail.open(super::align_editor_labels(ConfigMenu {
                                obj: (selected_item, self.door_list.clone()),
                                entry: vec![
                                    ConfigEntry::Item(
                                        ListItem::new(get_text("door_editor_name"), ListValue::Text(30, TextFlags::None, action.name.clone()))
                                            .with_label_width(16)
                                            .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<DoorList>>), value: String| {
                                                list.lock().unwrap()[*i].name = value;
                                            }),
                                    ),
                                    ConfigEntry::Item(
                                        ListItem::new(
                                            get_text("door_editor_description"),
                                            ListValue::Text(30, TextFlags::None, action.description.clone()),
                                        )
                                        .with_label_width(16)
                                        .with_update_text_value(
                                            &|(i, list): &(usize, Arc<Mutex<DoorList>>), value: String| {
                                                list.lock().unwrap()[*i].description = value;
                                            },
                                        ),
                                    ),
                                    ConfigEntry::Item(
                                        ListItem::new(
                                            get_text("door_editor_password"),
                                            ListValue::Text(30, TextFlags::Password, action.password.clone()),
                                        )
                                        .with_label_width(16)
                                        .with_update_text_value(
                                            &|(i, list): &(usize, Arc<Mutex<DoorList>>), value: String| {
                                                list.lock().unwrap()[*i].password = value;
                                            },
                                        ),
                                    ),
                                    ConfigEntry::Item(
                                        ListItem::new(get_text("door_editor_path"), ListValue::Text(30, TextFlags::None, action.path.clone()))
                                            .with_label_width(16)
                                            .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<DoorList>>), value: String| {
                                                list.lock().unwrap()[*i].path = value;
                                            }),
                                    ),
                                    ConfigEntry::Item(
                                        ListItem::new(
                                            get_text("door_editor_security"),
                                            ListValue::Security(action.securiy_level.clone(), action.securiy_level.to_string()),
                                        )
                                        .with_label_width(16)
                                        .with_update_sec_value(
                                            &|(i, list): &(usize, Arc<Mutex<DoorList>>), value: SecurityExpression| {
                                                list.lock().unwrap()[*i].securiy_level = value;
                                            },
                                        ),
                                    ),
                                    ConfigEntry::Item(
                                        ListItem::new(
                                            get_text("accounting_activity_per_use"),
                                            ListValue::Float(action.charge_per_use, action.charge_per_use.to_string()),
                                        )
                                        .with_status(get_text("accounting_activity_per_use-status"))
                                        .with_help(get_text("accounting_activity_per_use-help"))
                                        .with_update_float_value(
                                            &|(i, list): &(usize, Arc<Mutex<DoorList>>), value| {
                                                list.lock().unwrap()[*i].charge_per_use = value;
                                            },
                                        ),
                                    ),
                                    ConfigEntry::Item(
                                        ListItem::new(
                                            get_text("accounting_activity_per_minute"),
                                            ListValue::Float(action.charge_per_minute, action.charge_per_minute.to_string()),
                                        )
                                        .with_status(get_text("accounting_activity_per_minute-status"))
                                        .with_help(get_text("accounting_activity_per_minute-help"))
                                        .with_update_float_value(
                                            &|(i, list): &(usize, Arc<Mutex<DoorList>>), value| {
                                                list.lock().unwrap()[*i].charge_per_minute = value;
                                            },
                                        ),
                                    ),
                                    ConfigEntry::Item(
                                        ListItem::new(
                                            get_text("door_editor_door_type"),
                                            ListValue::ComboBox(ComboBox {
                                                cur_value: ComboBoxValue::new(format!("{}", action.door_type), format!("{}", action.door_type)),
                                                selected_item: 0,
                                                is_edit_open: false,
                                                first_item: 0,
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
                                                } else if value.cur_value.value == "Dos" {
                                                    list.lock().unwrap()[*i].door_type = DoorType::Dos;
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
                                    ConfigEntry::Item(
                                        ListItem::new(
                                            get_text("door_editor_drop_file"),
                                            ListValue::ComboBox(ComboBox {
                                                cur_value: ComboBoxValue::new(action.drop_file.to_string(), format!("{:?}", action.drop_file)),
                                                selected_item: 0,
                                                is_edit_open: false,
                                                first_item: 0,
                                                values: DropFile::iter()
                                                    .map(|drop_file| ComboBoxValue::new(drop_file.to_string(), format!("{drop_file:?}")))
                                                    .collect(),
                                            }),
                                        )
                                        .with_label_width(16)
                                        .with_update_combobox_value(
                                            &|(i, list): &(usize, Arc<Mutex<DoorList>>), value: &ComboBox| {
                                                if let Some(drop_file) = DropFile::iter().find(|drop_file| format!("{drop_file:?}") == value.cur_value.value) {
                                                    list.lock().unwrap()[*i].drop_file = drop_file;
                                                }
                                            },
                                        ),
                                    ),
                                    ConfigEntry::Item(
                                        ListItem::new(
                                            get_text("door_editor_dos_command"),
                                            ListValue::Text(60, TextFlags::None, action.dos_command.clone()),
                                        )
                                        .with_label_width(16)
                                        .with_update_text_value(
                                            &|(i, list): &(usize, Arc<Mutex<DoorList>>), value: String| {
                                                list.lock().unwrap()[*i].dos_command = value;
                                            },
                                        ),
                                    ),
                                    ConfigEntry::Item(
                                        ListItem::new(get_text("door_editor_dos_memory"), ListValue::U32(action.dos_memory_mb, 1, 512))
                                            .with_label_width(16)
                                            .with_update_u32_value(&|(i, list): &(usize, Arc<Mutex<DoorList>>), value: u32| {
                                                list.lock().unwrap()[*i].dos_memory_mb = value;
                                            }),
                                    ),
                                    ConfigEntry::Item(
                                        ListItem::new(
                                            get_text("door_editor_dos_max_seconds"),
                                            ListValue::U32(action.dos_max_runtime_seconds, 0, 86400),
                                        )
                                        .with_label_width(16)
                                        .with_update_u32_value(
                                            &|(i, list): &(usize, Arc<Mutex<DoorList>>), value: u32| {
                                                list.lock().unwrap()[*i].dos_max_runtime_seconds = value;
                                            },
                                        ),
                                    ),
                                ],
                            }));
                        } else {
                            self.insert_table.handle_key_press(key).unwrap();
                        }
                    }

                    _ => {
                        self.insert_table.handle_key_press(key).unwrap();
                    }
                },
                EditCommandMode::Config => {
                    return PageMessage::ResultState(self.menu.handle_key_press(key, &mut self.menu_state));
                }
            },
        }
        PageMessage::None
    }
}

pub fn edit_doors(_board: (usize, Arc<Mutex<IcyBoard>>), path: PathBuf) -> PageMessage {
    PageMessage::OpenSubPage(Box::new(DoorEditor::new(&path).unwrap()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn modal_hints_and_failed_save_preserve_door_modes() {
        let directory = tempfile::tempdir().unwrap();
        let parent = directory.path().join("blocked");
        std::fs::write(&parent, b"not a directory").unwrap();
        let path = parent.join("doors.toml");
        let mut editor = DoorEditor::new(&path).unwrap();
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 25)).unwrap();
        let mut assert_hint = |editor: &mut DoorEditor<'_>, hint: Option<&str>| {
            terminal.draw(|frame| editor.render(frame, Rect::new(0, 1, 80, 23))).unwrap();
            let border: String = (0..80).map(|x| terminal.backend().buffer()[(x, 23)].symbol()).collect();
            if let Some(hint) = hint {
                assert!(border.contains(&get_text(hint)));
            } else {
                assert!(!border.contains(&get_text("doors_editor_key_help")));
                assert!(!border.contains(&get_text("doors_editor_key_help_door")));
            }
        };

        assert_hint(&mut editor, Some("doors_editor_key_help"));
        editor.handle_key_press(key(KeyCode::F(2)));
        assert_hint(&mut editor, Some("doors_editor_key_help_door"));
        editor.handle_key_press(key(KeyCode::Enter));
        assert_hint(&mut editor, None);
        editor.handle_key_press(key(KeyCode::Tab));
        assert_eq!(editor.mode, EditCommandMode::Table);
        editor.handle_key_press(key(KeyCode::Esc));
        assert_hint(&mut editor, Some("doors_editor_key_help_door"));
        editor.handle_key_press(key(KeyCode::Esc));
        assert!(editor.save_changes.is_open());
        assert_hint(&mut editor, None);
        editor.handle_key_press(key(KeyCode::Right));
        assert!(matches!(
            editor.handle_key_press(key(KeyCode::Enter)),
            PageMessage::InfoBox(icy_board_tui::tab_page::InfoState::Error, _)
        ));
        assert!(!editor.save_changes.is_open());
        editor.handle_key_press(key(KeyCode::Tab));
        assert_eq!(editor.mode, EditCommandMode::Config);
        assert_hint(&mut editor, Some("doors_editor_key_help"));
        std::fs::remove_file(parent).unwrap();
        editor.handle_key_press(key(KeyCode::Esc));
        editor.handle_key_press(key(KeyCode::Right));
        assert!(matches!(editor.handle_key_press(key(KeyCode::Enter)), PageMessage::Close));
        assert_eq!(DoorList::load(&path).unwrap().len(), 1);
    }

    #[test]
    fn empty_door_table_accepts_focus_and_f2_creates_the_first_door() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("doors.toml");
        let mut editor = DoorEditor::new(&path).unwrap();
        assert!(editor.door_list.lock().unwrap().is_empty());

        editor.handle_key_press(key(KeyCode::Tab));
        assert_eq!(editor.mode, EditCommandMode::Table);
        editor.handle_key_press(key(KeyCode::PageDown));
        editor.handle_key_press(key(KeyCode::F(2)));

        assert_eq!(editor.door_list.lock().unwrap().len(), 1);
        assert_eq!(editor.insert_table.table_state.selected(), Some(0));
    }

    #[test]
    fn door_form_exposes_security_and_every_drop_file_format() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("doors.toml");
        let mut editor = DoorEditor::new(&path).unwrap();
        editor.handle_key_press(key(KeyCode::Tab));
        editor.handle_key_press(key(KeyCode::F(2)));
        editor.handle_key_press(key(KeyCode::Enter));

        let entries = &editor.detail.menu.as_ref().unwrap().entry;
        assert_eq!(
            entries
                .iter()
                .filter(|entry| matches!(entry, ConfigEntry::Item(item) if matches!(item.value, ListValue::Security(..))))
                .count(),
            1
        );
        let drop_files = entries.iter().find_map(|entry| match entry {
            ConfigEntry::Item(item) => match &item.value {
                ListValue::ComboBox(combo) if combo.values.len() == DropFile::iter().count() => Some(combo),
                _ => None,
            },
            _ => None,
        });
        assert_eq!(drop_files.unwrap().values.len(), 13);
    }
}
