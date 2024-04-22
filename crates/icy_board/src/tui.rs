use std::{
    collections::HashMap,
    io::{self, stdout, Read, Stdout, Write},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use chrono::{Datelike, Local, Timelike};
use crossterm::{
    cursor::MoveTo,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};

use crate::Res;
use icy_board_engine::{
    icy_board::{
        state::{IcyBoardState, NodeState, Session},
        IcyBoard, IcyBoardError,
    },
    vm::TerminalTarget,
};
use icy_board_tui::{
    get_text_args,
    theme::{DOS_BLACK, DOS_BLUE, DOS_LIGHT_GRAY, DOS_LIGHT_GREEN, DOS_WHITE},
};
use icy_engine::{ansi, TextPane};
use icy_net::{channel::ChannelConnection, ConnectionType};
use ratatui::{
    prelude::*,
    widgets::{canvas::Canvas, Paragraph},
};

use crate::{bbs::BBS, icy_engine_output::Screen, menu_runner::PcbBoardCommand};

pub struct Tui {
    screen: Screen,
    session: Arc<Mutex<Session>>,
    connection: ChannelConnection,
    status_bar: usize,
    handle: Arc<Mutex<Vec<Option<NodeState>>>>,
    node: usize,
}

impl Tui {
    pub fn local_mode(board: &Arc<Mutex<IcyBoard>>, bbs: &Arc<Mutex<BBS>>, sysop_mode: bool) -> Self {
        let mut session = Session::new();
        session.is_local = true;
        let ui_session = Arc::new(Mutex::new(session));
        let session = ui_session.clone();
        let (ui_connection, connection) = ChannelConnection::create_pair();
        let board = board.clone();
        let ui_node = bbs.lock().unwrap().create_new_node(ConnectionType::Channel);
        let node_state = bbs.lock().unwrap().open_connections.clone();
        let node = ui_node.clone();
        let handle = thread::spawn(move || {
            let mut state = IcyBoardState::new(board, node_state, node, Box::new(connection));
            if sysop_mode {
                state.session.is_sysop = true;
                state.set_current_user(0).unwrap();
            }
            let mut cmd = PcbBoardCommand::new(state);

            let orig_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |panic_info| {
                log::error!("IcyBoard thread crashed at {:?}", panic_info.location());
                log::error!("full info: {:?}", panic_info);

                orig_hook(panic_info);
            }));

            if !sysop_mode {
                match cmd.login() {
                    Ok(true) => {}
                    Ok(false) => {
                        return Ok(());
                    }
                    Err(err) => {
                        log::error!("error during login process {}", err);
                        return Ok(());
                    }
                }
            }

            loop {
                session.lock().unwrap().cur_user = cmd.state.session.cur_user;
                session.lock().unwrap().current_conference = cmd.state.session.current_conference.clone();
                session.lock().unwrap().disp_options = cmd.state.session.disp_options.clone();

                if let Err(err) = cmd.do_command() {
                    cmd.state.session.disp_options.reset_printout();
                    log::error!("session thread 'do_command': {}", err);
                    if cmd.state.set_color(TerminalTarget::Both, 4.into()).is_ok() {
                        let _ = cmd.state.print(TerminalTarget::Both, &format!("\r\nError: {}\r\n\r\n", err));
                        let _ = cmd.state.reset_color();
                    }
                    return Ok(());
                }
                cmd.state.session.disp_options.reset_printout();
                if cmd.state.session.request_logoff {
                    let _ = cmd.state.connection.shutdown();
                    return Ok(());
                }
                thread::sleep(Duration::from_millis(20));
            }
        });
        bbs.lock().unwrap().get_open_connections().as_ref().lock().unwrap()[node]
            .as_mut()
            .unwrap()
            .handle = Some(handle);

        Self {
            screen: Screen::new(),
            session: ui_session,
            connection: ui_connection,
            status_bar: 0,
            node,
            handle: bbs.lock().unwrap().get_open_connections().clone(),
        }
    }

    pub fn sysop_mode(bbs: &Arc<Mutex<BBS>>, node: usize) -> Res<Self> {
        let ui_session = Arc::new(Mutex::new(Session::new()));
        let (ui_connection, connection) = ChannelConnection::create_pair();
        if let Ok(bbs) = &mut bbs.lock() {
            /* let Some(node) = bbs.get_node(node) else {
                return Err(Box::new(IcyBoardError::Error("Node not found".to_string())));
            };*/
            bbs.get_open_connections().lock().unwrap()[node]
                .as_mut()
                .unwrap()
                .connections
                .lock()
                .unwrap()
                .push(Box::new(connection));

            Ok(Self {
                screen: Screen::new(),
                session: ui_session,
                connection: ui_connection,
                status_bar: 0,
                node,
                handle: bbs.get_open_connections().clone(),
            })
        } else {
            return Err(Box::new(IcyBoardError::Error("Node not found".to_string())));
        }
    }

    pub fn run(&mut self, board: &Arc<Mutex<IcyBoard>>) -> Res<()> {
        let mut terminal = init_terminal()?;
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(20);
        let mut redraw = true;
        loop {
            let mut screen_buf = [0; 1024 * 16];
            let mut parser = ansi::Parser::default();
            parser.bs_is_ctrl_char = true;
            loop {
                match self.connection.read(&mut screen_buf) {
                    Ok(size) => {
                        if size == 0 {
                            break;
                        }
                        redraw = true;
                        for &b in screen_buf[0..size].iter() {
                            self.screen.print(&mut parser, codepages::tables::CP437_TO_UNICODE[b as usize]);
                        }
                    }
                    Err(_) => {
                        // channel closed
                        return Ok(());
                    }
                }
            }
            if self.handle.lock().unwrap()[self.node].as_ref().unwrap().handle.as_ref().unwrap().is_finished() {
                restore_terminal()?;
                let handle = self.handle.lock().unwrap()[self.node].as_mut().unwrap().handle.take().unwrap();
                if let Err(_err) = handle.join() {
                    return Err(Box::new(IcyBoardError::ThreadCrashed));
                }
                return Ok(());
            }

            if redraw {
                redraw = false;
                let board = board.clone();
                let _ = terminal.draw(|frame| {
                    if let Ok(board) = &board.lock() {
                        self.ui(frame, board);
                    }
                });
            }
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        if key.modifiers.contains(KeyModifiers::ALT) {
                            match key.code {
                                KeyCode::Char('h') => {
                                    self.status_bar = (self.status_bar + 1) % 2;
                                    redraw = true;
                                }
                                KeyCode::Char('x') => {
                                    let _ = restore_terminal();

                                    return Ok(());
                                }
                                _ => {}
                            }
                        } else if key.modifiers.contains(KeyModifiers::CONTROL) {
                            match key.code {
                                KeyCode::Char(c) => {
                                    if c == 'x' || c == 'c' {
                                        let _ = disable_raw_mode();
                                        return Ok(());
                                    }
                                    if ('a'..='z').contains(&c) {
                                        self.add_input(((c as u8 - b'a' + 1) as char).to_string().chars())?;
                                    }
                                }

                                KeyCode::Left => self.add_input("\x01".chars())?,
                                KeyCode::Right => self.add_input("\x06".chars())?,
                                KeyCode::End => self.add_input("\x0B".chars())?,
                                _ => {}
                            }
                        } else {
                            match key.code {
                                KeyCode::Char(c) => self.add_input(c.to_string().chars())?,
                                KeyCode::Enter => self.add_input("\r".chars())?,
                                KeyCode::Backspace => self.add_input("\x08".chars())?,
                                KeyCode::Esc => self.add_input("\x1B".chars())?,
                                KeyCode::Tab => self.add_input("\x09".chars())?,
                                KeyCode::Delete => self.add_input("\x7F".chars())?,

                                KeyCode::Insert => self.add_input("\x1B[2~".chars())?,
                                KeyCode::Home => self.add_input("\x1B[H".chars())?,
                                KeyCode::End => self.add_input("\x1B[F".chars())?,
                                KeyCode::Up => self.add_input("\x1B[A".chars())?,
                                KeyCode::Down => self.add_input("\x1B[B".chars())?,
                                KeyCode::Right => self.add_input("\x1B[C".chars())?,
                                KeyCode::Left => self.add_input("\x1B[D".chars())?,
                                KeyCode::PageUp => self.add_input("\x1B[5~".chars())?,
                                KeyCode::PageDown => self.add_input("\x1B[6~".chars())?,
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
    }

    fn ui(&self, frame: &mut Frame, board: &IcyBoard) {
        let area = Rect::new(0, 0, frame.size().width.min(80), frame.size().height.min(24));
        frame.render_widget(self.main_canvas(area), area);

        let area = Rect::new(0, (frame.size().height - 1).min(24), frame.size().width.min(80), 1);
        frame.render_widget(self.status_bar(board), area);
        let p = self.screen.caret.get_position();
        frame.set_cursor(p.x.clamp(0, 80) as u16, (p.y - self.screen.buffer.get_first_visible_line()).clamp(0, 25) as u16);
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
                        Line::from(format!("(Local) {} Sec({})= {} Times On={}", user_name, current_conf, cur_security, times_on))
                            .style(Style::new().fg(DOS_BLACK)),
                    );
                    ctx.print(56.0, 0.0, Line::from("ALT-H=Help".to_string()).style(Style::new().fg(DOS_BLACK)));

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
                        Line::from(format!("U/L:{} ({}kb) D/L{} ({}kb) ", up, upbytes / 1024, dn, dnbytes / 1024)).style(Style::new().fg(DOS_BLACK)),
                    );
                }
                _ => {}
            })
            .background_color(DOS_LIGHT_GRAY)
            .x_bounds([0.0, 80.0])
    }

    fn main_canvas(&self, area: Rect) -> impl Widget + '_ {
        Canvas::default()
            .paint(move |ctx| {
                let buffer = &self.screen.buffer;
                for y in 0..area.height as i32 {
                    for x in 0..area.width as i32 {
                        let c = buffer.get_char((x, y + buffer.get_first_visible_line()));
                        let mut fg = c.attribute.get_foreground();
                        if c.attribute.is_bold() {
                            fg += 8;
                        }
                        let fg = buffer.palette.get_color(fg).get_rgb();
                        let bg = buffer.palette.get_color(c.attribute.get_background()).get_rgb();
                        let mut s = Style::new().bg(Color::Rgb(bg.0, bg.1, bg.2)).fg(Color::Rgb(fg.0, fg.1, fg.2));
                        if c.attribute.is_blinking() {
                            s = s.slow_blink();
                        }
                        let out_char = Line::from(c.ch.to_string()).style(s);

                        ctx.print(x as f64 + 1.0, (area.height as i32 - 1 - y) as f64, out_char);
                    }
                }
            })
            .background_color(Color::Black)
            .x_bounds([0.0, area.width as f64])
            .y_bounds([0.0, area.height as f64])
    }

    fn add_input(&mut self, c_seq: std::str::Chars<'_>) -> Res<()> {
        let mut s = String::new();
        for c in c_seq {
            s.push(c);
        }

        if let Err(_) = self.connection.write_all(s.as_bytes()) {
            return Ok(());
        }
        Ok(())
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
    stdout().execute(Clear(crossterm::terminal::ClearType::All)).unwrap();
    terminal
        .draw(|frame| {
            let mut r = frame.size();
            r.height = 1;
            let white = Style::default().fg(DOS_WHITE);
            let green = Style::default().fg(DOS_LIGHT_GREEN);
            let mut map = HashMap::new();
            map.insert("name".to_string(), "@".to_string());
            let txt = get_text_args("exit_icy_board_msg", map);
            let p1 = txt[0..txt.as_bytes().iter().position(|c| *c == b'@').unwrap()].to_string();
            let p2 = txt[txt.as_bytes().iter().position(|c| *c == b'@').unwrap() + 1..].to_string();
            let text = vec![Span::styled(p1, green), Span::styled("IcyBoard", white), Span::styled(p2, green)];
            frame.render_widget(Paragraph::new(Line::from(text)).alignment(Alignment::Center).bg(DOS_BLUE), r)
        })
        .unwrap();
    stdout().execute(MoveTo(0, 1)).unwrap();
}
