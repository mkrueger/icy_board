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
        doors::{BBSLink, Door, DoorList, DoorServerAccount, DoorType, DropFile, launch},
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

type DoorItem = ListItem<(usize, Arc<Mutex<DoorList>>)>;

fn door_item(key: &str, value: ListValue) -> DoorItem {
    ListItem::new(get_text(key), value)
        .with_label_width(16)
        .with_status(get_text(&format!("{key}-status")))
        .with_help(get_text(&format!("{key}-help")))
}

/// The path is a program, a door directory or a remote code, depending on the type.
const PATH_KEYS: [&str; 3] = ["door_editor_path_local", "door_editor_path_dos", "door_editor_path_bbslink"];

fn path_key(door_type: &DoorType) -> &'static str {
    match door_type {
        DoorType::Local => PATH_KEYS[0],
        DoorType::Dos => PATH_KEYS[1],
        DoorType::BBSlink => PATH_KEYS[2],
    }
}

/// Reserves room for every type's label so switching the type cannot clip it.
fn path_item(door_type: &DoorType, value: ListValue) -> DoorItem {
    let width = PATH_KEYS.iter().map(|key| Line::raw(get_text(key)).width() as u16).max().unwrap_or(0);
    door_item(path_key(door_type), value).with_label_width(width.max(16))
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
    path_label: &'static str,
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
            path_label: PATH_KEYS[0],
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
            args: Vec::new(),
            working_directory: String::new(),
            provide_socket_connection: false,
            max_parallel: 0,
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

    fn refresh_door_fields(&mut self) {
        let Some(menu) = self.detail.menu.as_mut() else { return };
        let ConfigEntry::Item(type_item) = &menu.entry[7] else { return };
        let ListValue::ComboBox(combo) = &type_item.value else { return };
        let local = combo.cur_value.value == "Local";
        let dos = combo.cur_value.value == "Dos";
        let path_label = if local {
            PATH_KEYS[0]
        } else if dos {
            PATH_KEYS[1]
        } else {
            PATH_KEYS[2]
        };
        if self.path_label != path_label {
            self.path_label = path_label;
            if let Some(ConfigEntry::Item(item)) = menu.entry.get_mut(3) {
                item.set_title(get_text(path_label));
                item.status = get_text(&format!("{path_label}-status"));
                item.help = get_text(&format!("{path_label}-help"));
            }
        }
        let fields = [
            ("door_editor_use_shell_execute", local),
            ("door_editor_args", local),
            ("door_editor_working_directory", local),
            ("door_editor_provide_socket", local),
            ("door_editor_max_parallel", local),
            ("door_editor_drop_file", local || dos),
            ("door_editor_dos_command", dos),
            ("door_editor_dos_memory", dos),
            ("door_editor_dos_max_seconds", dos),
        ];
        for (entry, (key, editable)) in menu.entry.iter_mut().skip(8).zip(fields) {
            let ConfigEntry::Item(item) = entry else { continue };
            if item.editable() == editable {
                continue;
            }
            item.set_editable(editable);
            item.status = get_text(&format!("{key}-status"));
            item.help = get_text(&format!("{key}-help"));
            if !editable {
                let reason = get_text("door_editor_unused_for_type");
                item.status.push_str(&format!(" - {reason}"));
                item.help.push_str(&format!("\n\n{reason}"));
            }
        }
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
                "doors_editor_key_help"
            } else {
                "doors_editor_key_help_door"
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
            area.inner(Margin { vertical: 2, horizontal: 3 }),
            get_text("doors_editor_edit_title"),
            "",
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
            self.refresh_door_fields();
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
                            self.path_label = path_key(&action.door_type);
                            self.detail.open(super::align_editor_labels(ConfigMenu {
                                obj: (selected_item, self.door_list.clone()),
                                entry: vec![
                                    ConfigEntry::Item(
                                        door_item("door_editor_name", ListValue::Text(30, TextFlags::None, action.name.clone())).with_update_text_value(
                                            &|(i, list): &(usize, Arc<Mutex<DoorList>>), value: String| {
                                                list.lock().unwrap()[*i].name = value;
                                            },
                                        ),
                                    ),
                                    ConfigEntry::Item(
                                        door_item("door_editor_description", ListValue::Text(30, TextFlags::None, action.description.clone()))
                                            .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<DoorList>>), value: String| {
                                                list.lock().unwrap()[*i].description = value;
                                            }),
                                    ),
                                    ConfigEntry::Item(
                                        door_item("door_editor_password", ListValue::Text(30, TextFlags::Password, action.password.clone()))
                                            .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<DoorList>>), value: String| {
                                                list.lock().unwrap()[*i].password = value;
                                            }),
                                    ),
                                    ConfigEntry::Item(
                                        path_item(&action.door_type, ListValue::Text(30, TextFlags::None, action.path.clone())).with_update_text_value(
                                            &|(i, list): &(usize, Arc<Mutex<DoorList>>), value: String| {
                                                list.lock().unwrap()[*i].path = value;
                                            },
                                        ),
                                    ),
                                    ConfigEntry::Item(
                                        door_item(
                                            "door_editor_security",
                                            ListValue::Security(action.securiy_level.clone(), action.securiy_level.to_string()),
                                        )
                                        .with_update_sec_value(
                                            &|(i, list): &(usize, Arc<Mutex<DoorList>>), value: SecurityExpression| {
                                                list.lock().unwrap()[*i].securiy_level = value;
                                            },
                                        ),
                                    ),
                                    ConfigEntry::Item(
                                        door_item(
                                            "accounting_activity_per_use",
                                            ListValue::Float(action.charge_per_use, action.charge_per_use.to_string()),
                                        )
                                        .with_update_float_value(
                                            &|(i, list): &(usize, Arc<Mutex<DoorList>>), value| {
                                                list.lock().unwrap()[*i].charge_per_use = value;
                                            },
                                        ),
                                    ),
                                    ConfigEntry::Item(
                                        door_item(
                                            "accounting_activity_per_minute",
                                            ListValue::Float(action.charge_per_minute, action.charge_per_minute.to_string()),
                                        )
                                        .with_update_float_value(
                                            &|(i, list): &(usize, Arc<Mutex<DoorList>>), value| {
                                                list.lock().unwrap()[*i].charge_per_minute = value;
                                            },
                                        ),
                                    ),
                                    ConfigEntry::Item(
                                        door_item(
                                            "door_editor_door_type",
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
                                        door_item("door_editor_use_shell_execute", ListValue::Bool(action.use_shell_execute)).with_update_bool_value(
                                            &|(i, list): &(usize, Arc<Mutex<DoorList>>), value: bool| {
                                                list.lock().unwrap()[*i].use_shell_execute = value;
                                            },
                                        ),
                                    ),
                                    ConfigEntry::Item(
                                        door_item("door_editor_args", ListValue::Text(60, TextFlags::None, launch::join_arguments(&action.args)))
                                            .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<DoorList>>), value: String| {
                                                if let Ok(args) = launch::parse_arguments(&value) {
                                                    list.lock().unwrap()[*i].args = args;
                                                }
                                            }),
                                    ),
                                    ConfigEntry::Item(
                                        door_item(
                                            "door_editor_working_directory",
                                            ListValue::Text(30, TextFlags::None, action.working_directory.clone()),
                                        )
                                        .with_update_text_value(
                                            &|(i, list): &(usize, Arc<Mutex<DoorList>>), value: String| {
                                                list.lock().unwrap()[*i].working_directory = value;
                                            },
                                        ),
                                    ),
                                    ConfigEntry::Item(
                                        door_item("door_editor_provide_socket", ListValue::Bool(action.provide_socket_connection)).with_update_bool_value(
                                            &|(i, list): &(usize, Arc<Mutex<DoorList>>), value: bool| {
                                                list.lock().unwrap()[*i].provide_socket_connection = value;
                                            },
                                        ),
                                    ),
                                    ConfigEntry::Item(
                                        door_item("door_editor_max_parallel", ListValue::U32(action.max_parallel, 0, 255)).with_update_u32_value(
                                            &|(i, list): &(usize, Arc<Mutex<DoorList>>), value: u32| {
                                                list.lock().unwrap()[*i].max_parallel = value;
                                            },
                                        ),
                                    ),
                                    ConfigEntry::Item(
                                        door_item(
                                            "door_editor_drop_file",
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
                                        .with_update_combobox_value(
                                            &|(i, list): &(usize, Arc<Mutex<DoorList>>), value: &ComboBox| {
                                                if let Some(drop_file) = DropFile::iter().find(|drop_file| format!("{drop_file:?}") == value.cur_value.value) {
                                                    list.lock().unwrap()[*i].drop_file = drop_file;
                                                }
                                            },
                                        ),
                                    ),
                                    ConfigEntry::Item(
                                        door_item("door_editor_dos_command", ListValue::Text(60, TextFlags::None, action.dos_command.clone()))
                                            .with_update_text_value(&|(i, list): &(usize, Arc<Mutex<DoorList>>), value: String| {
                                                list.lock().unwrap()[*i].dos_command = value;
                                            }),
                                    ),
                                    ConfigEntry::Item(
                                        door_item("door_editor_dos_memory", ListValue::U32(action.dos_memory_mb, 1, 512)).with_update_u32_value(
                                            &|(i, list): &(usize, Arc<Mutex<DoorList>>), value: u32| {
                                                list.lock().unwrap()[*i].dos_memory_mb = value;
                                            },
                                        ),
                                    ),
                                    ConfigEntry::Item(
                                        door_item("door_editor_dos_max_seconds", ListValue::U32(action.dos_max_runtime_seconds, 0, 86400))
                                            .with_update_u32_value(&|(i, list): &(usize, Arc<Mutex<DoorList>>), value: u32| {
                                                list.lock().unwrap()[*i].dos_max_runtime_seconds = value;
                                            }),
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
        self.refresh_door_fields();
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

    fn assert_door_fields(editor: &DoorEditor<'_>, door_type: &DoorType) {
        let menu = editor.detail.menu.as_ref().unwrap();
        assert_eq!(menu.entry.len(), 17);
        let ConfigEntry::Item(path) = &menu.entry[3] else { panic!("path field") };
        let key = path_key(door_type);
        assert_eq!(path.status, get_text(&format!("{key}-status")), "{door_type} path status");
        assert_eq!(path.help, get_text(&format!("{key}-help")), "{door_type} path help");
        for (index, entry) in menu.entry.iter().enumerate() {
            let ConfigEntry::Item(item) = entry else { panic!("expected a field") };
            let expected = match index {
                8..=12 => *door_type == DoorType::Local,
                13 => *door_type != DoorType::BBSlink,
                14..=16 => *door_type == DoorType::Dos,
                _ => true,
            };
            assert_eq!(item.editable(), expected, "{door_type}, field {index}");
            let reason = get_text("door_editor_unused_for_type");
            assert_eq!(item.help.matches(&reason).count(), usize::from(!expected));
            assert_eq!(item.status.matches(&reason).count(), usize::from(!expected));
        }
    }

    #[test]
    fn door_fields_follow_the_selected_type() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("doors.toml");
        for door_type in [DoorType::Local, DoorType::Dos, DoorType::BBSlink] {
            let mut list = DoorList::default();
            list.doors.push(Door {
                door_type: door_type.clone(),
                ..Door::default()
            });
            list.save(&path).unwrap();
            let mut editor = DoorEditor::new(&path).unwrap();
            editor.handle_key_press(key(KeyCode::Tab));
            editor.handle_key_press(key(KeyCode::Enter));
            assert_door_fields(&editor, &door_type);
        }
    }

    #[test]
    fn door_type_changes_refresh_grey_fields_and_preserve_values() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("doors.toml");
        let mut expected = Door {
            name: "GAME".into(),
            path: "doors/game".into(),
            use_shell_execute: true,
            args: vec!["--node".into(), "{node}".into()],
            working_directory: "doors/work".into(),
            provide_socket_connection: true,
            max_parallel: 3,
            dos_command: "BRE.BAT".into(),
            dos_memory_mb: 8,
            dos_max_runtime_seconds: 23,
            ..Door::default()
        };
        let mut list = DoorList::default();
        list.doors.push(expected.clone());
        list.save(&path).unwrap();
        let mut editor = DoorEditor::new(&path).unwrap();
        editor.handle_key_press(key(KeyCode::Tab));
        editor.handle_key_press(key(KeyCode::Enter));
        for _ in 0..7 {
            editor.handle_key_press(key(KeyCode::Down));
        }
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 25)).unwrap();
        for door_type in [
            DoorType::Local,
            DoorType::Dos,
            DoorType::BBSlink,
            DoorType::Local,
            DoorType::Dos,
            DoorType::Local,
        ] {
            editor.handle_key_press(key(KeyCode::Enter));
            let ConfigEntry::Item(item) = &editor.detail.menu.as_ref().unwrap().entry[7] else {
                panic!("type field")
            };
            let ListValue::ComboBox(combo) = &item.value else { panic!("type combo") };
            let current = combo.selected_item;
            let target = combo.values.iter().position(|value| value.value == door_type.to_string()).unwrap();
            for _ in 0..current.abs_diff(target) {
                editor.handle_key_press(key(if target > current { KeyCode::Down } else { KeyCode::Up }));
            }
            assert_door_fields(&editor, &expected.door_type);
            editor.handle_key_press(key(KeyCode::Enter));
            assert_eq!(editor.detail.state.selected, 7);
            assert_door_fields(&editor, &door_type);
            terminal.draw(|frame| editor.render(frame, Rect::new(0, 1, 80, 23))).unwrap();
            expected.door_type = door_type.clone();
            assert!(
                editor.door_list.lock().unwrap()[0] == expected,
                "{door_type}: switching types changed stored values"
            );

            let buffer = terminal.backend().buffer();
            let path_row: String = (4..76).map(|column| buffer[(column, 7)].symbol()).collect();
            let path_label = get_text(path_key(&door_type));
            assert!(
                path_row.split_once(&path_label).is_some_and(|(_, rest)| rest.trim_start().starts_with(':')),
                "{door_type}: path row is {path_row:?}, expected label {path_label:?}"
            );
            for (offset, field) in [
                "door_editor_use_shell_execute",
                "door_editor_args",
                "door_editor_working_directory",
                "door_editor_provide_socket",
                "door_editor_max_parallel",
                "door_editor_drop_file",
                "door_editor_dos_command",
                "door_editor_dos_memory",
                "door_editor_dos_max_seconds",
            ]
            .iter()
            .enumerate()
            {
                let ConfigEntry::Item(item) = &editor.detail.menu.as_ref().unwrap().entry[8 + offset] else {
                    panic!("field")
                };
                let label = get_text(field);
                let row_y = 12 + offset as u16;
                let row: String = (4..76).map(|column| buffer[(column, row_y)].symbol()).collect();
                let label_start = row.find(&label).unwrap_or_else(|| panic!("clipped {field}: {row}"));
                let label_x = 4 + row[..label_start].chars().count() as u16;
                let value_x = 4 + row[..row.find(':').unwrap()].chars().count() as u16 + 2;
                let theme = icy_board_tui::theme::get_tui_theme();
                let label_style = if item.editable() { theme.item } else { theme.table_inactive };
                let value_style = match (&item.value, item.editable()) {
                    (_, false) => theme.table_inactive,
                    (ListValue::Bool(true), true) => theme.true_value,
                    (ListValue::Bool(false), true) => theme.false_value,
                    _ => theme.value,
                };
                for column in label_x..label_x + label.chars().count() as u16 {
                    assert_eq!(Some(buffer[(column, row_y)].fg), label_style.fg, "{door_type}: {field} label");
                }
                assert_eq!(Some(buffer[(value_x, row_y)].fg), value_style.fg, "{door_type}: {field} value");
                assert_eq!(buffer[(3, row_y)].symbol(), "\u{2551}");
                assert_eq!(buffer[(76, row_y)].symbol(), "\u{2551}");
            }
            editor.handle_key_press(key(KeyCode::Down));
            let next_field = match door_type {
                DoorType::Local => 8,
                DoorType::Dos => 13,
                DoorType::BBSlink => 0,
            };
            assert_eq!(editor.detail.state.selected, next_field);
            editor.handle_key_press(key(KeyCode::Up));
            assert_eq!(editor.detail.state.selected, 7);
        }
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
                assert!(border.contains(&crate::editors::hint_text(hint)));
            } else {
                assert!(!border.contains(&crate::editors::hint_text("doors_editor_key_help")));
                assert!(!border.contains(&crate::editors::hint_text("doors_editor_key_help_door")));
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
