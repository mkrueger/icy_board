use crossterm::{
    ExecutableCommand,
    event::{KeyCode, KeyEvent, KeyModifiers},
};
use ratatui::{Frame, buffer::Buffer, layout::Rect, style::Style, widgets::StatefulWidget};

use crate::theme::get_tui_theme;

#[derive(Default, Clone, PartialEq)]
pub struct TextfieldState {
    first_char: usize,
    cursor_position: u16,
    has_focus: bool,
    area: Rect,
    mask: String,
    max_len: u16,
}

static mut IS_INSERT_MODE: bool = true;

impl TextfieldState {
    pub fn set_cursor_position(&self, frame: &mut Frame) {
        frame.set_cursor_position((self.area.x + self.cursor_position - self.first_char as u16, self.area.y));
    }

    pub fn max_len(&self) -> u16 {
        self.max_len
    }

    pub fn handle_input(&mut self, key: KeyEvent, value: &mut String) -> bool {
        let mut update = false;
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
            KeyEvent { code: KeyCode::Insert, .. } => {
                unsafe {
                    IS_INSERT_MODE = !IS_INSERT_MODE;
                }
                set_cursor_mode();
            }

            KeyEvent {
                code: KeyCode::Char(ch),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.insert_key(value, ch);
                update = true;
            }
            KeyEvent {
                code: KeyCode::Char(ch),
                modifiers: KeyModifiers::SHIFT,
                ..
            } => {
                self.insert_key(value, ch.to_ascii_uppercase());
                update = true;
            }

            KeyEvent { code: KeyCode::Delete, .. } => {
                if self.cursor_position < value.len() as u16 {
                    value.remove(self.cursor_position as usize);
                    update = true;
                }
            }

            KeyEvent { code: KeyCode::Backspace, .. } => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    value.remove(self.cursor_position as usize);
                    update = true;
                }
            }

            _ => {}
        }
        if self.cursor_position < self.first_char as u16 {
            self.first_char = self.cursor_position as usize;
        } else if self.cursor_position >= self.first_char as u16 + self.area.width {
            self.first_char = (self.cursor_position as usize + 1 - self.area.width as usize).min(value.len());
        }
        update
    }

    fn insert_key(&mut self, value: &mut String, ch: char) {
        if self.mask.is_empty() || self.mask.contains(ch) {
            self.cursor_position = self.cursor_position.clamp(0, value.len() as u16);
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

pub fn set_cursor_mode() {
    unsafe {
        if IS_INSERT_MODE {
            let _ = std::io::stdout().execute(crossterm::cursor::SetCursorStyle::BlinkingBar);
        } else {
            let _ = std::io::stdout().execute(crossterm::cursor::SetCursorStyle::BlinkingBlock);
        }
    }
}

pub struct TextField {
    value: String,
    text_style: Style,
    background_style: Style,
    background_symbol: char,
    max_len: usize,
}

impl TextField {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            text_style: get_tui_theme().text_field_text,
            background_style: get_tui_theme().text_field_background,
            background_symbol: get_tui_theme().text_field_filler_char,
            max_len: 0,
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

    pub fn with_max_len(mut self, max_len: usize) -> Self {
        self.max_len = max_len;
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
        let mut s = self.value[state.first_char..].to_string();
        while s.len() as u16 > area.width {
            s.pop();
        }

        buf.set_string(area.x, area.y, s, self.text_style);
        let len = self.value.len().saturating_sub(state.first_char);
        buf.set_string(
            area.x + len as u16,
            area.y,
            self.background_symbol.to_string().repeat((area.width as usize).saturating_sub(len)),
            self.background_style,
        );
    }
}
