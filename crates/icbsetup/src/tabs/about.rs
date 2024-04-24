use super::TabPage;
use icy_board_tui::theme::{DOS_LIGHT_BLUE, DOS_LIGHT_CYAN, DOS_LIGHT_GRAY, DOS_WHITE, THEME};
use ratatui::{
    layout::{Margin, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Widget},
    Frame,
};
use substring::Substring;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AboutTab {}

pub struct IceText {}

impl IceText {
    pub fn from_txt<'a>(text: String) -> Line<'a> {
        let mut spans = Vec::new();
        let mut color = DOS_WHITE;
        let mut offset = 0;

        for (i, c) in text.chars().enumerate() {
            let new_color = if c.is_uppercase() {
                DOS_LIGHT_GRAY
            } else if c.is_digit(10) {
                DOS_LIGHT_CYAN
            } else if c.is_ascii_punctuation() {
                DOS_LIGHT_BLUE
            } else {
                DOS_WHITE
            };
            if new_color != color {
                if offset < i {
                    spans.push(Span::from(text.substring(offset, i).to_string()).style(Style::new().fg(color)));
                }
                offset = i;
                color = new_color;
            }
        }
        if offset < text.len() {
            spans.push(Span::from(text.substring(offset, text.len()).to_string()).style(Style::new().fg(color)));
        }

        Line::from(spans)
    }
}

impl TabPage for AboutTab {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let text = vec![
            format!("IcyBoard Setup Utility v{}", crate::VERSION.to_string()),
            "written 2024 by Mike Krüger as part of the icy_board project".to_string(),
            "visit https://github.com/mkrueger/icy_board".to_string(),
            "for the latest version & discussions".to_string(),
        ];

        let width = (2 + text.iter().map(String::len).max().unwrap() as u16 + 2).min(area.width);

        let lines = (2 + text.len() as u16 + 2).min(area.height);
        let area = Rect::new(area.x + (area.width - width) / 2, (area.y + area.height - lines) / 2, width + 2, lines);

        Clear.render(area, frame.buffer_mut());
        let text: Vec<Line<'_>> = text
            .into_iter()
            .map(|t| IceText::from_txt(t).alignment(ratatui::layout::Alignment::Center))
            .collect();

        let block = Block::new()
            .style(THEME.content_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        let area2 = area.inner(&Margin { vertical: 2, horizontal: 2 });

        block.render(area, frame.buffer_mut());

        Paragraph::new(text).render(area2, frame.buffer_mut());
    }
}
