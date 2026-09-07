//! Operator-only event selection. The scheduler alone executes commands and owns
//! the journal lease; this screen reads a cached, read-only history snapshot.
use std::{collections::HashSet, sync::Arc, time::Duration};

use chrono::Local;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use icy_board_engine::icy_board::{
    IcyBoard,
    bbs::{BBS, EventMaintenanceStatus, OnlineEventStatus},
    events::{
        BoardEvent, EventExecution, EventMode,
        event_history::{EventHistory, EventHistoryEntry, EventResult},
    },
};
use icy_board_tui::{
    app::get_screen_size,
    get_text,
    theme::{DOS_BLUE, DOS_LIGHT_GRAY, DOS_RED, DOS_WHITE},
};
use ratatui::{
    Frame, Terminal,
    backend::Backend,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Clear, Paragraph, Row, Table, TableState, Wrap},
};
use tokio::sync::Mutex;

use crate::Res;

#[derive(Clone, Default)]
pub(crate) struct RuntimeStatus {
    pub offline: bool,
    pub restart: bool,
    pub maintenance: Option<EventMaintenanceStatus>,
    pub online: Option<OnlineEventStatus>,
    pub failure: Option<String>,
    pub request_error: Option<String>,
    pub pending: HashSet<String>,
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
        }
    }

    pub fn online_text(&self) -> Option<String> {
        self.online.as_ref().map(|status| {
            format!(
                "ONLINE: {} | {}s{}",
                status.description,
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
    })
}

struct Confirmation {
    event: BoardEvent,
    yes: bool,
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
}

impl EventScreen {
    async fn refresh(&mut self, board: &Arc<Mutex<IcyBoard>>) {
        let selected_id = self.selected().map(|event| event.id.clone());
        let root = {
            let board = board.lock().await;
            self.events = board.events.iter().cloned().collect();
            board.root_path.clone()
        };
        let selected = selected_id.and_then(|id| self.events.iter().position(|event| event.id == id)).unwrap_or(0);
        self.table.select((!self.events.is_empty()).then_some(selected));
        self.confirmation = None;
        // No EventHistory::open: opening would take/recover the scheduler lease.
        match tokio::task::spawn_blocking(move || EventHistory::read_entries(&root)).await {
            Ok(Ok(entries)) => {
                self.history = entries;
                self.history_error = None;
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

    fn selected(&self) -> Option<&BoardEvent> {
        self.table.selected().and_then(|index| self.events.get(index))
    }

    fn last_result(&self, id: &str) -> Option<&EventHistoryEntry> {
        self.history.iter().rev().find(|entry| entry.event_id == id)
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

    fn ui(&mut self, frame: &mut Frame, full_screen: bool) {
        let area = get_screen_size(frame, full_screen);
        frame.render_widget(Clear, area);
        let block = Block::bordered()
            .title(get_text("event_runtime_picker_title"))
            .title_bottom(get_text("event_runtime_picker_keys"))
            .style(Style::new().fg(DOS_WHITE).bg(DOS_BLUE));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let [status, list, history, help] =
            Layout::vertical([Constraint::Length(2), Constraint::Min(3), Constraint::Length(6), Constraint::Length(6)]).areas(inner);
        let status_text = [
            self.runtime.online_text(),
            self.runtime
                .failure
                .clone()
                .or_else(|| self.notice.clone())
                .or_else(|| self.runtime.request_error.clone()),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n");
        frame.render_widget(Paragraph::new(status_text), status);
        let rows = self
            .events
            .iter()
            .map(|event| {
                let enabled = get_text(if event.enabled { "event_runtime_enabled" } else { "event_runtime_disabled" });
                let result = if self.runtime.pending.contains(&event.id) {
                    get_text("event_runtime_queued_active")
                } else {
                    self.last_result(&event.id)
                        .map(|entry| result_text(&entry.result))
                        .unwrap_or_else(|| "—".into())
                };
                Row::new(vec![
                    event.description.clone(),
                    enabled,
                    format!("{}/{}", execution_text(event.execution), mode_text(event.mode)),
                    result,
                ])
            })
            .collect::<Vec<_>>();
        let table = Table::new(
            rows,
            [Constraint::Min(12), Constraint::Length(7), Constraint::Length(19), Constraint::Length(18)],
        )
        .header(Row::new(vec![
            get_text("event_runtime_event"),
            get_text("event_runtime_enabled_column"),
            get_text("event_runtime_mode_column"),
            get_text("event_runtime_result_column"),
        ]))
        .row_highlight_style(Style::new().fg(DOS_BLUE).bg(DOS_LIGHT_GRAY));
        frame.render_stateful_widget(table, list, &mut self.table);
        let detail = if let Some(error) = &self.history_error {
            error.clone()
        } else if let Some(entry) = self.selected().and_then(|event| self.last_result(&event.id)) {
            format!(
                "{} | {} | {} | {}\n{}: {}\n{}",
                entry.scheduled_for.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S"),
                result_text(&entry.result),
                get_text(if entry.manual { "event_runtime_manual" } else { "event_runtime_scheduled" }),
                entry.exit_code.map(|code| format!("exit={code}")).unwrap_or_default(),
                get_text("event_runtime_log"),
                entry.log_file.as_deref().unwrap_or("—"),
                entry.detail.as_deref().unwrap_or("")
            )
        } else {
            get_text("event_runtime_no_history")
        };
        frame.render_widget(
            Paragraph::new(detail)
                .wrap(Wrap { trim: true })
                .block(Block::bordered().title(get_text("event_runtime_history_title"))),
            history,
        );
        frame.render_widget(Paragraph::new(get_text("event_runtime_manual_help")).wrap(Wrap { trim: true }), help);
        if self.events.is_empty() {
            frame.render_widget(Paragraph::new(get_text("event_runtime_empty")), list);
        }
        if let Some(confirmation) = &self.confirmation {
            let popup = Rect::new(
                area.x + area.width.saturating_sub(74) / 2,
                area.y + area.height.saturating_sub(19) / 2,
                area.width.min(74),
                area.height.min(19),
            );
            frame.render_widget(Clear, popup);
            let block = Block::bordered()
                .title(get_text("event_runtime_confirm_title"))
                .style(Style::new().fg(DOS_WHITE).bg(DOS_RED));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);
            let [details, help, choices] = Layout::vertical([Constraint::Length(4), Constraint::Min(1), Constraint::Length(2)]).areas(inner);
            frame.render_widget(
                Paragraph::new(format!(
                    "{}\n{} / {} / {}\n{}",
                    confirmation.event.description,
                    execution_text(confirmation.event.execution),
                    mode_text(confirmation.event.mode),
                    get_text(if confirmation.event.enabled {
                        "event_runtime_enabled"
                    } else {
                        "event_runtime_disabled"
                    }),
                    confirmation.event.command
                ))
                .wrap(Wrap { trim: true }),
                details,
            );
            frame.render_widget(
                Paragraph::new(format!(
                    "{}\n\n{}",
                    get_text("event_runtime_manual_help"),
                    get_text("event_runtime_online_warning")
                ))
                .wrap(Wrap { trim: true }),
                help,
            );
            let yes = get_text("event_runtime_yes");
            let no = get_text("event_runtime_no");
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(if confirmation.yes {
                        format!("[{yes}]    {no}")
                    } else {
                        format!("{yes}    [{no}]")
                    }),
                    Line::from(get_text("event_runtime_confirm_keys")),
                ]),
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
        let board = board.lock().await;
        board.events.iter().find(|current| current.id == event.id) == Some(&event)
    };
    if !unchanged {
        return Err(get_text("event_runtime_changed"));
    }
    event.validate()?;
    let mut bbs = bbs.lock().await;
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
    screen.refresh(board).await;
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
        terminal.draw(|frame| screen.ui(frame, full_screen))?;
        if event::poll(Duration::ZERO)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            if screen.confirmation.is_some() || key.code == KeyCode::Enter {
                if let Some(event) = screen.confirm_key(key.code) {
                    screen.notice = Some(match queue_confirmed(event, board, bbs).await {
                        Ok(()) => get_text("event_runtime_queued"),
                        Err(error) => error,
                    });
                }
            } else {
                match key.code {
                    KeyCode::Esc | KeyCode::F(6) => return Ok(()),
                    KeyCode::Char('r' | 'R') | KeyCode::F(5) => screen.refresh(board).await,
                    KeyCode::Down => {
                        if !screen.events.is_empty() {
                            screen
                                .table
                                .select(Some((screen.table.selected().unwrap_or(0) + 1).min(screen.events.len() - 1)));
                        }
                    }
                    KeyCode::Up => screen.table.select(screen.table.selected().map(|index| index.saturating_sub(1))),
                    KeyCode::Home => screen.table.select((!screen.events.is_empty()).then_some(0)),
                    KeyCode::End => screen.table.select(screen.events.len().checked_sub(1)),
                    _ => {}
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

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
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| screen.ui(frame, false)).unwrap();
        let text = terminal.backend().buffer().content().iter().map(|cell| cell.symbol()).collect::<String>();
        assert!(text.contains("Nightly event"));
        assert!(text.contains(&get_text("event_runtime_disabled")));
        assert!(text.contains(&get_text("event_runtime_picker_keys")));
        screen.confirm_key(KeyCode::Enter);
        terminal.draw(|frame| screen.ui(frame, false)).unwrap();
        let text = terminal.backend().buffer().content().iter().map(|cell| cell.symbol()).collect::<String>();
        assert!(text.contains(&get_text("event_runtime_confirm_title")));
        assert!(text.contains(&format!("[{}]", get_text("event_runtime_no"))));
        assert!(text.contains(&get_text("event_runtime_confirm_keys")));
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
    }
}
