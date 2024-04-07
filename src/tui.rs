use std::{
    collections::VecDeque,
    io::{self, stdout, Stdout},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use chrono::{Datelike, Local, Timelike};
use crossterm::{
    cursor::MoveTo,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, EnterAlternateScreen, LeaveAlternateScreen,
    },
    ExecutableCommand,
};

use icy_board_engine::icy_board::{state::Session, IcyBoard, IcyBoardError};
use icy_engine::TextPane;
use icy_ppe::Res;
use ratatui::{
    prelude::*,
    widgets::{canvas::Canvas, Paragraph},
};

use crate::{
    bbs::PcbBoardCommand,
    call_wait_screen::{DOS_BLACK, DOS_BLUE, DOS_LIGHT_GREEN, DOS_WHITE},
    icy_engine_output::Screen,
};

pub struct Tui {
    screen: Arc<Mutex<Screen>>,
    input_buffer: Arc<Mutex<VecDeque<(bool, char)>>>,
    board: Arc<Mutex<IcyBoard>>,
    session: Arc<Mutex<Session>>,

    status_bar: usize,
    handle: Option<thread::JoinHandle<Result<(), String>>>,
}

impl Tui {
    pub fn new(
        cmd: PcbBoardCommand,
        screen: Arc<Mutex<Screen>>,
        input_buffer: Arc<Mutex<VecDeque<(bool, char)>>>,
    ) -> Self {
        let board = cmd.state.board.clone();
        let session = Arc::new(Mutex::new(cmd.state.session.clone()));
        let cmd = Arc::new(Mutex::new(cmd));
        let ui = session.clone();
        let join = thread::spawn(move || loop {
            let orig_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |panic_info| {
                log::error!("IcyBoard thread crashed at {:?}", panic_info.location());
                log::error!("full info: {:?}", panic_info);

                orig_hook(panic_info);
            }));

            if let Ok(lock) = &mut cmd.lock() {
                ui.lock().as_mut().unwrap().cur_user = lock.state.session.cur_user;
                ui.lock().as_mut().unwrap().current_conference =
                    lock.state.session.current_conference.clone();
                ui.lock().as_mut().unwrap().disp_options = lock.state.session.disp_options.clone();
                if let Err(err) = lock.do_command() {
                    lock.state.session.disp_options.reset_printout();
                    log::error!("Error: {}", err);
                    lock.state.set_color(4.into()).unwrap();
                    lock.state
                        .print(
                            icy_board_engine::vm::TerminalTarget::Both,
                            &format!("\r\nError: {}\r\n\r\n", err),
                        )
                        .unwrap();
                    lock.state.reset_color().unwrap();
                }
                lock.state.session.disp_options.reset_printout();

                if lock.state.session.request_logoff {
                    ui.lock().as_mut().unwrap().request_logoff = true;
                    return Ok(());
                }
            }
            thread::sleep(Duration::from_millis(20));
        });

        Self {
            screen,
            input_buffer,
            board,
            session,
            status_bar: 0,
            handle: Some(join),
        }
    }

    pub fn run(&mut self) -> Res<()> {
        let mut terminal = init_terminal()?;
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(20);
        while !self.session.lock().unwrap().request_logoff {
            if self.handle.as_ref().unwrap().is_finished() {
                restore_terminal()?;
                let handle = self.handle.take().unwrap();
                if let Err(_err) = handle.join() {
                    return Err(Box::new(IcyBoardError::ThreadCrashed));
                }
                return Ok(());
            }

            let _ = terminal.draw(|frame| {
                if let Ok(board) = &self.board.lock() {
                    self.ui(frame, board);
                }
            });
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        if key.modifiers.contains(KeyModifiers::ALT) {
                            match key.code {
                                KeyCode::Char('h') => {
                                    self.status_bar = (self.status_bar + 1) % 2;
                                }
                                KeyCode::Char('x') => {
                                    let _ = restore_terminal();

                                    return Ok(());
                                }
                                _ => {}
                            }
                        } else {
                            match key.code {
                                KeyCode::Char(c) => {
                                    if (c == 'x' || c == 'c')
                                        && key.modifiers.contains(KeyModifiers::CONTROL)
                                    {
                                        let _ = disable_raw_mode();
                                        panic!("Ctrl-X or Ctrl-C pressed");
                                    }
                                    self.add_input(c.to_string().chars())
                                }
                                KeyCode::Enter => self.add_input("\r".chars()),
                                KeyCode::Backspace => self.add_input("\x08".chars()),
                                KeyCode::Esc => self.add_input("\x1B".chars()),
                                KeyCode::Tab => self.add_input("\x09".chars()),
                                KeyCode::Delete => self.add_input("\x7F".chars()),

                                KeyCode::Insert => self.add_input("\x1B[2~".chars()),
                                KeyCode::Home => self.add_input("\x1B[H".chars()),
                                KeyCode::End => self.add_input("\x1B[F".chars()),
                                KeyCode::Up => self.add_input("\x1B[A".chars()),
                                KeyCode::Down => self.add_input("\x1B[B".chars()),
                                KeyCode::Right => self.add_input("\x1B[C".chars()),
                                KeyCode::Left => self.add_input("\x1B[D".chars()),
                                KeyCode::PageUp => self.add_input("\x1B[5~".chars()),
                                KeyCode::PageDown => self.add_input("\x1B[6~".chars()),
                                _ => {}
                            }
                        }
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }
        restore_terminal()?;
        Ok(())
    }

    fn ui(&self, frame: &mut Frame, board: &IcyBoard) {
        let area = Rect::new(
            0,
            0,
            frame.size().width.min(80),
            frame.size().height.min(24),
        );
        frame.render_widget(self.main_canvas(area), area);

        let area = Rect::new(
            0,
            (frame.size().height - 1).min(24),
            frame.size().width.min(80),
            1,
        );
        frame.render_widget(self.status_bar(board), area);

        if let Ok(b) = self.screen.lock() {
            let p = b.caret.get_position();

            frame.set_cursor(p.x as u16, (p.y - b.buffer.get_first_visible_line()) as u16);
        }
    }

    fn status_bar(&self, board: &IcyBoard) -> impl Widget + '_ {
        let user_name;
        let current_conf;
        let cur_security;
        let times_on;
        let up;
        let upbytes;
        let dn;
        let dnbytes;
        if let Ok(user) = self.session.lock() {
            current_conf = user.current_conference_number;
            cur_security = user.cur_security;
            if user.cur_user >= 0 {
                user_name = board.users[user.cur_user as usize].get_name().clone();
                times_on = board.users[user.cur_user as usize].stats.num_times_on;
                upbytes = board.users[user.cur_user as usize].stats.ul_tot_upld_bytes;
                up = board.users[user.cur_user as usize].stats.num_uploads;
                dnbytes = board.users[user.cur_user as usize].stats.ul_tot_dnld_bytes;
                dn = board.users[user.cur_user as usize].stats.num_downloads;
            } else {
                user_name = String::new();
                times_on = 0;
                up = 0;
                dn = 0;
                upbytes = 0;
                dnbytes = 0;
            }
        } else {
            user_name = String::new();
            current_conf = 0;
            cur_security = 0;
            times_on = 0;
            up = 0;
            dn = 0;
            upbytes = 0;
            dnbytes = 0;
        }

        Canvas::default()
            .paint(move |ctx| match self.status_bar {
                0 => {
                    let now = Local::now();
                    ctx.print(
                        0.0,
                        0.0,
                        Line::from(format!(
                            "(Local) {} Sec({})= {} Times On={}",
                            user_name, current_conf, cur_security, times_on
                        ))
                        .style(Style::new().fg(DOS_BLACK)),
                    );
                    ctx.print(
                        56.0,
                        0.0,
                        Line::from("ALT-H=Help".to_string()).style(Style::new().fg(DOS_BLACK)),
                    );

                    let t = now.time();
                    let d = now.date_naive();

                    ctx.print(
                        67.0,
                        0.0,
                        Line::from(format!(
                            "{:02}/{:02}/{:02} {}:{}",
                            d.month0() + 1,
                            d.day(),
                            d.year_ce().1 % 100,
                            t.hour(),
                            t.minute()
                        ))
                        .style(Style::new().fg(DOS_BLACK)),
                    );
                }
                1 => {
                    ctx.print(
                        0.0,
                        0.0,
                        Line::from(format!(
                            "U/L:{} ({}kb) D/L{} ({}kb) ",
                            up,
                            upbytes / 1024,
                            dn,
                            dnbytes / 1024
                        ))
                        .style(Style::new().fg(DOS_BLACK)),
                    );
                }
                _ => {}
            })
            .background_color(crate::call_wait_screen::DOS_LIGHTGRAY)
            .x_bounds([0.0, 80.0])
    }

    fn main_canvas(&self, area: Rect) -> impl Widget + '_ {
        Canvas::default()
            .paint(move |ctx| {
                if let Ok(screen) = self.screen.lock() {
                    let buffer = &screen.buffer;
                    for y in 0..area.height as i32 {
                        for x in 0..area.width as i32 {
                            let c = buffer.get_char((x, y + buffer.get_first_visible_line()));
                            let mut fg = c.attribute.get_foreground();
                            if c.attribute.is_bold() {
                                fg += 8;
                            }
                            let fg = buffer.palette.get_color(fg).get_rgb();
                            let bg = buffer
                                .palette
                                .get_color(c.attribute.get_background())
                                .get_rgb();
                            if c.attribute.get_foreground() == 14 {
                                println!("{} - {:?}", c.attribute.get_foreground(), fg);
                            }
                            let out_char = Line::from(c.ch.to_string()).style(
                                Style::new()
                                    .bg(Color::Rgb(bg.0, bg.1, bg.2))
                                    .fg(Color::Rgb(fg.0, fg.1, fg.2)),
                            );

                            ctx.print(
                                x as f64 + 1.0,
                                (area.height as i32 - 1 - y) as f64,
                                out_char,
                            );
                        }
                    }
                }
            })
            .background_color(Color::Black)
            .x_bounds([0.0, area.width as f64])
            .y_bounds([0.0, area.height as f64])
    }

    fn add_input(&self, c_seq: std::str::Chars<'_>) {
        for c in c_seq {
            self.input_buffer.lock().unwrap().push_back((true, c));
        }
    }
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

pub fn print_exit_screen() {
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout())).unwrap();
    stdout()
        .execute(Clear(crossterm::terminal::ClearType::All))
        .unwrap();
    terminal
        .draw(|frame| {
            let mut r = frame.size();
            r.height = 1;
            let white = Style::default().fg(DOS_WHITE);
            let green = Style::default().fg(DOS_LIGHT_GREEN);

            let text = vec![
                Span::styled("Thank you for using ", green),
                Span::styled("IcyBoard", white),
                Span::styled(" Professional BBS Software!", green),
            ];
            frame.render_widget(
                Paragraph::new(Line::from(text))
                    .alignment(Alignment::Center)
                    .bg(DOS_BLUE),
                r,
            )
        })
        .unwrap();
    stdout().execute(MoveTo(0, 1)).unwrap();
}
