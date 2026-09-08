use std::{
    collections::HashMap,
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use icy_board_engine::icy_board::{
    IcyBoard,
    commands::{ActionTrigger, AutoRun, Command, CommandAction, CommandType, Position},
    menu::Menu,
    security_expr::SecurityExpression,
};
use icy_board_tui::{
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, TextFlags},
    get_text, get_text_args,
    hotkeys::{Hotkey, HotkeyBar},
    insert_table::{Column, InsertTable},
    pcb_line::{get_styled_pcb_line, get_styled_pcb_line_with_highlight},
    theme::get_tui_theme,
};
use icy_engine::{AttributeColor, TextBuffer, TextPane};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, Paragraph, ScrollbarState, TableState, Widget, Wrap},
};

use crate::{tabs::preview::load_background, validation::resolve_display_file};

type CommandDraft = Arc<Mutex<Command>>;
type ActionDraft = Arc<Mutex<CommandAction>>;

const POSITION_WIDTH: u16 = 80;
const POSITION_HEIGHT: u16 = 25;
const FIELD_LABELS: [&str; 13] = [
    "mnu_editor_display_text",
    "mnu_editor_highlighted_text",
    "mnu_editor_position",
    "command_editor_keyword",
    "mnu_editor_autorun",
    "mnu_editor_time",
    "mnu_editor_help_file",
    "command_editor_security",
    "mnu_work_charge_use",
    "mnu_work_charge_minute",
    "command_editor_command_type",
    "command_editor_parameter",
    "mnu_editor_run_on_selection",
];

// Localized labels can exceed the old fixed 18-cell column. Use the same
// width for all fields and for the shared ComboBox's popup clipping budget.
fn field_label_width() -> u16 {
    FIELD_LABELS
        .iter()
        .chain(std::iter::once(&"mnu_work_arguments"))
        .map(|key| Line::from(get_text(key)).width() as u16)
        .max()
        .unwrap_or(18)
        .max(18)
}

const POSITION: usize = 2;
const AUTORUN_TIME: usize = 5;
const SECURITY: usize = 7;
const CHARGE_USE: usize = 8;
const CHARGE_MINUTE: usize = 9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogResult {
    Pending,
    Accepted,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditCommandMode {
    Config,
    Table,
}

pub(crate) fn auto_run_value(auto_run: &AutoRun) -> ComboBoxValue {
    let key = match auto_run {
        AutoRun::Disabled => "mnu_editor_autorun_disabled",
        AutoRun::FirstCmd => "mnu_editor_autorun_first",
        AutoRun::Every => "mnu_editor_autorun_every",
        AutoRun::After => "mnu_editor_autorun_after",
        AutoRun::Loop => "mnu_editor_autorun_loop",
    };
    ComboBoxValue::new(get_text(key), format!("{auto_run:?}"))
}

/// Repair both selection and viewport before handing state to the shared table.
pub(crate) fn normalize_table(table: &mut InsertTable<'_>, len: usize) {
    table.content_length = len;
    let selected = if len == 0 {
        None
    } else {
        Some(table.table_state.selected().unwrap_or(0).min(len - 1))
    };
    table.table_state.select(selected);
    *table.table_state.offset_mut() = table.table_state.offset().min(len.saturating_sub(1));
    table.scroll_state = ScrollbarState::new(len).position(selected.unwrap_or(0));
}

fn combo(values: Vec<ComboBoxValue>, current: ComboBoxValue) -> ComboBox {
    let selected = values.iter().position(|v| v.value == current.value).unwrap_or(0);
    ComboBox {
        cur_value: current,
        selected_item: selected,
        first_item: selected.saturating_sub(2),
        is_edit_open: false,
        values,
    }
}

fn text_item(key: &str, status: &str, value: String, update: fn(&mut Command, String)) -> ConfigEntry<CommandDraft> {
    ConfigEntry::Item(
        ListItem::new(get_text(key), ListValue::Text(42, TextFlags::None, value))
            .with_label_width(field_label_width())
            .with_status(get_text(status))
            .with_update_value(Box::new(move |cmd: &CommandDraft, value| {
                if let ListValue::Text(_, _, value) = value {
                    update(&mut cmd.lock().unwrap(), value.clone());
                }
            })),
    )
}

fn raw_item(key: &str, status: &str, value: String) -> ConfigEntry<CommandDraft> {
    ConfigEntry::Item(
        ListItem::new(get_text(key), ListValue::Text(28, TextFlags::None, value))
            .with_label_width(field_label_width())
            .with_status(get_text(status)),
    )
}

// ConfigMenu normally applies text edits during rendering. Flush explicitly so
// Enter/F10 is correct even without an intervening frame (paste/rapid input/tests).
fn flush<T>(config: &ConfigMenu<T>) {
    for item in config.iter() {
        if let Some(update) = &item.update_value {
            update(&config.obj, &item.value);
        }
    }
}

fn nested<T>(config: &ConfigMenu<T>, state: &ConfigMenuState) -> bool {
    state.is_path_browser_open()
        || config
            .get_item(state.selected)
            .is_some_and(|item| matches!(&item.value, ListValue::ComboBox(c) if c.is_edit_open))
}

fn normalize_config<T>(config: &ConfigMenu<T>, state: &mut ConfigMenuState) {
    state.selected = state.selected.min(config.count().saturating_sub(1));
}

fn field_text(config: &ConfigMenu<CommandDraft>, index: usize) -> &str {
    match config.get_item(index).map(|item| &item.value) {
        Some(ListValue::Text(_, _, text)) | Some(ListValue::Security(_, text)) => text,
        _ => "",
    }
}

fn parse_charge(text: &str) -> Option<f64> {
    text.trim().parse::<f64>().ok().filter(|v| v.is_finite() && *v >= 0.0)
}

fn parse_position(text: &str) -> Option<Position> {
    let (x, y) = text.split_once(',')?;
    Some(Position {
        x: x.trim().parse().ok()?,
        y: y.trim().parse().ok()?,
    })
}

fn position_text(pos: Position) -> String {
    format!("{},{}", pos.x, pos.y)
}

fn file_parameter(kind: CommandType) -> bool {
    matches!(
        kind,
        CommandType::Menu | CommandType::DisplayFile | CommandType::RunPPE | CommandType::StuffFile | CommandType::StuffFileSilent
    )
}

fn category(kind: CommandType) -> &'static str {
    if file_parameter(kind) {
        return "mnu_work_cat_files";
    }
    match kind {
        CommandType::QuitMenu | CommandType::ExitMenus | CommandType::Conference | CommandType::DisableMenuOption => "mnu_work_cat_navigation",
        CommandType::Door | CommandType::Script | CommandType::DisplayDir => "mnu_work_cat_board",
        CommandType::PrintText
        | CommandType::GotoXY
        | CommandType::RefreshDisplayString
        | CommandType::StuffText
        | CommandType::StuffTextSilent
        | CommandType::StuffTextAndExitMenu
        | CommandType::StuffTextAndExitMenuSilent => "mnu_work_cat_text",
        _ => "mnu_work_cat_commands",
    }
}

fn parameter_help(kind: CommandType) -> &'static str {
    if kind == CommandType::RunPPE {
        return "mnu_work_ppe_help";
    }
    if file_parameter(kind) {
        return "mnu_work_path_help";
    }
    match kind {
        CommandType::Conference => "mnu_work_conference_help",
        CommandType::Door | CommandType::DisplayDir => "mnu_work_context_help",
        CommandType::Script => "mnu_work_script_help",
        CommandType::GotoXY => "mnu_work_position_help",
        CommandType::PrintText
        | CommandType::StuffText
        | CommandType::StuffTextSilent
        | CommandType::StuffTextAndExitMenu
        | CommandType::StuffTextAndExitMenuSilent => "mnu_work_text_help",
        CommandType::Disabled | CommandType::DisableMenuOption | CommandType::QuitMenu | CommandType::ExitMenus | CommandType::RefreshDisplayString => {
            "mnu_work_no_parameter_help"
        }
        _ => "mnu_work_raw_help",
    }
}

fn type_choices(filter: &str) -> Vec<ComboBoxValue> {
    let filter = filter.to_lowercase();
    let mut choices: Vec<_> = CommandType::iter()
        .map(|kind| {
            let label = format!("{}: {}", get_text(category(kind)), kind);
            ComboBoxValue::new(label, format!("{kind:?}"))
        })
        .filter(|v| v.display.to_lowercase().contains(&filter) || v.value.to_lowercase().contains(&filter))
        .collect();
    choices.sort_by(|a, b| a.display.cmp(&b.display));
    choices
}

fn choice_type(value: &str) -> Option<CommandType> {
    // The engine's FromStr is incomplete (e.g. FlagFiles). Resolve every
    // advertised choice against the enum iterator, without changing the model.
    CommandType::iter().find(|kind| format!("{kind:?}") == value)
}

/// Only suggest conference-relative IDs when the board has exactly one
/// conference. MNU files do not carry a runtime conference; guessing is unsafe.
fn parameter_choices(board: &IcyBoard, kind: CommandType) -> Vec<ComboBoxValue> {
    match kind {
        CommandType::Conference => board
            .conferences
            .iter()
            .enumerate()
            .map(|(i, c)| ComboBoxValue::new(format!("{i}: {}", c.name), i.to_string()))
            .collect(),
        CommandType::Door if board.conferences.len() == 1 => board.conferences[0]
            .doors
            .as_ref()
            .map(|doors| {
                doors
                    .doors
                    .iter()
                    .enumerate()
                    .map(|(i, door)| ComboBoxValue::new(format!("{}: {}", i + 1, door.name), (i + 1).to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        CommandType::DisplayDir if board.conferences.len() == 1 => board.conferences[0]
            .directories
            .as_ref()
            .map(|dirs| {
                dirs.iter()
                    .enumerate()
                    .map(|(i, dir)| ComboBoxValue::new(format!("{}: {}", i + 1, dir.name), (i + 1).to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Split only at runtime token boundaries, never shell quotes. Keep the entire
/// suffix (including its first separator) visible: `file;;x` and `file ;x`
/// contain empty arguments that would be lost by trimming or joining tokens.
struct PpeParameter {
    raw: String,
    prefix: String,
    file: String,
    arguments: String,
}

impl PpeParameter {
    fn new(raw: &str) -> Self {
        let start = raw.len() - raw.trim_start_matches(' ').len();
        let end = start + raw[start..].find([' ', ';']).unwrap_or(raw.len() - start);
        Self {
            raw: raw.into(),
            prefix: raw[..start].into(),
            file: raw[start..end].into(),
            arguments: raw[end..].into(),
        }
    }

    fn parameter(&self, file: &str, arguments: &str) -> String {
        if file == self.file && arguments == self.arguments {
            return self.raw.clone();
        }
        // New arguments may omit the separator. Never insert a space before
        // a semicolon: that would introduce an additional empty runtime token.
        let separator = if arguments.is_empty() || arguments.starts_with([' ', ';']) { "" } else { " " };
        format!("{}{file}{separator}{arguments}", self.prefix)
    }
}

struct ActionEditor {
    draft: ActionDraft,
    target: Option<usize>,
    config: ConfigMenu<ActionDraft>,
    state: ConfigMenuState,
    raw: bool,
    type_filter: String,
    kind: CommandType,
    ppe: Option<PpeParameter>,
    error: String,
}

impl ActionEditor {
    fn new(action: CommandAction, target: Option<usize>, board: &IcyBoard) -> Self {
        let kind = action.command_type;
        let draft = Arc::new(Mutex::new(action));
        let mut editor = Self {
            config: ConfigMenu {
                obj: draft.clone(),
                entry: Vec::new(),
            },
            draft,
            target,
            state: ConfigMenuState::default(),
            raw: false,
            type_filter: String::new(),
            kind,
            ppe: None,
            error: String::new(),
        };
        editor.state.path_base = Some(board.root_path.clone());
        editor.rebuild(board);
        editor
    }

    fn rebuild(&mut self, board: &IcyBoard) {
        let action = self.draft.lock().unwrap().clone();
        let kind = action.command_type;
        self.kind = kind;
        self.ppe = (kind == CommandType::RunPPE && !self.raw).then(|| PpeParameter::new(&action.parameter));
        let mut values = type_choices("");
        let current = ComboBoxValue::new(format!("{}: {}", get_text(category(kind)), kind), format!("{kind:?}"));
        if !values.iter().any(|v| v.value == current.value) {
            values.push(current.clone());
        }
        let choices = parameter_choices(board, kind);
        let parameter = if self.raw {
            ListValue::Text(42, TextFlags::None, action.parameter.clone())
        } else if let Some(ppe) = &self.ppe {
            ListValue::Path(PathBuf::from(&ppe.file))
        } else if file_parameter(kind) {
            ListValue::Path(PathBuf::from(&action.parameter))
        } else if !choices.is_empty() {
            let current = choices
                .iter()
                .find(|v| v.value == action.parameter)
                .cloned()
                .unwrap_or_else(|| ComboBoxValue::new(action.parameter.clone(), action.parameter.clone()));
            let mut choices = choices;
            if !choices.iter().any(|v| v.value == current.value) {
                choices.insert(0, current.clone());
            }
            ListValue::ComboBox(combo(choices, current))
        } else {
            ListValue::Text(42, TextFlags::None, action.parameter.clone())
        };
        let mut parameter_item = ListItem::new(get_text("command_editor_parameter"), parameter)
            .with_label_width(field_label_width())
            .with_status(get_text(parameter_help(kind)));
        if self.ppe.is_none() {
            parameter_item = parameter_item.with_update_value(Box::new(|draft: &ActionDraft, value| {
                let value = match value {
                    ListValue::Text(_, _, text) => text.clone(),
                    ListValue::Path(path) => path.to_string_lossy().into_owned(),
                    ListValue::ComboBox(c) => c.cur_value.value.clone(),
                    _ => return,
                };
                draft.lock().unwrap().parameter = value;
            }));
        }
        self.config.entry = vec![
            ConfigEntry::Item(
                ListItem::new(get_text("command_editor_command_type"), ListValue::ComboBox(combo(values, current)))
                    .with_label_width(field_label_width())
                    .with_status(get_text("mnu_work_type_help"))
                    .with_update_combobox_value(&|draft: &ActionDraft, c| {
                        if let Some(kind) = choice_type(&c.cur_value.value) {
                            draft.lock().unwrap().command_type = kind;
                        }
                    }),
            ),
            ConfigEntry::Item(parameter_item),
            ConfigEntry::Item(
                ListItem::new(
                    get_text("mnu_editor_run_on_selection"),
                    ListValue::Bool(action.trigger == ActionTrigger::Selection),
                )
                .with_label_width(field_label_width())
                .with_status(get_text("mnu_work_trigger_help"))
                .with_update_bool_value(&|draft: &ActionDraft, selection| {
                    draft.lock().unwrap().trigger = if selection { ActionTrigger::Selection } else { ActionTrigger::Activation };
                }),
            ),
        ];
        if let Some(ppe) = &self.ppe {
            // Keep the existing trigger at index 2 for all action types.
            self.config.entry.push(ConfigEntry::Item(
                ListItem::new(get_text("mnu_work_arguments"), ListValue::Text(42, TextFlags::None, ppe.arguments.clone()))
                    .with_label_width(field_label_width())
                    .with_status(get_text("mnu_work_ppe_help")),
            ));
        }
        normalize_config(&self.config, &mut self.state);
    }

    fn sync(&mut self, board: &IcyBoard) {
        flush(&self.config);
        // Assisted PPE fields deliberately have no rendering callbacks. Read
        // both controls together, including direct F4 and renderless F10 edits.
        if let Some(ppe) = &self.ppe
            && let Some(ListValue::Path(file)) = self.config.get_item(1).map(|item| &item.value)
            && let Some(ListValue::Text(_, _, arguments)) = self.config.get_item(3).map(|item| &item.value)
        {
            self.draft.lock().unwrap().parameter = ppe.parameter(&file.to_string_lossy(), arguments);
        }
        let kind = self.draft.lock().unwrap().command_type;
        if kind != self.kind {
            self.raw = false;
            self.rebuild(board);
            self.state.selected = 1;
        }
    }

    fn validate(&mut self) -> bool {
        if self.ppe.is_some()
            && let Some(ListValue::Path(path)) = self.config.get_item(1).map(|item| &item.value)
            && path.to_string_lossy().contains([' ', ';'])
        {
            self.error = get_text("mnu_work_ppe_path_error");
            self.state.selected = 1;
            return false;
        }
        true
    }

    fn handle(&mut self, key: KeyEvent, board: &IcyBoard) -> DialogResult {
        self.error.clear();
        normalize_config(&self.config, &mut self.state);
        let in_control = nested(&self.config, &self.state);
        // Printable keys search the open type list rather than invoking j/k navigation.
        if self.state.selected == 0 && in_control {
            let changed = match key.code {
                KeyCode::Char(c) if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                    self.type_filter.push(c);
                    true
                }
                KeyCode::Backspace => {
                    self.type_filter.pop();
                    true
                }
                _ => false,
            };
            if changed {
                let values = type_choices(&self.type_filter);
                if let Some(item) = self.config.get_item_mut(0)
                    && let ListValue::ComboBox(c) = &mut item.value
                {
                    // An empty result must never reach ComboBox's unchecked Enter path.
                    c.values = values;
                    c.selected_item = 0;
                    c.first_item = 0;
                }
                return DialogResult::Pending;
            }
            if key.code == KeyCode::Enter
                && self
                    .config
                    .get_item(0)
                    .is_some_and(|item| matches!(&item.value, ListValue::ComboBox(c) if c.values.is_empty()))
            {
                return DialogResult::Pending;
            }
        }
        if in_control {
            self.config.handle_key_press(key, &mut self.state);
        } else {
            match key.code {
                KeyCode::Esc => return DialogResult::Cancelled,
                KeyCode::F(10) => {
                    self.sync(board);
                    return if self.validate() { DialogResult::Accepted } else { DialogResult::Pending };
                }
                KeyCode::F(2) => {
                    self.sync(board);
                    self.raw = !self.raw;
                    self.rebuild(board);
                }
                KeyCode::Tab => {
                    self.state.selected = (self.state.selected + 1) % self.config.count();
                }
                KeyCode::BackTab => {
                    self.state.selected = (self.state.selected + self.config.count() - 1) % self.config.count();
                }
                _ => {
                    if self.state.selected == 0 && key.code == KeyCode::Enter {
                        self.type_filter.clear();
                        self.sync(board);
                        self.rebuild(board);
                    }
                    self.config.handle_key_press(key, &mut self.state);
                }
            }
        }
        self.sync(board);
        DialogResult::Pending
    }
}

pub struct EditCommandDialog<'a> {
    pub command: CommandDraft,
    id: usize,
    mode: EditCommandMode,
    state: ConfigMenuState,
    config: ConfigMenu<CommandDraft>,
    insert_table: InsertTable<'a>,
    action_editor: Option<ActionEditor>,
    board: Arc<Mutex<IcyBoard>>,
    menu: Arc<Mutex<Menu>>,
    preview: TextBuffer,
    view_x: u16,
    view_y: u16,
    position_edit: Option<Position>,
    highlight: bool,
    preview_error: String,
    error: String,
}

impl<'a> EditCommandDialog<'a> {
    /// `id` remains the one-based model index used by the parent. A new draft
    /// uses len + 1, so it cannot accidentally hide an existing preview command.
    pub(crate) fn new(board: Arc<Mutex<IcyBoard>>, menu: Arc<Mutex<Menu>>, command: Command, id: usize) -> Self {
        let draft = Arc::new(Mutex::new(command.clone()));
        let root = board.lock().unwrap().root_path.clone();
        let display_file = menu.lock().unwrap().display_file.clone();
        let (buffer, preview_error) = if display_file.as_os_str().is_empty() {
            (TextBuffer::new((80, 25)), String::new())
        } else {
            let file = resolve_display_file(&board.lock().unwrap(), &display_file, 0);
            match load_background(&file) {
                Ok(buffer) => (buffer, String::new()),
                Err(err) => (
                    TextBuffer::new((80, 25)),
                    inert(&format!("{}: {}: {err}", get_text("mnu_work_preview_error"), file.display())),
                ),
            }
        };
        let config = ConfigMenu {
            obj: draft.clone(),
            entry: vec![
                text_item("mnu_editor_display_text", "mnu_editor_display_text_status", command.display.clone(), |c, v| {
                    c.display = v
                }),
                text_item(
                    "mnu_editor_highlighted_text",
                    "mnu_editor_highlighted_text_status",
                    command.lighbar_display.clone(),
                    |c, v| c.lighbar_display = v,
                ),
                raw_item("mnu_editor_position", "mnu_work_position_field_help", position_text(command.position)),
                text_item("command_editor_keyword", "mnu_editor_keyword_status", command.keyword.clone(), |c, v| {
                    c.keyword = v
                }),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("mnu_editor_autorun"),
                        ListValue::ComboBox(combo(AutoRun::iter().map(|v| auto_run_value(&v)).collect(), auto_run_value(&command.auto_run))),
                    )
                    .with_label_width(field_label_width())
                    .with_status(get_text("mnu_editor_autorun_status"))
                    .with_update_combobox_value(&|cmd: &CommandDraft, c| {
                        if let Ok(value) = AutoRun::from_str(&c.cur_value.value) {
                            cmd.lock().unwrap().auto_run = value;
                        }
                    }),
                ),
                raw_item("mnu_editor_time", "mnu_editor_time_status", command.autorun_time.to_string()),
                ConfigEntry::Item(
                    ListItem::new(get_text("mnu_editor_help_file"), ListValue::Path(PathBuf::from(&command.help)))
                        .with_label_width(field_label_width())
                        .with_status(get_text("mnu_work_path_help"))
                        .with_update_path_value(&|cmd: &CommandDraft, path| {
                            cmd.lock().unwrap().help = path.to_string_lossy().into_owned();
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("command_editor_security"),
                        ListValue::Security(command.security.clone(), command.security.to_string()),
                    )
                    .with_label_width(field_label_width())
                    .with_status(get_text("mnu_editor_security_status"))
                    .with_update_sec_value(&|cmd: &CommandDraft, value| {
                        cmd.lock().unwrap().security = value;
                    }),
                ),
                raw_item("mnu_work_charge_use", "mnu_work_charge_help", command.charge_per_use.to_string()),
                raw_item("mnu_work_charge_minute", "mnu_work_charge_help", command.charge_per_minute.to_string()),
            ],
        };
        let table_draft = draft.clone();
        let mut insert_table = InsertTable {
            scroll_state: ScrollbarState::default(),
            table_state: TableState::default(),
            columns: vec![
                Column::new(get_text("command_editor_command_type")).with_width(24),
                Column::new(get_text("command_editor_header_parameter")).with_width(30),
                Column::new(get_text("mnu_work_trigger")),
            ],
            numbered: true,
            get_content: Box::new(move |_, i, j| {
                let command = table_draft.lock().unwrap();
                let Some(action) = command.actions.get(*i) else {
                    return Line::default();
                };
                Line::from(match j {
                    0 => action.command_type.to_string(),
                    1 => action.parameter.clone(),
                    2 => get_text(if action.trigger == ActionTrigger::Selection {
                        "mnu_work_selection"
                    } else {
                        "mnu_work_activation"
                    }),
                    _ => String::new(),
                })
            }),
            content_length: 0,
        };
        normalize_table(&mut insert_table, command.actions.len());
        let mut state = ConfigMenuState::default();
        state.path_base = Some(root);
        Self {
            command: draft,
            id,
            mode: EditCommandMode::Config,
            state,
            config,
            insert_table,
            action_editor: None,
            board,
            menu,
            preview: buffer,
            view_x: 0,
            view_y: 0,
            position_edit: None,
            highlight: false,
            preview_error,
            error: String::new(),
        }
    }

    pub fn status(&self) -> String {
        if !self.error.is_empty() {
            return self.error.clone();
        }
        if self.position_edit.is_some() {
            return get_text("mnu_work_position_help");
        }
        if let Some(editor) = &self.action_editor {
            if !editor.error.is_empty() {
                return editor.error.clone();
            }
            if nested(&editor.config, &editor.state) && editor.state.selected == 0 {
                return format!("{} /{}", get_text("mnu_work_type_help"), editor.type_filter);
            }
            if editor.raw && editor.kind == CommandType::RunPPE {
                return get_text("mnu_work_ppe_help");
            }
            return editor.config.current_status_line(&editor.state);
        }
        if !self.preview_error.is_empty() {
            return self.preview_error.clone();
        }
        if self.mode == EditCommandMode::Table {
            get_text("mnu_work_action_keys")
        } else {
            self.config.current_status_line(&self.state)
        }
    }

    fn validate(&mut self) -> bool {
        flush(&self.config);
        let fee_use = parse_charge(field_text(&self.config, CHARGE_USE));
        let fee_minute = parse_charge(field_text(&self.config, CHARGE_MINUTE));
        let position = parse_position(field_text(&self.config, POSITION));
        let time = field_text(&self.config, AUTORUN_TIME).trim().parse::<u64>().ok();
        let security = SecurityExpression::from_str(field_text(&self.config, SECURITY));
        let failure = if fee_use.is_none() {
            Some((CHARGE_USE, "mnu_work_charge_error"))
        } else if fee_minute.is_none() {
            Some((CHARGE_MINUTE, "mnu_work_charge_error"))
        } else if position.is_none() {
            Some((POSITION, "mnu_work_position_error"))
        } else if time.is_none() {
            Some((AUTORUN_TIME, "mnu_work_time_error"))
        } else if security.is_err() {
            Some((SECURITY, "mnu_work_security_error"))
        } else {
            None
        };
        if let Some((index, message)) = failure {
            self.error = get_text(message);
            self.state.selected = index;
            self.mode = EditCommandMode::Config;
            return false;
        }
        if let (Some(fee_use), Some(fee_minute), Some(position), Some(time), Ok(security)) = (fee_use, fee_minute, position, time, security) {
            let mut command = self.command.lock().unwrap();
            command.charge_per_use = fee_use;
            command.charge_per_minute = fee_minute;
            command.position = position;
            command.autorun_time = time;
            command.security = security;
        }
        true
    }

    fn open_action(&mut self, target: Option<usize>, action: CommandAction) {
        self.action_editor = Some(ActionEditor::new(action, target, &self.board.lock().unwrap()));
    }

    fn move_action(&mut self, down: bool) {
        let Some(selected) = self.insert_table.table_state.selected() else {
            return;
        };
        let next = if down { selected.checked_add(1) } else { selected.checked_sub(1) };
        let mut command = self.command.lock().unwrap();
        if let Some(next) = next.filter(|&i| i < command.actions.len() && selected < command.actions.len()) {
            command.actions.swap(selected, next);
            self.insert_table.table_state.select(Some(next));
        }
    }

    pub fn handle_key_press(&mut self, key: KeyEvent) -> DialogResult {
        if key.kind == KeyEventKind::Release {
            return DialogResult::Pending;
        }
        if let Some(mut position) = self.position_edit {
            match key.code {
                KeyCode::Esc => self.position_edit = None,
                KeyCode::Enter | KeyCode::F(10) => {
                    self.command.lock().unwrap().position = position;
                    if let Some(item) = self.config.get_item_mut(POSITION) {
                        item.value = ListValue::Text(28, TextFlags::None, position_text(position));
                    }
                    self.position_edit = None;
                }
                KeyCode::F(6) => self.highlight = !self.highlight,
                _ => {
                    // Background dimensions and terminal clipping never change
                    // the menu's 80x25 coordinate system. Imported off-canvas
                    // coordinates stay intact until explicitly moved/edited.
                    match key.code {
                        KeyCode::Left | KeyCode::Char('h') => position.x = position.x.saturating_sub(1),
                        KeyCode::Right | KeyCode::Char('l') if position.x < POSITION_WIDTH - 1 => position.x += 1,
                        KeyCode::Up | KeyCode::Char('k') => position.y = position.y.saturating_sub(1),
                        KeyCode::Down | KeyCode::Char('j') if position.y < POSITION_HEIGHT - 1 => position.y += 1,
                        _ => {}
                    }
                    self.position_edit = Some(position);
                }
            }
            return DialogResult::Pending;
        }
        if let Some(editor) = &mut self.action_editor {
            let result = editor.handle(key, &self.board.lock().unwrap());
            if result == DialogResult::Accepted {
                let action = editor.draft.lock().unwrap().clone();
                let mut command = self.command.lock().unwrap();
                let selected = if let Some(target) = editor.target {
                    if let Some(old) = command.actions.get_mut(target) {
                        *old = action;
                    }
                    target
                } else {
                    let at = self
                        .insert_table
                        .table_state
                        .selected()
                        .map_or(0, |i| i.saturating_add(1))
                        .min(command.actions.len());
                    command.actions.insert(at, action);
                    at
                };
                self.insert_table.table_state.select(Some(selected));
                normalize_table(&mut self.insert_table, command.actions.len());
            }
            if result != DialogResult::Pending {
                self.action_editor = None;
            }
            return DialogResult::Pending;
        }
        normalize_config(&self.config, &mut self.state);
        normalize_table(&mut self.insert_table, self.command.lock().unwrap().actions.len());
        if self.mode == EditCommandMode::Config && nested(&self.config, &self.state) {
            self.config.handle_key_press(key, &mut self.state);
            flush(&self.config);
            return DialogResult::Pending;
        }
        match key.code {
            KeyCode::Esc => return DialogResult::Cancelled,
            KeyCode::F(10) => return if self.validate() { DialogResult::Accepted } else { DialogResult::Pending },
            KeyCode::Tab | KeyCode::BackTab => {
                flush(&self.config);
                self.mode = if self.mode == EditCommandMode::Config {
                    EditCommandMode::Table
                } else {
                    EditCommandMode::Config
                };
            }
            _ if self.mode == EditCommandMode::Config => {
                self.error.clear();
                if self.state.selected == POSITION && matches!(key.code, KeyCode::F(4) | KeyCode::Enter) {
                    if let Some(position) = parse_position(field_text(&self.config, POSITION)) {
                        self.position_edit = Some(position);
                    } else {
                        self.error = get_text("mnu_work_position_error");
                    }
                } else {
                    self.config.handle_key_press(key, &mut self.state);
                }
                flush(&self.config);
            }
            _ => {
                match key.code {
                    KeyCode::Char('1') | KeyCode::PageUp => self.move_action(false),
                    KeyCode::Char('2') | KeyCode::PageDown => self.move_action(true),
                    KeyCode::Delete => {
                        let mut command = self.command.lock().unwrap();
                        if let Some(i) = self.insert_table.table_state.selected().filter(|&i| i < command.actions.len()) {
                            command.actions.remove(i);
                        }
                    }
                    KeyCode::Insert => self.open_action(None, CommandAction::default()),
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        let action = self
                            .insert_table
                            .table_state
                            .selected()
                            .and_then(|i| self.command.lock().unwrap().actions.get(i).cloned());
                        if let Some(action) = action {
                            self.open_action(None, action);
                        }
                    }
                    KeyCode::Char('m' | 'p' | 'd' | 'q') => {
                        let kind = match key.code {
                            KeyCode::Char('m') => CommandType::Menu,
                            KeyCode::Char('p') => CommandType::RunPPE,
                            KeyCode::Char('d') => CommandType::Door,
                            _ => CommandType::QuitMenu,
                        };
                        self.open_action(
                            None,
                            CommandAction {
                                command_type: kind,
                                ..Default::default()
                            },
                        );
                    }
                    KeyCode::Enter => {
                        if let Some(i) = self.insert_table.table_state.selected() {
                            let action = self.command.lock().unwrap().actions.get(i).cloned();
                            if let Some(action) = action {
                                self.open_action(Some(i), action);
                            }
                        }
                    }
                    _ => {
                        let _ = self.insert_table.handle_key_press(key);
                    }
                }
                normalize_table(&mut self.insert_table, self.command.lock().unwrap().actions.len());
            }
        }
        DialogResult::Pending
    }

    pub fn ui(&mut self, frame: &mut Frame, screen: Rect) {
        let screen = screen.intersection(frame.area());
        if let Some(position) = self.position_edit {
            self.render_position(frame, screen, position);
            return;
        }
        let area = screen.inner(Margin::new(1, 1));
        Clear.render(area, frame.buffer_mut());
        if area.width < 70 || area.height < 16 {
            Paragraph::new(get_text("mnu_work_small_screen"))
                .style(get_tui_theme().item)
                .wrap(Wrap { trim: true })
                .render(area, frame.buffer_mut());
            return;
        }
        let hints = HotkeyBar::new([
            Hotkey::new(KeyCode::F(10), get_text("mnu_work_apply")),
            Hotkey::new(KeyCode::Esc, get_text("mnu_work_cancel")),
            Hotkey::new(KeyCode::Tab, get_text("mnu_work_fields_actions")),
        ]);
        Block::new()
            .title_alignment(Alignment::Center)
            .title(
                Line::styled(
                    get_text_args("mnu_editor_command_title", HashMap::from([("id".into(), self.id.to_string())])),
                    get_tui_theme().dialog_box_title,
                )
                .centered(),
            )
            .title_bottom(hints.line())
            .style(get_tui_theme().background)
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(get_tui_theme().dialog_box)
            .render(area, frame.buffer_mut());
        let [header, samples, actions, help] =
            Layout::vertical([Constraint::Length(10), Constraint::Length(2), Constraint::Min(2), Constraint::Length(2)]).areas(area.inner(Margin::new(1, 1)));
        normalize_config(&self.config, &mut self.state);
        let command = self.command.lock().unwrap().clone();
        let [normal, highlight] = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(samples);
        render_sample(frame, normal, "mnu_work_normal", &command.display, false);
        render_sample(
            frame,
            highlight,
            "mnu_work_highlight",
            display_text(&command, true),
            command.lighbar_display.is_empty(),
        );
        normalize_table(&mut self.insert_table, command.actions.len());
        let selection = self.insert_table.table_state.selected();
        if self.mode == EditCommandMode::Config {
            self.insert_table.table_state.select(None);
        }
        if actions.width > 0 && actions.height > 0 {
            self.insert_table.render_table(frame, actions);
        }
        self.insert_table.table_state.select(selection);
        Paragraph::new(if self.mode == EditCommandMode::Table && self.error.is_empty() {
            get_text("mnu_work_action_keys")
        } else {
            self.status()
        })
        .style(if self.error.is_empty() {
            get_tui_theme().menu_label
        } else {
            get_tui_theme().false_value
        })
        .wrap(Wrap { trim: true })
        .render(help, frame.buffer_mut());
        // Render controls last: the autorun popup and path browser must cover
        // fields/previews beneath them, not be overwritten by the value pass.
        render_config(&mut self.config, &mut self.state, frame, header, self.mode == EditCommandMode::Config);
        if let Some(editor) = &mut self.action_editor {
            let popup = Rect::new(area.x + 1, area.y + 2, area.width - 2, area.height.saturating_sub(4));
            Clear.render(popup, frame.buffer_mut());
            Block::new()
                .title(Line::styled(get_text("mnu_editor_edit_action"), get_tui_theme().dialog_box_title))
                .title_bottom(
                    HotkeyBar::new([
                        Hotkey::new(KeyCode::F(10), get_text("mnu_work_apply")),
                        Hotkey::new(KeyCode::Esc, get_text("mnu_work_cancel")),
                        Hotkey::new(KeyCode::F(2), get_text("mnu_work_raw")),
                    ])
                    .line(),
                )
                .borders(Borders::ALL)
                .style(get_tui_theme().background)
                .border_style(get_tui_theme().dialog_box)
                .render(popup, frame.buffer_mut());
            let field_height = if editor.ppe.is_some() { 8 } else { 6 };
            let [fields, help] = Layout::vertical([Constraint::Length(field_height), Constraint::Min(1)]).areas(popup.inner(Margin::new(1, 1)));
            let kind = editor.draft.lock().unwrap().command_type;
            let hint = if !editor.error.is_empty() {
                editor.error.clone()
            } else if editor.state.selected == 0 {
                format!(
                    "{} /{}\n{}\n{}",
                    get_text("mnu_work_type_help"),
                    editor.type_filter,
                    get_text(parameter_help(kind)),
                    get_text("mnu_work_trigger_help")
                )
            } else {
                format!(
                    "{}\n{}\n{}",
                    get_text(parameter_help(kind)),
                    get_text("mnu_work_trigger_help"),
                    get_text("mnu_work_type_help")
                )
            };
            Paragraph::new(hint)
                .style(if editor.error.is_empty() {
                    get_tui_theme().menu_label
                } else {
                    get_tui_theme().false_value
                })
                .wrap(Wrap { trim: true })
                .render(help, frame.buffer_mut());
            render_config(&mut editor.config, &mut editor.state, frame, fields, true);
        }
    }

    fn render_position(&mut self, frame: &mut Frame, screen: Rect, position: Position) {
        Clear.render(screen, frame.buffer_mut());
        let width = POSITION_WIDTH.min(screen.width);
        let help_height = screen.height.min(2);
        let height = POSITION_HEIGHT.min(screen.height - help_height);
        if width > 0 && height > 0 {
            let area = Rect::new(screen.x + (screen.width - width) / 2, screen.y, width, height);
            self.view_x = follow_position(self.view_x, position.x, width, POSITION_WIDTH);
            self.view_y = follow_position(self.view_y, position.y, height, POSITION_HEIGHT);
            let canvas = self.position_canvas(position);
            for y in 0..height {
                for x in 0..width {
                    frame.buffer_mut()[(area.x + x, area.y + y)] = canvas[(self.view_x + x, self.view_y + y)].clone();
                }
            }
            if position.x < POSITION_WIDTH && position.y < POSITION_HEIGHT {
                frame.set_cursor_position((area.x + position.x - self.view_x, area.y + position.y - self.view_y));
            }
        }
        let help = Rect::new(screen.x, screen.bottom() - help_height, screen.width, help_height);
        // Keep coordinates visible even when the translated key hint nearly
        // fills an 80-column row (notably German).
        let status = format!(
            "({},{}) {} {}",
            position.x,
            position.y,
            get_text(if self.highlight { "mnu_work_highlight" } else { "mnu_work_normal" }),
            self.preview_error
        );
        let keys = HotkeyBar::new([
            Hotkey::alternatives([KeyCode::Left, KeyCode::Right, KeyCode::Up, KeyCode::Down], get_text("mnu_work_move")),
            Hotkey::new(KeyCode::F(6), get_text("mnu_work_highlight")),
            Hotkey::alternatives([KeyCode::Enter, KeyCode::F(10)], get_text("mnu_work_apply")),
            Hotkey::new(KeyCode::Esc, get_text("mnu_work_cancel")),
        ]);
        Line::styled(status, get_tui_theme().menu_label).render(Rect::new(help.x, help.y, help.width, 1), frame.buffer_mut());
        if help_height > 1 {
            keys.line().render(Rect::new(help.x, help.y + 1, help.width, 1), frame.buffer_mut());
        }
    }

    fn position_canvas(&self, position: Position) -> Buffer {
        // A short decoded file is only a background, not a smaller menu. Paint
        // and overlay in model coordinates before clipping, just like PreviewTab.
        let mut canvas = Buffer::empty(Rect::new(0, 0, POSITION_WIDTH, POSITION_HEIGHT));
        for y in 0..self.preview.height().clamp(0, i32::from(POSITION_HEIGHT)) as u16 {
            for x in 0..self.preview.width().clamp(0, i32::from(POSITION_WIDTH)) as u16 {
                let ch = self.preview.char_at((i32::from(x), i32::from(y)).into());
                let color = |color: AttributeColor, bold: bool| match color {
                    AttributeColor::Palette(index) => {
                        let index = if bold && index < 8 { index + 8 } else { index };
                        let (r, g, b) = self.preview.palette.rgb(u32::from(index));
                        Color::Rgb(r, g, b)
                    }
                    AttributeColor::ExtendedPalette(index) => Color::Indexed(index),
                    AttributeColor::Rgb(r, g, b) => Color::Rgb(r, g, b),
                    AttributeColor::Transparent => Color::Reset,
                };
                let mut style = Style::default()
                    .fg(color(ch.attribute.foreground_color(), ch.attribute.is_bold()))
                    .bg(color(ch.attribute.background_color(), false));
                if ch.attribute.is_blinking() {
                    style = style.add_modifier(Modifier::SLOW_BLINK);
                }
                let unicode = self.preview.buffer_type.convert_to_unicode(ch.ch);
                canvas[(x, y)].set_symbol(&inert(&unicode.to_string())).set_style(style);
            }
        }
        {
            let menu = self.menu.lock().unwrap();
            for (index, command) in menu.commands.iter().enumerate() {
                if Some(index) == self.id.checked_sub(1) {
                    continue;
                }
                render_at(&mut canvas, command.position, get_styled_pcb_line(&inert(&command.display)));
            }
        }
        let draft = self.command.lock().unwrap();
        let line = get_styled_pcb_line_with_highlight(&inert(display_text(&draft, self.highlight)), self.highlight && draft.lighbar_display.is_empty());
        render_at(&mut canvas, position, line);
        canvas
    }
}

fn inert(text: &str) -> String {
    text.chars().map(|ch| if ch.is_control() { '\u{fffd}' } else { ch }).collect()
}

fn follow_position(origin: u16, selected: u16, visible: u16, extent: u16) -> u16 {
    let selected = selected.min(extent - 1);
    let origin = origin.min(extent - visible);
    if selected < origin {
        selected
    } else if selected >= origin + visible {
        selected + 1 - visible
    } else {
        origin
    }
}

fn display_text(command: &Command, highlight: bool) -> &str {
    if highlight && !command.lighbar_display.is_empty() {
        &command.lighbar_display
    } else {
        &command.display
    }
}

fn render_at(canvas: &mut Buffer, position: Position, line: Line<'_>) {
    if position.x >= POSITION_WIDTH || position.y >= POSITION_HEIGHT {
        return;
    }
    line.render(Rect::new(position.x, position.y, POSITION_WIDTH - position.x, 1), canvas);
}

fn render_sample(frame: &mut Frame, area: Rect, label: &str, value: &str, highlight: bool) {
    let [label_area, value_area] = Layout::horizontal([Constraint::Length(18), Constraint::Min(0)]).areas(area);
    Line::styled(get_text(label), get_tui_theme().menu_label).render(label_area, frame.buffer_mut());
    get_styled_pcb_line_with_highlight(&inert(value), highlight).render(value_area, frame.buffer_mut());
}

fn render_config<T>(config: &mut ConfigMenu<T>, state: &mut ConfigMenuState, frame: &mut Frame, area: Rect, active: bool) {
    if area.width < 2 || area.height == 0 {
        return;
    }
    normalize_config(config, state);
    // These dialogs contain flat, one-row fields. Reconcile the viewport on
    // every frame, including resize and direct selection after validation.
    state.first_row = state.first_row.min((config.count() as u16).saturating_sub(area.height));
    let selected = state.selected as u16;
    if selected < state.first_row {
        state.first_row = selected;
    } else if selected >= state.first_row + area.height {
        state.first_row = selected - area.height + 1;
    }
    // Shared ComboBox popups use their longest label, not the available width.
    // Clip display labels only; IDs and raw model values remain untouched.
    let max_label = area.width.saturating_sub(field_label_width() + 7) as usize;
    for i in 0..config.count() {
        if let Some(item) = config.get_item_mut(i)
            && let ListValue::ComboBox(c) = &mut item.value
        {
            // The shared popup measures bytes. Bound bytes as well as cells,
            // preserving UTF-8 boundaries for non-ASCII conference/door names.
            for value in &mut c.values {
                while value.display.len() > max_label {
                    value.display.pop();
                }
            }
            while c.cur_value.display.len() > max_label {
                c.cur_value.display.pop();
            }
        }
    }
    if active {
        config.render(area, frame, state);
    } else {
        // Paint only the value pass. No invalid selection is ever installed in
        // the real state and no editor/cursor lookup uses a sentinel index.
        let mut inactive = ConfigMenuState::default();
        inactive.selected = config.count();
        inactive.first_row = state.first_row;
        Clear.render(area, frame.buffer_mut());
        ConfigMenu::display_list(&config.obj, &mut 0, &mut config.entry, area, &mut 0, &mut 0, frame, &mut inactive, false);
    }
}

#[cfg(test)]
#[path = "edit_command_dialog_tests.rs"]
mod tests;
