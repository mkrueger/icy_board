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
    widgets::{
        block::Title,
        canvas::{Canvas, Rectangle},
        *,
    },
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
        frame.render_widget(self.main_canvas(frame.size()), frame.size());
    }

    fn main_canvas(&self, rect: Rect) -> impl Widget + '_ {
        let now = Local::now();
        let width = rect.width as f64 - 2.0; // -2 for border
        let height = rect.height as f64;
        let ver = VERSION.to_string();
        Canvas::default()
            .marker(Marker::Block)
            .paint(move |ctx| {
                // draw node
                let node_txt = "https://github.com/mkrueger/icy_board".to_string();
                ctx.print(
                    4.0 + (width - node_txt.len() as f64) / 2.0,
                    height - 1.0,
                    Line::from(node_txt).style(Style::new().fg(DOS_WHITE)),
                );

                render_button(ctx, 4.0, height - 2.0, width - 7.0, &self.board_name, SelectState::Selected);

                let y_padding = -2.0;
                let button_space = width / 3.0;
                let button_width = (button_space * 19.0 / 26.0).floor();
                let left_pos = ((width + button_space - button_width - 3.0 * button_space.floor()) / 2.0).ceil();

                for (i, b) in self.buttons.iter().enumerate() {
                    render_button(
                        ctx,
                        left_pos + button_space * (i % 3) as f64,
                        height - 4.0 + y_padding * (i / 3) as f64,
                        button_width,
                        &b.title,
                        self.get_select_state(i as i32),
                    );
                }

                let selected_button = (self.y * 3 + self.x) as usize;
                // draw description
                ctx.print(
                    4.0 + (width - self.buttons[selected_button].description.len() as f64) / 2.0,
                    8.0,
                    Line::from(self.buttons[selected_button].description.to_string()).style(
                        Style::new()
                            //.bg(bg)
                            .fg(DOS_WHITE),
                    ),
                );

                // draw separator
                let separator_y = 7.0;
                for i in 0..=(width as usize) {
                    ctx.print(i as f64, separator_y, Line::from("═").style(Style::new().fg(DOS_WHITE)));
                }

                render_label(ctx, 4.0, separator_y - 2.0, width - 7.0, &get_text("call_wait_screen_sys_ready"));

                render_label(
                    ctx,
                    4.0,
                    separator_y - 4.0,
                    width - 7.0,
                    format!("{} {}", get_text("call_wait_screen_last_caller"), self.statistics.last_caller).as_str(),
                );

                let label_space = width / 4.0;
                let label_size = (label_space * 14.0 / 19.0).floor();
                let left_pos = ((width + label_space - label_size - 4.0 * label_space.floor()) / 2.0).ceil();

                render_label(
                    ctx,
                    left_pos,
                    separator_y - 6.0,
                    label_size,
                    format!("{} {}", get_text("call_wait_screen_num_calls"), self.statistics.total.calls).as_str(),
                );

                render_label(
                    ctx,
                    left_pos + label_space * 1.0,
                    separator_y - 6.0,
                    label_size,
                    format!("{} {}", get_text("call_wait_screen_num_msgs"), self.statistics.total.messages).as_str(),
                );

                render_label(
                    ctx,
                    left_pos + label_space * 2.0,
                    separator_y - 6.0,
                    label_size,
                    format!("{} {}", get_text("call_wait_screen_num_dls"), self.statistics.total.downloads).as_str(),
                );

                render_label(
                    ctx,
                    left_pos + label_space * 3.0,
                    separator_y - 6.0,
                    label_size,
                    format!("{} {}", get_text("call_wait_screen_num_uls"), self.statistics.total.uploads).as_str(),
                );
            })
            .background_color(DOS_BLUE)
            .x_bounds([0.0, width])
            .y_bounds([0.0, height])
            .block(
                Block::default()
                    .title(Title::from(Line::from(format!(" {} ", now.date_naive())).style(Style::new().white())).alignment(Alignment::Left))
                    .title_style(Style::new().fg(DOS_YELLOW))
                    .title_alignment(Alignment::Center)
                    .title(format!("  IcyBoard v{}  ", ver))
                    .title(
                        Title::from(Line::from(format!(" {} ", now.time().with_nanosecond(0).unwrap())).style(Style::new().white()))
                            .alignment(Alignment::Right),
                    )
                    .title(
                        Title::from(Line::from("  (C) Copyright Mike Krüger, 2024 ").style(Style::new().white()))
                            .alignment(Alignment::Center)
                            .position(block::Position::Bottom),
                    )
                    .style(Style::new().bg(DOS_BLUE))
                    .border_type(BorderType::Double)
                    .border_style(Style::new().white())
                    .borders(Borders::ALL),
            )
    }

    fn get_select_state(&self, button: i32) -> SelectState {
        let selected_button = self.y * 3 + self.x;
        if self.selected.is_none() {
            if button == selected_button {
                return SelectState::Selected;
            }
            return SelectState::None;
        }
        if button == selected_button {
            return SelectState::Pressed;
        }
        SelectState::None
    }

    fn set_if_valid(&mut self, x: i32, y: i32) {
        let selected_button = y * 3 + x;
        if selected_button >= 0 && selected_button < self.buttons.len() as i32 {
            self.x = x;
            self.y = y;
        }
    }
}

#[derive(PartialEq)]
enum SelectState {
    None,
    Selected,
    Pressed,
}

impl SelectState {
    pub fn get_fg(&self) -> Color {
        match self {
            SelectState::None => DOS_WHITE,
            SelectState::Selected => DOS_BLACK,
            SelectState::Pressed => DOS_BLACK,
        }
    }

    pub fn get_bg(&self) -> Color {
        match self {
            SelectState::None => DOS_RED,
            SelectState::Selected => DOS_LIGHT_GRAY,
            SelectState::Pressed => DOS_LIGHT_GRAY,
        }
    }
}

fn render_button(ctx: &mut canvas::Context<'_>, x: f64, y: f64, width: f64, title: &str, selected: SelectState) {
    let bg = selected.get_bg();

    if selected != SelectState::Pressed {
        ctx.draw(&Rectangle {
            x,
            y,
            width,
            height: 0.0,
            color: bg,
        });

        for i in 0..=(width as usize) {
            ctx.print(x + 1.0 + i as f64, y - 1.0, Line::from("▀").style(Style::new().fg(DOS_BLACK)));
        }

        ctx.print(x + width + 1.0, y, Line::from("▄").style(Style::new().fg(DOS_BLACK)));
        ctx.print(
            x + (width - title.len() as f64) / 2.0,
            y,
            Line::from(title.to_string()).style(Style::new().bg(bg).fg(selected.get_fg())),
        );
    } else {
        for i in 0..=(width as usize) {
            ctx.print(x + 1.0 + i as f64, y, Line::from("▄").style(Style::new().fg(DOS_LIGHT_GRAY)));
            ctx.print(x + 1.0 + i as f64, y - 1.0, Line::from("▀").style(Style::new().fg(DOS_LIGHT_GRAY)));
        }

        ctx.print(x + width + 1.0, y, Line::from("▄").style(Style::new().fg(DOS_LIGHT_GRAY)));
    }
}

fn render_label(ctx: &mut canvas::Context<'_>, x: f64, y: f64, width: f64, title: &str) {
    let bg = DOS_CYAN;

    ctx.draw(&Rectangle {
        x,
        y,
        width,
        height: 0.0,
        color: bg,
    });

    for i in 0..=(width as usize) {
        ctx.print(x + 1.0 + i as f64, y - 1.0, Line::from("▀").style(Style::new().fg(DOS_BLACK)));
    }

    ctx.print(x + width + 1.0, y, Line::from("▄").style(Style::new().fg(DOS_BLACK)));
    ctx.print(
        x + (width - title.len() as f64) / 2.0,
        y,
        Line::from(title.to_string()).style(Style::new().bg(bg).fg(DOS_BLACK)),
    );
}
