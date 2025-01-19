use std::{
    collections::HashMap,
    io::{self, stdout, Stdout},
    path::PathBuf,
    sync::Arc,
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
    terminal::Clear,
    ExecutableCommand,
};
use icy_board_engine::icy_board::{
    bbs::{BBSMessage, BBS},
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
use tokio::sync::{mpsc, Mutex};

use crate::icy_engine_output::Screen;

pub struct Tui {
    sysop_mode: bool,
    screen: Arc<std::sync::Mutex<Screen>>,
    tx: mpsc::Sender<SendData>,
    status_bar: usize,
    handle: Arc<Mutex<Vec<Option<NodeState>>>>,
    node: usize,
    node_state: Arc<Mutex<Vec<Option<NodeState>>>>,
}

impl Tui {
    pub async fn local_mode(board: &Arc<tokio::sync::Mutex<IcyBoard>>, bbs: &Arc<Mutex<BBS>>, login_sysop: bool, ppe: Option<PathBuf>) -> Self {
        let board = board.clone();
        let bbs2 = bbs.clone();

        let ui_node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let node_state = bbs.lock().await.open_connections.clone();
        let node = ui_node.clone();
        let screen = Arc::new(std::sync::Mutex::new(Screen::new()));
        let (ui_connection, connection) = ChannelConnection::create_pair();
        let node_state2 = node_state.clone();

        let options = LoginOptions { login_sysop, ppe, local: true };
        let handle = std::thread::Builder::new()
            .name("Local mode handle".to_string())
            .spawn(move || {
                tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                    if let Err(err) = handle_client(bbs2, board, node_state2, node, Box::new(connection), Some(options)).await {
                        log::error!("Error running backround client: {}", err);
                    }
                });
            })
            .unwrap();
        bbs.lock().await.get_open_connections().await.as_ref().lock().await[node]
            .as_mut()
            .unwrap()
            .handle = Some(handle);
        let (_handle2, tx) = crate::terminal_thread::start_update_thread(Box::new(ui_connection), screen.clone());

        Self {
            sysop_mode: false,
            screen,
            tx,
            status_bar: 0,
            node,
            node_state,
            handle: bbs.lock().await.get_open_connections().await.clone(),
        }
    }

    async fn logoff_sysop(&self, bbs: &mut Arc<Mutex<BBS>>) -> Res<()> {
        if self.sysop_mode {
            bbs.lock().await.bbs_channels[self.node].as_ref().unwrap().send(BBSMessage::SysopLogout).await?;
        }
        Ok(())
    }

    pub async fn sysop_mode(bbs: &Arc<Mutex<BBS>>, node: usize) -> Res<Self> {
        let (ui_connection, connection) = ChannelConnection::create_pair();
        let mut bbs = bbs.lock().await;
        log::info!("Creating sysop mode");
        let node_state = bbs.open_connections.clone();

        if let Some(node_state) = bbs.get_open_connections().await.lock().await[node].as_mut() {
            node_state.sysop_connection = Some(connection);
        }
        bbs.bbs_channels[node].as_ref().unwrap().send(BBSMessage::SysopLogin).await?;

        let screen = Arc::new(std::sync::Mutex::new(Screen::new()));
        log::info!("Run terminal thread");
        let (_handle2, tx) = crate::terminal_thread::start_update_thread(Box::new(ui_connection), screen.clone());

        Ok(Self {
            sysop_mode: true,
            screen,
            tx,
            status_bar: 0,
            node,
            node_state,
            handle: bbs.get_open_connections().await.clone(),
        })
    }

    pub async fn run(&mut self, bbs: &mut Arc<Mutex<BBS>>, board: &Arc<tokio::sync::Mutex<IcyBoard>>) -> Res<()> {
        let mut terminal = init_terminal()?;
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(20);
        terminal.clear()?;
        //   let mut redraw = true;
        loop {
            if let Some(Some(node_state)) = self.handle.lock().await.get_mut(self.node) {
                if let Some(handle) = node_state.handle.as_ref() {
                    if handle.is_finished() {
                        let handle = node_state.handle.take().unwrap();
                        if let Err(_err) = handle.join() {
                            return Err(Box::new(IcyBoardError::ThreadCrashed));
                        }
                        return Ok(());
                    }
                } else {
                    // thread has gone
                    return Ok(());
                }
            }
            //if redraw
            {
                //  redraw = false;
                let status_bar_info = StatusBarInfo::get_info(board, &self.node_state, self.node).await;
                let _ = terminal.draw(|frame| {
                    self.ui(frame, status_bar_info);
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
                                    self.logoff_sysop(bbs).await?;
                                    return Ok(());
                                }
                                _ => {}
                            }
                        } else if key.modifiers.contains(KeyModifiers::CONTROL) {
                            match key.code {
                                KeyCode::Char(c) => {
                                    if c == 'x' || c == 'c' {
                                        self.logoff_sysop(bbs).await?;
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

    fn ui(&self, frame: &mut Frame, status_bar_info: StatusBarInfo) {
        let width = frame.area().width.min(80);
        let height = frame.area().height.min(24);

        let mut area = Rect::new((frame.area().width - width) / 2, (frame.area().height - height) / 2, width, height);

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

        self.draw_statusbar(frame, area, status_bar_info);
        let pos: icy_engine::Position = screen.caret.get_position();
        frame.set_cursor_position((area.x + pos.x as u16, y + pos.y as u16 - buffer.get_first_visible_line() as u16));
    }

    fn draw_statusbar(&self, frame: &mut Frame, area: Rect, status_bar_info: StatusBarInfo) {
        frame.buffer_mut().set_style(area, Style::new().bg(DOS_LIGHT_GRAY));

        match self.status_bar {
            0 => {
                let connection = "Local";

                let line = Line::from(vec![
                    Span::from(format!("{}", self.node + 1)).style(Style::new().fg(DOS_YELLOW).bg(DOS_RED)),
                    Span::from(format!("({}) {} - {}", connection, status_bar_info.user_name, status_bar_info.city,))
                        .style(Style::new().fg(DOS_BLACK).bg(DOS_LIGHT_GRAY)),
                ]);
                frame.buffer_mut().set_line(area.x, area.y, &line, area.width);

                let graphics = match status_bar_info.graphics_mode {
                    GraphicsMode::Ctty => "N",
                    GraphicsMode::Ansi => "A",
                    GraphicsMode::Graphics => "G",
                    GraphicsMode::Avatar => "V",
                    GraphicsMode::Rip => "R",
                };

                let line = Line::from(vec![Span::from(format!(
                    "{} ({})  Sec({})={}  Times On={}  Up:Dn={}:{}",
                    graphics,
                    status_bar_info.last_on.format(&status_bar_info.date_format),
                    status_bar_info.current_conf,
                    status_bar_info.cur_security,
                    status_bar_info.times_on,
                    status_bar_info.up,
                    status_bar_info.dn
                ))
                .style(Style::new().fg(DOS_BLACK).bg(DOS_LIGHT_GRAY))]);
                frame.buffer_mut().set_line(area.x, area.y + 1, &line, area.width);
                let hlp = "ALT-H=Help".to_string();
                let len = hlp.len() as u16;
                frame
                    .buffer_mut()
                    .set_span(area.x + 56, area.y, &Span::from(hlp).style(Style::new().fg(DOS_BLACK).bg(DOS_LIGHT_GRAY)), len);

                let min_on = (Utc::now() - status_bar_info.logon_time).num_minutes();

                let time = format!("{:<3} {}", min_on, status_bar_info.logon_time.format("%H:%M"));
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
                    Span::from(format!(
                        "{} / {} mail: {}",
                        status_bar_info.bus_phone, status_bar_info.home_phone, status_bar_info.email
                    ))
                    .style(Style::new().fg(DOS_BLACK).bg(DOS_LIGHT_GRAY)),
                ]);
                frame.buffer_mut().set_line(area.x, area.y, &line, area.width);

                let line = Line::from(vec![Span::from(format!("  C1: {:40} C2: {:40}", status_bar_info.cmt1, status_bar_info.cmt2))
                    .style(Style::new().fg(DOS_BLACK).bg(DOS_LIGHT_GRAY))]);
                frame.buffer_mut().set_line(area.x, area.y + 1, &line, area.width);
            }
            3 => {
                let line = Line::from(vec![
                    Span::from(format!("{}", self.node + 1)).style(Style::new().fg(DOS_YELLOW).bg(DOS_RED)),
                    Span::from(format!(
                        " Msgs Left: {:7}  Files U/L: {:7}  Bytes U/L: {:7}",
                        status_bar_info.msg_left, status_bar_info.up, status_bar_info.upbytes,
                    ))
                    .style(Style::new().fg(DOS_BLACK).bg(DOS_LIGHT_GRAY)),
                ]);
                frame.buffer_mut().set_line(area.x, area.y, &line, area.width);

                let line = Line::from(vec![Span::from(format!(
                    "  Msgs Read: {:7}  Files D/L: {:7}  Bytes D/L: {:7}  Today: {:7}",
                    status_bar_info.msg_read,
                    status_bar_info.dn,
                    status_bar_info.dnbytes,
                    status_bar_info.today_dn as i64 + status_bar_info.today_ul as i64,
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
    Ok(ratatui::init())
}

fn restore_terminal() -> io::Result<()> {
    ratatui::restore();
    Ok(())
}

pub fn print_exit_screen() {
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout())).unwrap();
    stdout().execute(Clear(crossterm::terminal::ClearType::All)).unwrap();
    terminal
        .draw(|frame| {
            let mut r = frame.area();
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

#[derive(Default)]
pub struct StatusBarInfo {
    pub user_name: String,
    pub city: String,
    pub current_conf: u16,
    pub cur_security: u8,
    pub times_on: u64,
    pub up: u64,
    pub dn: u64,
    pub upbytes: u64,
    pub dnbytes: u64,
    pub graphics_mode: GraphicsMode,
    pub last_on: chrono::DateTime<Utc>,
    pub logon_time: chrono::DateTime<Utc>,
    pub msg_left: u64,
    pub msg_read: u64,
    pub today_ul: u64,
    pub today_dn: u64,
    pub bus_phone: String,
    pub home_phone: String,
    pub email: String,
    pub cmt1: String,
    pub cmt2: String,
    pub date_format: String,
}

impl StatusBarInfo {
    pub async fn get_info(board: &Arc<tokio::sync::Mutex<IcyBoard>>, node_state: &Arc<Mutex<Vec<Option<NodeState>>>>, node: usize) -> Self {
        let l = node_state.lock().await;
        let node_state = l[node].as_ref().unwrap();
        let current_conf = node_state.cur_conference;
        let graphics_mode = node_state.graphics_mode;
        let logon_time = node_state.logon_time;
        let cur_user = node_state.cur_user as usize;
        let board = board.lock().await;
        if cur_user >= board.users.len() {
            return Default::default();
        }

        let cur_security = board.users[cur_user].security_level;
        let user_name = board.users[cur_user].get_name().clone();
        let city = board.users[cur_user].city_or_state.clone();
        let times_on = board.users[cur_user].stats.num_times_on;
        let upbytes = board.users[cur_user].stats.total_upld_bytes;
        let up = board.users[cur_user].stats.num_uploads;
        let dnbytes = board.users[cur_user].stats.total_dnld_bytes;
        let dn = board.users[cur_user].stats.num_downloads;
        let today_dn = board.users[cur_user].stats.today_dnld_bytes;
        let today_ul = board.users[cur_user].stats.total_dnld_bytes;
        let last_on = board.users[cur_user].stats.last_on;
        let msg_left = board.users[cur_user].stats.messages_left;
        let msg_read = board.users[cur_user].stats.messages_read;
        let bus_phone = board.users[cur_user].bus_data_phone.clone();
        let home_phone = board.users[cur_user].home_voice_phone.clone();
        let email = board.users[cur_user].email.clone();
        let cmt1 = board.users[cur_user].user_comment.clone();
        let cmt2 = board.users[cur_user].sysop_comment.clone();
        let date_format = board.config.board.date_format.clone();
        Self {
            user_name,
            city,
            current_conf,
            cur_security,
            times_on,
            up,
            dn,
            upbytes,
            dnbytes,
            graphics_mode,
            last_on,
            logon_time,
            msg_left,
            msg_read,
            today_ul,
            today_dn,
            bus_phone,
            home_phone,
            email,
            cmt1,
            cmt2,
            date_format,
        }
    }
}
