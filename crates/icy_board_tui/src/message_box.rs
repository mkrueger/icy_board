use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Text},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Widget, Wrap},
};

use crate::{
    BORDER_SET, get_text,
    tab_page::{InfoState, Page, PageMessage},
    theme::get_tui_theme,
};

/// A modal that reports what went wrong and waits for a key.
pub struct MessageBox {
    title: String,
    message: String,
}

impl MessageBox {
    pub fn new(state: InfoState, message: String) -> Self {
        let title = match state {
            InfoState::Info => get_text("message_box_info_title"),
            InfoState::Warning => get_text("message_box_warning_title"),
            InfoState::Error => get_text("message_box_error_title"),
        };
        Self { title, message }
    }
}

impl Page for MessageBox {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let width = area.width.clamp(20, 60);
        let lines: Vec<Line> = self.message.lines().map(Line::raw).collect();
        let height = (lines.len() as u16 + 4).min(area.height);
        let area = Rect {
            x: area.x + (area.width.saturating_sub(width)) / 2,
            y: area.y + (area.height.saturating_sub(height)) / 2,
            width,
            height,
        };
        Clear.render(area, frame.buffer_mut());

        let block = Block::new()
            .style(get_tui_theme().background)
            .borders(Borders::ALL)
            .border_set(BORDER_SET)
            .border_style(get_tui_theme().menu_box)
            .padding(Padding::new(1, 1, 1, 0))
            .title_alignment(ratatui::layout::Alignment::Center)
            .title(self.title.clone())
            .title_bottom(get_text("message_box_dismiss"));

        Paragraph::new(Text::from(lines))
            .style(get_tui_theme().item)
            .wrap(Wrap { trim: false })
            .block(block)
            .render(area, frame.buffer_mut());
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        match key.code {
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char(' ') => PageMessage::Close,
            _ => PageMessage::None,
        }
    }
}
