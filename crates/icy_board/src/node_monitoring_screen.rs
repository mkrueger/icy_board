use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::Res;
use chrono::{Local, Timelike};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    get_text,
    theme::{DOS_BLUE, DOS_LIGHT_CYAN, DOS_LIGHT_GRAY, DOS_RED, DOS_YELLOW},
};
use ratatui::{
    prelude::*,
    widgets::{block::Title, *},
};

use crate::bbs::BBS;

pub enum NodeMonitoringScreenMessage {
    Exit,
    EnterNode(usize),
}

pub struct NodeMonitoringScreen {
    nodes: usize,
    scroll_state: ScrollbarState,
    table_state: TableState,
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

    pub fn run(
        &mut self,
        terminal: &mut Terminal<impl Backend>,
        board: &Arc<tokio::sync::Mutex<IcyBoard>>,
        bbs: &mut Arc<Mutex<BBS>>,
    ) -> Res<NodeMonitoringScreenMessage> {
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(1000);
        loop {
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            let mut page_len = 0;
            terminal.draw(|frame| {
                page_len = (frame.size().height as usize).saturating_sub(3);
                self.ui(frame, board, bbs);
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
                                    if bbs.lock().unwrap().get_open_connections().lock().unwrap()[i].is_some() {
                                        return Ok(NodeMonitoringScreenMessage::EnterNode(i));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                //     self.on_tick();
                last_tick = Instant::now();
            }
        }
    }

    fn ui(&mut self, frame: &mut Frame, board: &Arc<tokio::sync::Mutex<IcyBoard>>, bbs: &mut Arc<Mutex<BBS>>) {
        let now = Local::now();
        let mut footer = get_text("icbmoni_footer");
        if let Some(i) = self.table_state.selected() {
            if bbs.lock().unwrap().get_open_connections().lock().unwrap()[i].is_some() {
                footer = get_text("icbmoni_on_note_footer")
            }
        }

        let b = Block::default()
            .title(Title::from(Line::from(format!(" {} ", now.date_naive())).style(Style::new().white())).alignment(Alignment::Left))
            .title(Title::from(Span::from(get_text("icbmoni_title")).style(Style::new().fg(DOS_YELLOW).bg(DOS_RED).bold())).alignment(Alignment::Center))
            .title(Title::from(Line::from(format!(" {} ", now.time().with_nanosecond(0).unwrap())).style(Style::new().white())).alignment(Alignment::Right))
            .title(
                Title::from(Span::from(footer).style(Style::new().fg(DOS_YELLOW).bg(DOS_RED)))
                    .alignment(Alignment::Center)
                    .position(block::Position::Bottom),
            )
            .style(Style::new().bg(DOS_BLUE))
            .border_type(BorderType::Double)
            .border_style(Style::new().fg(DOS_YELLOW))
            .borders(Borders::ALL);
        b.render(frame.size(), frame.buffer_mut());
        let area = frame.size();
        self.render_table(frame, area, board, bbs);
        self.render_scrollbar(frame, area);
    }

    fn render_table(&mut self, frame: &mut Frame, _area: Rect, _board: &Arc<tokio::sync::Mutex<IcyBoard>>, bbs: &mut Arc<Mutex<BBS>>) {
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

        if let Ok(mut bbs) = bbs.lock() {
            if let Ok(con) = bbs.get_open_connections().lock() {
                let rows = con.iter().enumerate().map(|(i, node_state)| {
                    if let Some(state) = node_state {
                        let user_name = //if state.cur_user < 0 {
                            get_text("icbmoni_log_in")
                      /*  } else {
                            println!("user !!!");
                            /*
                            tokio::runtime::Builder::new_current_thread()
                                .enable_all()
                                .build()
                                .unwrap()
                                .block_on(async { board.lock().await.users[state.cur_user as usize].get_name().clone() })*/
                        }*/;

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
                .highlight_style(Style::default().fg(DOS_BLUE).bg(DOS_LIGHT_GRAY))
                .style(Style::default().fg(DOS_YELLOW).bg(DOS_BLUE))
                .highlight_spacing(HighlightSpacing::Always);
                let area = frame.size().inner(&Margin { vertical: 1, horizontal: 1 });
                frame.render_stateful_widget(table, area, &mut self.table_state);
            }
        }
    }

    fn render_scrollbar(&mut self, frame: &mut Frame, _area: Rect) {
        let area = frame.size().inner(&Margin { vertical: 1, horizontal: 0 });
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
