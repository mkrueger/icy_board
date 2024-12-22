use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::Res;
use chrono::{Local, Timelike};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use icy_board_engine::icy_board::{bbs::BBS, state::NodeState, IcyBoard};
use icy_board_tui::{
    app::get_screen_size,
    get_text,
    theme::{DOS_BLUE, DOS_LIGHT_CYAN, DOS_LIGHT_GRAY, DOS_RED, DOS_YELLOW},
};
use icy_net::ConnectionType;
use ratatui::{
    prelude::*,
    widgets::{block::Title, *},
};
use tokio::sync::Mutex;

pub enum NodeMonitoringScreenMessage {
    Exit,
    EnterNode(usize),
}

pub struct NodeMonitoringScreen {
    nodes: usize,
    scroll_state: ScrollbarState,
    table_state: TableState,
}

pub struct Info {
    pub user_activity: icy_board_engine::icy_board::state::UserActivity,
    pub cur_user: Option<String>,
    pub connection_type: ConnectionType,
}

pub struct Connection {
    pub name: String,
    pub endpoint: String,
}

impl Info {
    fn new(board: &IcyBoard, state: &NodeState) -> Info {
        let user = if state.cur_user >= 0 {
            Some(board.users[state.cur_user as usize].name.clone())
        } else {
            None
        };

        Info {
            user_activity: state.user_activity,
            cur_user: user,
            connection_type: state.connection_type,
        }
    }
}

impl NodeMonitoringScreen {
    pub async fn new(board: &Arc<tokio::sync::Mutex<IcyBoard>>) -> Self {
        let nodes = board.lock().await.config.board.num_nodes;
        Self {
            nodes: nodes as usize,
            scroll_state: ScrollbarState::default().content_length(nodes as usize),
            table_state: TableState::default().with_selected(0),
        }
    }

    pub async fn run(
        &mut self,
        terminal: &mut Terminal<impl Backend>,
        board: &Arc<Mutex<IcyBoard>>,
        bbs: &mut Arc<Mutex<BBS>>,
        full_screen: bool,
    ) -> Res<NodeMonitoringScreenMessage> {
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(1000);
        let mut node_info: Vec<Option<Info>> = Vec::new();
        let mut connections: Vec<Connection> = Vec::new();
        loop {
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            let mut page_len = 0;
            if node_info.is_empty() || last_tick.elapsed() >= tick_rate {
                let board = board.lock().await;
                node_info.clear();
                bbs.lock().await.clear_closed_connections().await;
                for a in bbs.lock().await.get_open_connections().await.lock().await.iter() {
                    if let Some(a) = a {
                        node_info.push(Some(Info::new(&board, a)));
                    } else {
                        node_info.push(None);
                    }
                }
                connections.clear();

                if board.config.login_server.telnet.is_enabled {
                    connections.push(Connection {
                        name: "Telnet".to_string(),
                        endpoint: format!("{}:{}", board.config.login_server.telnet.address, board.config.login_server.telnet.port),
                    });
                }

                if board.config.login_server.ssh.is_enabled {
                    connections.push(Connection {
                        name: "SSH".to_string(),
                        endpoint: format!("{}:{}", board.config.login_server.ssh.address, board.config.login_server.ssh.port),
                    });
                }

                if board.config.login_server.secure_websocket.is_enabled {
                    connections.push(Connection {
                        name: "Websocket".to_string(),
                        endpoint: format!(
                            "{}:{}",
                            board.config.login_server.secure_websocket.address, board.config.login_server.secure_websocket.port
                        ),
                    });
                }

                last_tick = Instant::now();
            }

            terminal.draw(|frame| {
                page_len = (frame.area().height as usize).saturating_sub(3);
                self.ui(frame, &node_info, &connections, full_screen);
            })?;
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Esc => {
                                return Ok(NodeMonitoringScreenMessage::Exit);
                            }
                            KeyCode::Home => {
                                self.table_state.select(Some(0));
                            }
                            KeyCode::End => {
                                if self.nodes > 0 {
                                    self.table_state.select(Some(self.nodes - 1));
                                }
                            }

                            KeyCode::PageUp => {
                                if let Some(idx) = self.table_state.selected() {
                                    self.table_state.select(Some(idx.saturating_sub(page_len)));
                                }
                            }
                            KeyCode::PageDown => {
                                if let Some(idx) = self.table_state.selected() {
                                    self.table_state.select(Some((idx + page_len).min(self.nodes - 1)));
                                }
                            }

                            KeyCode::Down | KeyCode::Char('s') => {
                                if let Some(idx) = self.table_state.selected() {
                                    if idx + 1 < self.nodes {
                                        self.table_state.select(Some(idx + 1));
                                    }
                                }
                            }
                            KeyCode::Up | KeyCode::Char('w') => {
                                if let Some(idx) = self.table_state.selected() {
                                    if idx > 0 {
                                        self.table_state.select(Some(idx - 1));
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                if let Some(i) = self.table_state.selected() {
                                    if bbs.lock().await.get_open_connections().await.lock().await[i].is_some() {
                                        return Ok(NodeMonitoringScreenMessage::EnterNode(i));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    fn ui(&mut self, frame: &mut Frame, infos: &Vec<Option<Info>>, connections: &Vec<Connection>, full_screen: bool) {
        let now = Local::now();
        let mut footer = get_text("icbmoni_footer");
        if let Some(i) = self.table_state.selected() {
            if infos[i].is_some() {
                footer = get_text("icbmoni_on_note_footer")
            }
        }
        let area: Rect = get_screen_size(&frame, full_screen);

        let b = Block::default()
            .title_alignment(Alignment::Left)
            .title(Title::from(Line::from(format!(" {} ", now.date_naive())).style(Style::new().white())))
            .title_alignment(Alignment::Center)
            .title(Title::from(
                Span::from(get_text("icbmoni_title")).style(Style::new().fg(DOS_YELLOW).bg(DOS_RED).bold()),
            ))
            .title_alignment(Alignment::Right)
            .title(Title::from(
                Line::from(format!(" {} ", now.time().with_nanosecond(0).unwrap())).style(Style::new().white()),
            ))
            .title_alignment(Alignment::Center)
            .title_bottom(Line::from(Span::from(footer).style(Style::new().fg(DOS_YELLOW).bg(DOS_RED))))
            .style(Style::new().bg(DOS_BLUE))
            .border_type(BorderType::Double)
            .border_style(Style::new().fg(DOS_YELLOW))
            .borders(Borders::ALL);
        b.render(area, frame.buffer_mut());
        let vertical = Layout::vertical([Constraint::Fill(1), Constraint::Length(connections.len().max(5) as u16 + 1)]);
        let [node_area, connections_area] = vertical.areas(area);
        self.render_table(frame, node_area, infos);
        self.render_scrollbar(frame, node_area);
        self.render_connections(frame, connections_area, connections);
    }

    fn render_connections(&self, frame: &mut Frame, connections_area: Rect, connections: &[Connection]) {
        let mut area = connections_area.inner(Margin { vertical: 1, horizontal: 1 });
        Line::from("═".repeat(area.width as usize))
            .style(Style::new().fg(DOS_YELLOW))
            .centered()
            .render(area, frame.buffer_mut());
        area.y += 1;

        for con in connections {
            let n = if con.name.is_empty() { &con.name } else { "0.0.0.0" };
            let text = format!("{} on {}:{}", con.name, n, con.endpoint);
            let text = Text::from(text);
            text.render(area, frame.buffer_mut());
            area.y += 1;
        }
    }
    fn render_table(&mut self, frame: &mut Frame, area: Rect, infos: &Vec<Option<Info>>) {
        let header = [
            "#".to_string(),
            get_text("icbmoni_status_header"),
            get_text("icbmoni_user_header"),
            get_text("icbmoni_protocol_header"),
        ]
        .into_iter()
        .map(Cell::from)
        .collect::<Row>()
        .style(Style::default().fg(DOS_LIGHT_CYAN).bg(DOS_BLUE).bold())
        .height(1);
        let rows = infos.iter().enumerate().map(|(i, node_state)| {
            if let Some(state) = node_state {
                let user_name = if let Some(user) = &state.cur_user {
                    user.clone()
                } else {
                    get_text("icbmoni_log_in")
                };

                let activity = match state.user_activity {
                    icy_board_engine::icy_board::state::UserActivity::LoggingIn => get_text("icbmoni_user_log_in"),
                    icy_board_engine::icy_board::state::UserActivity::BrowseMenu => get_text("icbmoni_user_browse_menu"),
                    icy_board_engine::icy_board::state::UserActivity::EnterMessage => get_text("icbmoni_user_enter_message"),
                    icy_board_engine::icy_board::state::UserActivity::CommentToSysop => get_text("icbmoni_comment_to_sysop"),
                    icy_board_engine::icy_board::state::UserActivity::BrowseFiles => get_text("icbmoni_user_browse_files"),
                    icy_board_engine::icy_board::state::UserActivity::ReadMessages => get_text("icbmoni_user_read_messages"),
                    icy_board_engine::icy_board::state::UserActivity::ReadBulletins => get_text("icbmoni_user_read_bulletins"),
                    icy_board_engine::icy_board::state::UserActivity::TakeSurvey => get_text("icbmoni_user_take_survey"),
                    icy_board_engine::icy_board::state::UserActivity::UploadFiles => get_text("icbmoni_user_upload"),

                    icy_board_engine::icy_board::state::UserActivity::DownloadFiles => get_text("icbmoni_user_download"),
                    icy_board_engine::icy_board::state::UserActivity::Goodbye => get_text("icbmoni_user_logoff"),
                    icy_board_engine::icy_board::state::UserActivity::RunningDoor => get_text("icbmoni_user_door"),
                    icy_board_engine::icy_board::state::UserActivity::ChatWithSysop => get_text("icbmoni_user_chat_with_sysop"),
                    icy_board_engine::icy_board::state::UserActivity::GroupChat => get_text("icbmoni_user_group_chat"),
                    icy_board_engine::icy_board::state::UserActivity::PagingSysop => get_text("icbmoni_user_page_sysop"),
                    icy_board_engine::icy_board::state::UserActivity::ReadBroadcast => get_text("icbmoni_user_read_broadcast"),
                };

                Row::new(vec![
                    Cell::from(format!("{:<3}", i + 1)),
                    Cell::from(activity),
                    Cell::from(user_name),
                    Cell::from(format!("{:?}", state.connection_type)),
                ])
            } else {
                Row::new(vec![
                    Cell::from(format!("{:<3}", i + 1)),
                    Cell::from(get_text("icbmoni_no_caller")),
                    Cell::from(""),
                    Cell::from(""),
                ])
            }
        });
        let bar = " █ ";
        let table = Table::new(
            rows,
            [
                // + 1 is for padding.
                Constraint::Length(4),
                Constraint::Min(15),
                Constraint::Min(25),
                Constraint::Min(20),
            ],
        )
        .header(header)
        .highlight_symbol(Text::from(vec!["".into(), bar.into(), bar.into(), "".into()]))
        .row_highlight_style(Style::default().fg(DOS_BLUE).bg(DOS_LIGHT_GRAY))
        .style(Style::default().fg(DOS_YELLOW).bg(DOS_BLUE))
        .highlight_spacing(HighlightSpacing::Always);
        let mut area = area.inner(Margin { vertical: 1, horizontal: 1 });
        area.width -= 1;
        frame.render_stateful_widget(table, area, &mut self.table_state);
    }

    fn render_scrollbar(&mut self, frame: &mut Frame, _area: Rect) {
        let area = frame.area().inner(Margin { vertical: 1, horizontal: 0 });
        let mut scroll_state = self
            .scroll_state
            .position(self.table_state.offset())
            .content_length(self.nodes.saturating_sub(area.height as usize))
            .viewport_content_length(area.height as usize);

        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .thumb_symbol("█")
                .track_symbol(Some("░"))
                .end_symbol(Some("▼")),
            area,
            &mut scroll_state,
        );
    }
}
