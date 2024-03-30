use std::{
    borrow::Borrow,
    io::{self, stdout, Stdout},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use chrono::{Local, Timelike};
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use icy_board_engine::icy_board::{
    text_messages::{
        DOSBUSY, DOSBUSYDESC, DOSNOTBUSY, DOSNOTBUSYDESC, LASTCALLER, NUMCALLS, NUMDOWN,
        NUMMESSAGES, NUMUP, SYSOPBUSY, SYSOPBUSYDESC, SYSOPNOTBUSY, SYSOPNOTBUSYDESC, SYSTEMAVAIL,
        USERBUSY, USERBUSYDESC, USERNOTBUSY, USERNOTBUSYDESC,
    },
    IcyBoard, IcyBoardError,
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

use crate::{call_stat::CallStat, VERSION};

pub const DOS_BLACK: Color = Color::Rgb(0, 0, 0);
pub const DOS_BLUE: Color = Color::Rgb(0, 0, 0xAA);
// pub const DOS_GREEN: Color = Color::Rgb(0, 0xAA, 0);
pub const DOS_CYAN: Color = Color::Rgb(0, 0xAA, 0xAA);
pub const DOS_RED: Color = Color::Rgb(0xAA, 0, 0);
// pub const DOS_MAGENTA: Color = Color::Rgb(0xAA, 0, 0xAA);
// pub const DOS_BROWN: Color = Color::Rgb(0xAA, 0x55, 0);
pub const DOS_LIGHTGRAY: Color = Color::Rgb(0xAA, 0xAA, 0xAA);

// pub const DOS_DARKGRAY: Color = Color::Rgb(0x55, 0x55, 0x55);
// pub const DOS_LIGHT_BLUE: Color = Color::Rgb(0x55, 0x55, 0xFF);
pub const DOS_LIGHT_GREEN: Color = Color::Rgb(0x55, 0xFF, 0x55);
// pub const DOS_LIGHT_CYAN: Color = Color::Rgb(0x55, 0xFF, 0xFF);
// pub const DOS_LIGHT_RED: Color = Color::Rgb(0xFF, 0x55, 0x55);
// pub const DOS_LIGHT_MAGENTA: Color = Color::Rgb(0xFF, 0x55, 0xFF);
pub const DOS_YELLOW: Color = Color::Rgb(0xFF, 0xFF, 0x55);
pub const DOS_WHITE: Color = Color::Rgb(0xFF, 0xFF, 0xFF);

#[derive(Clone, Copy)]
pub enum CallWaitMessage {
    User(bool),
    Sysop(bool),
    Exit(bool),
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
    board: Arc<Mutex<IcyBoard>>,
    buttons: Vec<Button>,
    call_stat: CallStat,

    last_caller_txt: String,
    calls_txt: String,
    msgs_txt: String,
    dls_txt: String,
    uls_txt: String,
    sysavail_txt: String,
}

impl CallWaitScreen {
    pub fn new(board: Arc<Mutex<IcyBoard>>) -> Res<Self> {
        let buttons;

        if let Ok(board) = board.lock().as_ref() {
            buttons = vec![
                Button {
                    title: board.display_text.get_display_text(USERBUSY)?.text,
                    description: board.display_text.get_display_text(USERBUSYDESC)?.text,
                    message: CallWaitMessage::User(true),
                },
                Button {
                    title: board.display_text.get_display_text(SYSOPBUSY)?.text,
                    description: board.display_text.get_display_text(SYSOPBUSYDESC)?.text,
                    message: CallWaitMessage::Sysop(true),
                },
                Button {
                    title: board.display_text.get_display_text(DOSBUSY)?.text,
                    description: board.display_text.get_display_text(DOSBUSYDESC)?.text,
                    message: CallWaitMessage::Exit(true),
                },
                Button {
                    title: board.display_text.get_display_text(USERNOTBUSY)?.text,
                    description: board.display_text.get_display_text(USERNOTBUSYDESC)?.text,
                    message: CallWaitMessage::User(false),
                },
                Button {
                    title: board.display_text.get_display_text(SYSOPNOTBUSY)?.text,
                    description: board.display_text.get_display_text(SYSOPNOTBUSYDESC)?.text,
                    message: CallWaitMessage::Sysop(false),
                },
                Button {
                    title: board.display_text.get_display_text(DOSNOTBUSY)?.text,
                    description: board.display_text.get_display_text(DOSNOTBUSYDESC)?.text,
                    message: CallWaitMessage::Exit(false),
                },
            ];
        } else {
            return Err(Box::new(IcyBoardError::Error(
                "Board is locked".to_string(),
            )));
        }

        let file = board.lock().as_ref().unwrap().data.stats_file.to_string();
        let file_name = board.lock().as_ref().unwrap().resolve_file(&file);
        let call_stat = CallStat::load(&file_name)?;
        let last_caller_txt = board
            .lock()
            .as_ref()
            .unwrap()
            .display_text
            .get_display_text(LASTCALLER)?
            .text;
        let calls_txt = board
            .lock()
            .as_ref()
            .unwrap()
            .display_text
            .get_display_text(NUMCALLS)?
            .text;
        let msgs_txt = board
            .lock()
            .as_ref()
            .unwrap()
            .display_text
            .get_display_text(NUMMESSAGES)?
            .text;
        let dls_txt = board
            .lock()
            .as_ref()
            .unwrap()
            .display_text
            .get_display_text(NUMDOWN)?
            .text;
        let uls_txt = board
            .lock()
            .as_ref()
            .unwrap()
            .display_text
            .get_display_text(NUMUP)?
            .text;
        let sysavail_txt = board
            .lock()
            .as_ref()
            .unwrap()
            .display_text
            .get_display_text(SYSTEMAVAIL)?
            .text;

        Ok(Self {
            x: 0,
            y: 0,
            selected: None,
            call_stat,
            buttons,
            last_caller_txt,
            calls_txt,
            msgs_txt,
            dls_txt,
            uls_txt,
            sysavail_txt,
            board,
        })
    }

    pub fn run(&mut self) -> Res<CallWaitMessage> {
        let mut terminal = init_terminal()?;
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(16);
        loop {
            let _ = terminal.draw(|frame| self.ui(frame));
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? && self.selected.is_none() {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Down | KeyCode::Char('s') => self.y = (self.y + 1).min(1),
                        KeyCode::Up | KeyCode::Char('w') => self.y = (self.y - 1).max(0),
                        KeyCode::Right | KeyCode::Char('d') => self.x = (self.x + 1).min(2),
                        KeyCode::Left | KeyCode::Char('a') => self.x = (self.x - 1).max(0),
                        KeyCode::Enter => {
                            self.selected = Some(Instant::now());
                        }
                        _ => {}
                    }
                }
            }

            if let Some(selected) = self.selected {
                if selected.elapsed() >= Duration::from_millis(500) {
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
        let width = rect.width as f64 - 2.0;
        let height = rect.height as f64 - 2.0;
        let ver = VERSION.to_string();

        Canvas::default()
        .marker(Marker::Block)
        .paint(move |ctx| {

            // draw node
            let node_txt = format!("Node {}", self.board.lock().borrow().as_ref().unwrap().data.node_num);
            ctx.print(4.0 + (width - node_txt.len() as f64)  / 2.0,  height - 1.0,
            Line::from(node_txt).style(Style::new()
            .fg(DOS_WHITE)));

            render_button(ctx, 4.0, height - 2.0, width - 7.0, &self.board.lock().borrow().as_ref().unwrap().data.board_name, SelectState::Selected);

            let x_padding = 7.0;
            let y_padding = -2.0;
            let button_width = 19.0;
            for (i, b) in self.buttons.iter().enumerate() {
                render_button(ctx,
                    4.0 + x_padding * (i % 3) as f64 + button_width * (i % 3) as f64,
                    height - 4.0 + y_padding * (i / 3) as f64,
                    button_width,
                    &b.title,
                    self.get_select_state(i as i32));
            }

            let selected_button = (self.y * 3 + self.x) as usize;
            // draw description
            ctx.print(4.0 + (width - self.buttons[selected_button].description.len() as f64)  / 2.0,  8.0,
            Line::from(self.buttons[selected_button].description.to_string()).style(Style::new()
            //.bg(bg)
            .fg(DOS_WHITE)));

            // draw separator
            let separator_y = 7.0;
            for i in 0..=(width as usize) {
                ctx.print(i as f64, separator_y, Line::from("═").style(Style::new()
                .fg(DOS_WHITE)));
            }

            //self.icy_board.lock().borrow().as_ref().unwrap().data.stat

            render_label(ctx, 4.0, separator_y - 2.0, width - 7.0, &self.sysavail_txt);
            render_label(ctx, 4.0, separator_y - 4.0, width - 7.0, format!("{} {}", self.last_caller_txt, self.call_stat.last_caller).as_str());

            let label_padding = 5.0;
            let label_size = 14.0;

            render_label(ctx, 4.0, separator_y - 6.0, label_size, format!("{} {}", self.calls_txt, self.call_stat.new_calls).as_str());

            render_label(ctx, 4.0 + label_padding * 1.0 + label_size, separator_y - 6.0, label_size, format!("{} {}", self.msgs_txt, self.call_stat.new_msgs).as_str());

            render_label(ctx, 4.0 + label_padding * 2.0 + label_size * 2.0, separator_y - 6.0, label_size, format!("{} {}", self.dls_txt, self.call_stat.total_dn).as_str());

            render_label(ctx, 4.0 + label_padding * 3.0 + label_size * 3.0, separator_y - 6.0, label_size, format!("{} {}", self.uls_txt, self.call_stat.total_up).as_str());

        }).background_color(DOS_BLUE)
        .x_bounds([0.0, width])
        .y_bounds([0.0,height])

        .block(Block::default()

        .title(Title::from(Line::from(format!(" {} ", now.date_naive())).style(Style::new().white())).alignment(Alignment::Left))

        .title_style(Style::new().fg(DOS_YELLOW))
        .title_alignment(Alignment::Center)
        .title(format!("  IcyBoard v{}  ", ver))
        .title(Title::from(Line::from(format!(" {} ", now.time().with_nanosecond(0).unwrap())).style(Style::new().white())).alignment(Alignment::Right))
        .title(Title::from(Line::from("  (C) Copyright Mike Krüger, 2024, https://github.com/mkrueger/icy_board  ")
        .style(Style::new().white()))
        .alignment(Alignment::Center)
        .position(block::Position::Bottom))
        .style(Style::new().bg(DOS_BLUE))
        .border_type(BorderType::Double)
        .border_style(Style::new().white())
        .borders(Borders::ALL))
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
            SelectState::Selected => DOS_LIGHTGRAY,
            SelectState::Pressed => DOS_LIGHTGRAY,
        }
    }
}

fn render_button(
    ctx: &mut canvas::Context<'_>,
    x: f64,
    y: f64,
    width: f64,
    title: &str,
    selected: SelectState,
) {
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
            ctx.print(
                x + 1.0 + i as f64,
                y - 1.0,
                Line::from("▀").style(Style::new().fg(DOS_BLACK)),
            );
        }

        ctx.print(
            x + width + 1.0,
            y,
            Line::from("▄").style(Style::new().fg(DOS_BLACK)),
        );
        ctx.print(
            x + (width - title.len() as f64) / 2.0,
            y,
            Line::from(title.to_string()).style(Style::new().bg(bg).fg(selected.get_fg())),
        );
    } else {
        for i in 0..=(width as usize) {
            ctx.print(
                x + 1.0 + i as f64,
                y,
                Line::from("▄").style(Style::new().fg(DOS_LIGHTGRAY)),
            );
            ctx.print(
                x + 1.0 + i as f64,
                y - 1.0,
                Line::from("▀").style(Style::new().fg(DOS_LIGHTGRAY)),
            );
        }

        ctx.print(
            x + width + 1.0,
            y,
            Line::from("▄").style(Style::new().fg(DOS_LIGHTGRAY)),
        );
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
        ctx.print(
            x + 1.0 + i as f64,
            y - 1.0,
            Line::from("▀").style(Style::new().fg(DOS_BLACK)),
        );
    }

    ctx.print(
        x + width + 1.0,
        y,
        Line::from("▄").style(Style::new().fg(DOS_BLACK)),
    );
    ctx.print(
        x + (width - title.len() as f64) / 2.0,
        y,
        Line::from(title.to_string()).style(Style::new().bg(bg).fg(DOS_BLACK)),
    );
}

fn init_terminal() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(stdout()))
}

pub fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
