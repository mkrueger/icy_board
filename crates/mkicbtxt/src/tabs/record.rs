use crossterm::event::KeyCode;
use icy_board_engine::icy_board::icb_text::{IcbTextFile, IcbTextStyle, IceText, TextEntry, DEFAULT_DISPLAY_TEXT};
use icy_board_tui::{
    pcb_line::get_styled_pcb_line,
    theme::{DOS_BLACK, DOS_BLUE, DOS_LIGHT_BLUE, DOS_LIGHT_CYAN, DOS_LIGHT_GRAY, DOS_LIGHT_GREEN, DOS_LIGHT_MAGENTA, DOS_LIGHT_RED, DOS_WHITE, DOS_YELLOW},
};
use itertools::Itertools;
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Cell, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState, Widget},
    Frame,
};

use super::TabPage;

pub struct RecordTab<'a> {
    icb_txt: &'a mut IcbTextFile,
    scroll_state: ScrollbarState,
    table_state: TableState,
    filtered_entries: Vec<usize>,
}

impl<'a> RecordTab<'a> {
    pub fn new(icb_txt: &'a mut IcbTextFile) -> Self {
        let scroll_state = ScrollbarState::default().content_length(icb_txt.len());
        let filtered_entries = (1..icb_txt.len()).collect_vec();

        Self {
            icb_txt,
            scroll_state,
            table_state: TableState::default().with_selected(0),
            filtered_entries,
        }
    }
    pub fn entries(&self) -> usize {
        self.filtered_entries.len()
    }

    pub fn is_dirty(&self, icb_text: &IcbTextFile) -> bool {
        self.icb_txt != icb_text
    }

    pub fn get_original_entry(&mut self) -> Option<&TextEntry> {
        if let Some(idx) = self.table_state.selected() {
            if idx < self.filtered_entries.len() {
                DEFAULT_DISPLAY_TEXT.get(self.filtered_entries[idx])
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_selected_entry_mut(&mut self) -> Option<&mut TextEntry> {
        if let Some(idx) = self.table_state.selected() {
            if idx < self.filtered_entries.len() {
                self.icb_txt.get_mut(self.filtered_entries[idx])
            } else {
                None
            }
        } else {
            None
        }
    }

    fn render_table(&mut self, frame: &mut Frame, mut area: Rect) {
        area.width -= 1;
        let rows = self.filtered_entries.iter().map(|i| {
            let entry = self.icb_txt.get(*i).unwrap();
            Row::new(vec![Cell::from(get_styled_pcb_line(&entry.text))]).style(convert_style(entry.style))
        });

        // let bar = " █ ";
        let table = Table::new(
            rows,
            [
                // + 1 is for padding.
                Constraint::Fill(0),
            ],
        )
        .highlight_style(Style::default().fg(DOS_BLUE).bg(DOS_LIGHT_GRAY))
        .style(Style::default().fg(DOS_YELLOW).bg(DOS_BLACK))
        .highlight_spacing(HighlightSpacing::Always);
        frame.render_stateful_widget(table, area, &mut self.table_state);
    }

    fn render_scrollbar(&mut self, frame: &mut Frame, area: Rect) {
        let mut scroll_state = self
            .scroll_state
            .position(self.table_state.offset())
            .content_length(self.entries().saturating_sub(area.height as usize))
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

    pub fn set_filter(&mut self, filter: &str) {
        let filter = filter.to_ascii_lowercase();
        self.filtered_entries = (1..self.icb_txt.len())
            .filter(|i| {
                let entry = self.icb_txt.get(*i).unwrap();
                entry.text.to_ascii_lowercase().contains(&filter)
                    || DEFAULT_DISPLAY_TEXT
                        .get_display_text(IceText::from(*i))
                        .unwrap()
                        .text
                        .to_ascii_lowercase()
                        .contains(&filter)
            })
            .collect_vec();
        self.table_state.select(Some(0));
    }

    pub fn jump(&mut self, number: usize) {
        if number < self.entries() {
            self.table_state.select(Some(number));
        }
    }

    pub(crate) fn selected_record(&self) -> usize {
        self.table_state.selected().unwrap()
    }
}

pub fn convert_style(text_style: icy_board_engine::icy_board::icb_text::IcbTextStyle) -> Style {
    let color = match text_style {
        IcbTextStyle::Plain => DOS_LIGHT_GRAY,
        IcbTextStyle::Red => DOS_LIGHT_RED,
        IcbTextStyle::Green => DOS_LIGHT_GREEN,
        IcbTextStyle::Yellow => DOS_YELLOW,
        IcbTextStyle::Blue => DOS_LIGHT_BLUE,
        IcbTextStyle::Purple => DOS_LIGHT_MAGENTA,
        IcbTextStyle::Cyan => DOS_LIGHT_CYAN,
        IcbTextStyle::White => DOS_WHITE,
    };

    Style::default().fg(color)
}

impl<'a> TabPage for RecordTab<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        if self.filtered_entries.is_empty() {
            Line::from(Span::styled("No entries found".to_string(), Style::default().fg(DOS_LIGHT_RED))).render(area, frame.buffer_mut());
        }

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
                if self.entries() > 0 {
                    self.table_state.select(Some(self.entries() - 1));
                }
            }

            KeyCode::PageUp => {
                if let Some(idx) = self.table_state.selected() {
                    self.table_state.select(Some(idx.saturating_sub(page_len)));
                }
            }
            KeyCode::PageDown => {
                if let Some(idx) = self.table_state.selected() {
                    self.table_state.select(Some((idx + page_len).min(self.entries() - 1)));
                }
            }

            KeyCode::Down | KeyCode::Char('s') => {
                if let Some(idx) = self.table_state.selected() {
                    if idx + 1 < self.entries() {
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
            if sel < self.filtered_entries.len() {
                let txt = DEFAULT_DISPLAY_TEXT.get_display_text(IceText::from(self.filtered_entries[sel])).unwrap().text;
                format!("{}/{} {}", self.filtered_entries[sel], self.icb_txt.len() - 1, txt)
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        crate::app::ResultState { status_line, cursor: None }
    }
}

#[cfg(test)]
mod tests {
    use icy_board_tui::theme::{DOS_BLACK, DOS_CYAN};

    #[test]
    fn test_pcb_line() {
        let line = "Hello @X03World";
        let styled = super::get_styled_pcb_line(line);
        assert_eq!(styled.spans[0], ratatui::text::Span::raw("Hello "));
        assert_eq!(
            styled.spans[1],
            ratatui::text::Span::styled("World", ratatui::style::Style::default().fg(DOS_CYAN).bg(DOS_BLACK))
        );
    }
}
