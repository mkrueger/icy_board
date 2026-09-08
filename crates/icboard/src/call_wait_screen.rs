use std::{
    future::Future,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{Res, SHOW_TOTAL_STATS, event_screen::RuntimeStatus};
use chrono::{Local, Timelike};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use icy_board_engine::icy_board::{
    IcyBoard,
    bbs::{BBS, EventMaintenancePhase},
    state::NodeStatus,
    statistics::Statistics,
};
use icy_board_tui::{
    app::get_screen_size,
    get_text, get_text_args,
    theme::{DOS_BLACK, DOS_BLUE, DOS_CYAN, DOS_LIGHT_GRAY, DOS_RED, DOS_WHITE, DOS_YELLOW},
};
use ratatui::{
    Frame, Terminal,
    buffer::Buffer,
    layout::{Constraint, Layout, Margin, Position, Rect},
    prelude::Backend,
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget, Wrap},
};

use tokio::sync::Mutex;

use crate::{GIT_HASH, VERSION};

fn program_title(version: &str, hash: &str) -> String {
    if hash.is_empty() {
        format!("  IcyBoard v{version}  ")
    } else {
        format!("  IcyBoard v{version} ({hash})  ")
    }
}

#[derive(Clone)]
pub enum CallWaitMessage {
    /// Scheduler asks the service owner for a stop/reload/restart handshake.
    EventRestart,
    User,
    Sysop,
    Exit,
    Monitor,
    EventMonitor,
    LogViewer,
    SystemStatus,

    ToggleCallLog,
    TogglePageBell,
    ToggleAlarm,
    SystemManager,
    Setup,

    IcbText,
    ToggleStatistics,
    ShowStatistics,
    RunPPE(PathBuf, Option<String>, Option<String>, Option<Vec<String>>),
}

struct Button {
    pub title: String,
    pub description: String,
    pub message: CallWaitMessage,
}

pub struct CallWaitScreen {
    x: i32,
    y: i32,
    pub selected: Option<Instant>,
    buttons: Vec<Button>,
    board_name: String,
    date_format: String,
    statistics: Statistics,
    paging_alert: Option<String>,
    error_message: Option<String>,
    runtime: RuntimeStatus,
}

impl CallWaitScreen {
    pub async fn new(board: &Arc<Mutex<IcyBoard>>) -> Res<Self> {
        let buttons = vec![
            // Row 1
            Button {
                title: get_text("call_wait_screen_user_button"),
                description: get_text("call_wait_screen_user_button_descr"),
                message: CallWaitMessage::User,
            },
            Button {
                title: get_text("call_wait_screen_sysop_button"),
                description: get_text("call_wait_screen_sysop_button_descr"),
                message: CallWaitMessage::Sysop,
            },
            Button {
                title: get_text("call_wait_screen_exit_button"),
                description: get_text("call_wait_screen_exit_button_descr"),
                message: CallWaitMessage::Exit,
            },
            // Row 2
            Button {
                title: get_text("call_wait_screen_log_button"),
                description: get_text("call_wait_screen_log_button_descr"),
                message: CallWaitMessage::LogViewer,
            },
            Button {
                title: get_text("call_wait_screen_status_button"),
                description: get_text("call_wait_screen_status_button_descr"),
                message: CallWaitMessage::SystemStatus,
            },
            Button {
                title: get_text("call_wait_screen_event_monitor_button"),
                description: get_text("call_wait_screen_event_monitor_button_descr"),
                message: CallWaitMessage::EventMonitor,
            },
            // Row 3
            Button {
                title: if board.lock().await.config.options.alarm {
                    get_text("call_wait_screen_call_log_on")
                } else {
                    get_text("call_wait_screen_call_log_off")
                },
                description: get_text("call_wait_screen_call_log_descr"),
                message: CallWaitMessage::ToggleCallLog,
            },
            Button {
                title: if board.lock().await.config.options.alarm {
                    get_text("call_wait_screen_page_bell_on")
                } else {
                    get_text("call_wait_screen_page_bell_off")
                },
                description: get_text("call_wait_screen_page_bell_descr"),
                message: CallWaitMessage::TogglePageBell,
            },
            Button {
                title: if board.lock().await.config.options.alarm {
                    get_text("call_wait_screen_alarm_on")
                } else {
                    get_text("call_wait_screen_alarm_off")
                },
                description: get_text("call_wait_screen_alarm_descr"),
                message: CallWaitMessage::ToggleAlarm,
            },
            // Row 4
            Button {
                title: get_text("call_wait_screen_system_manager"),
                description: get_text("call_wait_screen_system_manager_descr"),
                message: CallWaitMessage::SystemManager,
            },
            Button {
                title: get_text("call_wait_screen_icb_text"),
                description: get_text("call_wait_screen_icb_text_descr"),
                message: CallWaitMessage::IcbText,
            },
            Button {
                title: get_text("call_wait_screen_setup"),
                description: get_text("call_wait_screen_setup_descr"),
                message: CallWaitMessage::Setup,
            },
            // Row 4
            Button {
                title: if unsafe { SHOW_TOTAL_STATS } {
                    get_text("call_wait_screen_total_statistics")
                } else {
                    get_text("call_wait_screen_today_statistics")
                },
                description: get_text("call_wait_screen_statistics_descr"),
                message: CallWaitMessage::ToggleStatistics,
            },
            Button {
                title: get_text("call_wait_screen_monitor_button_not_busy"),
                description: get_text("call_wait_screen_monitor_button_not_busy_descr"),
                message: CallWaitMessage::Monitor,
            },
            Button {
                title: get_text("call_wait_screen_show_statistics"),
                description: get_text("call_wait_screen_show_statistics_descr"),
                message: CallWaitMessage::ShowStatistics,
            },
        ];

        let board_name = board.lock().await.config.board.name.clone();
        let date_format = board.lock().await.config.board.date_format.clone();
        Ok(Self {
            x: 0,
            y: 0,
            selected: None,
            buttons,
            board_name,
            date_format,
            statistics: Statistics::default(),
            paging_alert: None,
            error_message: None,
            runtime: RuntimeStatus::default(),
        })
    }

    pub fn show_error(&mut self, message: impl Into<String>) {
        self.selected = None;
        self.error_message = Some(message.into());
    }

    /// Keep drawing through service drain, command execution and reload/restart.
    /// The pinned operation is never cancelled by a tick. No operator input is read.
    pub async fn during_event<B: Backend, F: Future>(
        &mut self,
        terminal: &mut Terminal<B>,
        bbs: &Arc<Mutex<BBS>>,
        full_screen: bool,
        operation: F,
    ) -> Res<F::Output>
    where
        B::Error: Send + Sync + 'static,
    {
        tokio::pin!(operation);
        let mut status = RuntimeStatus::default();
        let mut redraw = tokio::time::interval(Duration::from_millis(250));
        redraw.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            status.refresh(bbs);
            terminal.draw(|frame| Self::event_ui(frame, full_screen, &status, self.error_message.as_deref()))?;
            tokio::select! {
                result = &mut operation => return Ok(result),
                _ = redraw.tick() => {},
            }
        }
    }

    fn event_ui(frame: &mut Frame, full_screen: bool, runtime: &RuntimeStatus, operator_error: Option<&str>) {
        let area = get_screen_size(frame, full_screen);
        let mut args = std::collections::HashMap::new();
        let mut failed = false;
        let mut failure_hint = "event_runtime_repair_hint";
        let (description, phase) = if let Some(error) = &runtime.failure {
            failed = true;
            (get_text("event_runtime_scheduler_stopped"), error.clone())
        } else if let Some(status) = &runtime.maintenance {
            let key = match &status.phase {
                EventMaintenancePhase::Waiting(time) => {
                    args.insert("time".to_string(), time.clone());
                    "event_runtime_waiting"
                }
                EventMaintenancePhase::Draining(count) => {
                    args.insert("count".to_string(), count.to_string());
                    "event_runtime_draining"
                }
                EventMaintenancePhase::Stopping => "event_runtime_stopping",
                EventMaintenancePhase::Running => "event_runtime_running",
                EventMaintenancePhase::Reloading => "event_runtime_reloading",
                EventMaintenancePhase::ReloadFailed(error) => {
                    failed = true;
                    args.insert("error".to_string(), error.clone());
                    "event_runtime_reload_failed"
                }
                EventMaintenancePhase::ListenerFailed(error) => {
                    failed = true;
                    failure_hint = "event_runtime_listener_failed_hint";
                    args.insert("error".to_string(), error.clone());
                    "event_runtime_listener_failed"
                }
                EventMaintenancePhase::Restarting => "event_runtime_restarting",
            };
            (status.description.clone(), get_text_args(key, args))
        } else {
            (String::new(), get_text("event_runtime_gate_closed"))
        };
        let mut text = if runtime.failure.is_some() {
            format!("{description}\n\n{}\n\n{phase}", get_text("event_runtime_failed_hint"))
        } else if failed {
            // Keep phase-specific recovery instructions visible even when an error is very long.
            format!("{description}\n\n{}\n\n{phase}", get_text(failure_hint))
        } else {
            format!("{description}\n\n{phase}\n\n{}", get_text("event_runtime_offline_hint"))
        };
        if let Some(online) = runtime.online_text() {
            text = format!("{online}\n\n{text}");
        }
        if let Some(error) = operator_error {
            failed = true;
            text = format!("{error}\n\n{text}");
        }
        frame.render_widget(Clear, area);
        frame.render_widget(
            Paragraph::new(text)
                .wrap(Wrap { trim: true })
                .style(Style::new().fg(DOS_WHITE).bg(if failed { DOS_RED } else { DOS_BLUE }))
                .block(
                    Block::bordered()
                        .border_type(BorderType::Double)
                        .title(get_text("event_runtime_title"))
                        .title_bottom(Line::from(Local::now().format(" %H:%M:%S ").to_string()).right_aligned()),
                ),
            area,
        );
    }

    pub async fn reset(&mut self, board: &Arc<Mutex<IcyBoard>>) {
        self.selected = None;

        let config = &board.lock().await.config;

        self.buttons[6].title = if config.options.call_log {
            get_text("call_wait_screen_call_log_on")
        } else {
            get_text("call_wait_screen_call_log_off")
        };
        self.buttons[7].title = if config.options.page_bell {
            get_text("call_wait_screen_page_bell_on")
        } else {
            get_text("call_wait_screen_page_bell_off")
        };
        self.buttons[8].title = if config.options.alarm {
            get_text("call_wait_screen_alarm_on")
        } else {
            get_text("call_wait_screen_alarm_off")
        };

        self.buttons[12].title = if unsafe { SHOW_TOTAL_STATS } {
            get_text("call_wait_screen_total_statistics")
        } else {
            get_text("call_wait_screen_today_statistics")
        };
    }

    pub async fn run<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
        board: &Arc<Mutex<IcyBoard>>,
        bbs: &Arc<Mutex<BBS>>,
        full_screen: bool,
    ) -> Res<CallWaitMessage>
    where
        B::Error: Send + Sync + 'static,
    {
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(1000);
        loop {
            self.runtime.refresh(bbs);
            if self.runtime.restart && self.runtime.failure.is_none() {
                return Ok(CallWaitMessage::EventRestart);
            }
            if self.runtime.offline || self.runtime.failure.is_some() {
                self.selected = None;
                terminal.draw(|frame| Self::event_ui(frame, full_screen, &self.runtime, self.error_message.as_deref()))?;
                tokio::time::sleep(Duration::from_millis(250)).await;
                continue;
            }
            self.statistics = board.lock().await.statistics.clone();
            self.paging_alert = Self::paging_alert(board, bbs).await;

            if terminal.get_frame().area().width > 1 && terminal.get_frame().area().height > 1 {
                terminal.draw(|frame| self.ui(frame, full_screen))?;
            }

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());

            if event::poll(timeout)?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                if self.error_message.take().is_some() {
                    self.selected = None;
                    continue;
                }
                if self.selected.is_some() {
                    continue;
                }
                match key.code {
                    KeyCode::Esc => {
                        return Ok(CallWaitMessage::Exit);
                    }
                    KeyCode::Down | KeyCode::Char('s') => self.set_if_valid(self.x, self.y + 1),
                    KeyCode::Up | KeyCode::Char('w') => self.set_if_valid(self.x, self.y - 1),
                    KeyCode::Right | KeyCode::Char('d') => self.set_if_valid(self.x + 1, self.y),
                    KeyCode::Left | KeyCode::Char('a') => self.set_if_valid(self.x - 1, self.y),
                    KeyCode::Enter => {
                        self.selected = Some(Instant::now());
                    }
                    _ => {}
                }
            }

            if let Some(selected) = self.selected
                && selected.elapsed() >= Duration::from_millis(150)
            {
                return Ok(self.buttons[(self.y * 3 + self.x) as usize].message.clone());
            }

            if last_tick.elapsed() >= tick_rate {
                //     self.on_tick();
                last_tick = Instant::now();
            }
        }
    }

    async fn paging_alert(board: &Arc<Mutex<IcyBoard>>, bbs: &Arc<Mutex<BBS>>) -> Option<String> {
        let open_connections = bbs.lock().await.open_connections.clone();
        let paging_nodes = {
            let connections = open_connections.lock().await;
            connections
                .iter()
                .flatten()
                .filter(|node| matches!(node.status, NodeStatus::PagingSysop))
                .map(|node| (node.node_number, node.cur_user))
                .collect::<Vec<_>>()
        };
        let (node, cur_user) = *paging_nodes.first()?;
        let user = {
            let board = board.lock().await;
            usize::try_from(cur_user)
                .ok()
                .and_then(|index| board.users.get(index))
                .map(|user| user.get_name().clone())
                .unwrap_or_else(|| get_text("call_wait_screen_unknown_caller"))
        };
        let mut args = std::collections::HashMap::new();
        args.insert("node".to_string(), node.to_string());
        args.insert("user".to_string(), user);
        args.insert("count".to_string(), paging_nodes.len().to_string());
        Some(get_text_args("call_wait_screen_sysop_page", args))
    }

    fn ui(&self, frame: &mut Frame, full_screen: bool) {
        let now = Local::now();

        let dt = now.format(&self.date_format);

        let ver = VERSION.to_string();
        let area = get_screen_size(frame, full_screen);
        let screen_area = area;

        let b = Block::default()
            .title_top(Line::from(format!(" {} ", dt)).style(Style::new().white()).left_aligned())
            .title_top(Line::from(program_title(&ver, GIT_HASH)).fg(DOS_YELLOW).centered())
            .title(
                Line::from(format!(" {} ", now.time().with_nanosecond(0).unwrap()))
                    .style(Style::new().white())
                    .right_aligned(),
            )
            .title_bottom(Line::from("  (C) Copyright Mike Krüger, 2024 ").style(Style::new().white()).right_aligned())
            .style(Style::new().bg(DOS_BLUE))
            .border_type(BorderType::Double)
            .border_style(Style::new().white())
            .borders(Borders::ALL);
        frame.render_widget(b, area);
        let vertical: Layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(7),
        ]);

        let [header, mut title, mut button_bar, footer, separator, mut stats] = vertical.areas(area.inner(Margin { vertical: 1, horizontal: 1 }));

        // draw node
        Line::from(self.runtime.online_text().unwrap_or_else(|| "https://github.com/mkrueger/icy_board".into()))
            .style(Style::new().fg(DOS_WHITE))
            .centered()
            .render(header, frame.buffer_mut());
        let selected_button = (self.y * 3 + self.x) as usize;
        title.width = title.width.saturating_sub(1);
        PcbButton::new(self.board_name.clone())
            .theme(Theme {
                text: DOS_BLACK,
                background: DOS_LIGHT_GRAY,
            })
            .render(title.inner(Margin { horizontal: 2, vertical: 0 }), frame.buffer_mut());

        let horizontal = Layout::horizontal([Constraint::Percentage(33), Constraint::Percentage(33), Constraint::Percentage(33)]);

        button_bar.y = button_bar.y.saturating_sub(1);

        //button_bar.width -= 2;
        let [mut row1, mut row2, mut row3] = horizontal.areas(button_bar);

        row1.height = 1;
        row1 = row1.inner(Margin { vertical: 0, horizontal: 2 });

        row2.height = 1;
        row2 = row2.inner(Margin { vertical: 0, horizontal: 2 });

        row3.height = 1;
        row3 = row3.inner(Margin { vertical: 0, horizontal: 2 });

        for (i, b) in self.buttons.iter().enumerate() {
            if i % 3 == 0 {
                row1.y += 2;
                row2.y += 2;
                row3.y += 2;
            }

            PcbButton::new(b.title.clone()).state(self.get_select_state(i as i32)).render(
                match i % 3 {
                    2 => row3,
                    1 => row2,
                    _ => row1,
                },
                frame.buffer_mut(),
            );
        }

        Line::from(
            self.runtime
                .request_error
                .clone()
                .unwrap_or_else(|| self.buttons[selected_button].description.to_string()),
        )
        .style(Style::new().fg(DOS_WHITE))
        .centered()
        .render(footer.inner(Margin { horizontal: 1, vertical: 0 }), frame.buffer_mut());

        // draw description
        Line::from("═".repeat(stats.width as usize))
            .style(Style::new().fg(DOS_WHITE))
            .centered()
            .render(separator, frame.buffer_mut());

        stats.y += 1;
        stats.height = stats.height.saturating_sub(1);

        let mut area = stats.inner(Margin { horizontal: 3, vertical: 0 });
        area.height = 1;

        let stat_teme = Theme {
            text: DOS_BLACK,
            background: DOS_CYAN,
        };
        let (ready_text, ready_theme) = self.paging_alert.as_ref().map_or_else(
            || (get_text("call_wait_screen_sys_ready"), stat_teme),
            |alert| {
                (
                    alert.clone(),
                    Theme {
                        text: DOS_WHITE,
                        background: DOS_RED,
                    },
                )
            },
        );
        PcbButton::new(ready_text).theme(ready_theme).render(area, frame.buffer_mut());
        stats.y += 2;
        stats.height = stats.height.saturating_sub(2);

        let mut area = stats.inner(Margin { horizontal: 3, vertical: 0 });
        area.height = 1;

        PcbButton::new(format!(
            "{} {}",
            get_text("call_wait_screen_last_caller"),
            self.statistics
                .last_callers
                .last()
                .map_or(&get_text("call_wait_screen_last_caller_none"), |c| &c.user_name)
        ))
        .theme(stat_teme)
        .render(area, frame.buffer_mut());

        stats.y += 1;
        stats.height = stats.height.saturating_sub(1);
        let horizontal = Layout::horizontal([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ]);

        let [mut calls, mut msgs, mut dls, mut uls] = horizontal.areas(stats.inner(Margin { vertical: 1, horizontal: 1 }));
        calls.height = 1;
        msgs.height = 1;
        dls.height = 1;
        uls.height = 1;

        let horizontal = 2;
        unsafe {
            let stats = if SHOW_TOTAL_STATS { &self.statistics.total } else { &self.statistics.today };
            PcbButton::new(format!("{} {}", get_text("call_wait_screen_num_calls"), stats.calls))
                .theme(stat_teme)
                .render(calls.inner(Margin { horizontal, vertical: 0 }), frame.buffer_mut());

            PcbButton::new(format!("{} {}", get_text("call_wait_screen_num_msgs"), stats.messages))
                .theme(stat_teme)
                .render(msgs.inner(Margin { horizontal, vertical: 0 }), frame.buffer_mut());

            PcbButton::new(format!("{} {}", get_text("call_wait_screen_num_dls"), stats.downloads))
                .theme(stat_teme)
                .render(dls.inner(Margin { horizontal, vertical: 0 }), frame.buffer_mut());

            PcbButton::new(format!("{} {}", get_text("call_wait_screen_num_uls"), stats.uploads))
                .theme(stat_teme)
                .render(uls.inner(Margin { horizontal, vertical: 0 }), frame.buffer_mut());
        }

        if let Some(message) = &self.error_message {
            let width = screen_area.width.saturating_sub(8).clamp(30, 72).min(screen_area.width);
            let height = 8.min(screen_area.height.saturating_sub(2)).max(3).min(screen_area.height);
            let popup = Rect::new(
                screen_area.x + screen_area.width.saturating_sub(width) / 2,
                screen_area.y + screen_area.height.saturating_sub(height) / 2,
                width,
                height,
            );
            frame.render_widget(Clear, popup);
            Paragraph::new(format!("{message}\n\nPress any key to continue."))
                .wrap(Wrap { trim: true })
                .style(Style::new().fg(DOS_WHITE).bg(DOS_RED))
                .block(Block::bordered().title(" Error ").border_type(BorderType::Double))
                .render(popup, frame.buffer_mut());
        }
    }

    fn get_select_state(&self, button: i32) -> State {
        let selected_button = self.y * 3 + self.x;
        if self.selected.is_none() {
            if button == selected_button {
                return State::Selected;
            }
            return State::Normal;
        }
        if button == selected_button {
            return State::Active;
        }
        State::Normal
    }

    fn set_if_valid(&mut self, x: i32, y: i32) {
        let selected_button = y * 3 + x;
        if selected_button >= 0 && selected_button < self.buttons.len() as i32 {
            self.x = x;
            self.y = y;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Normal,
    Selected,
    Active,
}

impl State {
    pub fn get_fg(&self) -> Color {
        match self {
            State::Normal => DOS_WHITE,
            State::Selected => DOS_BLACK,
            State::Active => DOS_BLACK,
        }
    }

    pub fn get_bg(&self) -> Color {
        match self {
            State::Normal => DOS_RED,
            State::Selected => DOS_LIGHT_GRAY,
            State::Active => DOS_LIGHT_GRAY,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Theme {
    text: Color,
    background: Color,
}

struct PcbButton<'a> {
    label: Line<'a>,
    theme: Option<Theme>,
    state: State,
}

impl<'a> PcbButton<'a> {
    pub fn new<T: Into<Line<'a>>>(label: T) -> Self {
        PcbButton {
            label: label.into(),
            theme: None,
            state: State::Normal,
        }
    }

    pub const fn theme(mut self, theme: Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    pub const fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }
}

impl<'a> Widget for PcbButton<'a> {
    #[allow(clippy::cast_possible_truncation)]
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.state == State::Active {
            buf.set_string(
                area.x + 1,
                area.y,
                "▀".repeat(area.width as usize),
                Style::new().fg(DOS_BLUE).bg(DOS_LIGHT_GRAY),
            );
            buf.set_string(
                area.x + 1,
                area.y + 1,
                "▀".repeat(area.width as usize),
                Style::new().fg(DOS_LIGHT_GRAY).bg(DOS_BLUE),
            );
            return;
        }

        let (fg, bg) = if let Some(theme) = self.theme {
            (theme.text, theme.background)
        } else {
            (self.state.get_fg(), self.state.get_bg())
        };
        buf.set_style(area, Style::new().bg(bg).fg(fg));
        if !buf.area.contains(Position::new(area.x + 1, area.y + 1)) {
            return;
        }
        buf.set_string(area.x + 1, area.y + 1, "▀".repeat(area.width as usize), Style::new().fg(DOS_BLACK).bg(DOS_BLUE));
        buf.set_string(area.x + area.width, area.y, "▀", Style::new().fg(DOS_BLUE).bg(DOS_BLACK));

        // render label centered
        buf.set_line(
            area.x + (area.width.saturating_sub(self.label.width() as u16)) / 2,
            area.y + (area.height.saturating_sub(1)) / 2,
            &self.label,
            area.width,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    #[test]
    fn the_title_names_the_commit_when_it_is_known() {
        assert_eq!(program_title("0.2.1", "a1b2c3d"), "  IcyBoard v0.2.1 (a1b2c3d)  ");
        assert_eq!(program_title("0.2.1", ""), "  IcyBoard v0.2.1  ");
    }

    #[test]
    fn offline_gate_and_sticky_failure_render_without_a_phase() {
        let mut runtime = RuntimeStatus {
            offline: true,
            ..RuntimeStatus::default()
        };
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal
            .draw(|frame| CallWaitScreen::event_ui(frame, false, &runtime, Some("Board lock unavailable")))
            .unwrap();
        let text = terminal.backend().buffer().content().iter().map(|cell| cell.symbol()).collect::<String>();
        assert!(text.contains("OFFLINE"));
        assert!(text.contains("Board lock unavailable"));
        assert!(text.contains(&get_text("event_runtime_gate_closed")));
        runtime.failure = Some("Sticky journal failure".into());
        terminal.draw(|frame| CallWaitScreen::event_ui(frame, false, &runtime, None)).unwrap();
        let text = terminal.backend().buffer().content().iter().map(|cell| cell.symbol()).collect::<String>();
        assert!(text.contains("Sticky journal failure"));
        assert!(text.contains("OFFLINE"));
    }

    #[tokio::test]
    async fn online_snapshot_keeps_callwait_buttons_and_the_event_monitor_visible() {
        let board = Arc::new(Mutex::new(IcyBoard::new()));
        let mut screen = CallWaitScreen::new(&board).await.unwrap();
        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        bbs.lock().await.event_online_status = Some(icy_board_engine::icy_board::bbs::OnlineEventStatus {
            event_id: "online".into(),
            description: "Foreground report".into(),
            started: chrono::Utc::now(),
            log_file: None,
            running_long: true,
        });
        screen.runtime.refresh(&bbs);
        assert!(!screen.runtime.offline);
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| screen.ui(frame, false)).unwrap();
        let text = terminal.backend().buffer().content().iter().map(|cell| cell.symbol()).collect::<String>();
        assert!(text.contains("ONLINE: Foreground report"));
        assert!(text.contains(&get_text("call_wait_screen_event_monitor_button")));
        assert!(text.contains(&screen.buttons[0].title));
        assert_eq!(screen.buttons.len(), 15);
    }

    /// The events screen has to stay reachable without a hidden function key.
    #[tokio::test]
    async fn the_event_monitor_replaces_the_duplicate_shell_button() {
        let board = Arc::new(Mutex::new(IcyBoard::new()));
        let screen = CallWaitScreen::new(&board).await.unwrap();
        assert_eq!(
            screen.buttons.iter().filter(|button| matches!(button.message, CallWaitMessage::Exit)).count(),
            1
        );
        let monitor = screen
            .buttons
            .iter()
            .position(|button| matches!(button.message, CallWaitMessage::EventMonitor))
            .expect("event monitor button");
        assert_eq!(screen.buttons[monitor].title, get_text("call_wait_screen_event_monitor_button"));
    }

    #[tokio::test]
    async fn operator_buttons_have_distinct_actions_and_visible_labels() {
        let board = Arc::new(Mutex::new(IcyBoard::new()));
        let screen = CallWaitScreen::new(&board).await.unwrap();
        assert_eq!(screen.buttons.len(), 15);
        assert!(matches!(screen.buttons[0].message, CallWaitMessage::User));
        assert!(matches!(screen.buttons[1].message, CallWaitMessage::Sysop));
        assert!(matches!(screen.buttons[2].message, CallWaitMessage::Exit));
        assert!(matches!(screen.buttons[3].message, CallWaitMessage::LogViewer));
        assert!(matches!(screen.buttons[4].message, CallWaitMessage::SystemStatus));
        assert!(matches!(screen.buttons[5].message, CallWaitMessage::EventMonitor));
        assert!(matches!(screen.buttons[6].message, CallWaitMessage::ToggleCallLog));
        assert!(matches!(screen.buttons[12].message, CallWaitMessage::ToggleStatistics));
        assert!(matches!(screen.buttons[14].message, CallWaitMessage::ShowStatistics));
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| screen.ui(frame, false)).unwrap();
        let text = terminal.backend().buffer().content().iter().map(|cell| cell.symbol()).collect::<String>();
        for button in &screen.buttons[..6] {
            assert!(text.contains(&button.title), "missing button {}", button.title);
        }
    }

    #[test]
    fn tool_error_is_rendered_as_a_visible_dialog() {
        let buttons = (0..15)
            .map(|_| Button {
                title: "Tool".into(),
                description: "Description".into(),
                message: CallWaitMessage::ToggleAlarm,
            })
            .collect();
        let mut screen = CallWaitScreen {
            x: 0,
            y: 0,
            selected: None,
            buttons,
            board_name: "Test Board".into(),
            date_format: "%m/%d/%y".into(),
            statistics: Statistics::default(),
            paging_alert: None,
            error_message: None,
            runtime: RuntimeStatus::default(),
        };
        screen.show_error("icbsetup exited with exit status: 7");
        let backend = TestBackend::new(80, 25);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| screen.ui(frame, false)).unwrap();

        let text = terminal.backend().buffer().content().iter().map(|cell| cell.symbol()).collect::<String>();
        assert!(text.contains("icbsetup exited with exit status: 7"));
        assert!(text.contains("Press any key to continue."));
    }

    #[test]
    fn sysop_page_replaces_ready_indicator() {
        let buttons = (0..15)
            .map(|_| Button {
                title: "Tool".into(),
                description: "Description".into(),
                message: CallWaitMessage::ToggleAlarm,
            })
            .collect();
        let screen = CallWaitScreen {
            x: 0,
            y: 0,
            selected: None,
            buttons,
            board_name: "Test Board".into(),
            date_format: "%m/%d/%y".into(),
            statistics: Statistics::default(),
            paging_alert: Some("SYSOP PAGE: Node 2 - Alice (1 active)".into()),
            error_message: None,
            runtime: RuntimeStatus::default(),
        };
        let backend = TestBackend::new(80, 25);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| screen.ui(frame, false)).unwrap();

        let text = terminal.backend().buffer().content().iter().map(|cell| cell.symbol()).collect::<String>();
        assert!(text.contains("SYSOP PAGE: Node 2 - Alice (1 active)"));
        assert!(!text.contains("System is Ready For Callers"));
    }
}
