pub trait FormattingBackend {
    fn ensure_text_or_newline(&mut self, start: std::ops::Range<usize>, arg: &str);
    fn indent(&mut self, indent: &str, span: core::ops::Range<usize>);
    fn ensure_space_before(&mut self, start: usize);
    fn ensure_no_space_after(&mut self, start: usize);
}

pub struct StringFormattingBackend {
    pub text: Vec<char>,
    pub edits: Vec<(std::ops::Range<usize>, String)>,
}

impl FormattingBackend for StringFormattingBackend {
    fn ensure_text_or_newline(&mut self, range: std::ops::Range<usize>, arg: &str) {
        for i in range.start..range.end {
            let c = self.text[i];
            if c != ' ' && c != '\t' {
                return;
            }
        }
        self.edits.push((range, arg.to_string()));
    }

    fn indent(&mut self, indent_str: &str, range: core::ops::Range<usize>) {
        let mut i: usize = range.start - 1;
        while i > 0 {
            let c = self.text[i];
            if c == '\n' || c == '\r' {
                break;
            }
            if c != ' ' && c != '\t' {
                break;
            }
            i -= 1;
        }
        self.edits.push((i + 1..range.start, indent_str.to_string()));
    }

    fn ensure_space_before(&mut self, start: usize) {
        let mut i: usize = start - 1;
        while i > 0 {
            let c = self.text[i];
            if c == '\n' || c == '\r' {
                return;
            }
            if c != ' ' && c != '\t' {
                break;
            }
            i -= 1;
        }
        let str = if self.text[i] == '(' { String::new() } else { " ".to_string() };
        self.edits.push((i + 1..start, str));
    }

    fn ensure_no_space_after(&mut self, start: usize) {
        let mut i = start;
        while i < self.text.len() {
            if self.text[i] == '\n' {
                return;
            }
            if self.text[i] != ' ' && self.text[i] != '\t' {
                break;
            }
            i += 1;
        }
        self.edits.push((start..i, String::new()));
    }
}
