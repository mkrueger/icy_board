use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Margin, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Widget},
    Frame,
};

use crate::{
    get_text,
    theme::{get_tui_theme, DOS_DARK_GRAY, DOS_LIGHT_GRAY, DOS_WHITE},
};

pub enum SaveChangesMessage {
    None,
    Cancel,
    Save,
    Close,
}

pub struct SaveChangesDialog {
    save: bool,
}

impl SaveChangesDialog {
    pub fn new() -> Self {
        Self { save: false }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let save_text = format!("{} ", get_text("icbtext_save_changes"));
        let mut save_area = Rect::new(
            area.x + (area.width - (save_text.len() as u16 + 10)) / 2,
            area.y + (area.height - 3) / 2,
            save_text.len() as u16 + 10,
            3,
        );

        Clear.render(save_area, frame.buffer_mut());

        Block::new()
            .borders(Borders::ALL)
            .style(get_tui_theme().content_box)
            .border_type(BorderType::Plain)
            .render(save_area, frame.buffer_mut());

        let field = Line::from(vec![
            Span::styled(save_text, Style::default().fg(DOS_LIGHT_GRAY)),
            Span::styled(get_text("yes"), Style::default().fg(if self.save { DOS_WHITE } else { DOS_DARK_GRAY })),
            Span::styled("/", Style::default().fg(DOS_LIGHT_GRAY)),
            Span::styled(get_text("no"), Style::default().fg(if !self.save { DOS_WHITE } else { DOS_DARK_GRAY })),
        ]);
        save_area.y += 1;
        save_area.x += 1;
        field.render(save_area.inner(Margin { horizontal: 1, vertical: 0 }), frame.buffer_mut());
    }

    pub fn handle_key_press(&mut self, key: KeyEvent) -> SaveChangesMessage {
        use KeyCode::*;
        match key.code {
            Left | Right => {
                self.save = !self.save;
                SaveChangesMessage::None
            }
            Enter => {
                if self.save {
                    SaveChangesMessage::Save
                } else {
                    SaveChangesMessage::Close
                }
            }
            Esc => SaveChangesMessage::Cancel,
            _ => SaveChangesMessage::None,
        }
    }
}
