pub trait FormattingBackend {
    fn ensure_text_or_newline(&mut self, start: std::ops::Range<usize>, arg: &str);
    fn indent(&mut self, indent: &str, span: core::ops::Range<usize>);
    fn ensure_space_before(&mut self, start: usize);
    fn ensure_no_space_after(&mut self, start: usize);
    fn ensure_no_space_before(&mut self, start: usize);
    /// Puts one space on each side of what stands between two nodes, an operator
    /// the syntax tree does not keep a token for.
    fn ensure_space_around(&mut self, range: std::ops::Range<usize>);
    /// Leaves at most `max` blank lines in front of `before`. Statement spans do
    /// not always reach to the last token, so the gap is measured from the text.
    fn limit_blank_lines(&mut self, before: usize, max: usize);
}

pub struct StringFormattingBackend {
    pub text: Vec<char>,
    pub edits: Vec<(std::ops::Range<usize>, String)>,
}

impl StringFormattingBackend {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.chars().collect(),
            edits: Vec::new(),
        }
    }

    /// The text with every edit applied, from the back so the offsets hold.
    pub fn apply(mut self) -> String {
        self.edits.sort_by_key(|(range, _)| range.start);
        for (range, edit) in self.edits.iter().rev() {
            self.text.splice(range.clone(), edit.chars());
        }
        self.text.iter().collect()
    }
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
        let mut i = range.start;
        while i > 0 {
            let c = self.text[i - 1];
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
            self.edits.push((i..range.start, indent_str.to_string()));
        }
    }

    fn ensure_space_before(&mut self, start: usize) {
        if start == 0 {
            return;
        }
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
        // A comment keeps the distance it was written with.
        if i < self.text.len() && (self.text[i] == ';' || self.text[i] == '\'') {
            return;
        }
        self.edits.push((start..i, String::new()));
    }

    fn ensure_no_space_before(&mut self, start: usize) {
        if start == 0 {
            return;
        }
        let mut i = start;
        while i > 0 {
            let c = self.text[i - 1];
            if c != ' ' && c != '\t' {
                break;
            }
            i -= 1;
        }
        if i < start {
            self.edits.push((i..start, String::new()));
        }
    }

    fn ensure_space_around(&mut self, range: std::ops::Range<usize>) {
        let text: String = self.text[range.clone()].iter().collect();
        let Some(replacement) = space_around(&text) else {
            return;
        };
        if text != replacement {
            self.edits.push((range, replacement));
        }
    }

    fn limit_blank_lines(&mut self, before: usize, max: usize) {
        if before > self.text.len() {
            return;
        }
        let mut start = before;
        while start > 0 && self.text[start - 1].is_whitespace() {
            start -= 1;
        }
        let newlines: Vec<usize> = (start..before).filter(|i| self.text[*i] == '\n').collect();
        let Some((inner, replacement)) = blank_line_edit(&newlines, max) else {
            return;
        };
        self.edits.push((inner, replacement));
    }
}

/// The stretch between the first and the last line break that has to go, and
/// what is left of it.
pub fn blank_line_edit(newlines: &[usize], max: usize) -> Option<(std::ops::Range<usize>, String)> {
    let max = max.max(1);
    if newlines.len() <= max + 1 {
        return None;
    }
    let first = newlines[0];
    let last = newlines[newlines.len() - 1];
    Some((first + 1..last, "\n".repeat(max - 1)))
}

/// ` = ` for what stands between two nodes, or nothing when a line break or a
/// comment is in the way.
pub fn space_around(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() || text.contains('\n') || trimmed.contains(';') || trimmed.contains('\'') {
        return None;
    }
    Some(format!(" {trimmed} "))
}
