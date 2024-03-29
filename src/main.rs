use std::{
    io::{self, stdout, Stdout},
    time::{Duration, Instant},
};

use chrono::{Local, Timelike};
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::*,
    widgets::{
        block::Title,
        canvas::{Canvas, Rectangle},
        *,
    },
};
use semver::Version;

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}

fn main() -> io::Result<()> {
    App::run()
}

const DOS_BLACK: Color = Color::Rgb(0, 0, 0);
const DOS_RED: Color = Color::Rgb(0xAA, 0, 0);
const DOS_BLUE: Color = Color::Rgb(0, 0, 0xAA);
const DOS_CYAN: Color = Color::Rgb(0, 0xAA, 0xAA);

const DOS_GRAY: Color = Color::Rgb(0xAA, 0xAA, 0xAA);
const DOS_YELLOW: Color = Color::Rgb(0xFF, 0xFF, 0x55);
const DOS_WHITE: Color = Color::Rgb(0xFF, 0xFF, 0xFF);

struct Button {
    pub title: String,
    pub description: String,
}

struct App {
    x: i32,
    y: i32,
    selected: Option<Instant>,

    buttons: Vec<Button>,
}

impl App {
    pub fn new() -> Self {
        let buttons = vec![
            Button {
                title: "User - Busy".to_string(),
                description: "Log in as a regular user. Callers will get a busy signal."
                    .to_string(),
            },
            Button {
                title: "Sysop - Busy".to_string(),
                description: "Log in as the Sysop. Callers will get a busy signal.".to_string(),
            },
            Button {
                title: "Exit - Busy".to_string(),
                description: "Drop to OS. Callers will get a busy signal.".to_string(),
            },
            Button {
                title: "User - Not Busy".to_string(),
                description: "Log in as a regular user. RING Alert will be activated.".to_string(),
            },
            Button {
                title: "Sysop - Not Busy".to_string(),
                description: "Log in as the Sysop. RING Alert will be activated.".to_string(),
            },
            Button {
                title: "Exit - Not Busy".to_string(),
                description: "Drop to OS. RING Alert will be activated.".to_string(),
            },
        ];
        Self {
            x: 0,
            y: 0,
            selected: None,
            buttons,
        }
    }

    pub fn run() -> io::Result<()> {
        let mut terminal = init_terminal()?;
        let mut app = Self::new();
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(16);
        loop {
            let _ = terminal.draw(|frame| app.ui(frame));
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? && app.selected.is_none() {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Down | KeyCode::Char('j') => app.y = (app.y + 1).min(1),
                        KeyCode::Up | KeyCode::Char('k') => app.y = (app.y - 1).max(0),
                        KeyCode::Right | KeyCode::Char('l') => app.x = (app.x + 1).min(2),
                        KeyCode::Left | KeyCode::Char('h') => app.x = (app.x - 1).max(0),

                        KeyCode::Enter => {
                            app.selected = Some(Instant::now());
                        }
                        _ => {}
                    }
                }
            }

            if let Some(selected) = app.selected {
                if selected.elapsed() >= Duration::from_millis(500) {
                    break;
                }
            }

            if last_tick.elapsed() >= tick_rate {
                //     app.on_tick();
                last_tick = Instant::now();
            }
        }
        restore_terminal()
    }

    fn ui(&self, frame: &mut Frame) {
        frame.render_widget(self.main_canvas(frame.size()), frame.size());
    }

    fn main_canvas(&self, rect: Rect) -> impl Widget + '_ {
        let now = Local::now();
        let width = rect.width as f64 - 2.0;
        let height = rect.height as f64 - 2.0;

        Canvas::default()
        .marker(Marker::Block)
        .paint(move |ctx| {
            let title = "PCBoard Bullettin Board - Node 1";
            render_button(ctx, 4.0, height - 2.0, width - 7.0, title, SelectState::Selected);

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

            render_label(ctx, 4.0, separator_y - 2.0, width - 7.0, "Waiting for call on Port 128.0.0.1:4711...");
            render_label(ctx, 4.0, separator_y - 4.0, width - 7.0, "Last Caller: Omnibrain");

            let label_padding = 5.0;
            let label_size = 14.0;

            render_label(ctx, 4.0, separator_y - 6.0, label_size, "Calls: 47");

            render_label(ctx, 4.0 + label_padding * 1.0 + label_size, separator_y - 6.0, label_size, "Msgs: 2");

            render_label(ctx, 4.0 + label_padding * 2.0 + label_size * 2.0, separator_y - 6.0, label_size, "D/Ls: 0");

            render_label(ctx, 4.0 + label_padding * 3.0 + label_size * 3.0, separator_y - 6.0, label_size, "U/Ls: 0");

        }).background_color(DOS_BLUE)
        .x_bounds([0.0, width])
        .y_bounds([0.0,height])

        .block(Block::default()

        .title(Title::from(Line::from(format!(" {} ", now.date_naive())).style(Style::new().white())).alignment(Alignment::Left))

        .title_style(Style::new().fg(DOS_YELLOW))
        .title_alignment(Alignment::Center)
        .title(format!("  IcyBoard v{}  ", VERSION.to_string()))
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
            SelectState::Selected => DOS_GRAY,
            SelectState::Pressed => DOS_GRAY,
        }
    }
}

fn render_button(
    ctx: &mut canvas::Context<'_>,
    mut x: f64,
    mut y: f64,
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
                Line::from("▄").style(Style::new().fg(DOS_GRAY)),
            );
            ctx.print(
                x + 1.0 + i as f64,
                y - 1.0,
                Line::from("▀").style(Style::new().fg(DOS_GRAY)),
            );
        }

        ctx.print(
            x + width + 1.0,
            y,
            Line::from("▄").style(Style::new().fg(DOS_GRAY)),
        );
    }
}

fn render_label(ctx: &mut canvas::Context<'_>, mut x: f64, mut y: f64, width: f64, title: &str) {
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

fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
