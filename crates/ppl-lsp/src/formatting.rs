use icy_board_engine::formatting::{FormattingBackend, blank_line_edit, space_around};
use ropey::Rope;
use tower_lsp::lsp_types::{Range, TextEdit};

use crate::offset_to_position;

pub struct VSCodeFormattingBackend<'a> {
    pub edits: Vec<TextEdit>,
    pub rope: &'a Rope,
}

impl<'a> VSCodeFormattingBackend<'a> {
    fn char_at(&self, offset: usize) -> Option<char> {
        (offset < self.rope.len_chars()).then(|| self.rope.char(offset))
    }

    fn push(&mut self, range: std::ops::Range<usize>, new_text: String) {
        let (Some(start), Some(end)) = (offset_to_position(range.start, self.rope), offset_to_position(range.end, self.rope)) else {
            return;
        };
        self.edits.push(TextEdit {
            range: Range::new(start, end),
            new_text,
        });
    }
}

impl<'a> FormattingBackend for VSCodeFormattingBackend<'a> {
    fn ensure_text_or_newline(&mut self, range: std::ops::Range<usize>, arg: &str) {
        for i in range.start..range.end {
            let Some(c) = self.char_at(i) else {
                return;
            };
            if c != ' ' && c != '\t' {
                return;
            }
        }
        self.push(range, arg.to_string());
    }

    fn indent(&mut self, indent_str: &str, range: core::ops::Range<usize>) {
        let mut i = range.start;
        while i > 0 {
            let Some(c) = self.char_at(i - 1) else {
                return;
            };
            if c == '\n' || c == '\r' {
                break;
            }
            // Something else stands on this line, so this is not its indentation.
            if c != ' ' && c != '\t' {
                return;
            }
            i -= 1;
        }
        if i != range.start || !indent_str.is_empty() {
            self.push(i..range.start, indent_str.to_string());
        }
    }

    fn ensure_space_before(&mut self, start: usize) {
        if start == 0 {
            return;
        }
        let mut i = start - 1;
        while i > 0 {
            let Some(c) = self.char_at(i) else {
                return;
            };
            if c == '\n' || c == '\r' {
                return;
            }
            if c != ' ' && c != '\t' {
                break;
            }
            i -= 1;
        }
        let text = if self.char_at(i) == Some('(') { String::new() } else { " ".to_string() };
        self.push(i + 1..start, text);
    }

    fn ensure_no_space_after(&mut self, start: usize) {
        let mut i = start;
        while i < self.rope.len_chars() {
            let c = self.rope.char(i);
            if c == '\n' {
                return;
            }
            if c != ' ' && c != '\t' {
                break;
            }
            i += 1;
        }
        // A comment keeps the distance it was written with.
        if matches!(self.char_at(i), Some(';') | Some('\'')) {
            return;
        }
        self.push(start..i, String::new());
    }

    fn ensure_no_space_before(&mut self, start: usize) {
        if start == 0 {
            return;
        }
        let mut i = start;
        while i > 0 {
            let Some(c) = self.char_at(i - 1) else {
                return;
            };
            if c != ' ' && c != '\t' {
                break;
            }
            i -= 1;
        }
        if i < start {
            self.push(i..start, String::new());
        }
    }

    fn ensure_space_around(&mut self, range: std::ops::Range<usize>) {
        let Some(slice) = self.rope.get_slice(range.clone()) else {
            return;
        };
        let text = slice.to_string();
        let Some(replacement) = space_around(&text) else {
            return;
        };
        if text != replacement {
            self.push(range, replacement);
        }
    }

    fn limit_blank_lines(&mut self, before: usize, max: usize) {
        if before > self.rope.len_chars() {
            return;
        }
        let mut start = before;
        while start > 0 && self.rope.char(start - 1).is_whitespace() {
            start -= 1;
        }
        let newlines: Vec<usize> = (start..before).filter(|i| self.rope.char(*i) == '\n').collect();
        let Some((inner, replacement)) = blank_line_edit(&newlines, max) else {
            return;
        };
        self.push(inner, replacement);
    }
}
