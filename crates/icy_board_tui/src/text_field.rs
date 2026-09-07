use crossterm::{
    ExecutableCommand,
    event::{KeyCode, KeyEvent, KeyModifiers},
};
use ratatui::{Frame, buffer::Buffer, layout::Rect, style::Style, text::Line, widgets::StatefulWidget};

use crate::theme::get_tui_theme;

#[derive(Default, Clone, PartialEq)]
pub struct TextfieldState {
    // String positions are UTF-8 byte offsets; cursor_column is a terminal-cell offset.
    first_char: usize,
    cursor_position: usize,
    cursor_column: u16,
    has_focus: bool,
    area: Rect,
    mask: String,
    max_len: u16,
}

static mut IS_INSERT_MODE: bool = true;

impl TextfieldState {
    pub fn set_cursor_position(&self, frame: &mut Frame) {
        if !self.area.is_empty() {
            frame.set_cursor_position((self.area.x.saturating_add(self.cursor_column.min(self.area.width - 1)), self.area.y));
        }
    }

    fn char_boundary(value: &str, offset: usize) -> usize {
        let mut offset = offset.min(value.len());
        while !value.is_char_boundary(offset) {
            offset -= 1;
        }
        offset
    }

    fn update_viewport(&mut self, value: &str) {
        self.cursor_position = Self::char_boundary(value, self.cursor_position);
        self.first_char = Self::char_boundary(value, self.first_char).min(self.cursor_position);
        if self.area.width == 0 {
            self.first_char = self.cursor_position;
            self.cursor_column = 0;
            return;
        }

        let mut width = Line::from(&value[self.first_char..self.cursor_position]).width();
        while width >= usize::from(self.area.width) && self.first_char < self.cursor_position {
            self.first_char += value[self.first_char..].chars().next().unwrap().len_utf8();
            width = Line::from(&value[self.first_char..self.cursor_position]).width();
        }
        self.cursor_column = width as u16;
    }

    pub fn max_len(&self) -> u16 {
        self.max_len
    }

    pub fn handle_input(&mut self, key: KeyEvent, value: &mut String) -> bool {
        let mut update = false;
        self.cursor_position = Self::char_boundary(value, self.cursor_position);
        match key {
            KeyEvent { code: KeyCode::Left, .. } => {
                self.cursor_position = Self::char_boundary(value, self.cursor_position.saturating_sub(1));
            }
            KeyEvent { code: KeyCode::Home, .. } => {
                self.cursor_position = 0;
            }
            KeyEvent { code: KeyCode::Right, .. } => {
                if let Some(ch) = value[self.cursor_position..].chars().next() {
                    self.cursor_position += ch.len_utf8();
                }
            }
            KeyEvent { code: KeyCode::End, .. } => {
                self.cursor_position = value.len();
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
                if self.cursor_position < value.len() {
                    value.remove(self.cursor_position);
                    update = true;
                }
            }

            KeyEvent { code: KeyCode::Backspace, .. } if self.cursor_position > 0 => {
                self.cursor_position = Self::char_boundary(value, self.cursor_position - 1);
                value.remove(self.cursor_position);
                update = true;
            }

            _ => {}
        }
        self.update_viewport(value);
        update
    }

    fn insert_key(&mut self, value: &mut String, ch: char) {
        if self.mask.is_empty() || self.mask.contains(ch) {
            self.cursor_position = Self::char_boundary(value, self.cursor_position);
            let next_position = self.cursor_position + ch.len_utf8();
            if next_position < usize::from(self.max_len) || self.max_len == 0 {
                value.insert(self.cursor_position, ch);
                self.cursor_position = next_position;
            }
        }
    }

    pub fn with_position(mut self, position: u16) -> Self {
        self.cursor_position = usize::from(position);
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

impl Default for TextField {
    fn default() -> Self {
        Self::new()
    }
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
        state.area = area;
        state.update_viewport(&self.value);
        if area.is_empty() {
            return;
        }
        let (end_x, _) = buf.set_stringn(area.x, area.y, &self.value[state.first_char..], usize::from(area.width), self.text_style);
        buf.set_string(
            end_x,
            area.y,
            self.background_symbol.to_string().repeat(usize::from(area.right().saturating_sub(end_x))),
            self.background_style,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(state: &mut TextfieldState, value: &mut String, code: KeyCode) -> bool {
        state.handle_input(KeyEvent::new(code, KeyModifiers::NONE), value)
    }

    fn render(value: &str, state: &mut TextfieldState, width: u16) -> Buffer {
        let area = Rect::new(2, 1, width, 1);
        let mut buffer = Buffer::empty(Rect::new(0, 0, width + 4, 3));
        TextField::new()
            .with_value(value.to_string())
            .with_background_symbol('_')
            .render(area, &mut buffer, state);
        assert!(value.is_char_boundary(state.cursor_position));
        assert!(value.is_char_boundary(state.first_char));
        assert!(state.first_char <= state.cursor_position);
        assert!(width == 0 || state.cursor_column < width);
        buffer
    }

    #[test]
    fn unicode_path_editing_after_selection() {
        let mut value = "/tmp/é猫😀.txt".to_string();
        let mut state = TextfieldState::default().with_position(5);
        render(&value, &mut state, 12);
        for expected in [7, 10, 14] {
            assert!(!key(&mut state, &mut value, KeyCode::Right));
            assert_eq!(state.cursor_position, expected);
        }
        key(&mut state, &mut value, KeyCode::Left);
        assert_eq!(state.cursor_position, 10);
        assert!(key(&mut state, &mut value, KeyCode::Delete));
        assert_eq!(value, "/tmp/é猫.txt");
        assert!(key(&mut state, &mut value, KeyCode::Backspace));
        assert_eq!(value, "/tmp/é.txt");
        assert_eq!(state.cursor_position, 7);
        assert!(key(&mut state, &mut value, KeyCode::Char('界')));
        assert_eq!(value, "/tmp/é界.txt");
        assert_eq!(state.cursor_position, 10);
        render(&value, &mut state, 8);
        key(&mut state, &mut value, KeyCode::Home);
        key(&mut state, &mut value, KeyCode::Left);
        assert_eq!(state.cursor_position, 0);
        key(&mut state, &mut value, KeyCode::End);
        key(&mut state, &mut value, KeyCode::Right);
        assert_eq!(state.cursor_position, value.len());
    }

    #[test]
    fn render_clamps_stale_offsets_after_replacement() {
        let mut state = TextfieldState::default().with_position(30);
        state.first_char = 20;
        render("é猫", &mut state, 6);
        assert_eq!(state.cursor_position, 5);
        assert_eq!(state.first_char, 5);

        // Both offsets came from the old ASCII value and now split UTF-8 characters.
        state.cursor_position = 4;
        state.first_char = 1;
        let buffer = render("é猫.txt", &mut state, 10);
        assert_eq!(state.cursor_position, 2);
        assert_eq!(state.first_char, 0);
        assert_eq!(state.cursor_column, 1);
        assert_eq!(buffer[(2, 1)].symbol(), "é");
        assert_eq!(buffer[(3, 1)].symbol(), "猫");
        assert_eq!(buffer[(9, 1)].symbol(), "_");

        render("", &mut state, 5);
        assert_eq!((state.cursor_position, state.first_char, state.cursor_column), (0, 0, 0));
    }

    #[test]
    fn input_clamps_stale_offsets_without_rendering() {
        for code in [KeyCode::Left, KeyCode::Right, KeyCode::Delete, KeyCode::Backspace, KeyCode::Char('😀')] {
            for offset in [1, 4, 100, usize::MAX] {
                let mut state = TextfieldState {
                    cursor_position: offset,
                    first_char: usize::MAX,
                    area: Rect::new(0, 0, 5, 1),
                    ..Default::default()
                };
                let mut value = "é猫".to_string();
                key(&mut state, &mut value, code);
                render(&value, &mut state, 5);
            }
        }
    }

    #[test]
    fn unicode_render_uses_display_columns_and_preserves_layout() {
        let mut state = TextfieldState::default();
        let buffer = render("é猫e\u{301}", &mut state, 6);
        assert_eq!(buffer[(2, 1)].symbol(), "é");
        assert_eq!(buffer[(3, 1)].symbol(), "猫");
        assert_eq!(buffer[(5, 1)].symbol(), "e\u{301}");
        assert_eq!(buffer[(6, 1)].symbol(), "_");
        assert_eq!(buffer[(7, 1)].symbol(), "_");
        assert_eq!(buffer[(8, 1)].symbol(), " ");

        state.cursor_position = "é猫e\u{301}".len();
        render("é猫e\u{301}", &mut state, 6);
        assert_eq!(state.cursor_column, 4);
        render("é猫e\u{301}", &mut state, 4);
        assert_eq!(state.first_char, 2);
        assert_eq!(state.cursor_column, 3);
    }

    #[test]
    fn small_and_empty_areas_are_safe() {
        for width in 0..=4 {
            let mut value = "猫😀é".to_string();
            let mut state = TextfieldState::default().with_position(u16::MAX);
            state.first_char = usize::MAX;
            render(&value, &mut state, width);
            for code in [
                KeyCode::Home,
                KeyCode::Right,
                KeyCode::Right,
                KeyCode::Left,
                KeyCode::Backspace,
                KeyCode::Delete,
                KeyCode::End,
            ] {
                key(&mut state, &mut value, code);
                render(&value, &mut state, width);
            }
            let buffer = render("猫", &mut TextfieldState::default(), width);
            if width == 1 {
                assert_eq!(buffer[(2, 1)].symbol(), "_");
            }
        }

        let mut state = TextfieldState::default().with_position(100);
        state.first_char = 100;
        TextField::new().render(Rect::new(0, 0, 4, 0), &mut Buffer::empty(Rect::default()), &mut state);
        assert_eq!((state.cursor_position, state.first_char), (0, 0));
        assert!(state.area.is_empty());
    }

    #[test]
    fn ascii_scrolling_masks_and_byte_limits_are_preserved() {
        let mut state = TextfieldState::default().with_mask("abcé".to_string()).with_max_len(4);
        let mut value = String::new();
        render(&value, &mut state, 3);
        for ch in ['a', 'b', 'c', 'a', 'x'] {
            key(&mut state, &mut value, KeyCode::Char(ch));
        }
        assert_eq!(value, "abc");
        assert_eq!((state.cursor_position, state.first_char, state.cursor_column), (3, 1, 2));
        key(&mut state, &mut value, KeyCode::Home);
        assert_eq!(state.first_char, 0);
        state.handle_input(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::SHIFT), &mut value);
        assert_eq!(value, "abc"); // Uppercase A is outside the mask.
        value.clear();
        key(&mut state, &mut value, KeyCode::Char('é'));
        key(&mut state, &mut value, KeyCode::Char('é'));
        key(&mut state, &mut value, KeyCode::Char('a'));
        assert_eq!(value, "éa");
        assert_eq!(state.cursor_position, 3);
    }

    #[test]
    fn long_values_do_not_truncate_byte_offsets_to_u16() {
        let mut value = "é".repeat(usize::from(u16::MAX));
        let mut state = TextfieldState::default();
        key(&mut state, &mut value, KeyCode::End);
        assert_eq!(state.cursor_position, value.len());
        key(&mut state, &mut value, KeyCode::Backspace);
        assert_eq!(state.cursor_position, value.len());
        render(&value, &mut state, 4);
    }
}
