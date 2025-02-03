use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::Res;
use chrono::{Local, Timelike};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use icy_board_engine::icy_board::{statistics::Statistics, IcyBoard};
use icy_board_tui::{
    app::get_screen_size,
    get_text,
    theme::{DOS_BLUE, DOS_LIGHT_CYAN, DOS_LIGHT_GRAY, DOS_RED, DOS_YELLOW},
};
use ratatui::{
    prelude::*,
    widgets::{block::Title, *},
};

#[derive(PartialEq)]
pub enum SystemStatisticsScreenMessage {
    Exit,
    Reset,
}

const NUM_LINES: usize = 1 + 2 * 6;

pub struct SystemStatisticsScreen {
    statistics: Statistics,
    scroll_state: ScrollbarState,
    table_state: TableState,
}

impl SystemStatisticsScreen {
    pub async fn new(board: &Arc<tokio::sync::Mutex<IcyBoard>>) -> Self {
        let statistics = board.lock().await.statistics.clone();
        Self {
            statistics,
            scroll_state: ScrollbarState::default().content_length(NUM_LINES),
            table_state: TableState::default().with_selected(0),
        }
    }

    pub async fn run(&mut self, terminal: &mut Terminal<impl Backend>, full_screen: bool) -> Res<SystemStatisticsScreenMessage> {
        let last_tick = Instant::now();
        let tick_rate = Duration::from_millis(1000);
        loop {
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            let mut page_len = 0;

            terminal.draw(|frame| {
                page_len = (frame.area().height as usize).saturating_sub(3);
                self.ui(frame, full_screen);
            })?;
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Esc => {
                                return Ok(SystemStatisticsScreenMessage::Exit);
                            }
                            KeyCode::Home => {
                                self.table_state.select(Some(0));
                            }
                            KeyCode::End => {
                                self.table_state.select(Some(NUM_LINES - 1));
                            }

                            KeyCode::PageUp => {
                                if let Some(idx) = self.table_state.selected() {
                                    self.table_state.select(Some(idx.saturating_sub(page_len)));
                                }
                            }
                            KeyCode::PageDown => {
                                if let Some(idx) = self.table_state.selected() {
                                    self.table_state.select(Some((idx + page_len).min(NUM_LINES - 1)));
                                }
                            }

                            KeyCode::Down | KeyCode::Char('s') => {
                                if let Some(idx) = self.table_state.selected() {
                                    if idx + 1 < NUM_LINES {
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
                            KeyCode::Delete => {
                                return Ok(SystemStatisticsScreenMessage::Reset);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    fn ui(&mut self, frame: &mut Frame, full_screen: bool) {
        let now = Local::now();
        let footer = get_text("icb_system_statistics_footer");

        let area: Rect = get_screen_size(&frame, full_screen);

        let b = Block::default()
            .title_alignment(Alignment::Left)
            .title(Title::from(Line::from(format!(" {} ", now.date_naive())).style(Style::new().white())))
            .title_alignment(Alignment::Center)
            .title(Title::from(
                Span::from(get_text("icb_system_statistics_title")).style(Style::new().fg(DOS_YELLOW).bg(DOS_RED).bold()),
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
        self.render_table(frame, area);
        self.render_scrollbar(frame, area);
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let header = [get_text("icb_system_statistics_header"), "#".to_string()]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(Style::default().fg(DOS_LIGHT_CYAN).bg(DOS_BLUE).bold())
            .height(1);
        let rows = vec![
            Row::new(vec![
                Cell::from(get_text("icb_system_statistics_total_calls")),
                Cell::from(self.statistics.total.calls.to_string()),
            ]),
            Row::new(vec![
                Cell::from(get_text("icb_system_statistics_total_messages")),
                Cell::from(self.statistics.total.messages.to_string()),
            ]),
            Row::new(vec![
                Cell::from(get_text("icb_system_statistics_total_uploads")),
                Cell::from(self.statistics.total.uploads.to_string()),
            ]),
            Row::new(vec![
                Cell::from(get_text("icb_system_statistics_total_uploads_kb")),
                Cell::from(self.statistics.total.uploads_kb.to_string()),
            ]),
            Row::new(vec![
                Cell::from(get_text("icb_system_statistics_total_downloads")),
                Cell::from(self.statistics.total.downloads.to_string()),
            ]),
            Row::new(vec![
                Cell::from(get_text("icb_system_statistics_total_downloads_kb")),
                Cell::from(self.statistics.total.downloads_kb.to_string()),
            ]),
            Row::new(vec![Cell::from(String::new()), Cell::from(String::new())]),
            Row::new(vec![
                Cell::from(get_text("icb_system_statistics_today_calls")),
                Cell::from(self.statistics.today.calls.to_string()),
            ]),
            Row::new(vec![
                Cell::from(get_text("icb_system_statistics_today_messages")),
                Cell::from(self.statistics.today.messages.to_string()),
            ]),
            Row::new(vec![
                Cell::from(get_text("icb_system_statistics_today_uploads")),
                Cell::from(self.statistics.today.uploads.to_string()),
            ]),
            Row::new(vec![
                Cell::from(get_text("icb_system_statistics_today_uploads_kb")),
                Cell::from(self.statistics.today.uploads_kb.to_string()),
            ]),
            Row::new(vec![
                Cell::from(get_text("icb_system_statistics_today_downloads")),
                Cell::from(self.statistics.today.downloads.to_string()),
            ]),
            Row::new(vec![
                Cell::from(get_text("icb_system_statistics_today_downloads_kb")),
                Cell::from(self.statistics.today.downloads_kb.to_string()),
            ]),
        ];

        let bar = " █ ";
        let table = Table::new(
            rows,
            [
                // + 1 is for padding.
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
            .content_length(NUM_LINES)
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
