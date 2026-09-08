use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::Res;
use chrono::Local;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use icy_board_engine::icy_board::{IcyBoard, bbs::BBS, state::NodeState};
use icy_board_tui::{
    app::get_screen_size,
    get_text, get_text_args,
    theme::{DOS_LIGHT_CYAN, DOS_LIGHT_GRAY, DOS_YELLOW},
};
use icy_net::ConnectionType;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::Mutex;

use crate::cws_chrome;

pub enum NodeMonitoringScreenMessage {
    Exit,
    EnterNode(usize),
}

#[derive(Clone)]
pub struct WebAdminInfo {
    pub url: String,
    pub token: String,
}

pub struct NodeMonitoringScreen {
    nodes: usize,
    date_format: String,
    scroll_state: ScrollbarState,
    table_state: TableState,
}

pub struct Info {
    pub user_activity: String,
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
            user_activity: state.operation.clone(),
            cur_user: user,
            connection_type: state.connection_type,
        }
    }
}

impl NodeMonitoringScreen {
    pub async fn new(board: &Arc<tokio::sync::Mutex<IcyBoard>>) -> Self {
        let board = board.lock().await;
        let nodes = board.config.board.num_nodes;
        Self {
            nodes: nodes as usize,
            date_format: board.config.board.date_format.clone(),
            scroll_state: ScrollbarState::default().content_length(nodes as usize),
            table_state: TableState::default().with_selected(0),
        }
    }

    pub async fn run<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
        board: &Arc<Mutex<IcyBoard>>,
        bbs: &mut Arc<Mutex<BBS>>,
        full_screen: bool,
        web_admin: Option<&WebAdminInfo>,
    ) -> Res<NodeMonitoringScreenMessage>
    where
        B::Error: Send + Sync + 'static,
    {
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(1000);
        let mut node_info: Vec<Option<Info>> = Vec::new();
        let mut connections: Vec<Connection> = Vec::new();
        loop {
            if bbs.lock().await.event_restart_requested {
                return Ok(NodeMonitoringScreenMessage::Exit);
            }
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            let mut page_len = 0;
            if node_info.is_empty() || last_tick.elapsed() >= tick_rate {
                let board = board.lock().await;
                node_info.clear();
                bbs.lock().await.clear_closed_connections().await;
                for a in bbs.lock().await.get_open_connections().lock().await.iter() {
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
                self.ui(frame, &node_info, &connections, web_admin, full_screen);
            })?;
            if event::poll(timeout)?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
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
                        if let Some(idx) = self.table_state.selected()
                            && idx + 1 < self.nodes
                        {
                            self.table_state.select(Some(idx + 1));
                        }
                    }
                    KeyCode::Up | KeyCode::Char('w') => {
                        if let Some(idx) = self.table_state.selected()
                            && idx > 0
                        {
                            self.table_state.select(Some(idx - 1));
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(i) = self.table_state.selected()
                            && bbs.lock().await.get_open_connections().lock().await[i].is_some()
                        {
                            return Ok(NodeMonitoringScreenMessage::EnterNode(i));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn ui(&mut self, frame: &mut Frame, infos: &[Option<Info>], connections: &[Connection], web_admin: Option<&WebAdminInfo>, full_screen: bool) {
        let now = Local::now();
        let mut footer = "icbmoni_footer";
        if let Some(i) = self.table_state.selected()
            && infos.get(i).is_some_and(Option::is_some)
        {
            footer = "icbmoni_on_note_footer"
        }
        let area: Rect = get_screen_size(frame, full_screen);

        let b = cws_chrome::screen(&get_text("icbmoni_title"), &self.date_format, now, area.width).title_bottom(cws_chrome::hotkeys(footer));
        b.render(area, frame.buffer_mut());
        let extra_lines = if web_admin.is_some() { 2usize } else { 0 };
        // One separator row plus the top/bottom margins; keep the footer free.
        let bottom_len = (connections.len() + extra_lines + 3).max(6).min(usize::from(area.height)) as u16;
        let vertical = Layout::vertical([Constraint::Fill(1), Constraint::Length(bottom_len)]);
        let [node_area, connections_area] = vertical.areas(area);
        self.render_table(frame, node_area, infos);
        self.render_scrollbar(frame, node_area);
        self.render_connections(frame, connections_area, connections, web_admin);
    }

    fn render_connections(&self, frame: &mut Frame, connections_area: Rect, connections: &[Connection], web_admin: Option<&WebAdminInfo>) {
        let area = connections_area.intersection(frame.area()).inner(Margin { vertical: 1, horizontal: 1 });
        let mut lines = vec![Line::from("═".repeat(area.width as usize)).style(Style::new().fg(DOS_YELLOW))];

        for con in connections {
            lines.push(Line::from(format!("{} on {}", con.name, con.endpoint)).style(Style::new().fg(DOS_LIGHT_GRAY)));
        }

        if let Some(admin) = web_admin {
            let mut args = HashMap::new();
            args.insert("url".to_string(), admin.url.clone());
            let url_line = get_text_args("icbmoni_web_admin_url", args);
            lines.push(Line::from(url_line).style(Style::new().fg(DOS_LIGHT_CYAN)));

            let mut args = HashMap::new();
            args.insert("token".to_string(), admin.token.clone());
            let token_line = get_text_args("icbmoni_web_admin_token", args);
            lines.push(Line::from(token_line).style(Style::new().fg(DOS_LIGHT_CYAN)));
        }

        // Text::render styles its whole rectangle, not just the written glyphs.
        // Render each line into a single bounded row so cyan cannot reach the footer.
        for (line, row) in lines.into_iter().zip(area.rows()) {
            line.render(row, frame.buffer_mut());
        }
    }
    fn render_table(&mut self, frame: &mut Frame, area: Rect, infos: &[Option<Info>]) {
        let header = [
            "#".to_string(),
            get_text("icbmoni_status_header"),
            get_text("icbmoni_user_header"),
            get_text("icbmoni_protocol_header"),
        ]
        .into_iter()
        .map(Cell::from)
        .collect::<Row>()
        .style(cws_chrome::theme().config_title)
        .height(1);
        let rows = infos.iter().enumerate().map(|(i, node_state)| {
            if let Some(state) = node_state {
                let user_name = if let Some(user) = &state.cur_user {
                    user.clone()
                } else {
                    get_text("icbmoni_log_in")
                };
                let activity = state.user_activity.clone();
                /*/
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
                };*/

                Row::new(vec![
                    Cell::from((i + 1).to_string()),
                    Cell::from(activity),
                    Cell::from(user_name),
                    Cell::from(format!("{:?}", state.connection_type)),
                ])
            } else {
                Row::new(vec![
                    Cell::from((i + 1).to_string()),
                    Cell::from(get_text("icbmoni_no_caller")),
                    Cell::from(""),
                    Cell::from(""),
                ])
            }
        });
        let number_width = self.nodes.max(infos.len()).to_string().len().max(2) as u16;
        let table = Table::new(
            rows,
            [
                Constraint::Length(number_width),
                Constraint::Fill(3),
                Constraint::Fill(2),
                // Longest protocol name: SecureWebsocket.
                Constraint::Length(15),
            ],
        )
        .header(header)
        .column_spacing(1)
        .row_highlight_style(cws_chrome::theme().selected_item)
        .style(cws_chrome::theme().table)
        .highlight_spacing(HighlightSpacing::Never);
        let mut area = area.inner(Margin { vertical: 1, horizontal: 1 });
        area.width = area.width.saturating_sub(1);
        frame.render_stateful_widget(table, area, &mut self.table_state);
    }

    fn render_scrollbar(&mut self, frame: &mut Frame, area: Rect) {
        let area = area.inner(Margin { vertical: 1, horizontal: 0 });
        let mut scroll_state = self
            .scroll_state
            .position(self.table_state.offset())
            .content_length(self.nodes.saturating_sub(area.height as usize))
            .viewport_content_length(area.height as usize);

        frame.render_stateful_widget(
            Scrollbar::default()
                .style(cws_chrome::theme().dialog_box)
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

#[cfg(test)]
mod tests {
    use super::*;
    use icy_board_tui::theme::DOS_BLUE;
    use ratatui::backend::TestBackend;

    #[test]
    fn monitor_uses_common_header_in_fixed_and_fullscreen_modes() {
        let mut screen = screen(4);
        screen.date_format = "DATE".into();
        for full_screen in [false, true] {
            let mut terminal = Terminal::new(TestBackend::new(132, 40)).unwrap();
            let mut area = Rect::default();
            terminal
                .draw(|frame| {
                    area = get_screen_size(frame, full_screen);
                    screen.ui(frame, &[None, None, None, None], &[], None, full_screen);
                })
                .unwrap();
            cws_chrome::assert_header(terminal.backend().buffer(), area, &get_text("icbmoni_title"));
        }
    }

    fn screen(nodes: usize) -> NodeMonitoringScreen {
        NodeMonitoringScreen {
            nodes,
            date_format: "%m/%d/%y".into(),
            scroll_state: ScrollbarState::default().content_length(nodes),
            table_state: TableState::default().with_selected(0),
        }
    }

    fn row(buffer: &Buffer, y: u16) -> String {
        (0..buffer.area.width).map(|x| buffer[(x, y)].symbol()).collect()
    }

    fn admin() -> WebAdminInfo {
        WebAdminInfo {
            url: "http://127.0.0.1:8787/".into(),
            token: "test-token".into(),
        }
    }

    #[test]
    fn status_and_long_protocol_fit_at_80_columns() {
        let mut screen = screen(2);
        let infos = [
            None,
            Some(Info {
                user_activity: "Kein Anrufer auf diesem Node".into(),
                cur_user: Some("Test Benutzer".into()),
                connection_type: ConnectionType::SecureWebsocket,
            }),
        ];
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| screen.ui(frame, &infos, &[], None, false)).unwrap();
        let buffer = terminal.backend().buffer();
        assert!(row(buffer, 2).contains(&get_text("icbmoni_no_caller")));
        let active = row(buffer, 3);
        for text in ["Kein Anrufer auf diesem Node", "Test Benutzer", "SecureWebsocket"] {
            assert!(active.contains(text), "missing {text}: {active}");
        }
        let header = row(buffer, 1);
        let gap = header.find(&get_text("icbmoni_status_header")).unwrap() - header.find('#').unwrap();
        assert!(gap <= 4, "excessive gap between node and status: {header}");
    }

    #[test]
    fn web_admin_color_does_not_reach_the_bottom_border() {
        let mut screen = screen(1);
        let connections = [Connection {
            name: "Telnet".into(),
            endpoint: ":1337".into(),
        }];
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| screen.ui(frame, &[None], &connections, Some(&admin()), false)).unwrap();
        let buffer = terminal.backend().buffer();
        for x in 0..80 {
            assert_eq!(buffer[(x, 24)].fg, DOS_YELLOW, "bottom border color at column {x}");
        }
        assert!(row(buffer, 23).contains("test-token"));
    }

    #[test]
    fn all_connections_and_admin_lines_fit_above_the_footer() {
        let mut screen = screen(1);
        let connections: Vec<_> = ["Telnet", "SSH", "Websocket"]
            .into_iter()
            .map(|name| Connection {
                name: name.into(),
                endpoint: "localhost:1234".into(),
            })
            .collect();
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| screen.ui(frame, &[None], &connections, Some(&admin()), false)).unwrap();
        let buffer = terminal.backend().buffer();
        let text = (0..24).map(|y| row(buffer, y)).collect::<Vec<_>>().join("\n");
        for expected in [
            "Telnet on localhost:1234",
            "SSH on localhost:1234",
            "Websocket on localhost:1234",
            "http://127.0.0.1:8787/",
            "test-token",
        ] {
            assert!(text.contains(expected), "missing {expected}");
        }
    }

    #[test]
    fn connection_lines_never_paint_outside_their_inner_area() {
        let screen = screen(0);
        let connections = [Connection {
            name: "Telnet".into(),
            endpoint: ":1337".into(),
        }];
        for height in 0..9 {
            let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
            let area = Rect::new(10, 5, 70, height);
            let inner = area.inner(Margin { vertical: 1, horizontal: 1 });
            terminal
                .draw(|frame| {
                    let bounds = frame.area();
                    frame.buffer_mut().set_style(bounds, Style::new().fg(DOS_YELLOW).bg(DOS_BLUE));
                    screen.render_connections(frame, area, &connections, Some(&admin()));
                })
                .unwrap();
            let buffer = terminal.backend().buffer();
            for y in 0..30 {
                for x in 0..100 {
                    if !inner.contains(Position::new(x, y)) {
                        assert_eq!(buffer[(x, y)].fg, DOS_YELLOW, "color outside height {height} at {x},{y}");
                        assert_eq!(buffer[(x, y)].symbol(), " ", "text outside height {height} at {x},{y}");
                    }
                }
            }
        }
    }
}
