use crossterm::event::KeyCode;
use icy_board_engine::icy_board::icb_text::{IcbTextFile, IcbTextStyle, IceText, TextEntry, DEFAULT_DISPLAY_TEXT};
use icy_board_tui::theme::{
    DOS_BLACK, DOS_BLUE, DOS_BROWN, DOS_CYAN, DOS_DARK_GRAY, DOS_GREEN, DOS_LIGHT_BLUE, DOS_LIGHT_CYAN, DOS_LIGHT_GRAY, DOS_LIGHT_GREEN, DOS_LIGHT_MAGENTA,
    DOS_LIGHT_RED, DOS_MAGENTA, DOS_RED, DOS_WHITE, DOS_YELLOW,
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

#[derive(Debug)]
enum PcbState {
    Default,
    GotAt,
    ReadColor1,
    ReadColor2,
}

pub fn get_styled_pcb_line(txt: &str) -> Line {
    let mut spans = Vec::new();

    let mut span_builder = String::new();
    let mut last_fg = None;
    let mut last_bg = None;

    let mut cur_fg_color = None;
    let mut cur_bg_color = None;
    let mut state = PcbState::Default;

    for ch in txt.chars() {
        match state {
            PcbState::ReadColor1 => {
                match ch.to_ascii_uppercase() {
                    '0' => cur_bg_color = Some(DOS_BLACK),
                    '1' => cur_bg_color = Some(DOS_BLUE),
                    '2' => cur_bg_color = Some(DOS_GREEN),
                    '3' => cur_bg_color = Some(DOS_CYAN),
                    '4' => cur_bg_color = Some(DOS_RED),
                    '5' => cur_bg_color = Some(DOS_MAGENTA),
                    '6' => cur_bg_color = Some(DOS_BROWN),
                    '7' => cur_bg_color = Some(DOS_LIGHT_GRAY),

                    '8' => cur_bg_color = Some(DOS_BLACK),
                    '9' => cur_bg_color = Some(DOS_BLUE),
                    'A' => cur_bg_color = Some(DOS_GREEN),
                    'B' => cur_bg_color = Some(DOS_CYAN),
                    'C' => cur_bg_color = Some(DOS_RED),
                    'D' => cur_bg_color = Some(DOS_MAGENTA),
                    'E' => cur_bg_color = Some(DOS_BROWN),
                    'F' => cur_bg_color = Some(DOS_LIGHT_GRAY),

                    _ => {
                        span_builder.push('@');
                        span_builder.push('X');
                        span_builder.push(ch);
                        state = PcbState::Default;
                        continue;
                    }
                }
                state = PcbState::ReadColor2;
            }

            PcbState::ReadColor2 => {
                match ch.to_ascii_uppercase() {
                    '0' => cur_fg_color = Some(DOS_BLACK),
                    '1' => cur_fg_color = Some(DOS_BLUE),
                    '2' => cur_fg_color = Some(DOS_GREEN),
                    '3' => cur_fg_color = Some(DOS_CYAN),
                    '4' => cur_fg_color = Some(DOS_RED),
                    '5' => cur_fg_color = Some(DOS_MAGENTA),
                    '6' => cur_fg_color = Some(DOS_BROWN),
                    '7' => cur_fg_color = Some(DOS_LIGHT_GRAY),

                    '8' => cur_fg_color = Some(DOS_DARK_GRAY),
                    '9' => cur_fg_color = Some(DOS_LIGHT_BLUE),
                    'A' => cur_fg_color = Some(DOS_LIGHT_GREEN),
                    'B' => cur_fg_color = Some(DOS_LIGHT_CYAN),
                    'C' => cur_fg_color = Some(DOS_LIGHT_RED),
                    'D' => cur_fg_color = Some(DOS_LIGHT_MAGENTA),
                    'E' => cur_fg_color = Some(DOS_YELLOW),
                    'F' => cur_fg_color = Some(DOS_WHITE),

                    _ => {
                        span_builder.push('@');
                        span_builder.push('X');
                        span_builder.push('0');
                        span_builder.push(ch);
                    }
                }
                state = PcbState::Default;
            }

            PcbState::GotAt => {
                if ch.to_ascii_uppercase() == 'X' {
                    state = PcbState::ReadColor1;
                } else {
                    span_builder.push('@');
                    span_builder.push(ch);
                    state = PcbState::Default;
                }
            }

            PcbState::Default => {
                if last_fg != cur_fg_color || last_bg != cur_bg_color {
                    if !span_builder.is_empty() {
                        if let (Some(fg), Some(bg)) = (last_bg, last_fg) {
                            spans.push(Span::styled(span_builder.clone(), Style::default().fg(fg).fg(bg)));
                        } else {
                            spans.push(Span::raw(span_builder.clone()));
                        }
                        span_builder.clear();
                    }
                    last_fg = cur_fg_color;
                    last_bg = cur_bg_color;
                }

                if ch == '@' {
                    state = PcbState::GotAt;
                } else {
                    span_builder.push(ch);
                }
            }
        }
    }

    if !span_builder.is_empty() {
        if let (Some(fg), Some(bg)) = (cur_fg_color, cur_bg_color) {
            spans.push(Span::styled(span_builder.clone(), Style::default().fg(fg).bg(bg)));
        } else {
            spans.push(Span::raw(span_builder.clone()));
        }
        span_builder.clear();
    }
    Line::from(spans)
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
