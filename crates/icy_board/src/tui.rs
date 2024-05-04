use std::{
    collections::HashMap,
    io::{self, stdout, Stdout},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::{
    bbs::{handle_client, LoginOptions},
    terminal_thread::SendData,
    Res,
};
use chrono::Utc;
use crossterm::{
    cursor::MoveTo,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use icy_board_engine::icy_board::{
    state::{GraphicsMode, NodeState},
    IcyBoard, IcyBoardError,
};
use icy_board_tui::{
    get_text_args,
    theme::{DOS_BLACK, DOS_BLUE, DOS_LIGHT_GRAY, DOS_LIGHT_GREEN, DOS_RED, DOS_WHITE, DOS_YELLOW},
};
use icy_engine::TextPane;
use icy_net::{channel::ChannelConnection, ConnectionType};
use ratatui::{prelude::*, widgets::Paragraph};
use tokio::sync::mpsc;

use crate::{bbs::BBS, icy_engine_output::Screen};

pub struct Tui {
    screen: Arc<Mutex<Screen>>,
    tx: mpsc::Sender<SendData>,
    status_bar: usize,
    handle: Arc<Mutex<Vec<Option<NodeState>>>>,
    node: usize,
    node_state: Arc<Mutex<Vec<Option<NodeState>>>>,
}

impl Tui {
    pub fn local_mode(board: &Arc<tokio::sync::Mutex<IcyBoard>>, bbs: &Arc<Mutex<BBS>>, login_sysop: bool, ppe: Option<PathBuf>) -> Self {
        let board = board.clone();
        let ui_node = bbs.lock().unwrap().create_new_node(ConnectionType::Channel);
        let node_state = bbs.lock().unwrap().open_connections.clone();
        let node = ui_node.clone();
        let screen = Arc::new(Mutex::new(Screen::new()));
        let (ui_connection, connection) = ChannelConnection::create_pair();
        let node_state2 = node_state.clone();

        let options = LoginOptions { login_sysop, ppe };
        let handle = std::thread::Builder::new()
            .name("Local mode handle".to_string())
            .spawn(move || {
                tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                    if let Err(err) = handle_client(board, node_state2, node, Box::new(connection), Some(options)).await {
                        log::error!("Error running backround client: {}", err);
                    }
                });
            })
            .unwrap();
        bbs.lock().unwrap().get_open_connections().as_ref().lock().unwrap()[node]
            .as_mut()
            .unwrap()
            .handle = Some(handle);
        let (_handle2, tx) = crate::terminal_thread::start_update_thread(Box::new(ui_connection), screen.clone());

        Self {
            screen,
            tx,
            status_bar: 0,
            node,
            node_state,
            handle: bbs.lock().unwrap().get_open_connections().clone(),
        }
    }
    pub fn sysop_mode(bbs: &Arc<Mutex<BBS>>, node: usize) -> Res<Self> {
        let (ui_connection, connection) = ChannelConnection::create_pair();
        if let Ok(bbs) = &mut bbs.lock() {
            log::info!("Creating sysop mode");
            let node_state = bbs.open_connections.clone();

            bbs.get_open_connections().lock().unwrap()[node].as_mut().unwrap().sysop_connection = Some(connection);

            let screen = Arc::new(Mutex::new(Screen::new()));
            log::info!("Run terminal thread");
            let (_handle2, tx) = crate::terminal_thread::start_update_thread(Box::new(ui_connection), screen.clone());

            Ok(Self {
                screen,
                tx,
                status_bar: 0,
                node,
                node_state,
                handle: bbs.get_open_connections().clone(),
            })
        } else {
            return Err(Box::new(IcyBoardError::Error("Node not found".to_string())));
        }
    }

    pub async fn run(&mut self, board: &Arc<tokio::sync::Mutex<IcyBoard>>) -> Res<()> {
        let mut terminal = init_terminal()?;
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(20);
        terminal.clear()?;
        //   let mut redraw = true;
        loop {
            if self.handle.lock().unwrap()[self.node].as_ref().unwrap().handle.as_ref().unwrap().is_finished() {
                restore_terminal()?;
                let handle = self.handle.lock().unwrap()[self.node].as_mut().unwrap().handle.take().unwrap();
                if let Err(_err) = handle.join() {
                    return Err(Box::new(IcyBoardError::ThreadCrashed));
                }
                return Ok(());
            }

            //if redraw
            {
                //  redraw = false;
                let board = board.clone();
                let _ = terminal.draw(|frame| {
                    self.ui(frame, board);
                });
            }
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        if key.modifiers.contains(KeyModifiers::ALT) {
                            match key.code {
                                KeyCode::Char('h') => {
                                    self.status_bar = (self.status_bar + 1) % 4;
                                    //redraw = true;
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
                                        self.add_input(((c as u8 - b'a' + 1) as char).to_string().chars()).await?;
                                    }
                                }

                                KeyCode::Left => self.add_input("\x01".chars()).await?,
                                KeyCode::Right => self.add_input("\x06".chars()).await?,
                                KeyCode::End => self.add_input("\x0B".chars()).await?,
                                _ => {}
                            }
                        } else {
                            match key.code {
                                KeyCode::Char(c) => self.add_input(c.to_string().chars()).await?,
                                KeyCode::Enter => self.add_input("\r".chars()).await?,
                                KeyCode::Backspace => self.add_input("\x08".chars()).await?,
                                KeyCode::Esc => self.add_input("\x1B".chars()).await?,
                                KeyCode::Tab => self.add_input("\x09".chars()).await?,
                                KeyCode::Delete => self.add_input("\x7F".chars()).await?,
                                KeyCode::Insert => self.add_input("\x1B[2~".chars()).await?,
                                KeyCode::Home => self.add_input("\x1B[H".chars()).await?,
                                KeyCode::End => self.add_input("\x1B[F".chars()).await?,
                                KeyCode::Up => self.add_input("\x1B[A".chars()).await?,
                                KeyCode::Down => self.add_input("\x1B[B".chars()).await?,
                                KeyCode::Right => self.add_input("\x1B[C".chars()).await?,
                                KeyCode::Left => self.add_input("\x1B[D".chars()).await?,
                                KeyCode::PageUp => self.add_input("\x1B[5~".chars()).await?,
                                KeyCode::PageDown => self.add_input("\x1B[6~".chars()).await?,
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

    fn ui(&self, frame: &mut Frame, board: Arc<tokio::sync::Mutex<IcyBoard>>) {
        let width = frame.size().width.min(80);
        let height = frame.size().height.min(24);

        let mut area = Rect::new((frame.size().width - width) / 2, (frame.size().height - height) / 2, width, height);

        let screen = &self.screen.lock().unwrap();
        let buffer = &screen.buffer;
        for y in 0..area.height as i32 {
            for x in 0..area.width as i32 {
                let c = buffer.get_char((x, y + buffer.get_first_visible_line()));
                let mut fg = c.attribute.get_foreground();
                if c.attribute.is_bold() {
                    fg += 8;
                }
                let fg = buffer.palette.get_color(fg).get_rgb();
                let bg = buffer.palette.get_color(c.attribute.get_background()).get_rgb();
                let mut s: Style = Style::new().bg(Color::Rgb(bg.0, bg.1, bg.2)).fg(Color::Rgb(fg.0, fg.1, fg.2));
                if c.attribute.is_blinking() {
                    s = s.slow_blink();
                }
                let span = Span::from(c.ch.to_string()).style(s);
                frame.buffer_mut().set_span(area.x + x as u16, area.y + y as u16, &span, 1);
            }
        }
        let y = area.y as u16;
        area.y += area.height;
        area.height = 2;

        self.draw_statusbar(frame, board, area);
        let pos: icy_engine::Position = screen.caret.get_position();
        frame.set_cursor(area.x + pos.x as u16, y + pos.y as u16 - buffer.get_first_visible_line() as u16);
    }

    fn draw_statusbar(&self, frame: &mut Frame, board: Arc<tokio::sync::Mutex<IcyBoard>>, area: Rect) {
        let mut user_name = String::new();
        let mut city = String::new();
        let mut current_conf = 0;
        let mut cur_security = 0;
        let mut times_on = 0;
        let mut up = 0;
        let mut dn = 0;
        let mut upbytes = 0;
        let mut dnbytes = 0;
        let mut graphics_mode = GraphicsMode::Ansi;
        let mut last_on = Utc::now();
        let mut logon_time = Utc::now();
        let mut msg_left = 0;
        let mut msg_read = 0;
        let mut today_ul = 0;
        let mut today_dn = 0;
        let mut bus_phone = String::new();
        let mut home_phone = String::new();
        let mut email = String::new();
        let mut cmt1 = String::new();
        let mut cmt2 = String::new();
        let mut date_format = String::new();

        if let Some(node_state) = &self.node_state.lock().unwrap()[self.node as usize] {
            current_conf = node_state.cur_conference;
            graphics_mode = node_state.graphics_mode;
            logon_time = node_state.logon_time;
            if node_state.cur_user >= 0 { /*
                 tokio::runtime::Builder::new_multi_thread()
                 .enable_all()
                 .build()
                 .unwrap()
                 .block_on(async {
                     let board = board.lock().await;
                     cur_security = board.users[node_state.cur_user as usize].security_level;
                     user_name = board.users[node_state.cur_user as usize].get_name().clone();
                     city = board.users[node_state.cur_user as usize].city_or_state.clone();
                     times_on = board.users[node_state.cur_user as usize].stats.num_times_on;
                     upbytes = board.users[node_state.cur_user as usize].stats.total_upld_bytes;
                     up = board.users[node_state.cur_user as usize].stats.num_uploads;
                     dnbytes = board.users[node_state.cur_user as usize].stats.total_dnld_bytes;
                     dn = board.users[node_state.cur_user as usize].stats.num_downloads;
                     today_dn = board.users[node_state.cur_user as usize].stats.today_dnld_bytes;
                     today_ul = board.users[node_state.cur_user as usize].stats.total_dnld_bytes;
                     last_on = board.users[node_state.cur_user as usize].stats.last_on;
                     msg_left = board.users[node_state.cur_user as usize].stats.messages_left;
                     msg_read = board.users[node_state.cur_user as usize].stats.messages_read;
                     bus_phone = board.users[node_state.cur_user as usize].bus_data_phone.clone();
                     home_phone = board.users[node_state.cur_user as usize].home_voice_phone.clone();
                     email = board.users[node_state.cur_user as usize].email.clone();
                     cmt1 = board.users[node_state.cur_user as usize].user_comment.clone();
                     cmt2 = board.users[node_state.cur_user as usize].sysop_comment.clone();
                     date_format = board.config.board.date_format.clone();
                 });*/
            }
        }

        frame.buffer_mut().set_style(area, Style::new().bg(DOS_LIGHT_GRAY));

        match self.status_bar {
            0 => {
                let connection = "Local";

                let line = Line::from(vec![
                    Span::from(format!("{}", self.node + 1)).style(Style::new().fg(DOS_YELLOW).bg(DOS_RED)),
                    Span::from(format!("({}) {} - {}", connection, user_name, city,)).style(Style::new().fg(DOS_BLACK).bg(DOS_LIGHT_GRAY)),
                ]);
                frame.buffer_mut().set_line(area.x, area.y, &line, area.width);

                let graphics = match graphics_mode {
                    GraphicsMode::Ctty => "N",
                    GraphicsMode::Ansi => "A",
                    GraphicsMode::Graphics => "G",
                    GraphicsMode::Avatar => "V",
                    GraphicsMode::Rip => "R",
                };

                let line = Line::from(vec![Span::from(format!(
                    "{} ({})  Sec({})={}  Times On={}  Up:Dn={}:{}",
                    graphics,
                    last_on.format(&date_format),
                    current_conf,
                    cur_security,
                    times_on,
                    up,
                    dn
                ))
                .style(Style::new().fg(DOS_BLACK).bg(DOS_LIGHT_GRAY))]);
                frame.buffer_mut().set_line(area.x, area.y + 1, &line, area.width);
                let hlp = "ALT-H=Help".to_string();
                let len = hlp.len() as u16;
                frame
                    .buffer_mut()
                    .set_span(area.x + 56, area.y, &Span::from(hlp).style(Style::new().fg(DOS_BLACK).bg(DOS_LIGHT_GRAY)), len);

                let min_on = (Utc::now() - logon_time).num_minutes();

                let time = format!("{:<3} {}", min_on, logon_time.format("%H:%M"));
                let len = time.len() as u16;
                frame.buffer_mut().set_span(
                    area.x + area.width - len - 2,
                    area.y,
                    &Span::from(time).style(Style::new().fg(DOS_BLACK).bg(DOS_LIGHT_GRAY)),
                    len,
                );

                let time = format!("{}", Utc::now().format("%H:%M"));
                let len = time.len() as u16;
                frame.buffer_mut().set_span(
                    area.x + area.width - len - 2,
                    area.y + 1,
                    &Span::from(time).style(Style::new().fg(DOS_BLACK).bg(DOS_LIGHT_GRAY)),
                    len,
                );
            }
            1 => {
                let line = Line::from(vec![
                    Span::from(format!("{}", self.node + 1)).style(Style::new().fg(DOS_YELLOW).bg(DOS_RED)),
                    Span::from(format!(" Alt-> X=OS")).style(Style::new().fg(DOS_BLACK).bg(DOS_LIGHT_GRAY)),
                ]);
                frame.buffer_mut().set_line(area.x, area.y, &line, area.width);
            }
            2 => {
                let line = Line::from(vec![
                    Span::from(format!("{}", self.node + 1)).style(Style::new().fg(DOS_YELLOW).bg(DOS_RED)),
                    Span::from(format!("{} / {} mail: {}", bus_phone, home_phone, email)).style(Style::new().fg(DOS_BLACK).bg(DOS_LIGHT_GRAY)),
                ]);
                frame.buffer_mut().set_line(area.x, area.y, &line, area.width);

                let line = Line::from(vec![
                    Span::from(format!("  C1: {:40} C2: {:40}", cmt1, cmt2)).style(Style::new().fg(DOS_BLACK).bg(DOS_LIGHT_GRAY))
                ]);
                frame.buffer_mut().set_line(area.x, area.y + 1, &line, area.width);
            }
            3 => {
                let line = Line::from(vec![
                    Span::from(format!("{}", self.node + 1)).style(Style::new().fg(DOS_YELLOW).bg(DOS_RED)),
                    Span::from(format!(" Msgs Left: {:7}  Files U/L: {:7}  Bytes U/L: {:7}", msg_left, up, upbytes,))
                        .style(Style::new().fg(DOS_BLACK).bg(DOS_LIGHT_GRAY)),
                ]);
                frame.buffer_mut().set_line(area.x, area.y, &line, area.width);

                let line = Line::from(vec![Span::from(format!(
                    "  Msgs Read: {:7}  Files D/L: {:7}  Bytes D/L: {:7}  Today: {:7}",
                    msg_read,
                    dn,
                    dnbytes,
                    today_dn as i64 + today_ul as i64,
                ))
                .style(Style::new().fg(DOS_BLACK).bg(DOS_LIGHT_GRAY))]);
                frame.buffer_mut().set_line(area.x, area.y + 1, &line, area.width);
            }
            _ => {}
        }

        /*
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
            .x_bounds([0.0, 80.0])*/
    }

    async fn add_input(&mut self, c_seq: std::str::Chars<'_>) -> Res<()> {
        let mut s = Vec::new();
        for c in c_seq {
            s.push(c as u8);
        }
        let _res = self.tx.send(SendData::Data(s)).await;
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
