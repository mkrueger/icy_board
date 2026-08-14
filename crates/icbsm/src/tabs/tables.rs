use std::sync::{Arc, Mutex};

use chrono::Utc;
use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::icy_board::{
    IcyBoard,
    user_maintenance::{self, CounterInit, CounterScope, SecurityTables, TableEntry, TableKind, UserSelection},
};
use icy_board_tui::{
    BORDER_SET,
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, ConfigMenuState, EditMessage, ListItem, ListValue, ResultState},
    get_text, get_text_args,
    select_menu::MenuItem,
    tab_page::{Page, PageMessage},
    theme::get_tui_theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Widget, Wrap},
};
use std::collections::HashMap;

use super::{MaintenanceOp, MaintenancePage, MenuPage};

/// How many steps a security table can hold. The original wrote as many as the
/// sysop typed; a screen full is more than any board has ever needed.
const TABLE_ROWS: usize = 10;

/// The submenu behind "Adjust Security Levels", with the entries the original had.
pub fn security_menu_page(icy_board: Arc<Mutex<IcyBoard>>) -> MenuPage {
    MenuPage::new(
        get_text("icbsm_menu_adjust_security"),
        vec![
            MenuItem::new(0, 'A', get_text("icbsm_sec_by_ranges")),
            MenuItem::new(1, 'B', get_text("icbsm_sec_by_ranges_expired")),
            MenuItem::new(2, 'C', get_text("icbsm_sec_by_file_ratio")),
            MenuItem::new(3, 'D', get_text("icbsm_sec_by_byte_ratio")),
            MenuItem::new(4, 'E', get_text("icbsm_sec_by_uploads")),
            MenuItem::new(5, 'F', get_text("icbsm_sec_by_downloads")),
            MenuItem::new(6, 'G', get_text("icbsm_sec_table_file_ratio")),
            MenuItem::new(7, 'H', get_text("icbsm_sec_table_byte_ratio")),
            MenuItem::new(8, 'I', get_text("icbsm_sec_table_uploads")),
            MenuItem::new(9, 'J', get_text("icbsm_sec_table_downloads")),
            MenuItem::new(10, 'K', get_text("icbsm_sec_copy_expired")),
            MenuItem::new(11, 'L', get_text("icbsm_sec_init_counters")),
        ],
        Box::new(move |id| {
            let board = icy_board.clone();
            let page: Box<dyn Page> = match id {
                0 => Box::new(MaintenancePage::new(board, MaintenanceOp::AdjustSecurity)),
                1 => Box::new(MaintenancePage::new(board, MaintenanceOp::AdjustSecurityExpired)),
                2 => Box::new(TableApplyPage::new(board, TableKind::FileRatio)),
                3 => Box::new(TableApplyPage::new(board, TableKind::ByteRatio)),
                4 => Box::new(TableApplyPage::new(board, TableKind::Uploads)),
                5 => Box::new(TableApplyPage::new(board, TableKind::Downloads)),
                6 => Box::new(TableEditPage::new(board, TableKind::FileRatio)),
                7 => Box::new(TableEditPage::new(board, TableKind::ByteRatio)),
                8 => Box::new(TableEditPage::new(board, TableKind::Uploads)),
                9 => Box::new(TableEditPage::new(board, TableKind::Downloads)),
                10 => Box::new(MaintenancePage::new(board, MaintenanceOp::CopyExpiredSecurity)),
                _ => Box::new(MaintenancePage::new(board, MaintenanceOp::InitializeCounters)),
            };
            Some(page)
        }),
    )
}

fn table_title(kind: TableKind) -> String {
    get_text(match kind {
        TableKind::FileRatio => "icbsm_sec_table_file_ratio",
        TableKind::ByteRatio => "icbsm_sec_table_byte_ratio",
        TableKind::Uploads => "icbsm_sec_table_uploads",
        TableKind::Downloads => "icbsm_sec_table_downloads",
    })
}

fn apply_title(kind: TableKind) -> String {
    get_text(match kind {
        TableKind::FileRatio => "icbsm_sec_by_file_ratio",
        TableKind::ByteRatio => "icbsm_sec_by_byte_ratio",
        TableKind::Uploads => "icbsm_sec_by_uploads",
        TableKind::Downloads => "icbsm_sec_by_downloads",
    })
}

/// A step is only part of the table once it names a security level.
fn rows_to_entries(rows: &[(f64, u32); TABLE_ROWS]) -> Vec<TableEntry> {
    rows.iter()
        .filter(|(_, security)| *security > 0)
        .map(|(value, security)| TableEntry {
            value: *value,
            security: (*security).min(255) as u8,
        })
        .collect()
}

#[derive(Clone)]
struct TableRows {
    rows: [(f64, u32); TABLE_ROWS],
}

type Obj = Arc<Mutex<TableRows>>;

macro_rules! row_items {
    ($rows:expr, $($index:literal),+) => {
        vec![$(
            ConfigEntry::Item(
                ListItem::new(get_text("icbsm_table_value"), ListValue::Float($rows[$index].0, $rows[$index].0.to_string()))
                    .with_label_width(10)
                    .with_update_float_value(&|o: &Obj, v: f64| o.lock().unwrap().rows[$index].0 = v),
            ),
            ConfigEntry::Item(
                ListItem::new(get_text("icbsm_table_security"), ListValue::U32($rows[$index].1, 0, 255))
                    .with_label_width(10)
                    .with_update_u32_value(&|o: &Obj, v: u32| o.lock().unwrap().rows[$index].1 = v),
            ),
        )+]
    };
}

/// Builds the table a sysop hands to "Adjust by ..." later.
pub struct TableEditPage {
    icy_board: Arc<Mutex<IcyBoard>>,
    kind: TableKind,
    menu: ConfigMenu<Obj>,
    state: ConfigMenuState,
    message: Option<String>,
}

impl TableEditPage {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>, kind: TableKind) -> Self {
        let mut rows = [(0.0, 0u32); TABLE_ROWS];
        {
            let board = icy_board.lock().unwrap();
            let users_file = board.resolve_file(&board.config.paths.user_file);
            let tables = SecurityTables::load_for(&users_file);
            for (row, entry) in rows.iter_mut().zip(tables.get(kind).iter()) {
                *row = (entry.value, entry.security as u32);
            }
        }

        let entry = vec![ConfigEntry::Separator, ConfigEntry::Table(2, row_items!(rows, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9))];

        Self {
            icy_board,
            kind,
            menu: ConfigMenu {
                obj: Arc::new(Mutex::new(TableRows { rows })),
                entry,
            },
            state: ConfigMenuState::default(),
            message: None,
        }
    }

    fn save(&mut self) {
        let rows = self.menu.obj.lock().unwrap().rows;
        let entries = rows_to_entries(&rows);

        let board = self.icy_board.lock().unwrap();
        let users_file = board.resolve_file(&board.config.paths.user_file);
        drop(board);

        let mut tables = SecurityTables::load_for(&users_file);
        *tables.get_mut(self.kind) = entries;
        self.message = Some(match tables.save_for(&users_file) {
            Ok(()) => get_text("icbsm_table_saved"),
            Err(err) => get_text_args("icbsm_save_failed", HashMap::from([("error".to_string(), err.to_string())])),
        });
    }
}

impl Page for TableEditPage {
    fn render(&mut self, frame: &mut Frame, disp_area: Rect) {
        let area = disp_area.inner(Margin { vertical: 1, horizontal: 2 });
        Clear.render(area, frame.buffer_mut());

        if let Some(message) = &self.message {
            render_box(
                frame,
                area,
                table_title(self.kind),
                get_text("icbsm_done_keys"),
                vec![Line::from(message.clone())],
            );
            return;
        }

        let block = Block::new()
            .style(get_tui_theme().background)
            .borders(Borders::ALL)
            .border_set(BORDER_SET)
            .border_style(get_tui_theme().dialog_box)
            .padding(Padding::new(2, 2, 1, 0))
            .title_alignment(Alignment::Center)
            .title(table_title(self.kind))
            .title_bottom(Span::styled(get_text("icbsm_table_keys"), get_tui_theme().key_binding));
        block.render(area, frame.buffer_mut());

        let inner = area.inner(Margin { vertical: 1, horizontal: 2 });
        self.menu.render(inner, frame, &mut self.state);
    }

    fn request_status(&self) -> ResultState {
        ResultState {
            edit_msg: EditMessage::None,
            status_line: get_text("icbsm_table_hint"),
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if self.message.is_some() {
            return PageMessage::Close;
        }
        if key.code == KeyCode::PageDown || key.code == KeyCode::F(2) {
            self.save();
            return PageMessage::None;
        }
        let res = self.menu.handle_key_press(key, &mut self.state);
        if res.edit_msg == EditMessage::Close {
            return PageMessage::Close;
        }
        PageMessage::ResultState(res)
    }
}

/// Reads a table back and moves the security levels it names.
pub struct TableApplyPage {
    icy_board: Arc<Mutex<IcyBoard>>,
    kind: TableKind,
    entries: Vec<TableEntry>,
    result: Option<String>,
}

impl TableApplyPage {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>, kind: TableKind) -> Self {
        let entries = {
            let board = icy_board.lock().unwrap();
            let users_file = board.resolve_file(&board.config.paths.user_file);
            SecurityTables::load_for(&users_file).get(kind).clone()
        };
        Self {
            icy_board,
            kind,
            entries,
            result: None,
        }
    }

    fn run(&mut self) {
        let mut board = self.icy_board.lock().unwrap();
        let users_file = board.resolve_file(&board.config.paths.user_file);
        if let Err(err) = user_maintenance::create_backup(&users_file) {
            self.result = Some(get_text_args("icbsm_backup_failed", HashMap::from([("error".to_string(), err.to_string())])));
            return;
        }

        let report = user_maintenance::adjust_by_table(&mut board.users, &UserSelection::default(), self.kind, &self.entries, Utc::now());
        let save = board.save_userbase();
        drop(board);

        self.result = Some(match save {
            Ok(()) => get_text_args(
                "icbsm_done_count",
                HashMap::from([
                    ("changed".to_string(), report.changed.to_string()),
                    ("matched".to_string(), report.matched.to_string()),
                ]),
            ),
            Err(err) => get_text_args("icbsm_save_failed", HashMap::from([("error".to_string(), err.to_string())])),
        });
    }
}

impl Page for TableApplyPage {
    fn render(&mut self, frame: &mut Frame, disp_area: Rect) {
        let area = disp_area.inner(Margin { vertical: 1, horizontal: 2 });
        Clear.render(area, frame.buffer_mut());

        let (lines, bottom) = if let Some(result) = &self.result {
            (vec![Line::from(result.clone())], get_text("icbsm_done_keys"))
        } else if self.entries.is_empty() {
            (vec![Line::from(get_text("icbsm_table_empty"))], get_text("icbsm_done_keys"))
        } else {
            let mut lines = vec![Line::from(get_text("icbsm_table_header")), Line::from("")];
            for entry in &self.entries {
                lines.push(Line::from(format!("  {:>12}  {:>3}", entry.value, entry.security)));
            }
            (lines, get_text("icbsm_criteria_keys"))
        };

        render_box(frame, area, apply_title(self.kind), bottom, lines);
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if self.result.is_some() || self.entries.is_empty() {
            return PageMessage::Close;
        }
        match key.code {
            KeyCode::Esc => PageMessage::Close,
            KeyCode::Enter | KeyCode::PageDown | KeyCode::F(2) => {
                self.run();
                PageMessage::None
            }
            _ => PageMessage::None,
        }
    }
}

fn render_box(frame: &mut Frame, area: Rect, title: String, bottom: String, lines: Vec<Line<'static>>) {
    let block = Block::new()
        .style(get_tui_theme().background)
        .borders(Borders::ALL)
        .border_set(BORDER_SET)
        .border_style(get_tui_theme().dialog_box)
        .padding(Padding::new(2, 2, 1, 0))
        .title_alignment(Alignment::Center)
        .title(title)
        .title_bottom(Span::styled(bottom, get_tui_theme().key_binding));

    Paragraph::new(Text::from(lines))
        .style(get_tui_theme().item)
        .wrap(Wrap { trim: false })
        .block(block)
        .render(area, frame.buffer_mut());
}

/// The three ways the original offered to reset the transfer counters.
pub fn counter_init_combo(current: CounterInit) -> ComboBox {
    let values = vec![
        ComboBoxValue::new(get_text("icbsm_counters_zero"), "Zero"),
        ComboBoxValue::new(get_text("icbsm_counters_up_from_down"), "UploadsFromDownloads"),
        ComboBoxValue::new(get_text("icbsm_counters_down_from_up"), "DownloadsFromUploads"),
    ];
    let selected = match current {
        CounterInit::Zero => 0,
        CounterInit::UploadsFromDownloads => 1,
        CounterInit::DownloadsFromUploads => 2,
    };
    ComboBox {
        is_edit_open: false,
        cur_value: values[selected].clone(),
        values,
        selected_item: selected,
        first_item: 0,
    }
}

pub fn counter_init_from(value: &str) -> CounterInit {
    match value {
        "UploadsFromDownloads" => CounterInit::UploadsFromDownloads,
        "DownloadsFromUploads" => CounterInit::DownloadsFromUploads,
        _ => CounterInit::Zero,
    }
}

pub fn counter_scope(files: bool, bytes: bool) -> CounterScope {
    CounterScope { files, bytes }
}
