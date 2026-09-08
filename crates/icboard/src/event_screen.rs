//! Operator-only event selection. The scheduler alone executes commands and owns
//! the journal lease; this screen reads a cached, read-only history snapshot.
use std::{
    collections::HashSet,
    path::{Component, Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use chrono::{DateTime, Datelike, Local, Utc};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use icy_board_engine::icy_board::{
    IcyBoard,
    bbs::{BBS, EventMaintenanceStatus, OnlineEventStatus},
    events::{
        BoardEvent, EventExecution, EventMode, EventWindow,
        event_history::{EventHistory, EventHistoryEntry, EventResult, LOG_DIRECTORY},
    },
};
use icy_board_tui::{
    app::get_screen_size,
    chrome::{dim_background, key_hint},
    get_text,
    theme::get_tui_theme,
};
use ratatui::{
    Frame, Terminal,
    backend::Backend,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Block, Clear, Paragraph, Row, Table, TableState, Widget, Wrap},
};
use tokio::sync::Mutex;

use crate::{Res, log_screen::sanitize};

/// Long enough to keep the operator's key, short enough to keep the clock and
/// the queue state moving.
const POLL_INTERVAL: Duration = Duration::from_millis(200);

#[derive(Clone, Default)]
pub(crate) struct RuntimeStatus {
    pub offline: bool,
    pub restart: bool,
    pub maintenance: Option<EventMaintenanceStatus>,
    pub online: Option<OnlineEventStatus>,
    pub failure: Option<String>,
    pub request_error: Option<String>,
    pub pending: HashSet<String>,
    pub window: Option<EventWindow>,
}

impl RuntimeStatus {
    /// Never block a progress redraw on a writer. Keep the last snapshot on contention.
    pub fn refresh(&mut self, bbs: &Arc<Mutex<BBS>>) {
        if let Ok(bbs) = bbs.try_lock() {
            self.offline = bbs.admissions_closed() || bbs.event_scheduler_error.is_some() || bbs.event_restart_requested;
            self.restart = bbs.event_restart_requested;
            self.maintenance = bbs.event_maintenance_status.clone();
            self.online = bbs.event_online_status.clone();
            self.failure = bbs.event_scheduler_error.clone();
            self.request_error = bbs.event_request_error.clone();
            self.pending = bbs.event_active_ids.iter().chain(bbs.event_run_requests.iter()).cloned().collect();
            self.window = bbs.event_window.clone();
        }
    }

    pub fn online_text(&self) -> Option<String> {
        self.online.as_ref().map(|status| {
            format!(
                "ONLINE: {} | {}s{}",
                sanitize(&status.description),
                (chrono::Utc::now() - status.started).num_seconds().max(0),
                if status.running_long {
                    format!(" | {}", get_text("event_runtime_long"))
                } else {
                    String::new()
                }
            )
        })
    }
}

/// Used under the BBS lock before handing off to an offline tool or exiting.
/// Includes requests not yet consumed by the scheduler, not just running shells.
pub(crate) fn runtime_busy(bbs: &BBS) -> bool {
    bbs.admissions_closed()
        || bbs.event_restart_requested
        || bbs.event_scheduler_error.is_some()
        || bbs.event_online_status.is_some()
        || !bbs.event_active_ids.is_empty()
        || !bbs.event_run_requests.is_empty()
}

fn execution_text(execution: EventExecution) -> String {
    get_text(match execution {
        EventExecution::Maintenance => "event_runtime_maintenance",
        EventExecution::Online => "event_runtime_online",
    })
}

fn mode_text(mode: EventMode) -> String {
    get_text(match mode {
        EventMode::Fixed => "event_runtime_fixed",
        EventMode::Slide => "event_runtime_slide",
        EventMode::Idle => "event_runtime_idle",
    })
}

fn result_text(result: &EventResult) -> String {
    get_text(match result {
        EventResult::Pending => "event_runtime_pending",
        EventResult::Success => "event_runtime_success",
        EventResult::NonzeroExit => "event_runtime_nonzero",
        EventResult::SpawnError => "event_runtime_spawn_error",
        EventResult::WaitError => "event_runtime_wait_error",
        EventResult::Interrupted => "event_runtime_interrupted",
        EventResult::SkippedBusy => "event_runtime_skipped_busy",
        EventResult::Expired => "event_runtime_expired",
        EventResult::Superseded => "event_runtime_superseded",
    })
}

fn elapsed_text(seconds: i64) -> String {
    let seconds = seconds.max(0);
    let days = seconds / 86400;
    let clock = format!("{:02}:{:02}:{:02}", seconds / 3600 % 24, seconds / 60 % 60, seconds % 60);
    if days == 0 { clock } else { format!("{days}d {clock}") }
}

/// A journal start is an attempted start, not proof of a successfully spawned
/// child. Recovery timestamps and wait failures cannot establish its duration.
fn duration_text(entry: &EventHistoryEntry, now: DateTime<Utc>) -> String {
    let label = get_text(if entry.result == EventResult::Pending && entry.start.is_some() && entry.finish.is_none() {
        "event_runtime_elapsed"
    } else {
        "event_runtime_duration"
    });
    let duration = entry.start.and_then(|start| {
        let end = match entry.result {
            EventResult::Pending if entry.finish.is_none() => now,
            EventResult::Interrupted | EventResult::WaitError => return None,
            _ => entry.finish?,
        };
        (end >= start).then(|| elapsed_text((end - start).num_seconds()))
    });
    format!("{label}: {}", duration.unwrap_or_else(|| "—".into()))
}

/// Match the journal's deliberately narrow relative format, then check the
/// filesystem too. Never interpret an arbitrary command or description as a path.
/// Reject all symlinks (even inward ones), in keeping with the scheduler writer.
fn checked_log_path(root: &Path, stored: &str) -> Result<PathBuf, String> {
    let invalid = || get_text("event_runtime_log_invalid");
    let name = stored.strip_prefix(&format!("{LOG_DIRECTORY}/")).ok_or_else(invalid)?;
    if !name.ends_with(".log")
        || name.contains("..")
        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
        || Path::new(stored).components().any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(invalid());
    }
    let root = root.canonicalize().map_err(|_| invalid())?;
    let directory = root.join(LOG_DIRECTORY);
    let directory_meta = std::fs::symlink_metadata(&directory).map_err(|_| get_text("event_runtime_log_missing"))?;
    if !directory_meta.is_dir() || directory_meta.file_type().is_symlink() || directory.canonicalize().map_err(|_| invalid())? != directory {
        return Err(invalid());
    }
    let path = directory.join(name);
    let metadata = std::fs::symlink_metadata(&path).map_err(|_| get_text("event_runtime_log_missing"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(invalid());
    }
    let canonical = path.canonicalize().map_err(|_| invalid())?;
    if canonical.parent() != Some(directory.as_path()) {
        return Err(invalid());
    }
    Ok(canonical)
}

struct Confirmation {
    event: BoardEvent,
    yes: bool,
}

/// What the operator asked for; the loop owns board and BBS access.
enum Action {
    Nothing,
    Exit,
    Refresh,
    OpenLog(String),
    Queue(BoardEvent),
}

#[derive(Default)]
struct EventScreen {
    events: Vec<BoardEvent>,
    history: Vec<EventHistoryEntry>,
    history_error: Option<String>,
    table: TableState,
    confirmation: Option<Confirmation>,
    notice: Option<String>,
    runtime: RuntimeStatus,
    root: PathBuf,
    schedule_enabled: bool,
    /// None follows latest; an explicit key survives refresh and newly added runs.
    execution_key: Option<String>,
    detail_scroll: u16,
    history_job: Option<tokio::task::JoinHandle<Result<Vec<EventHistoryEntry>, String>>>,
}

impl EventScreen {
    fn start_refresh(&mut self, board: &Arc<Mutex<IcyBoard>>) -> bool {
        if self.history_job.is_some() {
            return false;
        }
        // Do not queue behind board reload or block return to the owner handshake.
        let Ok(board) = board.try_lock() else { return false };
        let selected_id = self.selected().map(|event| event.id.clone());
        self.events = board.events.iter().cloned().collect();
        self.schedule_enabled = board.config.event.enabled;
        let root = board.root_path.clone();
        drop(board);
        if self.root != root {
            self.history.clear();
            self.execution_key = None;
        }
        self.root = root.clone();
        let selected = selected_id.and_then(|id| self.events.iter().position(|event| event.id == id)).unwrap_or(0);
        self.table.select((!self.events.is_empty()).then_some(selected));
        self.confirmation = None;
        // No EventHistory::open: opening would take/recover the scheduler lease.
        self.history_job = Some(tokio::task::spawn_blocking(move || {
            EventHistory::read_entries(&root).map_err(|error| error.to_string())
        }));
        true
    }

    // The production loop calls this only for a completed worker, never waits on IO.
    async fn finish_refresh(&mut self) {
        let Some(job) = self.history_job.take() else { return };
        match job.await {
            Ok(Ok(entries)) => {
                self.history = entries;
                self.history_error = None;
                if self
                    .execution_key
                    .as_ref()
                    .is_some_and(|key| !self.history.iter().any(|entry| &entry.key == key))
                {
                    self.execution_key = None;
                }
            }
            result => {
                self.history.clear();
                let error = match result {
                    Ok(Err(error)) => error.to_string(),
                    Err(error) => error.to_string(),
                    _ => unreachable!(),
                };
                self.history_error = Some(format!("{}: {error}", get_text("event_runtime_history_error")));
            }
        }
    }

    #[cfg(test)]
    async fn refresh(&mut self, board: &Arc<Mutex<IcyBoard>>) {
        self.start_refresh(board);
        self.finish_refresh().await;
    }

    fn selected(&self) -> Option<&BoardEvent> {
        self.table.selected().and_then(|index| self.events.get(index))
    }

    fn last_result(&self, id: &str) -> Option<&EventHistoryEntry> {
        self.history.iter().rev().find(|entry| entry.event_id == id)
    }

    fn selected_execution(&self) -> Option<&EventHistoryEntry> {
        let id = &self.selected()?.id;
        self.execution_key
            .as_ref()
            .and_then(|key| self.history.iter().find(|entry| &entry.key == key && &entry.event_id == id))
            .or_else(|| self.last_result(id))
    }

    fn navigate_history(&mut self, older: bool) {
        let Some(event) = self.selected() else { return };
        let runs = self.history.iter().rev().filter(|entry| entry.event_id == event.id).collect::<Vec<_>>();
        let current = self
            .execution_key
            .as_ref()
            .and_then(|key| runs.iter().position(|entry| &entry.key == key))
            .unwrap_or(0);
        let index = if older {
            (current + 1).min(runs.len().saturating_sub(1))
        } else {
            current.saturating_sub(1)
        };
        self.execution_key = runs.get(index).map(|entry| entry.key.clone());
        self.detail_scroll = 0;
        self.notice = None;
    }

    fn log_action(&mut self) -> Action {
        // The latest pending history entry also carries current Online output.
        // Do not substitute another run's log when browsing an older/no-log run.
        match self.selected_execution().and_then(|entry| entry.log_file.clone()) {
            Some(path) => Action::OpenLog(path),
            None => {
                self.notice = Some(get_text("event_runtime_log_none"));
                Action::Nothing
            }
        }
    }

    /// Enter opens a default-No dialog. A second Enter never queues by accident.
    fn confirm_key(&mut self, key: KeyCode) -> Option<BoardEvent> {
        if let Some(confirmation) = &mut self.confirmation {
            match key {
                KeyCode::Left | KeyCode::Right | KeyCode::Tab => confirmation.yes = !confirmation.yes,
                KeyCode::Esc => self.confirmation = None,
                KeyCode::Enter => {
                    let confirmation = self.confirmation.take().unwrap();
                    return confirmation.yes.then_some(confirmation.event);
                }
                _ => {}
            }
        } else if key == KeyCode::Enter {
            if self.runtime.failure.is_some() || self.runtime.offline {
                self.notice = Some(get_text("event_runtime_unavailable"));
            } else if let Some(event) = self.selected().cloned() {
                if self.runtime.pending.contains(&event.id) {
                    self.notice = Some(get_text("event_runtime_duplicate"));
                } else {
                    self.confirmation = Some(Confirmation { event, yes: false });
                }
            }
        }
        None
    }

    fn select(&mut self, index: usize) {
        self.notice = None;
        let previous = self.table.selected();
        self.table
            .select((!self.events.is_empty()).then(|| index.min(self.events.len().saturating_sub(1))));
        if previous != self.table.selected() {
            self.execution_key = None;
            self.detail_scroll = 0;
        }
    }

    fn handle_key(&mut self, key: KeyCode) -> Action {
        if self.confirmation.is_some() {
            return self.confirm_key(key).map_or(Action::Nothing, Action::Queue);
        }
        let current = self.table.selected().unwrap_or(0);
        match key {
            KeyCode::Esc => Action::Exit,
            KeyCode::Enter => self.confirm_key(key).map_or(Action::Nothing, Action::Queue),
            KeyCode::Char('r' | 'R') | KeyCode::F(5) => Action::Refresh,
            KeyCode::Char('l' | 'L') => self.log_action(),
            KeyCode::PageUp | KeyCode::PageDown => {
                self.navigate_history(key == KeyCode::PageUp);
                Action::Nothing
            }
            KeyCode::Left | KeyCode::Right => {
                self.detail_scroll = if key == KeyCode::Right {
                    self.detail_scroll.saturating_add(1)
                } else {
                    self.detail_scroll.saturating_sub(1)
                };
                Action::Nothing
            }
            KeyCode::Down => {
                self.select(current + 1);
                Action::Nothing
            }
            KeyCode::Up => {
                self.select(current.saturating_sub(1));
                Action::Nothing
            }
            KeyCode::Home => {
                self.select(0);
                Action::Nothing
            }
            KeyCode::End => {
                self.select(usize::MAX);
                Action::Nothing
            }
            _ => Action::Nothing,
        }
    }

    /// An event without a description would otherwise be an empty row.
    fn label(&self, index: usize) -> String {
        let event = &self.events[index];
        let description = sanitize(&event.description);
        let command = sanitize(&event.command);
        if !description.trim().is_empty() {
            description
        } else if !command.trim().is_empty() {
            command
        } else {
            format!("{} #{}", get_text("event_runtime_unnamed"), index + 1)
        }
    }

    fn next_run(&self, event: &BoardEvent, now: DateTime<Local>) -> String {
        event
            .next_occurrence(&now)
            .map(|time| time.format("%a %H:%M").to_string())
            .unwrap_or_else(|| "—".into())
    }

    fn schedule_detail(&self, event: &BoardEvent, now: DateTime<Local>) -> String {
        let mut lines = vec![format!(
            "{}: {}",
            get_text("event_runtime_candidate"),
            event.next_occurrence(&now).map_or_else(
                || "—".into(),
                |at| format!("{} (+{})", at.format("%Y-%m-%d %H:%M:%S %:z"), elapsed_text((at - now).num_seconds()))
            )
        )];
        // These are independent restrictions; manual requests override schedule
        // switches/days/windows, but never the scheduler's gate or mode rules.
        if !self.schedule_enabled {
            lines.push(get_text("event_runtime_schedule_off"));
        }
        if !event.enabled {
            lines.push(get_text("event_runtime_event_off"));
        }
        if let Err(error) = event.validate() {
            lines.push(format!("{}: {}", get_text("event_runtime_invalid"), sanitize(&error)));
        } else if event.days.is_empty() {
            lines.push(get_text("event_runtime_no_days"));
        } else if !event.days.contains(now.weekday()) {
            lines.push(get_text("event_runtime_wrong_day"));
        } else if event.expired(now, now) {
            // There is no calendar date-range setting in BoardEvent. end_time
            // bounds starts on the occurrence's day, not execution duration.
            lines.push(get_text("event_runtime_window_ended"));
        }
        if self.runtime.pending.contains(&event.id) {
            lines.push(get_text("event_runtime_queued_active"));
        }
        if let Some(window) = &self.runtime.window
            && window.event.id == event.id
            && window.run_at <= now
            && !window.event.expired(window.run_at, now)
        {
            // This snapshot is maintenance-only and can lag a tick; never infer
            // an Online backlog or manufacture missed slots from wall time.
            lines.push(get_text("event_runtime_window_waiting"));
        }
        lines.join("\n")
    }

    fn detail(&self, now: DateTime<Local>) -> String {
        let Some(event) = self.selected() else {
            return get_text("event_runtime_no_history");
        };
        let mut lines = vec![self.schedule_detail(event, now)];
        if let Some(entry) = self.selected_execution() {
            lines.push(format!(
                "{} | {} | {}",
                entry.scheduled_for.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S %:z"),
                result_text(&entry.result),
                get_text(if entry.manual { "event_runtime_manual" } else { "event_runtime_scheduled" }),
            ));
            lines.push(format!(
                "{} | {}: {}",
                duration_text(entry, now.with_timezone(&Utc)),
                get_text("event_runtime_exit_code"),
                entry.exit_code.map_or_else(|| "—".into(), |code| code.to_string())
            ));
        } else {
            lines.push(get_text("event_runtime_no_history"));
        }
        lines.push(format!("{}: {}", get_text("event_editor_header_command"), sanitize(event.command.trim())));
        lines.push(format!("{}: {}", get_text("event_runtime_event"), sanitize(&event.description)));
        if let Some(entry) = self.selected_execution() {
            lines.push(format!(
                "{}: {}",
                get_text("event_runtime_log"),
                sanitize(entry.log_file.as_deref().unwrap_or("—"))
            ));
            lines.push(sanitize(&entry.description));
            // Sanitize each physical line independently; preserve readable detail
            // layout without passing any terminal/bidi controls to the renderer.
            lines.extend(entry.detail.as_deref().unwrap_or("").lines().map(sanitize));
            lines.push(get_text("event_runtime_timing_hint"));
        }
        if let Some(error) = &self.history_error {
            lines.push(sanitize(error));
        }
        lines.push(get_text("event_runtime_candidate_hint"));
        lines.push(get_text("event_runtime_manual_help"));
        lines.join("\n")
    }

    fn ui(&mut self, frame: &mut Frame, full_screen: bool) {
        self.ui_at(frame, full_screen, Local::now());
    }

    fn ui_at(&mut self, frame: &mut Frame, full_screen: bool, now: DateTime<Local>) {
        let theme = get_tui_theme();
        let area = get_screen_size(frame, full_screen);
        frame.render_widget(Clear, area);
        Block::new().style(theme.background).render(area, frame.buffer_mut());
        let block = Block::bordered()
            .title(Line::styled(format!(" {} ", get_text("event_runtime_picker_title")), theme.dialog_box_title))
            .title_bottom(key_hint(get_text("event_runtime_picker_keys")))
            .border_style(theme.dialog_box)
            .style(theme.background);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let [status, list, history] = Layout::vertical([Constraint::Length(2), Constraint::Min(3), Constraint::Length(12)]).areas(inner);
        let alert = self
            .runtime
            .failure
            .clone()
            .or_else(|| self.notice.clone())
            .or_else(|| self.runtime.request_error.clone());
        let status_text = [self.runtime.online_text(), alert.clone()]
            .into_iter()
            .flatten()
            .map(|text| sanitize(&text))
            .collect::<Vec<_>>()
            .join("\n");
        frame.render_widget(
            Paragraph::new(status_text).style(if alert.is_some() { theme.false_value } else { theme.description_text }),
            status,
        );
        let rows = self
            .events
            .iter()
            .enumerate()
            .map(|(index, event)| {
                let enabled = get_text(if event.enabled { "event_runtime_enabled" } else { "event_runtime_disabled" });
                let result = if self.runtime.pending.contains(&event.id) {
                    get_text("event_runtime_queued_active")
                } else {
                    self.last_result(&event.id)
                        .map(|entry| result_text(&entry.result))
                        .unwrap_or_else(|| "—".into())
                };
                Row::new(vec![
                    self.label(index),
                    enabled,
                    format!("{:02}:{:02}", event.time.get_hour(), event.time.get_minute()),
                    event.days.to_string(),
                    format!("{}/{}", execution_text(event.execution), mode_text(event.mode)),
                    self.next_run(event, now),
                    result,
                ])
            })
            .collect::<Vec<_>>();
        // Translated words decide the width; "No" and "Nein" are not the same size.
        let enabled_width = ["event_runtime_enabled", "event_runtime_disabled", "event_editor_header_enabled"]
            .into_iter()
            .map(|key| get_text(key).chars().count() as u16)
            .max()
            .unwrap_or(3);
        let table = Table::new(
            rows,
            [
                Constraint::Min(8),
                Constraint::Length(enabled_width),
                Constraint::Length(5),
                Constraint::Length(7),
                Constraint::Length(17),
                Constraint::Length(9),
                Constraint::Length(14),
            ],
        )
        .style(theme.table)
        .header(
            Row::new(vec![
                get_text("event_runtime_event"),
                get_text("event_editor_header_enabled"),
                get_text("event_editor_header_time"),
                get_text("event_editor_header_days"),
                get_text("event_runtime_mode_column"),
                get_text("event_runtime_next_column"),
                get_text("event_runtime_result_column"),
            ])
            .style(theme.config_title),
        )
        .row_highlight_style(theme.selected_item);
        frame.render_stateful_widget(table, list, &mut self.table);
        let execution_position = self.selected_execution().map_or_else(String::new, |selected| {
            let runs = self
                .history
                .iter()
                .rev()
                .filter(|entry| entry.event_id == selected.event_id)
                .collect::<Vec<_>>();
            let index = runs.iter().position(|entry| entry.key == selected.key).unwrap_or(0) + 1;
            format!(" ({index}/{})", runs.len())
        });
        let detail_block = Block::bordered()
            .border_style(theme.dialog_box)
            .title(Line::styled(
                format!(" {}{execution_position} ", get_text("event_runtime_history_title")),
                theme.dialog_box_title,
            ))
            .title_bottom(key_hint(get_text("event_runtime_detail_keys")));
        let detail_area = detail_block.inner(history);
        let paragraph = Paragraph::new(self.detail(now)).wrap(Wrap { trim: true }).style(theme.value);
        let max_scroll = paragraph
            .line_count(detail_area.width.max(1))
            .saturating_sub(detail_area.height as usize)
            .min(u16::MAX as usize) as u16;
        self.detail_scroll = self.detail_scroll.min(max_scroll);
        frame.render_widget(paragraph.scroll((self.detail_scroll, 0)).block(detail_block), history);
        if self.events.is_empty() {
            frame.render_widget(Paragraph::new(get_text("event_runtime_empty")).style(theme.description_text), list);
        }
        if let Some(confirmation) = &self.confirmation {
            let popup = Rect::new(
                area.x + area.width.saturating_sub(74) / 2,
                area.y + area.height.saturating_sub(19) / 2,
                area.width.min(74),
                area.height.min(19),
            );
            dim_background(frame.buffer_mut(), area);
            frame.render_widget(Clear, popup);
            let block = Block::bordered()
                .title(Line::styled(format!(" {} ", get_text("event_runtime_confirm_title")), theme.menu_box_title))
                .border_style(theme.menu_box)
                .style(theme.background);
            let inner = block.inner(popup);
            frame.render_widget(block, popup);
            let [details, help, choices] = Layout::vertical([Constraint::Length(4), Constraint::Min(1), Constraint::Length(2)]).areas(inner);
            frame.render_widget(
                Paragraph::new(format!(
                    "{}\n{} / {} / {}\n{}",
                    self.label(self.table.selected().unwrap_or(0)),
                    execution_text(confirmation.event.execution),
                    mode_text(confirmation.event.mode),
                    get_text(if confirmation.event.enabled {
                        "event_runtime_enabled"
                    } else {
                        "event_runtime_disabled"
                    }),
                    sanitize(&confirmation.event.command)
                ))
                .style(theme.value)
                .wrap(Wrap { trim: true }),
                details,
            );
            frame.render_widget(
                Paragraph::new(format!(
                    "{}\n\n{}",
                    get_text("event_runtime_manual_help"),
                    get_text("event_runtime_online_warning")
                ))
                .style(theme.description_text)
                .wrap(Wrap { trim: true }),
                help,
            );
            let yes = get_text("event_runtime_yes");
            let no = get_text("event_runtime_no");
            let (yes_style, no_style) = if confirmation.yes {
                (theme.selected_item, theme.value)
            } else {
                (theme.value, theme.selected_item)
            };
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(vec![
                        ratatui::text::Span::styled(format!(" {yes} "), yes_style),
                        ratatui::text::Span::raw("    "),
                        ratatui::text::Span::styled(format!(" {no} "), no_style),
                    ]),
                    key_hint(get_text("event_runtime_confirm_keys")),
                ])
                .style(theme.background),
                choices,
            );
        }
    }
}

async fn queue_confirmed(event: BoardEvent, board: &Arc<Mutex<IcyBoard>>, bbs: &Arc<Mutex<BBS>>) -> Result<(), String> {
    // Revalidate the record shown in the dialog, not a stale row index. Do not
    // hold Board and BBS together; acceptance is by stable ID and the scheduler
    // takes the execution snapshot when it consumes the request.
    let unchanged = {
        let board = board.try_lock().map_err(|_| get_text("event_runtime_unavailable"))?;
        board.events.iter().find(|current| current.id == event.id) == Some(&event)
    };
    if !unchanged {
        return Err(get_text("event_runtime_changed"));
    }
    event.validate()?;
    let mut bbs = bbs.try_lock().map_err(|_| get_text("event_runtime_unavailable"))?;
    if bbs.admissions_closed() || bbs.event_restart_requested || bbs.event_scheduler_error.is_some() {
        return Err(get_text("event_runtime_unavailable"));
    }
    bbs.request_event_run(event.id)
}

pub(crate) async fn run<B: Backend>(terminal: &mut Terminal<B>, board: &Arc<Mutex<IcyBoard>>, bbs: &Arc<Mutex<BBS>>, full_screen: bool) -> Res<()>
where
    B::Error: Send + Sync + 'static,
{
    let mut screen = EventScreen::default();
    let mut refresh_requested = true;
    let mut refreshed = std::time::Instant::now();
    let mut log_job: Option<(String, tokio::task::JoinHandle<Result<PathBuf, String>>)> = None;
    loop {
        let previous_error = screen.runtime.request_error.clone();
        screen.runtime.refresh(bbs);
        if screen.runtime.request_error.is_some() && screen.runtime.request_error != previous_error {
            screen.notice = screen.runtime.request_error.clone();
        }
        // Return to call-wait for the owner handshake, including failure without status.
        if screen.runtime.offline || screen.runtime.restart {
            return Ok(());
        }
        if screen.history_job.as_ref().is_some_and(|job| job.is_finished()) {
            screen.finish_refresh().await;
        }
        if log_job.as_ref().is_some_and(|(_, job)| job.is_finished()) {
            let (key, job) = log_job.take().unwrap();
            let result = job.await;
            // A slow filesystem must not open output for a run the operator has
            // since left. This also preserves the selected run on viewer return.
            if screen.selected_execution().is_some_and(|entry| entry.key == key) && screen.confirmation.is_none() {
                match result {
                    Ok(Ok(path)) => {
                        screen.runtime.refresh(bbs);
                        if screen.runtime.offline || screen.runtime.restart {
                            return Ok(());
                        }
                        screen.execution_key = Some(key);
                        crate::log_screen::run_file(terminal, board, bbs, full_screen, path).await?;
                        // Recheck the owner handshake before doing any more work.
                        continue;
                    }
                    Ok(Err(error)) => screen.notice = Some(error),
                    Err(_) => screen.notice = Some(get_text("event_runtime_log_invalid")),
                }
            }
        }
        // At most one outstanding history read/path check each. Slow IO never
        // prevents redraw, keys, or return to the call-wait owner handshake.
        if (refresh_requested || refreshed.elapsed() >= Duration::from_secs(5)) && screen.confirmation.is_none() && screen.start_refresh(board) {
            refresh_requested = false;
            refreshed = std::time::Instant::now();
        }
        terminal.draw(|frame| screen.ui(frame, full_screen))?;
        // The shared crossterm reader is never handed out for a zero timeout,
        // so polling with one would drop every key, Esc included.
        if event::poll(POLL_INTERVAL)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match screen.handle_key(key.code) {
                Action::Exit => return Ok(()),
                Action::Refresh => {
                    refresh_requested = true;
                }
                Action::OpenLog(stored) => {
                    if log_job.is_none()
                        && let Some(entry) = screen.selected_execution()
                    {
                        let key = entry.key.clone();
                        let root = screen.root.clone();
                        log_job = Some((key, tokio::task::spawn_blocking(move || checked_log_path(&root, &stored))));
                    }
                }
                Action::Queue(event) => {
                    screen.notice = Some(match queue_confirmed(event, board, bbs).await {
                        Ok(()) => get_text("event_runtime_queued"),
                        Err(error) => error,
                    });
                    screen.runtime.refresh(bbs);
                }
                Action::Nothing => {}
            }
        }
        tokio::task::yield_now().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use icy_board_engine::datetime::{IcbDoW, IcbTime};
    use ratatui::backend::TestBackend;

    fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(year, month, day, hour, minute, second).earliest().unwrap()
    }

    fn entry(event: &BoardEvent, key: &str) -> EventHistoryEntry {
        let time = at(2024, 6, 3, 3, 0, 0).with_timezone(&Utc);
        EventHistoryEntry {
            key: key.into(),
            event_id: event.id.clone(),
            description: event.description.clone(),
            scheduled_for: time - chrono::Duration::hours(2),
            start: Some(time),
            finish: Some(time + chrono::Duration::seconds(65)),
            result: EventResult::Success,
            exit_code: Some(0),
            log_file: Some(format!("event_logs/{key}.log")),
            manual: true,
            execution: event.execution,
            detail: None,
        }
    }

    fn screen() -> EventScreen {
        let mut screen = EventScreen::default();
        screen.events.push(BoardEvent {
            description: "Nightly event".into(),
            enabled: false,
            command: "echo test".into(),
            ..BoardEvent::default()
        });
        screen.table.select(Some(0));
        screen
    }

    #[test]
    fn every_key_the_footer_promises_is_handled() {
        let mut screen = screen();
        screen.events.push(BoardEvent {
            description: "Second event".into(),
            ..BoardEvent::default()
        });
        assert!(matches!(screen.handle_key(KeyCode::Esc), Action::Exit));
        assert!(matches!(screen.handle_key(KeyCode::Char('r')), Action::Refresh));
        assert!(matches!(screen.handle_key(KeyCode::F(5)), Action::Refresh));
        screen.handle_key(KeyCode::Down);
        assert_eq!(screen.table.selected(), Some(1));
        screen.handle_key(KeyCode::Down);
        assert_eq!(screen.table.selected(), Some(1), "the last row keeps the selection");
        screen.handle_key(KeyCode::Up);
        assert_eq!(screen.table.selected(), Some(0));
        screen.handle_key(KeyCode::End);
        assert_eq!(screen.table.selected(), Some(1));
        screen.handle_key(KeyCode::Home);
        assert_eq!(screen.table.selected(), Some(0));

        // Esc belongs to the dialog while it is open, and only then.
        assert!(matches!(screen.handle_key(KeyCode::Enter), Action::Nothing));
        assert!(screen.confirmation.is_some());
        assert!(matches!(screen.handle_key(KeyCode::Esc), Action::Nothing));
        assert!(screen.confirmation.is_none());
        assert!(matches!(screen.handle_key(KeyCode::Esc), Action::Exit));

        screen.handle_key(KeyCode::Enter);
        screen.handle_key(KeyCode::Right);
        assert!(matches!(screen.handle_key(KeyCode::Enter), Action::Queue(event) if event == screen.events[0]));
    }

    #[test]
    fn rows_name_events_that_carry_no_description() {
        let mut screen = screen();
        screen.events[0].description = "  ".into();
        screen.events[0].command = "nightly.sh".into();
        assert_eq!(screen.label(0), "nightly.sh");
        screen.events[0].command = String::new();
        assert_eq!(screen.label(0), format!("{} #1", get_text("event_runtime_unnamed")));
        screen.events[0].description = "Nightly".into();
        assert_eq!(screen.label(0), "Nightly");
    }

    #[test]
    fn confirmation_defaults_to_no_and_must_be_explicit() {
        let mut screen = screen();
        assert!(screen.confirm_key(KeyCode::Enter).is_none());
        assert!(!screen.confirmation.as_ref().unwrap().yes);
        assert!(screen.confirm_key(KeyCode::Enter).is_none());
        screen.confirm_key(KeyCode::Enter);
        screen.confirm_key(KeyCode::Right);
        assert_eq!(screen.confirm_key(KeyCode::Enter).unwrap(), screen.events[0]);
        screen.confirm_key(KeyCode::Enter);
        screen.confirm_key(KeyCode::Esc);
        assert!(screen.confirmation.is_none());
    }

    #[test]
    fn pending_and_failed_scheduler_cannot_open_confirmation() {
        let mut screen = screen();
        screen.runtime.pending.insert(screen.events[0].id.clone());
        screen.confirm_key(KeyCode::Enter);
        assert!(screen.confirmation.is_none());
        screen.runtime.pending.clear();
        screen.runtime.failure = Some("journal unavailable".into());
        screen.confirm_key(KeyCode::Enter);
        assert!(screen.confirmation.is_none());
    }

    #[test]
    fn schedule_reasons_and_countdown_follow_weekdays_and_year_rollover() {
        let mut screen = screen();
        screen.schedule_enabled = true;
        let mut event = screen.events[0].clone();
        event.enabled = true;
        event.time = IcbTime::parse("00:00:00");
        let now = at(2024, 12, 31, 23, 59, 30);
        let detail = screen.schedule_detail(&event, now);
        assert!(detail.contains("2025-01-01 00:00:00"));
        assert!(detail.contains("+00:00:30"));
        assert!(!detail.contains(&get_text("event_runtime_schedule_off")));
        event.days = IcbDoW::from("NYNNNNN".to_string()); // Monday only.
        let now = at(2024, 6, 4, 0, 0, 0); // Tuesday.
        let detail = screen.schedule_detail(&event, now);
        assert!(detail.contains("2024-06-10 00:00:00"));
        assert!(detail.contains("+6d 00:00:00"));
        assert!(detail.contains(&get_text("event_runtime_wrong_day")));
        event.days = IcbDoW::from("NNNNNNN".to_string());
        assert!(screen.schedule_detail(&event, now).contains(&get_text("event_runtime_no_days")));
        screen.schedule_enabled = false;
        event.enabled = false;
        let detail = screen.schedule_detail(&event, now);
        assert!(detail.contains(&get_text("event_runtime_schedule_off")));
        assert!(detail.contains(&get_text("event_runtime_event_off")));
        event.interval_minutes = Some(0);
        assert!(screen.schedule_detail(&event, now).contains(&get_text("event_runtime_invalid")));
    }

    #[test]
    fn daily_window_end_is_inclusive_and_no_calendar_range_is_invented() {
        let mut screen = screen();
        screen.schedule_enabled = true;
        let mut event = screen.events[0].clone();
        event.enabled = true;
        event.time = IcbTime::parse("03:00:00");
        event.interval_minutes = Some(30);
        event.end_time = Some(IcbTime::parse("04:00:00"));
        let detail = screen.schedule_detail(&event, at(2024, 6, 3, 3, 59, 59));
        assert!(detail.contains("2024-06-03 04:00:00"));
        assert!(detail.contains("+00:00:01"));
        assert!(
            !screen
                .schedule_detail(&event, at(2024, 6, 3, 4, 0, 0))
                .contains(&get_text("event_runtime_window_ended"))
        );
        let detail = screen.schedule_detail(&event, at(2024, 6, 3, 4, 0, 1));
        assert!(detail.contains(&get_text("event_runtime_window_ended")));
        assert!(detail.contains("2024-06-04 03:00:00"));
        // Strictly future candidate, not a retrospective run invented on startup.
        let detail = screen.schedule_detail(&event, at(2024, 2, 29, 4, 0, 1));
        assert!(detail.contains("2024-03-01 03:00:00"));
        event.end_time = Some(IcbTime::parse("02:00:00"));
        assert!(
            screen
                .schedule_detail(&event, at(2024, 6, 3, 3, 0, 0))
                .contains(&get_text("event_runtime_invalid"))
        );
    }

    #[test]
    fn observed_due_window_and_pending_are_distinct_from_future_candidates() {
        let mut screen = screen();
        let event = screen.events[0].clone();
        let due = at(2024, 6, 3, 3, 0, 0);
        let now = at(2024, 6, 4, 3, 0, 0);
        screen.runtime.window = Some(EventWindow {
            event: event.clone(),
            run_at: due,
            suspend_at: due,
            uploads_stop_at: None,
        });
        screen.runtime.pending.insert(event.id.clone());
        let detail = screen.schedule_detail(&event, now);
        assert!(
            detail.contains(&get_text("event_runtime_window_waiting")),
            "no end allows retained work beyond midnight"
        );
        assert!(detail.contains(&get_text("event_runtime_queued_active")));
        screen.runtime.window.as_mut().unwrap().event.end_time = Some(IcbTime::parse("04:00:00"));
        assert!(!screen.schedule_detail(&event, now).contains(&get_text("event_runtime_window_waiting")));
        screen.runtime.window.as_mut().unwrap().event.id = "other-event".into();
        assert!(!screen.schedule_detail(&event, now).contains(&get_text("event_runtime_window_waiting")));
        screen.runtime.pending.clear();
        screen.confirm_key(KeyCode::Enter);
        assert!(
            screen.confirmation.is_some(),
            "disabled/global-off schedule still permits explicit manual confirmation"
        );
        assert!(!screen.confirmation.as_ref().unwrap().yes);
    }

    #[test]
    fn recorded_duration_never_uses_scheduled_time_or_recovery_as_process_end() {
        let mut run = entry(&screen().events[0], "run");
        let now = run.start.unwrap() + chrono::Duration::seconds(90);
        assert_eq!(duration_text(&run, now), format!("{}: 00:01:05", get_text("event_runtime_duration")));
        run.result = EventResult::NonzeroExit;
        run.exit_code = Some(17);
        assert!(duration_text(&run, now).ends_with("00:01:05"));
        run.result = EventResult::Interrupted;
        assert!(duration_text(&run, now).ends_with('—'));
        run.result = EventResult::WaitError;
        assert!(duration_text(&run, now).ends_with('—'));
        run.result = EventResult::Pending;
        run.finish = None;
        assert_eq!(duration_text(&run, now), format!("{}: 00:01:30", get_text("event_runtime_elapsed")));
        assert!(duration_text(&run, run.start.unwrap() - chrono::Duration::seconds(1)).ends_with('—'));
        run.start = None;
        assert_eq!(duration_text(&run, now), format!("{}: —", get_text("event_runtime_duration")));
        for result in [EventResult::SkippedBusy, EventResult::Expired, EventResult::SpawnError, EventResult::Superseded] {
            run.result = result;
            run.finish = Some(now);
            assert!(duration_text(&run, now).ends_with('—'));
        }
        assert_eq!(elapsed_text(-1), "00:00:00");
        assert_eq!(elapsed_text(90061), "1d 01:01:01");
    }

    #[test]
    fn log_key_uses_selected_history_and_never_substitutes_another_run() {
        let mut screen = screen();
        assert!(matches!(screen.handle_key(KeyCode::Char('L')), Action::Nothing));
        assert_eq!(screen.notice, Some(get_text("event_runtime_log_none")));
        screen.history = vec![entry(&screen.events[0], "older"), entry(&screen.events[0], "latest")];
        assert!(matches!(screen.handle_key(KeyCode::Char('l')), Action::OpenLog(path) if path == "event_logs/latest.log"));
        screen.handle_key(KeyCode::PageUp);
        screen.handle_key(KeyCode::PageUp);
        assert_eq!(screen.selected_execution().unwrap().key, "older");
        assert!(matches!(screen.handle_key(KeyCode::Char('l')), Action::OpenLog(path) if path == "event_logs/older.log"));
        screen.history.push(entry(&screen.events[0], "newest"));
        assert_eq!(screen.selected_execution().unwrap().key, "older");
        screen.handle_key(KeyCode::PageDown);
        assert_eq!(screen.selected_execution().unwrap().key, "latest");
        screen.handle_key(KeyCode::PageDown);
        // Current Online output is still selected by its durable history entry.
        let newest = screen.history.last_mut().unwrap();
        newest.execution = EventExecution::Online;
        newest.result = EventResult::Pending;
        newest.finish = None;
        assert!(matches!(screen.handle_key(KeyCode::Char('l')), Action::OpenLog(path) if path == "event_logs/newest.log"));
        screen.history.last_mut().unwrap().log_file = None;
        assert!(matches!(screen.handle_key(KeyCode::Char('l')), Action::Nothing));
        screen.confirm_key(KeyCode::Enter);
        assert!(matches!(screen.handle_key(KeyCode::Char('l')), Action::Nothing), "dialog keys cannot open logs");
        screen.handle_key(KeyCode::Esc);
        screen.events.push(BoardEvent::default());
        screen.handle_key(KeyCode::Down);
        assert!(screen.execution_key.is_none());
        assert!(screen.selected_execution().is_none());
        screen.handle_key(KeyCode::Up);
        assert_eq!(screen.selected_execution().unwrap().key, "newest");
    }

    #[test]
    fn log_paths_accept_only_existing_regular_scheduler_logs() {
        let dir = tempfile::tempdir().unwrap();
        let logs = dir.path().join(LOG_DIRECTORY);
        std::fs::create_dir(&logs).unwrap();
        let log = logs.join("valid-123.log");
        std::fs::write(&log, "output").unwrap();
        assert_eq!(checked_log_path(dir.path(), "event_logs/valid-123.log").unwrap(), log.canonicalize().unwrap());
        for path in [
            "",
            "../secret.log",
            "event_logs/../secret.log",
            "event_logs/../../secret.log",
            "event_logs//valid-123.log",
            "event_logs/./valid-123.log",
            "event_logs/sub/file.log",
            "event_logs/bad..log",
            "event_logs/evil\u{001b}.log",
            "event_logs\\valid-123.log",
            "event_logs/no.txt",
            "event_logs/missing.log",
        ] {
            assert!(checked_log_path(dir.path(), path).is_err(), "accepted {path:?}");
        }
        assert!(
            checked_log_path(dir.path(), log.to_str().unwrap()).is_err(),
            "even in-root absolute paths are not journal format"
        );
        let outside = tempfile::NamedTempFile::new().unwrap();
        assert!(checked_log_path(dir.path(), outside.path().to_str().unwrap()).is_err());
        std::fs::create_dir(logs.join("directory.log")).unwrap();
        assert!(checked_log_path(dir.path(), "event_logs/directory.log").is_err());
        assert_eq!(std::fs::read_to_string(&log).unwrap(), "output");
    }

    #[cfg(unix)]
    #[test]
    fn log_paths_reject_file_and_directory_symlink_escapes() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret.log"), "secret").unwrap();
        symlink(outside.path(), root.path().join(LOG_DIRECTORY)).unwrap();
        assert!(checked_log_path(root.path(), "event_logs/secret.log").is_err());
        std::fs::remove_file(root.path().join(LOG_DIRECTORY)).unwrap();
        std::fs::create_dir(root.path().join(LOG_DIRECTORY)).unwrap();
        symlink(outside.path().join("secret.log"), root.path().join("event_logs/escape.log")).unwrap();
        assert!(checked_log_path(root.path(), "event_logs/escape.log").is_err());
        std::fs::write(root.path().join("event_logs/real.log"), "safe").unwrap();
        symlink("real.log", root.path().join("event_logs/alias.log")).unwrap();
        assert!(checked_log_path(root.path(), "event_logs/alias.log").is_err());
    }

    #[tokio::test]
    async fn board_contention_and_outstanding_reads_do_not_block_the_screen() {
        let board = Arc::new(Mutex::new(IcyBoard::new()));
        let mut screen = screen();
        let guard = board.lock().await;
        assert!(!screen.start_refresh(&board));
        drop(guard);
        screen.history_job = Some(tokio::spawn(std::future::pending()));
        assert!(!screen.start_refresh(&board));
        assert!(matches!(screen.handle_key(KeyCode::Esc), Action::Exit));
        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        bbs.lock().await.event_restart_requested = true;
        screen.runtime.refresh(&bbs);
        assert!(screen.runtime.restart);
        screen.history_job.take().unwrap().abort();
    }

    #[test]
    fn details_sanitize_controls_and_scroll_to_the_end_at_80x25_and_narrow_sizes() {
        for (width, height) in [(80, 25), (40, 18), (20, 10)] {
            let mut screen = screen();
            screen.schedule_enabled = true;
            screen.events[0].enabled = true;
            screen.events[0].description = "Safe\x1b[31m name\u{202e}".into();
            screen.events[0].command = "echo \x1b]0;BADTITLE\x07safe".into();
            let mut run = entry(&screen.events[0], "run");
            run.exit_code = Some(17);
            run.description = "old\x1b[2J name".into();
            run.detail = Some(format!("\x1b]0;hidden\x07{}\nLAST-DETAIL", "long line ".repeat(30)));
            screen.history.push(run);
            let now = at(2024, 6, 3, 3, 2, 0);
            let detail = screen.detail(now);
            assert!(!detail.contains('\x1b'));
            assert!(!detail.contains('\u{202e}'));
            assert!(!detail.contains("BADTITLE"));
            assert!(detail.contains("00:01:05"));
            assert!(detail.contains(&format!("{}: 17", get_text("event_runtime_exit_code"))));
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            let mut seen = String::new();
            for _ in 0..300 {
                terminal.draw(|frame| screen.ui_at(frame, false, now)).unwrap();
                seen.extend(terminal.backend().buffer().content().iter().map(|cell| cell.symbol()));
                screen.handle_key(KeyCode::Right);
            }
            assert!(seen.contains("LAST-DETAIL"), "detail must remain reachable at {width}x{height}");
            if width == 80 {
                assert!(seen.contains("00:01:05"));
                assert!(seen.contains("17"));
            }
            screen.handle_key(KeyCode::Left);
            screen.confirm_key(KeyCode::Enter);
            terminal.draw(|frame| screen.ui_at(frame, false, now)).unwrap();
        }
    }

    #[tokio::test]
    async fn queue_revalidates_and_refuses_duplicate_active_failed_and_operator_gate() {
        let event = screen().events.remove(0);
        let mut board = IcyBoard::new();
        board.events.push(event.clone());
        let board = Arc::new(Mutex::new(board));
        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        queue_confirmed(event.clone(), &board, &bbs).await.unwrap();
        assert!(queue_confirmed(event.clone(), &board, &bbs).await.is_err());
        {
            let mut bbs = bbs.lock().await;
            assert!(runtime_busy(&bbs));
            bbs.event_run_requests.clear();
            bbs.event_active_ids.insert(event.id.clone());
        }
        assert!(queue_confirmed(event.clone(), &board, &bbs).await.is_err());
        {
            let mut bbs = bbs.lock().await;
            bbs.event_active_ids.clear();
            bbs.event_scheduler_error = Some("failure".into());
        }
        assert!(queue_confirmed(event.clone(), &board, &bbs).await.is_err());
        {
            let mut bbs = bbs.lock().await;
            bbs.event_scheduler_error = None;
            bbs.operator_maintenance = true;
        }
        assert!(queue_confirmed(event.clone(), &board, &bbs).await.is_err());
        bbs.lock().await.operator_maintenance = false;
        board.lock().await.events[0].command = "changed".into();
        assert!(queue_confirmed(event, &board, &bbs).await.is_err());
    }

    #[test]
    fn picker_and_confirmation_fit_80_by_25() {
        let mut screen = screen();
        screen.events[0].command = "nightly.sh".into();
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| screen.ui(frame, false)).unwrap();
        let text = terminal.backend().buffer().content().iter().map(|cell| cell.symbol()).collect::<String>();
        assert!(text.contains("Nightly event"));
        assert!(text.contains("nightly.sh"), "the selected command belongs on screen");
        assert!(text.contains(&get_text("event_runtime_disabled")));
        assert!(text.contains(&get_text("event_runtime_next_column")));
        assert!(text.contains(&get_text("event_runtime_picker_keys")));
        screen.confirm_key(KeyCode::Enter);
        terminal.draw(|frame| screen.ui(frame, false)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        let text = buffer.content().iter().map(|cell| cell.symbol()).collect::<String>();
        assert!(text.contains(&get_text("event_runtime_confirm_title")));
        assert!(text.contains(&get_text("event_runtime_no")));
        assert!(text.contains(&get_text("event_runtime_confirm_keys")));
        // Cancel is preselected and has to look like it.
        let theme = get_tui_theme();
        assert!(buffer.content().iter().any(|cell| cell.bg == theme.selected_item.bg.unwrap()));
    }

    #[tokio::test]
    async fn history_is_read_only_and_cached_until_refresh() {
        let directory = tempfile::tempdir().unwrap();
        let event = screen().events.remove(0);
        let mut board = IcyBoard::new();
        board.root_path = directory.path().to_path_buf();
        board.events.push(event.clone());
        let board = Arc::new(Mutex::new(board));
        let mut journal = EventHistory::open(directory.path(), chrono::Utc::now()).unwrap();
        let entry = journal.claim(&event, chrono::Utc::now(), true).unwrap().unwrap();
        let mut screen = EventScreen::default();
        screen.refresh(&board).await;
        assert!(screen.history_error.is_none(), "reader must not acquire the writer's lease");
        assert_eq!(screen.last_result(&event.id).unwrap().result, EventResult::Pending);
        journal.finish(&entry.key, chrono::Utc::now(), EventResult::SkippedBusy, None, None).unwrap();
        assert_eq!(screen.last_result(&event.id).unwrap().result, EventResult::Pending);
        screen.refresh(&board).await;
        assert_eq!(screen.last_result(&event.id).unwrap().result, EventResult::SkippedBusy);
        screen.execution_key = Some(entry.key.clone());
        let second = journal.claim(&event, chrono::Utc::now(), true).unwrap().unwrap();
        let log = journal.start(&second.key, chrono::Utc::now()).unwrap();
        std::fs::create_dir_all(log.parent().unwrap()).unwrap();
        std::fs::write(&log, "live output").unwrap();
        screen.refresh(&board).await;
        assert_eq!(screen.selected_execution().unwrap().key, entry.key, "refresh preserves the selected execution");
        screen.handle_key(KeyCode::PageDown);
        let Action::OpenLog(stored) = screen.handle_key(KeyCode::Char('l')) else {
            panic!("live history output missing")
        };
        assert_eq!(checked_log_path(directory.path(), &stored).unwrap(), log);
        assert_eq!(
            journal.entries().last().unwrap().result,
            EventResult::Pending,
            "viewing must not recover/finish live claims"
        );
    }
}
