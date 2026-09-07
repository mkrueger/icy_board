use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::editors::EditorList;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use icy_board_engine::{
    Res,
    datetime::{IcbDoW, IcbTime},
    icy_board::{
        IcyBoard, IcyBoardSerializer,
        events::{
            BoardEvent, EventExecution, EventList, EventMode,
            event_history::{EventHistory, EventHistoryEntry, EventResult},
        },
    },
};
use icy_board_tui::{
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, EditMessage, ListItem, ListValue, ResultState, TextFlags},
    get_text, get_text_args,
    insert_table::{Column, InsertTable},
    tab_page::{InfoState, Page, PageMessage},
    theme::get_tui_theme,
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Clear, Paragraph, ScrollbarState, TableState, Widget, Wrap},
};

/// A staged list: neither record editing nor discarding changes touches the file.
pub struct EventListEditor<'a> {
    path: PathBuf,
    root: PathBuf,
    original: EventList,
    events: Arc<Mutex<EventList>>,
    table: InsertTable<'a>,
    detail: super::EditorDialog<BoardEvent>,
    save_changes: super::EditorSaveChanges,
    history: Option<HistoryView>,
}

struct HistoryView {
    description: String,
    entries: Vec<EventHistoryEntry>,
    selected: usize,
    scroll: u16,
}

fn result_label(result: &EventResult) -> String {
    let suffix = match result {
        EventResult::Pending => "pending",
        EventResult::Success => "success",
        EventResult::NonzeroExit => "nonzero_exit",
        EventResult::SpawnError => "spawn_error",
        EventResult::WaitError => "wait_error",
        EventResult::Interrupted => "interrupted",
        EventResult::SkippedBusy => "skipped_busy",
        EventResult::Expired => "expired",
        EventResult::Superseded => "superseded",
    };
    get_text(&format!("event_editor_result_{suffix}"))
}

fn execution_value(execution: EventExecution) -> &'static str {
    match execution {
        EventExecution::Maintenance => "maintenance",
        EventExecution::Online => "online",
    }
}

fn execution_label(execution: EventExecution) -> String {
    get_text(&format!("event_editor_execution_{}", execution_value(execution)))
}

/// Blank disables the option; reject zero, signs, fractions and overflow.
fn parse_minutes(text: &str) -> Option<Option<u32>> {
    if text.trim().is_empty() {
        return Some(None);
    }
    if !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse::<u32>().ok().filter(|minutes| *minutes > 0).map(Some)
}

fn mode_value(mode: EventMode) -> &'static str {
    match mode {
        EventMode::Fixed => "fixed",
        EventMode::Slide => "slide",
        EventMode::Idle => "idle",
    }
}

fn mode_label(mode: EventMode) -> String {
    get_text(&format!("event_editor_mode_{}", mode_value(mode)))
}

/// PCBoard lists the mode as a single letter and explains them below the list.
fn mode_letter(mode: EventMode) -> String {
    get_text(&format!("event_editor_mode_{}_letter", mode_value(mode)))
}

/// Do not use IcbTime::parse: it silently turns malformed input into midnight.
fn parse_time(text: &str) -> Option<IcbTime> {
    let parts = text.split(':').collect::<Vec<_>>();
    if !(parts.len() == 2 || parts.len() == 3) || parts.iter().any(|part| part.len() != 2 || !part.bytes().all(|b| b.is_ascii_digit())) {
        return None;
    }
    let hour = parts[0].parse::<u8>().ok()?;
    let minute = parts[1].parse::<u8>().ok()?;
    let second = if parts.len() == 3 { parts[2].parse::<u8>().ok()? } else { 0 };
    (hour < 24 && minute < 60 && second < 60).then(|| IcbTime::new(hour, minute, second))
}

fn parse_days(text: &str) -> Option<IcbDoW> {
    // Bound the input before calling the model parser (which accepts arbitrary lengths).
    (text.len() == 7 && text.bytes().all(|b| matches!(b, b'Y' | b'N' | b'y' | b'n'))).then(|| IcbDoW::from(text.to_ascii_uppercase()))
}

fn field(key: &str, value: ListValue) -> ConfigEntry<BoardEvent> {
    ConfigEntry::Item(
        ListItem::new(get_text(key), value)
            .with_label_width(16)
            .with_help(get_text(&format!("{key}-help")))
            .with_status(get_text(&format!("{key}-status"))),
    )
}

impl<'a> EventListEditor<'a> {
    pub(crate) fn new(path: &Path, root: &Path) -> Res<Self> {
        if path.as_os_str().is_empty() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, get_text("no_file_name_given")).into());
        }
        let original = if path.try_exists()? { EventList::load(&path)? } else { EventList::default() };
        let events = Arc::new(Mutex::new(original.clone()));
        let rows = events.clone();
        let len = original.len();
        Ok(Self {
            path: path.to_path_buf(),
            root: root.to_path_buf(),
            original,
            events,
            table: InsertTable {
                scroll_state: ScrollbarState::default().content_length(len),
                table_state: TableState::default().with_selected((len > 0).then_some(0)),
                numbered: true,
                // 5 record-number cells + 50 fixed cells leaves 22 for the command at 80 columns.
                columns: vec![
                    Column::new(get_text("event_editor_header_enabled")).with_width(5),
                    Column::new(get_text("event_editor_header_mode")).with_width(5),
                    Column::new(get_text("event_editor_header_time")).with_width(10),
                    Column::new(get_text("event_editor_header_days")).with_width(9),
                    Column::new(get_text("event_editor_header_description")).with_width(21),
                    Column::new(get_text("event_editor_header_command")).with_width(12),
                ],
                get_content: Box::new(move |_, row, col| {
                    let events = rows.lock().unwrap();
                    let Some(event) = events.get(*row) else { return Line::default() };
                    Line::from(match col {
                        // The list marks an active record with the editor's own check mark.
                        0 => (if event.enabled { "✓" } else { "✗" }).to_string(),
                        1 => mode_letter(event.mode),
                        2 => event.time.to_string(),
                        3 => event.days.to_string(),
                        4 => event.description.clone(),
                        5 => event.command.clone(),
                        _ => String::new(),
                    })
                }),
                content_length: len,
            },
            detail: super::EditorDialog::default(),
            save_changes: super::EditorSaveChanges::default(),
            history: None,
        })
    }

    fn open_detail(&mut self) {
        let Some(selected) = self.table.table_state.selected() else { return };
        let Some(event) = self.events.lock().unwrap().get(selected).cloned() else {
            return;
        };
        self.detail.open(super::align_editor_labels(ConfigMenu {
            entry: vec![
                field("event_editor_description", ListValue::Text(48, TextFlags::None, event.description.clone())),
                field("event_editor_enabled", ListValue::Bool(event.enabled)),
                field("event_editor_time", ListValue::Text(8, TextFlags::None, event.time.to_string())),
                field("event_editor_days", ListValue::Text(7, TextFlags::None, event.days.to_string())),
                field(
                    "event_editor_mode",
                    ListValue::ComboBox(ComboBox {
                        cur_value: ComboBoxValue::new(mode_label(event.mode), mode_value(event.mode)),
                        selected_item: [EventMode::Fixed, EventMode::Slide, EventMode::Idle]
                            .iter()
                            .position(|mode| *mode == event.mode)
                            .unwrap_or_default(),
                        is_edit_open: false,
                        first_item: 0,
                        values: [EventMode::Fixed, EventMode::Slide, EventMode::Idle]
                            .into_iter()
                            .map(|mode| ComboBoxValue::new(mode_label(mode), mode_value(mode)))
                            .collect(),
                    }),
                ),
                // A shell command is not a filename. F4 must not replace its arguments/quoting.
                field("event_editor_command", ListValue::Text(48, TextFlags::None, event.command.clone())),
                field(
                    "event_editor_end_time",
                    ListValue::Text(8, TextFlags::None, event.end_time.as_ref().map(ToString::to_string).unwrap_or_default()),
                ),
                field(
                    "event_editor_interval",
                    ListValue::Text(10, TextFlags::None, event.interval_minutes.map(|v| v.to_string()).unwrap_or_default()),
                ),
                field(
                    "event_editor_warning",
                    ListValue::Text(10, TextFlags::None, event.warning_minutes.map(|v| v.to_string()).unwrap_or_default()),
                ),
                field(
                    "event_editor_execution",
                    ListValue::ComboBox(ComboBox {
                        cur_value: ComboBoxValue::new(execution_label(event.execution), execution_value(event.execution)),
                        selected_item: usize::from(event.execution == EventExecution::Online),
                        is_edit_open: false,
                        first_item: 0,
                        values: [EventExecution::Maintenance, EventExecution::Online]
                            .into_iter()
                            .map(|execution| ComboBoxValue::new(execution_label(execution), execution_value(execution)))
                            .collect(),
                    }),
                ),
            ],
            obj: event,
        }));
    }

    fn apply_detail(&mut self) -> PageMessage {
        let Some(menu) = &self.detail.menu else { return PageMessage::None };
        // Read the form directly: saving must not depend on a render/update callback.
        let text = |index| match &menu.get_item(index).unwrap().value {
            ListValue::Text(_, _, text) => text.clone(),
            _ => unreachable!(),
        };
        let Some(time) = parse_time(&text(2)) else {
            self.detail.state.selected = 2;
            return PageMessage::InfoBox(InfoState::Warning, get_text("event_editor_invalid_time"));
        };
        let Some(days) = parse_days(&text(3)) else {
            self.detail.state.selected = 3;
            return PageMessage::InfoBox(InfoState::Warning, get_text("event_editor_invalid_days"));
        };
        let end_text = text(6);
        let end_time = if end_text.trim().is_empty() {
            None
        } else {
            let Some(end) = parse_time(&end_text).filter(|end| end.to_pcboard_time() >= time.to_pcboard_time()) else {
                self.detail.state.selected = 6;
                return PageMessage::InfoBox(InfoState::Warning, get_text("event_editor_invalid_end_time"));
            };
            Some(end)
        };
        let Some(interval_minutes) = parse_minutes(&text(7)) else {
            self.detail.state.selected = 7;
            return PageMessage::InfoBox(InfoState::Warning, get_text("event_editor_invalid_minutes"));
        };
        let Some(warning_minutes) = parse_minutes(&text(8)) else {
            self.detail.state.selected = 8;
            return PageMessage::InfoBox(InfoState::Warning, get_text("event_editor_invalid_minutes"));
        };
        // Preserve any model fields not exposed by this form.
        let mut event = menu.obj.clone();
        event.description = text(0);
        if let ListValue::Bool(enabled) = &menu.get_item(1).unwrap().value {
            event.enabled = *enabled;
        }
        event.time = time;
        event.days = days;
        if let ListValue::ComboBox(mode) = &menu.get_item(4).unwrap().value {
            event.mode = match mode.cur_value.value.as_str() {
                "slide" => EventMode::Slide,
                "idle" => EventMode::Idle,
                _ => EventMode::Fixed,
            };
        }
        event.command = text(5);
        event.end_time = end_time;
        event.interval_minutes = interval_minutes;
        event.warning_minutes = warning_minutes;
        if let ListValue::ComboBox(execution) = &menu.get_item(9).unwrap().value {
            event.execution = if execution.cur_value.value == "online" {
                EventExecution::Online
            } else {
                EventExecution::Maintenance
            };
        }
        if let Some(selected) = self.table.table_state.selected() {
            self.events.lock().unwrap()[selected] = event;
        }
        self.detail.close();
        PageMessage::None
    }

    fn open_history(&mut self) -> PageMessage {
        let Some(selected) = self.table.table_state.selected() else {
            return PageMessage::None;
        };
        let Some(event) = self.events.lock().unwrap().get(selected).cloned() else {
            return PageMessage::None;
        };
        // Never open/recover the scheduler journal or derive its root from the event file.
        match EventHistory::read_entries(&self.root) {
            Ok(entries) => {
                // Reverse first so equal timestamps prefer the last journal entry.
                let mut entries: Vec<_> = entries.into_iter().rev().filter(|entry| entry.event_id == event.id).collect();
                entries.sort_by_key(|entry| std::cmp::Reverse(entry.start.unwrap_or(entry.scheduled_for)));
                self.history = Some(HistoryView {
                    description: event.description,
                    entries,
                    selected: 0,
                    scroll: 0,
                });
                PageMessage::None
            }
            Err(error) => PageMessage::InfoBox(
                InfoState::Error,
                get_text_args("event_editor_history_failed", [("error".into(), error.to_string())].into()),
            ),
        }
    }

    fn render_history(&self, frame: &mut Frame, inner: Rect) {
        let Some(history) = &self.history else { return };
        let width = inner.width.min(76);
        let height = inner.height.min(21);
        let popup = Rect::new(
            inner.x + inner.width.saturating_sub(width) / 2,
            inner.y + inner.height.saturating_sub(height) / 2,
            width,
            height,
        );
        Clear.render(popup, frame.buffer_mut());
        let block =
            super::popup_frame(get_text("event_editor_history_title")).title_bottom(icy_board_tui::chrome::key_hint(get_text("event_editor_history_keys")));
        let mut lines = vec![Line::from(history.description.clone())];
        let value = |key: &str, text: String| Line::from(format!("{}: {text}", get_text(key)));
        if let Some(entry) = history.entries.get(history.selected) {
            let latest = &history.entries[0];
            lines.push(value(
                "event_editor_history_latest",
                format!(
                    "{} | {} | {}: {}",
                    latest.start.unwrap_or(latest.scheduled_for).to_rfc3339(),
                    result_label(&latest.result),
                    get_text("event_editor_history_exit"),
                    latest.exit_code.map(|code| code.to_string()).unwrap_or_else(|| "—".into()),
                ),
            ));
            lines.push(Line::from(format!("{} / {}", history.selected + 1, history.entries.len())));
            lines.push(value("event_editor_history_scheduled", entry.scheduled_for.to_rfc3339()));
            lines.push(value(
                "event_editor_history_start",
                entry.start.map(|time| time.to_rfc3339()).unwrap_or_else(|| "—".into()),
            ));
            lines.push(value(
                "event_editor_history_finish",
                entry.finish.map(|time| time.to_rfc3339()).unwrap_or_else(|| "—".into()),
            ));
            lines.push(value("event_editor_history_result", result_label(&entry.result)));
            lines.push(value(
                "event_editor_history_exit",
                entry.exit_code.map(|code| code.to_string()).unwrap_or_else(|| "—".into()),
            ));
            lines.push(value("event_editor_execution", execution_label(entry.execution)));
            lines.push(value("event_editor_history_log", entry.log_file.clone().unwrap_or_else(|| "—".into())));
            let modified = entry
                .log_file
                .as_ref()
                .and_then(|log| std::fs::metadata(self.root.join(log)).ok())
                .and_then(|meta| meta.modified().ok());
            lines.push(value(
                "event_editor_history_log_time",
                modified
                    .map(|time| chrono::DateTime::<chrono::Utc>::from(time).to_rfc3339())
                    .unwrap_or_else(|| get_text("event_editor_history_unavailable")),
            ));
            if let Some(detail) = &entry.detail {
                lines.push(value("event_editor_history_detail", detail.clone()));
            }
        } else {
            lines.push(Line::from(get_text("event_editor_history_empty")));
        }
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((history.scroll, 0))
            .render(popup, frame.buffer_mut());
    }
}

impl Page for EventListEditor<'_> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        let modal = self.detail.is_open() || self.history.is_some() || self.save_changes.is_open();
        let frame_block = super::list_editor_frame(get_text("event_editor_title"), String::new(), modal);
        let inner = frame_block.inner(area);
        frame_block.render(area, frame.buffer_mut());
        if inner.width > 1 && inner.height > 5 {
            // Reserve the footer even under modals so the table never jumps.
            let [list, footer] = Layout::vertical([Constraint::Min(1), Constraint::Length(3)]).areas(inner);
            self.table.render_list(frame, list);
            if self.table.content_length == 0 && list.height > 3 {
                Line::from(get_text("event_editor_empty"))
                    .centered()
                    .render(Rect::new(list.x, list.y + 3, list.width, 1), frame.buffer_mut());
            }
            // A modal owns the keyboard; the full list legend needs two key rows.
            super::render_editor_footer(
                frame,
                footer,
                vec![
                    Line::from(get_text("event_editor_mode_legend")).style(get_tui_theme().config_title),
                    icy_board_tui::chrome::key_hint(get_text("event_editor_keys")),
                    icy_board_tui::chrome::key_hint(get_text("event_editor_keys_more")),
                ],
                modal,
            );
        }
        if let Some(menu) = &mut self.detail.menu {
            let backdrop = frame.area();
            icy_board_tui::chrome::dim_background(frame.buffer_mut(), backdrop);
            let width = inner.width.min(72);
            let height = inner.height.min(16);
            let popup = Rect::new(
                inner.x + inner.width.saturating_sub(width) / 2,
                inner.y + inner.height.saturating_sub(height) / 2,
                width,
                height,
            );
            Clear.render(popup, frame.buffer_mut());
            let block =
                super::popup_frame(get_text("event_editor_detail_title")).title_bottom(icy_board_tui::chrome::key_hint(get_text("event_editor_detail_keys")));
            let content = block.inner(popup);
            block.render(popup, frame.buffer_mut());
            if content.width > 1 && content.height > 0 {
                super::render_config_form(frame, content, menu, &mut self.detail.state);
                if content.height > 10 && matches!(&menu.get_item(9).unwrap().value, ListValue::ComboBox(combo) if combo.cur_value.value == "online") {
                    Paragraph::new(get_text("event_editor_online_warning"))
                        .wrap(Wrap { trim: true })
                        .render(Rect::new(content.x, content.y + 10, content.width, content.height - 10), frame.buffer_mut());
                }
            }
        }
        self.render_history(frame, inner);
        self.save_changes.render(frame, area);
    }

    fn request_status(&self) -> ResultState {
        if self.history.is_some() {
            return ResultState::status_line(get_text("event_editor_history_status"));
        }
        if self.detail.is_open() {
            self.detail.status()
        } else {
            ResultState::status_line(get_text("event_editor_status"))
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if key.kind == KeyEventKind::Release {
            return PageMessage::None;
        }
        if let Some(history) = &mut self.history {
            match key.code {
                KeyCode::Esc => self.history = None,
                KeyCode::F(6) => return self.open_history(),
                KeyCode::F(1) => {
                    return PageMessage::ResultState(ResultState {
                        edit_msg: EditMessage::DisplayHelp(get_text("event_editor_history_help")),
                        ..ResultState::default()
                    });
                }
                KeyCode::Up | KeyCode::Home => {
                    history.selected = if key.code == KeyCode::Home { 0 } else { history.selected.saturating_sub(1) };
                    history.scroll = 0;
                }
                KeyCode::Down | KeyCode::End => {
                    let last = history.entries.len().saturating_sub(1);
                    history.selected = if key.code == KeyCode::End { last } else { (history.selected + 1).min(last) };
                    history.scroll = 0;
                }
                KeyCode::PageDown => history.scroll = history.scroll.saturating_add(5),
                KeyCode::PageUp => history.scroll = history.scroll.saturating_sub(5),
                _ => {}
            }
            return PageMessage::None;
        }
        if let Some(message) = self
            .save_changes
            .handle_key(key, || super::save_file(&self.path, || self.events.lock().unwrap().save(&self.path)))
        {
            return message;
        }
        if let Some(result) = self.detail.handle_input(key) {
            // Let expanded controls consume Escape before validating the draft.
            if result.edit_msg == EditMessage::Close {
                return self.apply_detail();
            }
            return PageMessage::ResultState(result);
        }
        let selected = self.table.table_state.selected();
        match key.code {
            KeyCode::Esc => {
                return self.save_changes.request_close(*self.events.lock().unwrap() != self.original);
            }
            KeyCode::F(1) => {
                return PageMessage::ResultState(ResultState {
                    edit_msg: EditMessage::DisplayHelp(get_text("event_editor_help")),
                    ..ResultState::default()
                });
            }
            KeyCode::F(6) => return self.open_history(),
            KeyCode::Enter => self.open_detail(),
            KeyCode::Insert => {
                let mut events = self.events.lock().unwrap();
                let index = selected.filter(|index| *index < events.len()).map_or(0, |index| index + 1);
                events.insert(
                    index,
                    BoardEvent {
                        enabled: false,
                        ..BoardEvent::default()
                    },
                );
                self.table.sync_rows(events.len(), Some(index));
            }
            KeyCode::F(5) => {
                let mut events = self.events.lock().unwrap();
                if let Some(index) = selected.filter(|index| *index < events.len()) {
                    let mut event = events[index].clone();
                    event.id = BoardEvent::new_id();
                    events.insert(index + 1, event);
                    self.table.sync_rows(events.len(), Some(index + 1));
                }
            }
            KeyCode::Delete => {
                self.table.remove_row(&mut *self.events.lock().unwrap());
            }
            KeyCode::PageUp => self.table.move_row(&mut self.events.lock().unwrap(), -1),
            KeyCode::PageDown => self.table.move_row(&mut self.events.lock().unwrap(), 1),
            _ => {
                let _ = self.table.handle_key_press(key);
                self.table
                    .sync_rows(self.events.lock().unwrap().len(), self.table.table_state.selected().or(Some(0)));
            }
        }
        PageMessage::None
    }
}

pub fn edit_events(board: Arc<Mutex<IcyBoard>>, path: PathBuf) -> PageMessage {
    let root = board.lock().unwrap().root_path.clone();
    match EventListEditor::new(&path, &root) {
        Ok(mut editor) => {
            editor.detail.state.path_base = Some(root);
            PageMessage::OpenSubPage(Box::new(editor))
        }
        Err(error) => PageMessage::InfoBox(
            InfoState::Error,
            get_text_args(
                "event_editor_load_failed",
                [("path".into(), path.display().to_string()), ("error".into(), error.to_string())].into(),
            ),
        ),
    }
}

#[cfg(test)]
#[path = "events_tests.rs"]
mod tests;
