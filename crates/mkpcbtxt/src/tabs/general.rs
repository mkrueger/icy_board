use crossterm::event::KeyCode;
use icy_board_engine::icy_board::icb_text::{IcbTextFile, IceText, DEFAULT_DISPLAY_TEXT};
use icy_board_tui::theme::{DOS_BLUE, DOS_LIGHT_GRAY, DOS_YELLOW};
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Cell, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState},
    Frame,
};

use super::TabPage;

pub struct GeneralTab {
    icb_txt: IcbTextFile,
    scroll_state: ScrollbarState,
    table_state: TableState,
}

impl GeneralTab {
    pub fn new(icb_txt: IcbTextFile) -> Self {
        let scroll_state = ScrollbarState::default().content_length(icb_txt.len());
        Self {
            icb_txt,
            scroll_state,
            table_state: TableState::default().with_selected(0),
        }
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let rows = self.icb_txt.iter().map(|txt| Row::new(vec![Cell::from(txt.text.to_string())]));
        let bar = " █ ";
        let table = Table::new(
            rows,
            [
                // + 1 is for padding.
                Constraint::Fill(0),
            ],
        )
        .highlight_style(Style::default().fg(DOS_BLUE).bg(DOS_LIGHT_GRAY))
        .style(Style::default().fg(DOS_YELLOW).bg(DOS_BLUE))
        .highlight_spacing(HighlightSpacing::Always);
        frame.render_stateful_widget(table, area, &mut self.table_state);
    }

    fn render_scrollbar(&mut self, frame: &mut Frame, area: Rect) {
        let mut scroll_state = self
            .scroll_state
            .position(self.table_state.offset())
            .content_length(self.icb_txt.len().saturating_sub(area.height as usize))
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

impl TabPage for GeneralTab {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.render_table(frame, area);
        self.render_scrollbar(frame, area);
    }

    fn handle_key_press(&mut self, key: crossterm::event::KeyEvent) -> crate::app::ResultState {
        let page_len = 10;
        match key.code {
            KeyCode::Home => {
                self.table_state.select(Some(0));
            }

            KeyCode::End => {
                if self.icb_txt.len() > 0 {
                    self.table_state.select(Some(self.icb_txt.len() - 1));
                }
            }

            KeyCode::PageUp => {
                if let Some(idx) = self.table_state.selected() {
                    self.table_state.select(Some(idx.saturating_sub(page_len)));
                }
            }
            KeyCode::PageDown => {
                if let Some(idx) = self.table_state.selected() {
                    self.table_state.select(Some((idx + page_len).min(self.icb_txt.len() - 1)));
                }
            }

            KeyCode::Down | KeyCode::Char('s') => {
                if let Some(idx) = self.table_state.selected() {
                    if idx + 1 < self.icb_txt.len() {
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
            _ => {}
        }

        self.request_status()
    }

    fn request_edit_mode(&mut self, _terminal: &mut icy_board_tui::TerminalType, _full_screen: bool) -> crate::app::ResultState {
        self.request_status()
    }

    fn request_status(&self) -> crate::app::ResultState {
        let status_line = if let Some(sel) = self.table_state.selected() {
            DEFAULT_DISPLAY_TEXT.get_display_text(IceText::from(sel)).unwrap().text.to_string()
        } else {
            String::new()
        };

        crate::app::ResultState { status_line, cursor: None }
    }
}
