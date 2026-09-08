//! Read-only CWS diagnostics. Neither listener snapshots nor this screen own
//! admission, maintenance acknowledgements, or the listener restart handshake.
use std::{
    io,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use icy_board_engine::icy_board::{
    IcyBoard,
    bbs::{BBS, EventMaintenancePhase, ListenerStatus},
    state::NodeState,
};
use icy_board_tui::{app::get_screen_size, chrome::key_hint, get_text, theme::get_tui_theme};
use ratatui::{
    Frame, Terminal,
    backend::Backend,
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Wrap},
};
use tokio::{sync::Mutex, task::JoinHandle};

use crate::{Res, event_screen::RuntimeStatus};

const REFRESH_INTERVAL: Duration = Duration::from_secs(1);
// Nonzero: a zero-timeout poll can lose keys in the shared crossterm reader.
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const GIB: u64 = 1024 * 1024 * 1024;
const STALE_AFTER: Duration = Duration::from_secs(2);

/// Local observation metadata, independent for each lock/IO source. Retaining
/// a cached value never advances its last-success timestamp.
#[derive(Default)]
struct Freshness {
    last_success: Option<(Instant, DateTime<Utc>)>,
    issue: Option<&'static str>,
    pending_since: Option<Instant>,
}

impl Freshness {
    fn success(&mut self) {
        self.success_at(Instant::now(), Utc::now());
    }

    fn success_at(&mut self, instant: Instant, utc: DateTime<Utc>) {
        self.last_success = Some((instant, utc));
        self.issue = None;
        self.pending_since = None;
    }

    fn stale(&self) -> bool {
        self.issue.is_some()
            || self.last_success.is_none_or(|(instant, _)| instant.elapsed() >= STALE_AFTER)
            || self.pending_since.is_some_and(|instant| instant.elapsed() >= STALE_AFTER)
    }

    fn text(&self) -> String {
        let mut text = get_text(if self.stale() { "system_status_stale" } else { "system_status_fresh" });
        if let Some(issue) = self.issue.or(self.pending_since.map(|_| "system_status_pending")) {
            text.push_str(&format!(" ({})", get_text(issue)));
        }
        match self.last_success {
            Some((instant, utc)) => text.push_str(&format!(" | {} | {}s", utc.format("%Y-%m-%d %H:%M:%S UTC"), instant.elapsed().as_secs())),
            None => text.push_str(&format!(" | {}", get_text("system_status_never"))),
        }
        text
    }
}

#[derive(Clone, Copy)]
struct DiskSpace {
    available: u64,
    total: u64,
}

impl DiskSpace {
    fn percent(self) -> Option<f64> {
        (self.total > 0 && self.available <= self.total).then(|| self.available as f64 * 100.0 / self.total as f64)
    }

    /// Display-only warning: strictly below 1 GiB OR below 10% available.
    /// No admission, event, upload, or other runtime policy is changed. Integer
    /// comparison avoids rounding a near-boundary percentage into/out of alarm.
    fn low(self) -> bool {
        self.available < GIB || (self.total > 0 && u128::from(self.available) * 10 < u128::from(self.total))
    }

    fn text(self) -> String {
        let percent = self
            .percent()
            .map_or_else(|| get_text("system_status_unavailable"), |percent| format!("{percent:.1}%"));
        let total = if self.total == 0 {
            get_text("system_status_unavailable")
        } else {
            format!("{:.2} GiB", self.total as f64 / GIB as f64)
        };
        let mut text = format!("{:.2} GiB / {total} ({percent})", self.available as f64 / GIB as f64);
        if self.low() {
            text.push_str(&format!(" — {}", get_text("system_status_disk_low")));
        }
        text
    }
}

struct DiskSample {
    space: DiskSpace,
    instant: Instant,
    utc: DateTime<Utc>,
}

struct Endpoint {
    name: &'static str,
    label: &'static str,
    configured: String,
    enabled: bool,
}

impl Endpoint {
    fn new(name: &'static str, label: &'static str, address: &str, port: u16, enabled: bool) -> Self {
        let address = if address.trim().is_empty() { "0.0.0.0" } else { address };
        let configured = if address.contains(':') && !address.starts_with('[') {
            format!("[{address}]:{port}")
        } else {
            format!("{address}:{port}")
        };
        Self {
            name,
            label,
            configured,
            enabled,
        }
    }
}

/// Do not render terminal controls or credential-bearing diagnostic messages.
/// No admin token/configuration secret is ever copied into this screen's model.
fn display_text(value: &str) -> String {
    let clean = crate::log_screen::sanitize(value);
    let lower = format!("{} {}", value.to_ascii_lowercase(), clean.to_ascii_lowercase());
    if ["token", "authorization", "bearer ", "password", "secret"]
        .iter()
        .any(|word| lower.contains(word))
    {
        return get_text("system_status_redacted");
    }
    clean.chars().take(2048).collect()
}

fn maintenance_text(runtime: &RuntimeStatus) -> String {
    let Some(status) = &runtime.maintenance else {
        return get_text(if runtime.offline {
            "system_status_admission_closed"
        } else {
            "system_status_none"
        });
    };
    let (key, detail) = match &status.phase {
        EventMaintenancePhase::Waiting(reason) => ("system_status_waiting", Some(display_text(reason))),
        EventMaintenancePhase::Draining(nodes) => ("system_status_draining", Some(nodes.to_string())),
        EventMaintenancePhase::Stopping => ("system_status_stopping", None),
        EventMaintenancePhase::Running => ("system_status_executing", None),
        EventMaintenancePhase::Reloading => ("system_status_reloading", None),
        EventMaintenancePhase::ReloadFailed(error) => ("system_status_reload_failed", Some(display_text(error))),
        EventMaintenancePhase::ListenerFailed(error) => ("system_status_listener_failed", Some(display_text(error))),
        EventMaintenancePhase::Restarting => ("system_status_restarting", None),
    };
    let mut text = format!("{} — {}", display_text(&status.description), get_text(key));
    if let Some(detail) = detail {
        text.push_str(&format!(": {detail}"));
    }
    text
}

struct DiskJob {
    root: PathBuf,
    task: JoinHandle<io::Result<DiskSample>>,
}

impl DiskJob {
    fn start(root: PathBuf) -> Self {
        let path = root.clone();
        Self {
            root,
            task: tokio::task::spawn_blocking(move || {
                let stats = fs4::statvfs(path)?;
                Ok(DiskSample {
                    space: DiskSpace {
                        available: stats.available_space(),
                        total: stats.total_space(),
                    },
                    instant: Instant::now(),
                    utc: Utc::now(),
                })
            }),
        }
    }
}

impl Drop for DiskJob {
    fn drop(&mut self) {
        // Cancels a queued job; an already-running filesystem syscall cannot be
        // interrupted. Never wait for it when returning control to the owner.
        self.task.abort();
    }
}

#[derive(Default)]
struct SystemStatusScreen {
    runtime: RuntimeStatus,
    started_at: Option<Instant>,
    listeners: Option<Vec<ListenerStatus>>,
    endpoints: Vec<Endpoint>,
    root: Option<PathBuf>,
    node_source: Option<Arc<Mutex<Vec<Option<NodeState>>>>>,
    nodes: Option<(usize, usize)>,
    disk: Option<DiskSpace>,
    config_freshness: Freshness,
    runtime_freshness: Freshness,
    nodes_freshness: Freshness,
    disk_freshness: Freshness,
    scroll: u16,
    max_scroll: u16,
    page: u16,
}

impl SystemStatusScreen {
    /// Poll the owner handoff independently of slower config/node/disk refreshes.
    /// Snapshot inline under one lock; do not change the event screen's model.
    fn refresh_runtime(&mut self, bbs: &Arc<Mutex<BBS>>) {
        if let Ok(bbs) = bbs.try_lock() {
            self.runtime.offline = bbs.admissions_closed() || bbs.event_scheduler_error.is_some() || bbs.event_restart_requested;
            self.runtime.restart = bbs.event_restart_requested;
            self.runtime.maintenance = bbs.event_maintenance_status.clone();
            self.runtime.online = bbs.event_online_status.clone();
            self.runtime.failure = bbs.event_scheduler_error.clone();
            self.runtime.request_error = bbs.event_request_error.clone();
            self.started_at = Some(bbs.started_at);
            self.listeners = Some(bbs.runtime_listeners.clone());
            self.node_source = Some(bbs.open_connections.clone());
            self.runtime_freshness.success();
        } else {
            self.runtime_freshness.issue = Some("system_status_busy");
        }
    }

    /// Independent try-locks: never nest board/BBS locks or await a writer. A
    /// busy snapshot retains its last successful value until the next refresh.
    fn refresh(&mut self, board: &Arc<Mutex<IcyBoard>>, bbs: &Arc<Mutex<BBS>>) {
        self.refresh_runtime(bbs);
        if let Ok(board) = board.try_lock() {
            if self.root.as_ref() != Some(&board.root_path) {
                self.disk = None;
                self.disk_freshness = Freshness::default();
            }
            self.root = Some(board.root_path.clone());
            let login = &board.config.login_server;
            let admin = &board.config.board.web_admin;
            self.endpoints = vec![
                Endpoint::new(
                    "Telnet",
                    "system_status_telnet",
                    &login.telnet.address,
                    login.telnet.port,
                    login.telnet.is_enabled,
                ),
                Endpoint::new("SSH", "system_status_ssh", &login.ssh.address, login.ssh.port, login.ssh.is_enabled),
                Endpoint::new(
                    "Secure WebSocket",
                    "system_status_websocket",
                    &login.secure_websocket.address,
                    login.secure_websocket.port,
                    login.secure_websocket.is_enabled,
                ),
                Endpoint::new("Web admin", "system_status_admin", &admin.address, admin.port, admin.enabled),
            ];
            self.config_freshness.success();
        } else {
            self.config_freshness.issue = Some("system_status_busy");
        }
        // Retain the independently owned node Arc so BBS contention does not
        // prevent a successful node refresh (or disguise node lock contention).
        if let Some(nodes) = &self.node_source {
            if let Ok(nodes) = nodes.try_lock() {
                self.nodes = Some((
                    nodes
                        .iter()
                        .flatten()
                        .filter(|node| node.handle.as_ref().is_none_or(|handle| !handle.is_finished()))
                        .count(),
                    nodes.len(),
                ));
                self.nodes_freshness.success();
            } else {
                self.nodes_freshness.issue = Some("system_status_busy");
            }
        }
    }

    fn finish_disk(&mut self, root: &PathBuf, sample: Option<DiskSample>) {
        if self.root.as_ref() != Some(root) {
            return;
        }
        self.disk_freshness.pending_since = None;
        if let Some(sample) = sample {
            self.disk = Some(sample.space);
            self.disk_freshness.success_at(sample.instant, sample.utc);
        } else {
            self.disk = None;
            self.disk_freshness.issue = Some("system_status_read_failed");
        }
    }

    fn lines(&self, width: u16) -> Vec<Line<'static>> {
        let theme = get_tui_theme();
        let unavailable = || get_text("system_status_unavailable");
        let status_row = |key: &str, value: String, warning: bool| {
            Line::from(vec![
                Span::styled(format!("{}: ", get_text(key)), theme.config_title),
                Span::styled(value, if warning { theme.false_value } else { theme.value }),
            ])
        };
        let sample_row = |key: &str, value: String, freshness: &Freshness, warning: bool| {
            let stale = freshness.stale();
            let value = if stale {
                format!("{value} [{}]", get_text("system_status_stale"))
            } else {
                value
            };
            status_row(key, value, warning || stale)
        };
        let runtime_known = self.runtime_freshness.last_success.is_some();
        let runtime_detail = |detail: Option<&str>| {
            detail
                .map(display_text)
                .unwrap_or_else(|| get_text(if runtime_known { "system_status_none" } else { "system_status_unavailable" }))
        };
        let uptime = self.started_at.map(|start| {
            let seconds = start.elapsed().as_secs();
            format!("{} {:02}:{:02}:{:02}", seconds / 86400, seconds / 3600 % 24, seconds / 60 % 60, seconds % 60)
        });
        let mut lines = vec![
            sample_row("system_status_uptime", uptime.unwrap_or_else(unavailable), &self.runtime_freshness, false),
            sample_row(
                "system_status_admission",
                get_text(if !runtime_known {
                    "system_status_unavailable"
                } else if self.runtime.offline {
                    "system_status_admission_closed"
                } else {
                    "system_status_admission_open"
                }),
                &self.runtime_freshness,
                self.runtime.offline,
            ),
            sample_row(
                "system_status_nodes",
                self.nodes.map(|(active, total)| format!("{active}/{total}")).unwrap_or_else(unavailable),
                &self.nodes_freshness,
                false,
            ),
            sample_row(
                "system_status_root",
                self.root.as_ref().map(|path| display_text(&path.to_string_lossy())).unwrap_or_else(unavailable),
                &self.config_freshness,
                false,
            ),
            sample_row(
                "system_status_disk",
                self.disk.map(DiskSpace::text).unwrap_or_else(unavailable),
                &self.disk_freshness,
                self.disk.is_some_and(DiskSpace::low),
            ),
            sample_row(
                "system_status_maintenance",
                if runtime_known { maintenance_text(&self.runtime) } else { unavailable() },
                &self.runtime_freshness,
                false,
            ),
            sample_row(
                "system_status_failure",
                runtime_detail(self.runtime.failure.as_deref()),
                &self.runtime_freshness,
                self.runtime.failure.is_some(),
            ),
            sample_row(
                "system_status_request_error",
                runtime_detail(self.runtime.request_error.as_deref()),
                &self.runtime_freshness,
                self.runtime.request_error.is_some(),
            ),
        ];
        if let Some(online) = &self.runtime.online {
            lines.push(sample_row(
                "system_status_online_event",
                display_text(&online.description),
                &self.runtime_freshness,
                false,
            ));
        }
        lines.push(Line::styled(get_text("system_status_listeners"), theme.config_title));
        if self.endpoints.is_empty() {
            lines.push(Line::from(unavailable()));
        }
        for endpoint in &self.endpoints {
            let listener = self
                .listeners
                .as_ref()
                .and_then(|listeners| listeners.iter().find(|status| status.name == endpoint.name));
            // Web admin remains a local HTTP service while BBS logins are gated.
            let gated = self.runtime.offline && endpoint.name != "Web admin";
            let actual = match listener {
                Some(status) => {
                    let mut text = format!(
                        "{} — {} | {} {}s",
                        get_text(if !status.running {
                            "system_status_stopped"
                        } else if gated {
                            "system_status_running_closed"
                        } else {
                            "system_status_running"
                        }),
                        status.address,
                        get_text("system_status_state_age"),
                        (Utc::now() - status.changed_at).num_seconds().max(0),
                    );
                    let absolute = format!(" | {}", status.changed_at.format("%Y-%m-%d %H:%M:%S UTC"));
                    // Preserve the relative age on narrow screens; show the UTC
                    // transition time too when the entire labeled row fits.
                    if sample_row(endpoint.label, format!("{text}{absolute}"), &self.runtime_freshness, false).width() <= usize::from(width) {
                        text.push_str(&absolute);
                    }
                    text
                }
                None => get_text(if self.listeners.is_none() {
                    "system_status_unavailable"
                } else if endpoint.enabled {
                    "system_status_not_started"
                } else {
                    "system_status_disabled"
                }),
            };
            lines.push(sample_row(
                endpoint.label,
                actual,
                &self.runtime_freshness,
                listener.is_some_and(|status| !status.running || gated),
            ));
            lines.push(sample_row(
                "system_status_configured",
                format!(
                    "{} — {}",
                    display_text(&endpoint.configured),
                    get_text(if endpoint.enabled {
                        "system_status_enabled"
                    } else {
                        "system_status_disabled"
                    })
                ),
                &self.config_freshness,
                false,
            ));
        }
        for (key, freshness) in [
            ("system_status_config_sample", &self.config_freshness),
            ("system_status_runtime_sample", &self.runtime_freshness),
            ("system_status_nodes_sample", &self.nodes_freshness),
            ("system_status_disk_sample", &self.disk_freshness),
        ] {
            lines.push(status_row(key, freshness.text(), freshness.stale()));
        }
        lines
    }

    fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Esc => return true,
            KeyCode::Down => self.scroll = self.scroll.saturating_add(1),
            KeyCode::Up => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(self.page.max(1)),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(self.page.max(1)),
            KeyCode::Home => self.scroll = 0,
            KeyCode::End => self.scroll = self.max_scroll,
            _ => {}
        }
        self.scroll = self.scroll.min(self.max_scroll);
        false
    }

    fn ui(&mut self, frame: &mut Frame, full_screen: bool) {
        let theme = get_tui_theme();
        let area = get_screen_size(frame, full_screen);
        frame.render_widget(Clear, area);
        let block = Block::bordered()
            .title(Line::styled(format!(" {} ", get_text("system_status_title")), theme.dialog_box_title))
            .title_bottom(key_hint(get_text("system_status_keys")))
            .border_style(theme.dialog_box)
            .style(theme.background);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        self.page = inner.height;
        let paragraph = Paragraph::new(self.lines(inner.width)).wrap(Wrap { trim: false }).style(theme.background);
        // Scroll rendered rows, not source lines: long paths/errors and translated
        // labels remain reachable on narrow terminals, including after resize.
        self.max_scroll = paragraph
            .line_count(inner.width.max(1))
            .saturating_sub(inner.height as usize)
            .min(u16::MAX as usize) as u16;
        self.scroll = self.scroll.min(self.max_scroll);
        frame.render_widget(paragraph.scroll((self.scroll, 0)), inner);
    }
}

/// The CWS owner must handle offline/restart events after return. This viewer
/// never clears flags, acknowledges a restart, or releases the admission gate.
pub(crate) async fn run<B: Backend>(terminal: &mut Terminal<B>, board: &Arc<Mutex<IcyBoard>>, bbs: &Arc<Mutex<BBS>>, full_screen: bool) -> Res<()>
where
    B::Error: Send + Sync + 'static,
{
    let mut screen = SystemStatusScreen::default();
    let mut pending: Option<DiskJob> = None;
    let mut next_refresh = Instant::now();
    loop {
        screen.refresh_runtime(bbs);
        if screen.runtime.offline || screen.runtime.restart {
            return Ok(());
        }
        if pending.as_ref().is_some_and(|job| job.task.is_finished()) {
            let mut job = pending.take().unwrap();
            let result = (&mut job.task).await;
            // Filesystem and worker failures are UI unavailability, not runtime
            // failures. Do not log/latch them or refresh last-success metadata.
            screen.finish_disk(&job.root, result.ok().and_then(Result::ok));
        }
        if Instant::now() >= next_refresh {
            screen.refresh(board, bbs);
            if screen.runtime.offline || screen.runtime.restart {
                return Ok(());
            }
            if pending.is_none()
                && let Some(root) = screen.root.as_ref().filter(|root| !root.as_os_str().is_empty())
            {
                screen.disk_freshness.pending_since = Some(Instant::now());
                pending = Some(DiskJob::start(root.clone()));
            }
            next_refresh = Instant::now() + REFRESH_INTERVAL;
        }
        terminal.draw(|frame| screen.ui(frame, full_screen))?;
        if event::poll(POLL_INTERVAL)?
            && let Event::Key(key) = event::read()?
            && matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
            && screen.handle_key(key.code)
        {
            return Ok(());
        }
        tokio::task::yield_now().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use icy_board_engine::icy_board::bbs::EventMaintenanceStatus;
    use ratatui::backend::TestBackend;

    fn screen() -> SystemStatusScreen {
        let mut screen = SystemStatusScreen {
            started_at: Some(Instant::now() - Duration::from_secs(90061)),
            endpoints: vec![
                Endpoint::new("Telnet", "system_status_telnet", "127.0.0.1", 0, true),
                Endpoint::new("SSH", "system_status_ssh", "::1", 22, false),
                Endpoint::new("Secure WebSocket", "system_status_websocket", "127.0.0.1", 443, true),
                Endpoint::new("Web admin", "system_status_admin", "127.0.0.1", 8080, true),
            ],
            listeners: Some(vec![
                ListenerStatus {
                    name: "Telnet".into(),
                    address: "127.0.0.1:43210".parse().unwrap(),
                    running: true,
                    changed_at: Utc::now() - chrono::Duration::seconds(60),
                },
                ListenerStatus {
                    name: "Web admin".into(),
                    address: "127.0.0.1:45678".parse().unwrap(),
                    running: false,
                    changed_at: Utc::now() - chrono::Duration::seconds(30),
                },
            ]),
            root: Some("/test/board".into()),
            nodes: Some((1, 4)),
            disk: Some(DiskSpace {
                available: 2 * GIB,
                total: 8 * GIB,
            }),
            ..Default::default()
        };
        screen.config_freshness.success();
        screen.runtime_freshness.success();
        screen.nodes_freshness.success();
        screen.disk_freshness.success();
        screen
    }

    fn rendered(terminal: &Terminal<TestBackend>) -> String {
        let buffer = terminal.backend().buffer();
        buffer
            .content()
            .chunks(buffer.area.width as usize)
            .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn render_all(screen: &mut SystemStatusScreen, terminal: &mut Terminal<TestBackend>) -> String {
        screen.scroll = 0;
        terminal.draw(|frame| screen.ui(frame, false)).unwrap();
        let mut text = rendered(terminal);
        while screen.scroll < screen.max_scroll {
            screen.handle_key(KeyCode::PageDown);
            terminal.draw(|frame| screen.ui(frame, false)).unwrap();
            text.push_str(&format!("\n{}", rendered(terminal)));
        }
        text
    }

    #[test]
    fn renderer_fits_80_by_25_and_distinguishes_actual_configured_disabled_and_not_started() {
        let mut screen = screen();
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        let text = render_all(&mut screen, &mut terminal);
        for expected in [
            "127.0.0.1:43210",
            "127.0.0.1:0",
            "[::1]:22",
            "127.0.0.1:45678",
            "/test/board",
            "2.00 GiB / 8.00 GiB (25.0%)",
            "1/4",
        ] {
            assert!(text.contains(expected), "missing {expected}: {text}");
        }
        for key in [
            "system_status_running",
            "system_status_stopped",
            "system_status_disabled",
            "system_status_not_started",
            "system_status_title",
            "system_status_config_sample",
            "system_status_runtime_sample",
            "system_status_nodes_sample",
            "system_status_disk_sample",
        ] {
            assert!(text.contains(&get_text(key)), "missing {key}: {text}");
        }
        assert_eq!(screen.page, 23);
    }

    #[test]
    fn narrow_renderer_scrolls_wrapped_rows_and_survives_tiny_sizes() {
        let mut screen = screen();
        screen.runtime.request_error = Some("request-detail ".repeat(20));
        for (width, height) in [(30, 10), (12, 5), (1, 1), (80, 25)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            if width == 30 {
                let text = render_all(&mut screen, &mut terminal);
                assert!(text.contains("127.0.0.1:8080"));
                assert!(text.contains("UTC"));
            }
            terminal.draw(|frame| screen.ui(frame, false)).unwrap();
            screen.handle_key(KeyCode::End);
            terminal.draw(|frame| screen.ui(frame, false)).unwrap();
            assert_eq!(screen.scroll, screen.max_scroll);
            if width == 30 {
                assert!(screen.scroll > 0);
            }
            screen.handle_key(KeyCode::PageUp);
            screen.handle_key(KeyCode::Home);
            assert_eq!(screen.scroll, 0);
        }
    }

    #[test]
    fn maintenance_and_sticky_errors_are_displayed_without_tokens_or_controls() {
        let mut screen = screen();
        screen.runtime.offline = true;
        screen.runtime.failure = Some("sticky-runtime-detail".into());
        screen.runtime.request_error = Some("rejected-request-detail".into());
        screen.runtime.maintenance = Some(EventMaintenanceStatus {
            description: "nightly-event".into(),
            phase: EventMaintenancePhase::ListenerFailed("bind-failure-detail".into()),
        });
        let text = screen.lines(80).iter().map(ToString::to_string).collect::<Vec<_>>().join("\n");
        for expected in ["sticky-runtime-detail", "rejected-request-detail", "nightly-event", "bind-failure-detail"] {
            assert!(text.contains(expected));
        }
        assert_eq!(display_text("web admin token: sensitive-value"), get_text("system_status_redacted"));
        assert_eq!(display_text("a\x1b[31mb\x1b[0m"), "ab");
        assert_eq!(display_text("web admin to\x1b[31mken: sensitive-value"), get_text("system_status_redacted"));
        screen.disk = None;
        assert!(
            screen
                .lines(80)
                .iter()
                .any(|line| line.to_string().contains(&get_text("system_status_unavailable")))
        );
    }

    #[tokio::test]
    async fn snapshot_contention_is_nonblocking_and_preserves_previous_values() {
        let board = Arc::new(Mutex::new(IcyBoard::new()));
        let bbs = Arc::new(Mutex::new(BBS::new(2)));
        let mut screen = screen();
        let board_guard = board.lock().await;
        let bbs_guard = bbs.lock().await;
        screen.refresh(&board, &bbs);
        assert_eq!(screen.nodes, Some((1, 4)));
        assert_eq!(screen.root.as_deref(), Some(std::path::Path::new("/test/board")));
        drop(board_guard);
        drop(bbs_guard);
        screen.refresh(&board, &bbs);
        assert_eq!(screen.nodes, Some((0, 2)));
        assert!(screen.listeners.as_ref().unwrap().is_empty());
    }

    #[tokio::test]
    async fn each_source_keeps_its_own_last_success_on_contention_and_recovers_independently() {
        let board = Arc::new(Mutex::new(IcyBoard::new()));
        let bbs = Arc::new(Mutex::new(BBS::new(2)));
        let mut screen = screen();
        screen.refresh(&board, &bbs);
        let nodes = bbs.lock().await.open_connections.clone();
        screen.disk_freshness.success();
        let disk_success = screen.disk_freshness.last_success;
        let config_success = screen.config_freshness.last_success;
        let board_guard = board.lock().await;
        screen.refresh(&board, &bbs);
        assert_eq!(screen.config_freshness.last_success, config_success);
        assert!(screen.config_freshness.stale());
        assert!(!screen.runtime_freshness.stale());
        assert!(!screen.nodes_freshness.stale());
        assert_eq!(screen.disk_freshness.last_success, disk_success);
        drop(board_guard);

        let runtime_success = screen.runtime_freshness.last_success;
        let bbs_guard = bbs.lock().await;
        screen.refresh(&board, &bbs);
        assert_eq!(screen.runtime_freshness.last_success, runtime_success);
        assert!(screen.runtime_freshness.stale());
        assert!(!screen.config_freshness.stale());
        assert!(!screen.nodes_freshness.stale(), "the cached node Arc is independently readable");
        let nodes_success = screen.nodes_freshness.last_success;
        let nodes_guard = nodes.lock().await;
        screen.refresh(&board, &bbs);
        assert_eq!(screen.nodes_freshness.last_success, nodes_success);
        assert_eq!(screen.runtime_freshness.last_success, runtime_success);
        assert!(screen.nodes_freshness.stale());
        drop(bbs_guard);

        screen.refresh(&board, &bbs);
        assert!(!screen.runtime_freshness.stale());
        assert!(!screen.config_freshness.stale());
        assert!(screen.nodes_freshness.stale());
        assert_eq!(screen.nodes, Some((0, 2)));
        let text = screen.lines(80).iter().map(ToString::to_string).collect::<Vec<_>>().join("\n");
        assert!(text.contains(&get_text("system_status_stale")));
        assert!(text.contains(&get_text("system_status_busy")));
        assert!(text.contains(&nodes_success.unwrap().1.format("%Y-%m-%d %H:%M:%S UTC").to_string()));
        drop(nodes_guard);
        screen.refresh(&board, &bbs);
        assert!(!screen.nodes_freshness.stale());
        assert_eq!(screen.disk_freshness.last_success, disk_success);
    }

    #[tokio::test]
    async fn initial_contention_is_unknown_not_a_fake_open_or_empty_snapshot() {
        let board = Arc::new(Mutex::new(IcyBoard::new()));
        let bbs = Arc::new(Mutex::new(BBS::new(2)));
        let _board_guard = board.lock().await;
        let _bbs_guard = bbs.lock().await;
        let mut screen = SystemStatusScreen::default();
        screen.refresh(&board, &bbs);
        assert!(screen.started_at.is_none() && screen.nodes.is_none() && screen.listeners.is_none());
        assert!(screen.config_freshness.last_success.is_none() && screen.runtime_freshness.last_success.is_none());
        let text = screen.lines(80).iter().map(ToString::to_string).collect::<Vec<_>>().join("\n");
        assert!(text.contains(&get_text("system_status_never")));
        assert!(!text.contains(&get_text("system_status_admission_open")));
    }

    #[test]
    fn disk_warning_boundaries_percentages_and_unknown_denominators() {
        for (available, total, percent, low) in [
            (GIB, 10 * GIB, Some(10.0), false),
            (GIB - 1, GIB - 1, Some(100.0), true),
            (2 * GIB, 40 * GIB, Some(5.0), true),
            (2 * GIB, 8 * GIB, Some(25.0), false),
            (0, GIB, Some(0.0), true),
            (0, 0, None, true),
            (GIB, 0, None, false),
            (2 * GIB, GIB, None, false),
            (u64::MAX, u64::MAX, Some(100.0), false),
        ] {
            let disk = DiskSpace { available, total };
            assert_eq!(disk.percent(), percent);
            assert_eq!(disk.low(), low, "available={available}, total={total}");
            assert_eq!(disk.text().contains(&get_text("system_status_disk_low")), low);
            assert!(!disk.text().contains("NaN") && !disk.text().contains("inf"));
        }
        // Even though the display rounds to 10.0%, the true ratio is below 10%.
        assert!(
            DiskSpace {
                available: GIB,
                total: 10 * GIB + 1
            }
            .low()
        );
        assert!(
            DiskSpace {
                available: u64::MAX / 10,
                total: u64::MAX
            }
            .low()
        );
        assert!(
            !DiskSpace {
                available: u64::MAX / 10 + 1,
                total: u64::MAX
            }
            .low()
        );
    }

    #[test]
    fn disk_failure_pending_age_and_changed_root_never_forge_fresh_values() {
        let mut screen = screen();
        let root = screen.root.clone().unwrap();
        let sample_instant = Instant::now() - Duration::from_secs(5);
        let sample_utc = Utc::now() - chrono::Duration::seconds(5);
        screen.disk_freshness.success_at(sample_instant, sample_utc);
        screen.disk_freshness.pending_since = Some(Instant::now() - Duration::from_secs(3));
        assert!(screen.disk_freshness.stale());
        assert!(screen.disk_freshness.text().contains(&get_text("system_status_pending")));
        assert!(screen.disk_freshness.text().contains("5s"));
        screen.finish_disk(&root, None);
        assert!(screen.disk.is_none());
        assert!(screen.disk_freshness.stale());
        assert_eq!(screen.disk_freshness.last_success, Some((sample_instant, sample_utc)));
        assert!(screen.disk_freshness.text().contains(&get_text("system_status_read_failed")));
        assert!(screen.runtime.failure.is_none());
        // A delayed UI collection retains the worker's actual completion time.
        screen.finish_disk(
            &root,
            Some(DiskSample {
                space: DiskSpace {
                    available: GIB,
                    total: 10 * GIB,
                },
                instant: sample_instant,
                utc: sample_utc,
            }),
        );
        assert!(screen.disk.is_some() && screen.disk_freshness.stale());
        assert_eq!(screen.disk_freshness.last_success, Some((sample_instant, sample_utc)));
        screen.finish_disk(
            &root,
            Some(DiskSample {
                space: DiskSpace {
                    available: GIB,
                    total: 10 * GIB,
                },
                instant: Instant::now(),
                utc: Utc::now(),
            }),
        );
        assert!(!screen.disk_freshness.stale());
        screen.root = Some("/different/board".into());
        screen.disk = None;
        screen.disk_freshness = Freshness::default();
        screen.finish_disk(
            &root,
            Some(DiskSample {
                space: DiskSpace {
                    available: GIB,
                    total: 10 * GIB,
                },
                instant: Instant::now(),
                utc: Utc::now(),
            }),
        );
        assert!(screen.disk.is_none() && screen.disk_freshness.last_success.is_none());
    }

    #[test]
    fn bound_but_gated_is_distinct_and_transition_time_is_absolute_only_when_it_fits() {
        let mut screen = screen();
        screen.runtime.offline = true;
        let listeners = screen.listeners.as_mut().unwrap();
        listeners[1].running = true;
        let changed_at = Utc::now() - chrono::Duration::seconds(120);
        listeners[0].changed_at = changed_at;
        let wide = screen.lines(200).iter().map(ToString::to_string).collect::<Vec<_>>();
        let telnet = wide
            .iter()
            .find(|line| line.starts_with(&format!("{}:", get_text("system_status_telnet"))))
            .unwrap();
        assert!(telnet.contains(&get_text("system_status_running_closed")));
        assert!(telnet.contains("120s"));
        assert!(telnet.contains(&changed_at.format("%Y-%m-%d %H:%M:%S UTC").to_string()));
        let admin = wide
            .iter()
            .find(|line| line.starts_with(&format!("{}:", get_text("system_status_admin"))))
            .unwrap();
        assert!(admin.contains(&get_text("system_status_running")));
        assert!(!admin.contains(&get_text("system_status_running_closed")));
        let narrow = screen.lines(30).iter().map(ToString::to_string).collect::<Vec<_>>();
        let telnet = narrow
            .iter()
            .find(|line| line.starts_with(&format!("{}:", get_text("system_status_telnet"))))
            .unwrap();
        assert!(telnet.contains("120s") && !telnet.contains("UTC"));
        screen.listeners.as_mut().unwrap()[0].changed_at = Utc::now() + chrono::Duration::seconds(60);
        assert!(
            screen
                .lines(30)
                .iter()
                .any(|line| line.to_string().contains("| ") && line.to_string().ends_with(" 0s"))
        );
    }

    #[tokio::test]
    async fn returns_ready_for_owner_when_admission_closes_or_restart_or_failure_is_requested() {
        let board = Arc::new(Mutex::new(IcyBoard::new()));
        let bbs = Arc::new(Mutex::new(BBS::new(2)));
        // Neither board nor node contention may delay handing off to the owner.
        let _board_guard = board.lock().await;
        let nodes = bbs.lock().await.open_connections.clone();
        let _nodes_guard = nodes.lock().await;
        for mode in 0..4 {
            let mut runtime = RuntimeStatus::default();
            {
                let mut bbs = bbs.lock().await;
                bbs.event_maintenance = mode == 0;
                bbs.operator_maintenance = mode == 1;
                bbs.event_restart_requested = mode == 2;
                bbs.event_scheduler_error = (mode == 3).then(|| "runtime-failure".into());
            }
            runtime.refresh(&bbs);
            assert!(runtime.offline || runtime.restart);
            let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
            tokio::time::timeout(Duration::from_secs(1), run(&mut terminal, &board, &bbs, false))
                .await
                .unwrap()
                .unwrap();
            let bbs = bbs.lock().await;
            assert_eq!(bbs.event_restart_requested, mode == 2);
            assert!(!bbs.event_listeners_stopped);
        }
    }

    #[tokio::test]
    async fn missing_disk_path_is_unavailable_not_a_runtime_failure() {
        let dir = tempfile::tempdir().unwrap();
        let mut job = DiskJob::start(dir.path().join("missing"));
        let mut screen = screen();
        screen.root = Some(job.root.clone());
        let previous = screen.disk_freshness.last_success;
        let result = (&mut job.task).await.unwrap();
        assert!(result.is_err());
        screen.finish_disk(&job.root, result.ok());
        assert!(screen.disk.is_none() && screen.disk_freshness.stale());
        assert_eq!(screen.disk_freshness.last_success, previous);
        assert!(screen.runtime.failure.is_none());
    }

    #[tokio::test]
    async fn disk_worker_panic_is_stale_not_a_runtime_failure() {
        let mut screen = screen();
        let mut job = DiskJob {
            root: screen.root.clone().unwrap(),
            task: tokio::spawn(async { panic!("synthetic disk worker panic") }),
        };
        let previous = screen.disk_freshness.last_success;
        let result = (&mut job.task).await;
        assert!(result.is_err());
        screen.finish_disk(&job.root, result.ok().and_then(Result::ok));
        assert!(screen.disk.is_none() && screen.disk_freshness.stale());
        assert_eq!(screen.disk_freshness.last_success, previous);
        assert!(screen.runtime.failure.is_none());
    }

    #[tokio::test]
    async fn disk_worker_records_success_after_filesystem_query() {
        let dir = tempfile::tempdir().unwrap();
        let before = Instant::now();
        let utc_before = Utc::now();
        let mut job = DiskJob::start(dir.path().to_path_buf());
        let sample = (&mut job.task).await.unwrap().unwrap();
        assert!(sample.instant >= before && sample.instant <= Instant::now());
        assert!(sample.utc >= utc_before && sample.utc <= Utc::now());
        let mut screen = screen();
        screen.root = Some(job.root.clone());
        screen.finish_disk(&job.root, Some(sample));
        assert!(screen.disk.is_some() && !screen.disk_freshness.stale());
    }
}
