use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::icy_board::commands::Position;
use icy_engine::{Buffer, TextPane};
use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    text::Span,
    widgets::{Clear, Widget},
    Frame,
};

pub struct PositionEditor {
    pub buffer: Buffer,
}

impl PositionEditor {
    pub fn ui(&self, frame: &mut Frame, pos: &Position, area: Rect) {
        let buffer = &self.buffer;
        Clear.render(area, frame.buffer_mut());
        for y in 0..area.height as i32 {
            for x in 0..area.width as i32 {
                let c = buffer.get_char((x, y + buffer.get_first_visible_line()));
                let mut fg = c.attribute.get_foreground();
                if c.attribute.is_bold() {
                    fg += 8;
                }
                let fg = buffer.palette.get_color(fg).get_rgb();
                let bg = buffer.palette.get_color(c.attribute.get_background()).get_rgb();
                let mut s: Style = Style::new().bg(Color::Rgb(bg.0, bg.1, bg.2)).fg(Color::Rgb(fg.0, fg.1, fg.2));
                if c.attribute.is_blinking() {
                    s = s.slow_blink();
                }
                let span = Span::from(c.ch.to_string()).style(s);
                frame.buffer_mut().set_span(area.x + x as u16, area.y + y as u16, &span, 1);
            }
        }

        frame.set_cursor_position((area.x + pos.x, area.y + pos.y));
    }

    pub fn handle_event(&mut self, event: KeyEvent, pos: &Position) -> Position {
        let mut res = pos.clone();

        match event.code {
            KeyCode::Char('h') | KeyCode::Left => res.x = res.x.saturating_sub(1),
            KeyCode::Char('l') | KeyCode::Right => res.x = (res.x + 1).min(self.buffer.get_width() as u16 - 1),
            KeyCode::Char('k') | KeyCode::Up => res.y = res.y.saturating_sub(1),
            KeyCode::Char('j') | KeyCode::Down => res.y = (res.y + 1).min(self.buffer.get_height() as u16 - 1),
            _ => {}
        }
        res
    }
}
