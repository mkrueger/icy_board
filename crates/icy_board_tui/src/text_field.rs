use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{buffer::Buffer, layout::Rect, style::Style, widgets::StatefulWidget, Frame};

use crate::theme::{DOS_BLUE, DOS_LIGHT_CYAN, DOS_LIGHT_GRAY};

#[derive(Default, Clone, PartialEq)]
pub struct TextfieldState {
    first_char: usize,
    cursor_position: u16,
    has_focus: bool,
    area: Rect,
    mask: String,
    max_len: u16,
}

impl TextfieldState {
    pub fn set_cursor_position(&self, frame: &mut Frame) {
        frame.set_cursor(self.area.x + self.cursor_position, self.area.y);
    }

    pub fn max_len(&self) -> u16 {
        self.max_len
    }

    pub fn handle_input(&mut self, key: KeyEvent, value: &mut String) {
        self.cursor_position = self.cursor_position.min(value.len() as u16);
        match key {
            KeyEvent { code: KeyCode::Left, .. } => {
                self.cursor_position = self.cursor_position.saturating_sub(1);
            }
            KeyEvent { code: KeyCode::Home, .. } => {
                self.cursor_position = 0;
            }
            KeyEvent { code: KeyCode::Right, .. } => {
                self.cursor_position = (self.cursor_position + 1).min(value.len() as u16);
            }
            KeyEvent { code: KeyCode::End, .. } => {
                self.cursor_position = value.len() as u16;
            }
            KeyEvent {
                code: KeyCode::Char(ch),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.insert_key(value, ch);
            }
            KeyEvent {
                code: KeyCode::Char(ch),
                modifiers: KeyModifiers::SHIFT,
                ..
            } => {
                self.insert_key(value, ch.to_ascii_uppercase());
            }

            KeyEvent { code: KeyCode::Delete, .. } => {
                if self.cursor_position < value.len() as u16 {
                    value.remove(self.cursor_position as usize);
                }
            }

            KeyEvent { code: KeyCode::Backspace, .. } => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    value.remove(self.cursor_position as usize);
                }
            }

            _ => {}
        }
    }

    fn insert_key(&mut self, value: &mut String, ch: char) {
        if self.mask.is_empty() || self.mask.contains(ch) {
            if self.cursor_position + 1 < self.max_len || self.max_len == 0 {
                value.insert(self.cursor_position as usize, ch);
                self.cursor_position += 1;
            }
        }
    }

    pub fn with_position(mut self, position: u16) -> Self {
        self.cursor_position = position;
        self
    }

    pub fn with_mask(mut self, mask: String) -> Self {
        self.mask = mask;
        self
    }

    pub fn with_max_len(mut self, max_len: u16) -> Self {
        self.max_len = max_len;
        self
    }
}

pub struct TextField {
    value: String,
    text_style: Style,
    background_style: Style,
    background_symbol: char,
}

impl TextField {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            text_style: Style::default().fg(DOS_LIGHT_CYAN).bg(DOS_BLUE),
            background_style: Style::default().fg(DOS_LIGHT_GRAY).bg(DOS_BLUE),
            background_symbol: '▒',
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn with_value(mut self, value: String) -> Self {
        self.value = value;
        self
    }

    pub fn with_text_style(mut self, style: Style) -> Self {
        self.text_style = style;
        self
    }

    pub fn with_background_style(mut self, style: Style) -> Self {
        self.background_style = style;
        self
    }

    pub fn with_background_symbol(mut self, symbol: char) -> Self {
        self.background_symbol = symbol;
        self
    }
}

impl StatefulWidget for TextField {
    type State = TextfieldState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if area.is_empty() {
            return;
        }
        state.area = area;
        buf.set_string(area.x, area.y, self.value[state.first_char..].to_string(), self.text_style);
        let len = self.value.len().saturating_sub(state.first_char);
        buf.set_string(
            area.x + len as u16,
            area.y,
            self.background_symbol.to_string().repeat((area.width as usize).saturating_sub(len)),
            self.background_style,
        );
    }
}
