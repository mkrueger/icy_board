use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Widget, Wrap},
};

use crate::{
    BORDER_SET,
    chrome::dim_background,
    get_text,
    hotkeys::HotkeyBar,
    tab_page::{InfoState, Page, PageMessage},
    theme::get_tui_theme,
};

/// A modal that reports what went wrong and waits for a key.
pub struct MessageBox {
    title: String,
    message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    #[test]
    fn message_keeps_background_symbols_and_themes_title_and_footer() {
        let mut message = MessageBox::new(InfoState::Warning, "Keep this message".into());
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        let mut expected = Buffer::empty(Rect::new(0, 0, 80, 25));
        terminal
            .draw(|frame| {
                let area = frame.area();
                Line::styled("Background", get_tui_theme().item).render(area, frame.buffer_mut());
                expected = frame.buffer_mut().clone();
                dim_background(&mut expected, area);
                message.render(frame, area);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(0, 0)], expected[(0, 0)]);
        for (y, text) in [(10, message.title.clone()), (14, HotkeyBar::for_id("message_box_dismiss").line().to_string())] {
            let row: String = (0..80).map(|x| buffer[(x, y)].symbol()).collect();
            assert!(row.contains(&text));
        }
        let title_x = 10 + (60 - Line::raw(&message.title).width() as u16) / 2;
        assert_eq!(buffer[(title_x, 10)].fg, get_tui_theme().dialog_box_title.fg.unwrap());
        assert!(message.is_modal());
        assert!(matches!(message.handle_key_press(KeyCode::F(1).into()), PageMessage::None));
        for key in [KeyCode::Enter, KeyCode::Esc, KeyCode::Char(' ')] {
            assert!(matches!(message.handle_key_press(key.into()), PageMessage::Close));
        }
    }

    #[test]
    fn message_stays_inside_tiny_and_offset_areas() {
        for (width, height) in [(0, 0), (1, 1), (2, 2), (8, 3), (20, 5), (80, 25)] {
            let mut message = MessageBox::new(InfoState::Error, "界 e\u{301}\nSecond line".into());
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| message.render(frame, frame.area())).unwrap();
        }
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        let mut message = MessageBox::new(InfoState::Info, "Offset".into());
        terminal.draw(|frame| message.render(frame, Rect::new(75, 23, 20, 10))).unwrap();
        assert_eq!(terminal.backend().buffer()[(74, 23)].symbol(), " ");
    }
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
        let backdrop = frame.area();
        dim_background(frame.buffer_mut(), backdrop);
        let area = area.intersection(backdrop);
        let width = area.width.min(60);
        let lines: Vec<Line> = self.message.lines().map(Line::raw).collect();
        let height = lines.len().saturating_add(4).min(area.height as usize) as u16;
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
            .title(Span::styled(self.title.clone(), get_tui_theme().dialog_box_title))
            .title_bottom(HotkeyBar::for_id("message_box_dismiss").line());

        Paragraph::new(Text::from(lines))
            .style(get_tui_theme().item)
            .wrap(Wrap { trim: false })
            .block(block)
            .render(area, frame.buffer_mut());
    }

    fn is_modal(&self) -> bool {
        true
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        match key.code {
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char(' ') => PageMessage::Close,
            _ => PageMessage::None,
        }
    }
}
