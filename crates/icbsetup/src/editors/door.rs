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
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, ConfigMenuState, EditMessage, ListItem, ListValue, ResultState, TextFlags},
    get_text, get_text_args,
    insert_table::{Column, InsertTable},
    tab_page::{InfoState, Page, PageMessage},
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

#[derive(Clone, Copy)]
enum DoorField {
    Name,
    Description,
    Type,
    Command,
    Arguments,
    WorkingDirectory,
    DosDirectory,
    Shell,
    Connection,
    DropFile,
    MaxParallel,
    DosMemory,
    DosTimeout,
    Security,
    Password,
    ChargePerUse,
    ChargePerMinute,
}

fn door_item(key: &str, value: ListValue) -> DoorItem {
    ListItem::new(get_text(key), value)
        .with_label_width(16)
        .with_status(get_text(&format!("{key}-status")))
        .with_help(get_text(&format!("{key}-help")))
}

const COMMAND_KEYS: [&str; 4] = [
    "door_editor_path_local",
    "door_editor_dos_command",
    "door_editor_path_bbslink",
    "door_editor_path_ppe",
];

fn command_key(door_type: &DoorType) -> &'static str {
    match door_type {
        DoorType::Local => COMMAND_KEYS[0],
        DoorType::Dos => COMMAND_KEYS[1],
        DoorType::BBSlink => COMMAND_KEYS[2],
    }
}

fn command_item(door: &Door) -> DoorItem {
    let width = COMMAND_KEYS.iter().map(|key| Line::raw(get_text(key)).width() as u16).max().unwrap_or(0);
    let dos = door.door_type == DoorType::Dos;
    let value = if dos { &door.dos_command } else { &door.path };
    door_item(command_key(&door.door_type), ListValue::Text(60, TextFlags::None, value.clone()))
        .with_label_width(width.max(16))
        .with_update_value(Box::new(move |(index, list), value| {
            let ListValue::Text(_, _, value) = value else { return };
            let mut list = list.lock().unwrap();
            if dos {
                list[*index].dos_command = value.clone();
            } else {
                list[*index].path = value.clone();
            }
        }))
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
    command_type: DoorType,
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
            command_type: DoorType::Local,
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
        let ConfigEntry::Item(type_item) = &menu.entry[DoorField::Type as usize] else {
            return;
        };
        let ListValue::ComboBox(combo) = &type_item.value else { return };
        let local = combo.cur_value.value == "Local";
        let dos = combo.cur_value.value == "Dos";
        let door_type = if local {
            DoorType::Local
        } else if dos {
            DoorType::Dos
        } else {
            DoorType::BBSlink
        };
        if self.command_type != door_type {
            for field in [DoorField::Command, DoorField::DosDirectory] {
                if let ConfigEntry::Item(item) = &menu.entry[field as usize]
                    && item.editable()
                    && let Some(update) = &item.update_value
                {
                    update(&menu.obj, &item.value);
                }
            }
            let mut door = menu.obj.1.lock().unwrap()[menu.obj.0].clone();
            door.door_type = door_type.clone();
            let replacement = command_item(&door);
            let ConfigEntry::Item(command) = &mut menu.entry[DoorField::Command as usize] else {
                return;
            };
            command.set_title(get_text(command_key(&door_type)));
            command.status = replacement.status;
            command.help = replacement.help;
            command.value = replacement.value;
            command.update_value = replacement.update_value;
            command.text_field_state = Default::default();
            let ConfigEntry::Item(directory) = &mut menu.entry[DoorField::DosDirectory as usize] else {
                return;
            };
            directory.value = ListValue::Text(30, TextFlags::None, if dos { door.path } else { String::new() });
            directory.text_field_state = Default::default();
            menu.obj.1.lock().unwrap()[menu.obj.0].door_type = door_type.clone();
            self.command_type = door_type;
        }
        let ConfigEntry::Item(command) = &menu.entry[DoorField::Command as usize] else {
            return;
        };
        let ppe = local
            && matches!(&command.value, ListValue::Text(_, _, path)
            if std::path::Path::new(path).extension().is_some_and(|extension| extension.eq_ignore_ascii_case("ppe")));
        let native = local && !ppe;
        let command_label = if ppe { COMMAND_KEYS[3] } else { command_key(&self.command_type) };
        let ConfigEntry::Item(command) = &mut menu.entry[DoorField::Command as usize] else {
            return;
        };
        command.set_title(get_text(command_label));
        command.status = get_text(&format!("{command_label}-status"));
        command.help = get_text(&format!("{command_label}-help"));
        let fields = [
            (DoorField::Name, "door_editor_name", true),
            (DoorField::Description, "door_editor_description", true),
            (DoorField::Arguments, "door_editor_args", native || dos),
            (DoorField::WorkingDirectory, "door_editor_working_directory", native),
            (DoorField::DosDirectory, "door_editor_path_dos", dos),
            (DoorField::Shell, "door_editor_use_shell_execute", native),
            (DoorField::Connection, "door_editor_provide_socket", native),
            (DoorField::DropFile, "door_editor_drop_file", native || dos),
            (DoorField::MaxParallel, "door_editor_max_parallel", native),
            (DoorField::DosMemory, "door_editor_dos_memory", dos),
            (DoorField::DosTimeout, "door_editor_dos_max_seconds", dos),
            (DoorField::Security, "door_editor_security", true),
            (DoorField::Password, "door_editor_password", true),
            (DoorField::ChargePerUse, "accounting_activity_per_use", true),
            (DoorField::ChargePerMinute, "accounting_activity_per_minute", true),
        ];
        for (field, key, editable) in fields {
            let ConfigEntry::Item(item) = &mut menu.entry[field as usize] else { continue };
            item.set_editable(editable);
            let key = match field {
                DoorField::Arguments if dos => "door_editor_args_dos",
                DoorField::DropFile if dos => "door_editor_drop_file_dos",
                _ => key,
            };
            item.status = get_text(&format!("{key}-status"));
            item.help = get_text(&format!("{key}-help"));
            if !editable {
                let reason = get_text(if ppe { "door_editor_unused_for_ppe" } else { "door_editor_unused_for_type" });
                item.status.push_str(&format!(" - {reason}"));
                item.help.push_str(&format!("\n\n{reason}"));
            }
        }
    }

    fn argument_error(&self, check_configuration: bool) -> Option<String> {
        let menu = self.detail.menu.as_ref()?;
        let ConfigEntry::Item(item) = &menu.entry[DoorField::Arguments as usize] else {
            return None;
        };
        if !item.editable() {
            return None;
        }
        let ListValue::Text(_, _, input) = &item.value else { return None };
        let Ok(arguments) = launch::parse_arguments(input) else {
            return Some(get_text("door_editor_args_invalid_quotes"));
        };
        let mut door = menu.obj.1.lock().unwrap()[menu.obj.0].clone();
        door.args = arguments;
        if let ConfigEntry::Item(item) = &menu.entry[DoorField::Connection as usize]
            && let ListValue::ComboBox(combo) = &item.value
        {
            door.provide_socket_connection = combo.cur_value.value == "socket";
        }
        if let ConfigEntry::Item(item) = &menu.entry[DoorField::DropFile as usize]
            && let ListValue::ComboBox(combo) = &item.value
            && let Some(drop_file) = DropFile::iter().find(|drop_file| format!("{drop_file:?}") == combo.cur_value.value)
        {
            door.drop_file = drop_file;
        }
        if self.command_type == DoorType::Dos {
            return icy_board_engine::icy_board::doors::dos::expand_run_batch(&door, 0, "DOOR.SYS")
                .err()
                .map(|_| get_text("door_editor_args_invalid_dos"));
        }
        if !check_configuration {
            return None;
        }
        launch::check_arguments(&door)
            .err()
            .map(|error| get_text_args("door_editor_args_invalid", HashMap::from([("error".to_string(), error.to_string())])))
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
            if self.detail.state.selected == DoorField::Arguments as usize
                && let Some(error) = self.argument_error(true)
            {
                return ResultState::status_line(error);
            }
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

        if self.detail.is_open()
            && self.detail.state.selected == DoorField::Arguments as usize
            && matches!(key.code, KeyCode::Up | KeyCode::Down | KeyCode::Enter | KeyCode::Esc)
            && let Some(error) = self.argument_error(false)
        {
            return PageMessage::InfoBox(InfoState::Error, error);
        }
        if let Some(result) = self.detail.handle_input(key) {
            self.refresh_door_fields();
            if result.edit_msg == EditMessage::Close {
                if let Some(error) = self.argument_error(true) {
                    self.detail.state.selected = DoorField::Arguments as usize;
                    return PageMessage::InfoBox(InfoState::Error, error);
                }
                if let Some(menu) = &self.detail.menu {
                    for entry in &menu.entry {
                        if let ConfigEntry::Item(item) = entry
                            && item.editable()
                            && let Some(update) = &item.update_value
                        {
                            update(&menu.obj, &item.value);
                        }
                    }
                }
                self.detail.close();
                return PageMessage::None;
            }
            return PageMessage::ResultState(result);
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
                            self.command_type = action.door_type.clone();
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
                                    ConfigEntry::Item(command_item(action)),
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
                                        door_item(
                                            "door_editor_path_dos",
                                            ListValue::Text(
                                                30,
                                                TextFlags::None,
                                                if action.door_type == DoorType::Dos {
                                                    action.path.clone()
                                                } else {
                                                    String::new()
                                                },
                                            ),
                                        )
                                        .with_update_text_value(
                                            &|(index, list): &(usize, Arc<Mutex<DoorList>>), value: String| {
                                                let mut list = list.lock().unwrap();
                                                if list[*index].door_type == DoorType::Dos {
                                                    list[*index].path = value;
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
                                        door_item(
                                            "door_editor_provide_socket",
                                            ListValue::ComboBox(ComboBox {
                                                cur_value: ComboBoxValue::new(
                                                    get_text(if action.provide_socket_connection {
                                                        "door_editor_connection_socket"
                                                    } else {
                                                        "door_editor_connection_stdio"
                                                    }),
                                                    if action.provide_socket_connection { "socket" } else { "stdio" },
                                                ),
                                                selected_item: 0,
                                                is_edit_open: false,
                                                first_item: 0,
                                                values: vec![
                                                    ComboBoxValue::new(get_text("door_editor_connection_stdio"), "stdio"),
                                                    ComboBoxValue::new(get_text("door_editor_connection_socket"), "socket"),
                                                ],
                                            }),
                                        )
                                        .with_update_combobox_value(
                                            &|(index, list): &(usize, Arc<Mutex<DoorList>>), value: &ComboBox| {
                                                list.lock().unwrap()[*index].provide_socket_connection = value.cur_value.value == "socket";
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
                                        door_item("door_editor_max_parallel", ListValue::U32(action.max_parallel, 0, 255)).with_update_u32_value(
                                            &|(i, list): &(usize, Arc<Mutex<DoorList>>), value: u32| {
                                                list.lock().unwrap()[*i].max_parallel = value;
                                            },
                                        ),
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
                                    ConfigEntry::Item(
                                        door_item(
                                            "door_editor_security",
                                            ListValue::Security(action.securiy_level.clone(), action.securiy_level.to_string()),
                                        )
                                        .with_update_sec_value(
                                            &|(index, list): &(usize, Arc<Mutex<DoorList>>), value: SecurityExpression| {
                                                list.lock().unwrap()[*index].securiy_level = value;
                                            },
                                        ),
                                    ),
                                    ConfigEntry::Item(
                                        door_item("door_editor_password", ListValue::Text(30, TextFlags::Password, action.password.clone()))
                                            .with_update_text_value(&|(index, list): &(usize, Arc<Mutex<DoorList>>), value: String| {
                                                list.lock().unwrap()[*index].password = value;
                                            }),
                                    ),
                                    ConfigEntry::Item(
                                        door_item(
                                            "accounting_activity_per_use",
                                            ListValue::Float(action.charge_per_use, action.charge_per_use.to_string()),
                                        )
                                        .with_update_float_value(
                                            &|(index, list): &(usize, Arc<Mutex<DoorList>>), value| {
                                                list.lock().unwrap()[*index].charge_per_use = value;
                                            },
                                        ),
                                    ),
                                    ConfigEntry::Item(
                                        door_item(
                                            "accounting_activity_per_minute",
                                            ListValue::Float(action.charge_per_minute, action.charge_per_minute.to_string()),
                                        )
                                        .with_update_float_value(
                                            &|(index, list): &(usize, Arc<Mutex<DoorList>>), value| {
                                                list.lock().unwrap()[*index].charge_per_minute = value;
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

    fn render_door(editor: &mut DoorEditor<'_>) -> ratatui::buffer::Buffer {
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| editor.render(frame, Rect::new(0, 1, 80, 23))).unwrap();
        terminal.backend().buffer().clone()
    }

    fn select_door_type(editor: &mut DoorEditor<'_>, door_type: DoorType) {
        editor.detail.state.selected = DoorField::Type as usize;
        editor.handle_key_press(key(KeyCode::Enter));
        let ConfigEntry::Item(item) = &editor.detail.menu.as_ref().unwrap().entry[DoorField::Type as usize] else {
            panic!("type field")
        };
        let ListValue::ComboBox(combo) = &item.value else { panic!("type combo") };
        let current = combo.selected_item;
        let target = combo.values.iter().position(|value| value.value == door_type.to_string()).unwrap();
        for _ in 0..current.abs_diff(target) {
            editor.handle_key_press(key(if target > current { KeyCode::Down } else { KeyCode::Up }));
        }
        editor.handle_key_press(key(KeyCode::Enter));
    }

    fn set_door_text(editor: &mut DoorEditor<'_>, field: DoorField, value: &str) {
        editor.detail.state.selected = field as usize;
        let ConfigEntry::Item(item) = &mut editor.detail.menu.as_mut().unwrap().entry[field as usize] else {
            panic!("text field")
        };
        let ListValue::Text(_, _, text) = &mut item.value else { panic!("text value") };
        text.clear();
        item.text_field_state = Default::default();
        for character in value.chars() {
            editor.handle_key_press(key(KeyCode::Char(character)));
        }
    }

    #[test]
    fn door_command_binding_survives_edits_type_switches_and_save() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("doors.toml");
        let mut list = DoorList::default();
        list.doors.push(Door {
            path: "native".into(),
            dos_command: "GAME.EXE".into(),
            ..Door::default()
        });
        list.save(&path).unwrap();
        let mut editor = DoorEditor::new(&path).unwrap();
        editor.handle_key_press(key(KeyCode::Tab));
        editor.handle_key_press(key(KeyCode::Enter));
        set_door_text(&mut editor, DoorField::Command, "native-edited");
        select_door_type(&mut editor, DoorType::Dos);
        assert_eq!(editor.door_list.lock().unwrap()[0].path, "native-edited");
        set_door_text(&mut editor, DoorField::Command, "START.BAT");
        set_door_text(&mut editor, DoorField::DosDirectory, "doors/game");
        select_door_type(&mut editor, DoorType::Local);
        render_door(&mut editor);
        assert_eq!(editor.door_list.lock().unwrap()[0].path, "doors/game");
        assert_eq!(editor.door_list.lock().unwrap()[0].dos_command, "START.BAT");
        select_door_type(&mut editor, DoorType::BBSlink);
        set_door_text(&mut editor, DoorField::Command, "lord");
        editor.handle_key_press(key(KeyCode::Esc));
        assert!(!editor.detail.is_open());
        editor.handle_key_press(key(KeyCode::Esc));
        editor.handle_key_press(key(KeyCode::Right));
        assert!(matches!(editor.handle_key_press(key(KeyCode::Enter)), PageMessage::Close));
        let saved = DoorList::load(&path).unwrap();
        assert!(saved[0].door_type == DoorType::BBSlink);
        assert_eq!(saved[0].path, "lord");
        assert_eq!(saved[0].dos_command, "START.BAT");
    }

    #[test]
    fn door_ppe_disables_native_options_and_restores_their_values() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("doors.toml");
        let mut list = DoorList::default();
        list.doors.push(Door {
            path: "doors/test.PPE".into(),
            args: vec!["--keep".into()],
            working_directory: "doors/work".into(),
            provide_socket_connection: true,
            max_parallel: 3,
            use_shell_execute: true,
            ..Door::default()
        });
        list.save(&path).unwrap();
        let mut editor = DoorEditor::new(&path).unwrap();
        editor.handle_key_press(key(KeyCode::Tab));
        editor.handle_key_press(key(KeyCode::Enter));
        let buffer = render_door(&mut editor);
        let row: String = (4..76).map(|column| buffer[(column, 7)].symbol()).collect();
        assert!(row.contains(&get_text("door_editor_path_ppe")), "{row}");
        for field in DoorField::Arguments as usize..=DoorField::DosTimeout as usize {
            let ConfigEntry::Item(item) = &editor.detail.menu.as_ref().unwrap().entry[field] else {
                panic!("field")
            };
            assert!(!item.editable());
            assert!(item.help.contains(&get_text("door_editor_unused_for_ppe")));
            let row_y = 4 + field as u16;
            let row: String = (4..76).map(|column| buffer[(column, row_y)].symbol()).collect();
            let label_x = 4 + row.chars().take_while(|character| character.is_whitespace()).count() as u16;
            assert_eq!(Some(buffer[(label_x, row_y)].fg), icy_board_tui::theme::get_tui_theme().table_inactive.fg);
        }
        editor.detail.state.selected = DoorField::Command as usize;
        editor.handle_key_press(key(KeyCode::Down));
        assert_eq!(editor.detail.state.selected, DoorField::Security as usize);
        set_door_text(&mut editor, DoorField::Command, "doors/native");
        render_door(&mut editor);
        assert_door_fields(&editor, &DoorType::Local);
        let mut expected = list[0].clone();
        expected.path = "doors/native".into();
        assert!(editor.door_list.lock().unwrap()[0] == expected);
    }

    #[test]
    fn door_argument_errors_are_visible_and_do_not_discard_input() {
        let directory = tempfile::tempdir().unwrap();
        let mut editor = DoorEditor::new(&directory.path().join("doors.toml")).unwrap();
        editor.handle_key_press(key(KeyCode::F(2)));
        editor.handle_key_press(key(KeyCode::Enter));
        set_door_text(&mut editor, DoorField::Arguments, "'unfinished");
        render_door(&mut editor);
        assert_eq!(editor.request_status().status_line, get_text("door_editor_args_invalid_quotes"));
        for code in [KeyCode::Down, KeyCode::Enter, KeyCode::Esc] {
            assert!(matches!(editor.handle_key_press(key(code)), PageMessage::InfoBox(InfoState::Error, _)));
            assert!(editor.detail.is_open());
            assert_eq!(editor.detail.state.selected, DoorField::Arguments as usize);
        }
        editor.handle_key_press(key(KeyCode::Char('\'')));
        assert!(editor.argument_error(true).is_none());
        editor.handle_key_press(key(KeyCode::Esc));
        assert_eq!(editor.door_list.lock().unwrap()[0].args, ["unfinished"]);
    }

    #[test]
    fn door_argument_dependencies_can_be_configured_before_closing() {
        let directory = tempfile::tempdir().unwrap();
        let mut editor = DoorEditor::new(&directory.path().join("doors.toml")).unwrap();
        editor.handle_key_press(key(KeyCode::F(2)));
        editor.handle_key_press(key(KeyCode::Enter));
        set_door_text(&mut editor, DoorField::Arguments, "{socketHandle}");
        assert!(editor.argument_error(true).is_some());
        assert!(!matches!(editor.handle_key_press(key(KeyCode::Down)), PageMessage::InfoBox(..)));
        assert_eq!(editor.detail.state.selected, DoorField::WorkingDirectory as usize);
        assert!(matches!(editor.handle_key_press(key(KeyCode::Esc)), PageMessage::InfoBox(InfoState::Error, _)));
        editor.detail.state.selected = DoorField::Connection as usize;
        editor.handle_key_press(key(KeyCode::Enter));
        editor.handle_key_press(key(KeyCode::Down));
        editor.handle_key_press(key(KeyCode::Enter));
        assert!(editor.argument_error(true).is_none());
        editor.handle_key_press(key(KeyCode::Esc));
        assert!(!editor.detail.is_open());
        assert!(editor.door_list.lock().unwrap()[0].provide_socket_connection);
        assert_eq!(editor.door_list.lock().unwrap()[0].args, ["{socketHandle}"]);
    }

    #[test]
    fn door_dos_arguments_preserve_paths_and_reject_batch_syntax() {
        let directory = tempfile::tempdir().unwrap();
        let mut editor = DoorEditor::new(&directory.path().join("doors.toml")).unwrap();
        editor.handle_key_press(key(KeyCode::F(2)));
        editor.handle_key_press(key(KeyCode::Enter));
        select_door_type(&mut editor, DoorType::Dos);
        set_door_text(&mut editor, DoorField::Arguments, "%PATH%");
        assert_eq!(editor.argument_error(true).unwrap(), get_text("door_editor_args_invalid_dos"));
        assert!(matches!(editor.handle_key_press(key(KeyCode::Esc)), PageMessage::InfoBox(InfoState::Error, _)));
        set_door_text(&mut editor, DoorField::Arguments, "'C:\\DOOR\\GAME DATA' {node} {dropFile}");
        assert!(editor.argument_error(true).is_none());
        editor.handle_key_press(key(KeyCode::Esc));
        assert!(!editor.detail.is_open());
        assert_eq!(editor.door_list.lock().unwrap()[0].args, ["C:\\DOOR\\GAME DATA", "{node}", "{dropFile}"]);
    }

    fn assert_door_fields(editor: &DoorEditor<'_>, door_type: &DoorType) {
        let menu = editor.detail.menu.as_ref().unwrap();
        assert_eq!(menu.entry.len(), 17);
        let ConfigEntry::Item(command) = &menu.entry[DoorField::Command as usize] else {
            panic!("command field")
        };
        let key = command_key(door_type);
        assert_eq!(command.status, get_text(&format!("{key}-status")), "{door_type} command status");
        assert_eq!(command.help, get_text(&format!("{key}-help")), "{door_type} command help");
        for (index, entry) in menu.entry.iter().enumerate() {
            let ConfigEntry::Item(item) = entry else { panic!("expected a field") };
            let expected = match index {
                4 | 9 => *door_type != DoorType::BBSlink,
                5 | 7 | 8 | 10 => *door_type == DoorType::Local,
                6 | 11 | 12 => *door_type == DoorType::Dos,
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
        for _ in 0..DoorField::Type as usize {
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
            let ConfigEntry::Item(item) = &editor.detail.menu.as_ref().unwrap().entry[DoorField::Type as usize] else {
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
            assert_eq!(editor.detail.state.selected, DoorField::Type as usize);
            assert_door_fields(&editor, &door_type);
            terminal.draw(|frame| editor.render(frame, Rect::new(0, 1, 80, 23))).unwrap();
            expected.door_type = door_type.clone();
            assert!(
                editor.door_list.lock().unwrap()[0] == expected,
                "{door_type}: switching types changed stored values"
            );

            let buffer = terminal.backend().buffer();
            let path_row: String = (4..76).map(|column| buffer[(column, 7)].symbol()).collect();
            let path_label = get_text(command_key(&door_type));
            assert!(
                path_row.split_once(&path_label).is_some_and(|(_, rest)| rest.trim_start().starts_with(':')),
                "{door_type}: path row is {path_row:?}, expected label {path_label:?}"
            );
            for (offset, field) in [
                "door_editor_args",
                "door_editor_working_directory",
                "door_editor_path_dos",
                "door_editor_use_shell_execute",
                "door_editor_provide_socket",
                "door_editor_drop_file",
                "door_editor_max_parallel",
                "door_editor_dos_memory",
                "door_editor_dos_max_seconds",
            ]
            .iter()
            .enumerate()
            {
                let ConfigEntry::Item(item) = &editor.detail.menu.as_ref().unwrap().entry[DoorField::Arguments as usize + offset] else {
                    panic!("field")
                };
                let label = get_text(field);
                let row_y = 8 + offset as u16;
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
            assert_eq!(editor.detail.state.selected, DoorField::Command as usize);
            editor.handle_key_press(key(KeyCode::Up));
            assert_eq!(editor.detail.state.selected, DoorField::Type as usize);
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
