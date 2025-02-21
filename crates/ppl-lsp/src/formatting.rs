use icy_board_engine::formatting::FormattingBackend;
use ropey::Rope;
use tower_lsp::lsp_types::{Position, Range, TextEdit};

use crate::offset_to_position;

pub struct VSCodeFormattingBackend<'a> {
    pub edits: Vec<TextEdit>,
    pub rope: &'a Rope,
}

impl<'a> FormattingBackend for VSCodeFormattingBackend<'a> {
    fn ensure_text_or_newline(&mut self, start: std::ops::Range<usize>, arg: &str) {
        let from = offset_to_position(start.start, &self.rope).unwrap();
        let to = offset_to_position(start.end, &self.rope).unwrap();

        if let Some(slice) = self.rope.get_slice(start.start..start.end) {
            if let Some(str) = slice.as_str() {
                for c in str.chars() {
                    if c != ' ' && c != '\t' {
                        return;
                    }
                }
            }
        }

        self.edits.push(TextEdit {
            range: Range::new(from, to),
            new_text: arg.to_string(),
        });
    }

    fn indent(&mut self, indent_str: &str, span: core::ops::Range<usize>) {
        let start = span.start;
        let line: usize = self.rope.try_char_to_line(start).unwrap();
        if let Some(line_span) = self.rope.get_line(line) {
            if let Some(slice) = line_span.as_str() {
                let chars = slice.chars().collect::<Vec<char>>();
                let mut i = 0;
                while i < chars.len() {
                    if chars[i] != ' ' && chars[i] != '\t' {
                        break;
                    }
                    i += 1;
                }

                self.edits.push(TextEdit {
                    range: Range::new(Position::new(line as u32, 0), Position::new(line as u32, i as u32)),
                    new_text: indent_str.into(),
                });
            }
        }
    }

    fn ensure_space_before(&mut self, start: usize) {
        let line: usize = self.rope.try_char_to_line(start).unwrap();
        let start_line_offset = self.rope.line_to_char(line);
        let char_in_line = start - start_line_offset;

        if let Some(line_span) = self.rope.get_line(line) {
            if let Some(slice) = line_span.as_str() {
                let chars = slice.chars().collect::<Vec<char>>();
                let mut i: usize = char_in_line - 1;
                while i > 0 {
                    if chars[i] != ' ' && chars[i] != '\t' {
                        break;
                    }
                    i -= 1;
                }
                if i == 0 {
                    return;
                }
                self.edits.push(TextEdit {
                    range: Range::new(Position::new(line as u32, i as u32 + 1), Position::new(line as u32, char_in_line as u32)),
                    new_text: if chars[i] == '(' { String::new() } else { " ".to_string() },
                });
            }
        }
    }

    fn ensure_no_space_after(&mut self, start: usize) {
        let line: usize = self.rope.try_char_to_line(start).unwrap();
        let start_line_offset = self.rope.line_to_char(line);
        let char_in_line = start - start_line_offset;

        if let Some(line_span) = self.rope.get_line(line) {
            if let Some(slice) = line_span.as_str() {
                let chars = slice.chars().collect::<Vec<char>>();
                let mut i: usize = char_in_line;
                while i < chars.len() {
                    if chars[i] != ' ' && chars[i] != '\t' {
                        break;
                    }
                    i += 1;
                }
                self.edits.push(TextEdit {
                    range: Range::new(Position::new(line as u32, char_in_line as u32), Position::new(line as u32, i as u32)),
                    new_text: String::new(),
                });
            }
        }
    }
}
