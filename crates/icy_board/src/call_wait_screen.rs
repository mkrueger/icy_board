use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use chrono::{Local, Timelike};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use icy_board_engine::icy_board::{statistics::Statistics, IcyBoard};
use icy_board_tui::{
    get_text,
    theme::{DOS_BLACK, DOS_BLUE, DOS_CYAN, DOS_LIGHT_GRAY, DOS_RED, DOS_WHITE, DOS_YELLOW},
};
use icy_ppe::Res;
use ratatui::{
    prelude::*,
    widgets::{block::Title, *},
};

use crate::VERSION;

#[derive(Clone, Copy)]
pub enum CallWaitMessage {
    User(bool),
    Sysop(bool),
    Exit(bool),
    Monitor,
}

struct Button {
    pub title: String,
    pub description: String,
    pub message: CallWaitMessage,
}

pub struct CallWaitScreen {
    x: i32,
    y: i32,
    selected: Option<Instant>,
    buttons: Vec<Button>,
    board_name: String,
    statistics: Statistics,
}

impl CallWaitScreen {
    pub fn new(board: &Arc<Mutex<IcyBoard>>) -> Res<Self> {
        let buttons = vec![
            Button {
                title: get_text("call_wait_screen_user_button_busy"),
                description: get_text("call_wait_screen_user_button_busy_descr"),
                message: CallWaitMessage::User(true),
            },
            Button {
                title: get_text("call_wait_screen_sysop_button_busy"),
                description: get_text("call_wait_screen_sysop_button_busy_descr"),
                message: CallWaitMessage::Sysop(true),
            },
            Button {
                title: get_text("call_wait_screen_dos_button_busy"),
                description: get_text("call_wait_screen_dos_button_busy_descr"),
                message: CallWaitMessage::Exit(true),
            },
            Button {
                title: get_text("call_wait_screen_user_button_not_busy"),
                description: get_text("call_wait_screen_user_button_not_busy_descr"),
                message: CallWaitMessage::User(false),
            },
            Button {
                title: get_text("call_wait_screen_sysop_button_not_busy"),
                description: get_text("call_wait_screen_sysop_button_not_busy_descr"),
                message: CallWaitMessage::Sysop(false),
            },
            Button {
                title: get_text("call_wait_screen_dos_button_not_busy"),
                description: get_text("call_wait_screen_dos_button_not_busy_descr"),
                message: CallWaitMessage::Exit(false),
            },
            Button {
                title: get_text("call_wait_screen_monitor_button_not_busy"),
                description: get_text("call_wait_screen_monitor_button_not_busy_descr"),
                message: CallWaitMessage::Monitor,
            },
        ];
        let board_name = board.lock().unwrap().config.board.name.clone();

        Ok(Self {
            x: 0,
            y: 0,
            selected: None,
            buttons,
            board_name,
            statistics: Statistics::default(),
        })
    }

    pub fn run(&mut self, terminal: &mut Terminal<impl Backend>, board: &Arc<Mutex<IcyBoard>>) -> Res<CallWaitMessage> {
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(1000);

        loop {
            if let Ok(board) = &board.lock() {
                self.statistics = board.statistics.clone();
            }

            terminal.draw(|frame| self.ui(frame))?;

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());

            if self.selected.is_none() && event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Esc => {
                                return Ok(CallWaitMessage::Exit(false));
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
                }
            }

            if let Some(selected) = self.selected {
                if selected.elapsed() >= Duration::from_millis(300) {
                    return Ok(self.buttons[(self.y * 3 + self.x) as usize].message);
                }
            }

            if last_tick.elapsed() >= tick_rate {
                //     self.on_tick();
                last_tick = Instant::now();
            }
        }
    }

    fn ui(&self, frame: &mut Frame) {
        let now = Local::now();
        let ver = VERSION.to_string();
        let area = frame.size();
        let b = Block::default()
            .title(Title::from(Line::from(format!(" {} ", now.date_naive())).style(Style::new().white())).alignment(Alignment::Left))
            .title_style(Style::new().fg(DOS_YELLOW))
            .title_alignment(Alignment::Center)
            .title(format!("  IcyBoard v{}  ", ver))
            .title(Title::from(Line::from(format!(" {} ", now.time().with_nanosecond(0).unwrap())).style(Style::new().white())).alignment(Alignment::Right))
            .title(
                Title::from(Line::from("  (C) Copyright Mike Krüger, 2024 ").style(Style::new().white()))
                    .alignment(Alignment::Center)
                    .position(block::Position::Bottom),
            )
            .style(Style::new().bg(DOS_BLUE))
            .border_type(BorderType::Double)
            .border_style(Style::new().white())
            .borders(Borders::ALL);
        b.render(area, frame.buffer_mut());

        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(7),
        ]);

        let [header, mut title, mut button_bar, footer, separator, mut stats] = vertical.areas(area.inner(&Margin { vertical: 1, horizontal: 1 }));

        // draw node
        Line::from("https://github.com/mkrueger/icy_board")
            .style(Style::new().fg(DOS_WHITE))
            .centered()
            .render(header, frame.buffer_mut());
        let selected_button = (self.y * 3 + self.x) as usize;

        title.width -= 1;
        PcbButton::new(self.board_name.clone())
            .theme(Theme {
                text: DOS_BLACK,
                background: DOS_LIGHT_GRAY,
            })
            .render(title.inner(&Margin { horizontal: 2, vertical: 0 }), frame.buffer_mut());

        let horizontal = Layout::horizontal([Constraint::Percentage(33), Constraint::Percentage(33), Constraint::Percentage(33)]);

        button_bar.y -= 1;
        //button_bar.width -= 2;
        let [mut row1, mut row2, mut row3] = horizontal.areas(button_bar);

        row1.height = 1;
        row1 = row1.inner(&Margin { vertical: 0, horizontal: 2 });

        row2.height = 1;
        row2 = row2.inner(&Margin { vertical: 0, horizontal: 2 });

        row3.height = 1;
        row3 = row3.inner(&Margin { vertical: 0, horizontal: 2 });

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

        Line::from(self.buttons[selected_button].description.to_string())
            .style(Style::new().fg(DOS_WHITE))
            .centered()
            .render(footer.inner(&Margin { horizontal: 1, vertical: 0 }), frame.buffer_mut());

        // draw description
        Line::from("═".repeat(stats.width as usize))
            .style(Style::new().fg(DOS_WHITE))
            .centered()
            .render(separator, frame.buffer_mut());

        stats.y += 1;
        stats.height -= 1;

        let mut area = stats.inner(&Margin { horizontal: 3, vertical: 0 });
        area.height = 1;

        let stat_teme = Theme {
            text: DOS_BLACK,
            background: DOS_CYAN,
        };
        PcbButton::new(get_text("call_wait_screen_sys_ready"))
            .theme(stat_teme)
            .render(area, frame.buffer_mut());
        stats.y += 2;
        stats.height -= 2;

        let mut area = stats.inner(&Margin { horizontal: 3, vertical: 0 });
        area.height = 1;

        PcbButton::new(format!(
            "{} {}",
            get_text("call_wait_screen_last_caller"),
            self.statistics.last_callers.last().map_or("", |c| &c.user_name)
        ))
        .theme(stat_teme)
        .render(area, frame.buffer_mut());

        stats.y += 1;
        stats.height -= 1;
        let horizontal = Layout::horizontal([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ]);

        let [mut calls, mut msgs, mut dls, mut uls] = horizontal.areas(stats.inner(&Margin { vertical: 1, horizontal: 1 }));
        calls.height = 1;
        msgs.height = 1;
        dls.height = 1;
        uls.height = 1;

        let horizontal = 2;
        PcbButton::new(format!("{} {}", get_text("call_wait_screen_num_calls"), self.statistics.total.calls))
            .theme(stat_teme)
            .render(calls.inner(&Margin { horizontal, vertical: 0 }), frame.buffer_mut());

        PcbButton::new(format!("{} {}", get_text("call_wait_screen_num_msgs"), self.statistics.total.messages))
            .theme(stat_teme)
            .render(msgs.inner(&Margin { horizontal, vertical: 0 }), frame.buffer_mut());

        PcbButton::new(format!("{} {}", get_text("call_wait_screen_num_dls"), self.statistics.total.downloads))
            .theme(stat_teme)
            .render(dls.inner(&Margin { horizontal, vertical: 0 }), frame.buffer_mut());

        PcbButton::new(format!("{} {}", get_text("call_wait_screen_num_uls"), self.statistics.total.uploads))
            .theme(stat_teme)
            .render(uls.inner(&Margin { horizontal, vertical: 0 }), frame.buffer_mut());
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
