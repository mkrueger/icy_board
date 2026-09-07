//! Character-oriented editor operations. Cursor columns are never UTF-8 offsets.
use super::{EditResult, EditState, EditUpdate, IceText, IcyBoardState, Res, TerminalTarget, display_flags};

impl EditState {
    pub(super) fn byte_index(text: &str, column: usize) -> usize {
        text.char_indices().nth(column).map_or(text.len(), |(offset, _)| offset)
    }

    pub(super) fn parse_command(input: &str) -> (String, Vec<String>) {
        let mut parts = input.split_whitespace();
        (parts.next().unwrap_or_default().to_ascii_uppercase(), parts.map(str::to_string).collect())
    }

    pub(super) fn save_result(&self, command: &str) -> EditResult {
        if self.msg.iter().all(|line| line.trim().is_empty()) {
            return EditResult::Abort;
        }
        match command {
            "S" => EditResult::SendMessage,
            "SC" => EditResult::CarbonCopy,
            "SA" => EditResult::AttachFile,
            "SN" => EditResult::SendNext,
            "SK" => EditResult::SendKill,
            _ => EditResult::Abort,
        }
    }

    /// PCBoard replaces the first case-sensitive match; a missing semicolon
    /// means deletion. An empty search is deliberately a no-op.
    pub(super) fn substitute(&mut self, y: usize, expression: &str) -> bool {
        let (old, new) = expression.split_once(';').unwrap_or((expression, ""));
        let Some(line) = self.msg.get_mut(y) else { return false };
        if old.is_empty() || !line.contains(old) {
            return false;
        }
        *line = line
            .replacen(old, new, 1)
            .chars()
            .filter(|c| !c.is_control())
            .take(self.max_line_length)
            .collect();
        true
    }

    pub(super) fn word_left(text: &str, x: usize) -> usize {
        let chars: Vec<char> = text.chars().collect();
        let mut x = x.min(chars.len());
        while x > 0 && chars[x - 1].is_whitespace() {
            x -= 1;
        }
        while x > 0 && !chars[x - 1].is_whitespace() {
            x -= 1;
        }
        x
    }

    pub(super) fn word_right(text: &str, x: usize) -> usize {
        let chars: Vec<char> = text.chars().collect();
        let mut x = x.min(chars.len());
        while x < chars.len() && !chars[x].is_whitespace() {
            x += 1;
        }
        while x < chars.len() && chars[x].is_whitespace() {
            x += 1;
        }
        x
    }

    pub(super) fn put_char(text: &mut String, x: usize, ch: char, insert: bool) {
        let len = text.chars().count();
        for _ in len..x {
            text.push(' ');
        }
        let byte = Self::byte_index(text, x);
        if !insert && byte < text.len() {
            text.remove(byte);
        }
        text.insert(byte, ch);
    }

    /// Split at the last word boundary inside the margin, or hard-wrap a long
    /// word. Do not mistake leading indentation for a word boundary.
    pub(super) fn wrap_once(text: &str, width: usize) -> (String, String) {
        if width == 0 || text.chars().count() <= width {
            return (text.to_string(), String::new());
        }
        let prefix: Vec<char> = text.chars().take(width + 1).collect();
        let indent = prefix.iter().take_while(|c| c.is_whitespace()).count();
        let split = (indent + 1..prefix.len()).rev().find(|&x| prefix[x].is_whitespace());
        match split {
            Some(x) => {
                let byte = Self::byte_index(text, x);
                (text[..byte].trim_end().to_string(), text[byte..].trim_start().to_string())
            }
            None => {
                let byte = Self::byte_index(text, width);
                (text[..byte].to_string(), text[byte..].to_string())
            }
        }
    }

    pub(super) fn type_char(&mut self, ch: char) -> EditUpdate {
        if ch.is_control() || self.max_lines == 0 || self.max_line_length == 0 {
            return EditUpdate::None;
        }
        self.cursor.x = self.cursor.x.clamp(0, self.max_line_length as i32);
        let x = self.cursor.x as usize;
        let insert = self.insert_mode;
        let mut line = self.cur_line().clone();
        Self::put_char(&mut line, x, ch, insert);
        if line.chars().count() > self.max_line_length && self.msg.len() >= self.max_lines {
            return EditUpdate::None;
        }
        *self.cur_line() = line;
        self.cursor.x += 1;
        if self.cur_line().chars().count() > self.max_line_length {
            return self.break_line(self.cursor.y);
        }
        EditUpdate::UpdateCurrentLineFrom(x)
    }

    pub(super) fn left_margin(&self) -> i32 {
        if self.max_line_length == 72 { 5 } else { 0 }
    }

    pub(super) fn screen_line(&self, y: usize) -> String {
        if y >= self.max_lines {
            return String::new();
        }
        let prefix = if self.max_line_length == 72 {
            format!("{:>3}: ", y + 1)
        } else {
            String::new()
        };
        let text: String = self
            .msg
            .get(y)
            .into_iter()
            .flat_map(|line| line.chars())
            .filter(|ch| !ch.is_control())
            .take(self.max_line_length)
            .collect();
        // Clipping is display-only; never discard caller-supplied draft text.
        (prefix + text.as_str()).chars().take(79).collect()
    }

    pub(super) fn toggle_width(&mut self) -> bool {
        if self.max_line_length == 72 {
            self.max_line_length = 79;
        } else if self.max_line_length == 79 && self.msg.iter().all(|line| line.chars().count() <= 72) {
            self.max_line_length = 72;
        } else {
            return false;
        }
        self.cursor.x = self.cursor.x.clamp(0, self.max_line_length as i32);
        true
    }

    pub(super) fn bound_viewport(&mut self, page_len: u16) {
        let visible = Self::visible_line_count(page_len);
        self.cursor.y = self.cursor.y.clamp(0, self.max_lines.saturating_sub(1) as i32);
        self.cursor.x = self.cursor.x.clamp(0, self.max_line_length as i32);
        let y = self.cursor.y as usize;
        self.top_line = self.top_line.min(self.max_lines.saturating_sub(visible));
        if y < self.top_line {
            self.top_line = y;
        } else if y >= self.top_line + visible {
            self.top_line = y + 1 - visible;
        }
    }

    pub(super) fn move_page(&mut self, down: bool, page_len: u16) {
        self.bound_viewport(page_len);
        let visible = Self::visible_line_count(page_len);
        let step = visible.saturating_sub(2).max(1);
        if down {
            let next = self.top_line.saturating_add(step).min(self.max_lines.saturating_sub(visible));
            if next != self.top_line {
                self.top_line = next;
                self.cursor.y = (next + 2.min(visible - 1)) as i32;
            }
        } else if self.top_line > 0 {
            self.top_line = self.top_line.saturating_sub(step);
            self.cursor.y = (self.top_line + visible.saturating_sub(3)) as i32;
        }
        self.bound_viewport(page_len);
    }

    pub(super) fn tab(&mut self) -> EditUpdate {
        if self.max_lines == 0 || self.max_line_length == 0 {
            return EditUpdate::None;
        }
        let x = self.cursor.x.max(0) as usize;
        let last_tab = (self.max_line_length.saturating_sub(1) / 8) * 8;
        if x >= last_tab.saturating_sub(1) {
            return self.press_enter();
        }
        let count = (8 - x % 8).min(self.max_line_length.saturating_sub(x));
        // Tab in overwrite mode moves the cursor without erasing existing text.
        if !self.insert_mode || (x >= self.cur_line().chars().count() && self.cursor.y as usize + 1 >= self.msg.len()) {
            self.cursor.x = (x + count) as i32;
            return EditUpdate::UpdateCurrentLineFrom(x);
        }
        let original = self.msg.clone();
        let cursor = self.cursor;
        for _ in 0..count {
            if self.type_char(' ') == EditUpdate::None {
                self.msg = original;
                self.cursor = cursor;
                return EditUpdate::None;
            }
        }
        EditUpdate::UpdateLinesFrom(cursor.y as usize)
    }

    /// Atomic insertion: a quote that cannot fit leaves the draft untouched.
    pub(super) fn insert_quote(&mut self, start: usize, end: usize) -> bool {
        let source: Vec<&String> = self.quote_text.iter().filter(|line| !line.starts_with('\x01')).collect();
        if start == 0 || end < start || end > source.len() || self.max_line_length <= 3 {
            return false;
        }
        let mut lines = Vec::new();
        for raw in &source[start - 1..end] {
            let mut text: String = raw.chars().filter(|c| !c.is_control()).collect();
            loop {
                let (line, rest) = Self::wrap_once(&text, self.max_line_length - 3);
                lines.push(format!("-> {line}"));
                if self.msg.len().saturating_add(lines.len()) > self.max_lines {
                    return false;
                }
                if rest.is_empty() {
                    break;
                }
                text = rest;
            }
        }
        let y = (self.cursor.y.max(0) as usize).min(self.msg.len());
        let y = if self.msg.get(y).is_some_and(|line| !line.is_empty()) { y + 1 } else { y };
        let count = lines.len();
        self.msg.splice(y..y, lines);
        self.cursor = (0, (y + count).min(self.max_lines.saturating_sub(1))).into();
        true
    }

    pub(super) async fn quote(&mut self, state: &mut IcyBoardState) -> Res<()> {
        let source: Vec<&String> = self.quote_text.iter().filter(|line| !line.starts_with('\x01')).collect();
        if source.is_empty() {
            return Ok(());
        }
        for (i, line) in source.iter().enumerate() {
            let safe: String = line.chars().filter(|c| !c.is_control()).collect();
            state.println(TerminalTarget::Both, &format!("{}: {safe}", i + 1)).await?;
        }
        let total = source.len();
        let answer = state
            .input_field(IceText::QuoteStart, 20, "0123456789Qq ", "", Some("1".into()), display_flags::NEWLINE)
            .await?;
        let (first, rest) = Self::parse_command(&answer);
        if first == "Q" || state.session.request_logoff {
            return Ok(());
        }
        let start = if first.is_empty() { 1 } else { first.parse().unwrap_or(0) };
        if start == 0 || start > total {
            state.display_text(IceText::NoSuchLineNumber, display_flags::NEWLINE).await?;
            return Ok(());
        }
        let answer = if let Some(end) = rest.first() {
            end.clone()
        } else {
            state
                .input_field(
                    IceText::QuoteEnd,
                    10,
                    "0123456789Qq",
                    "",
                    Some((start + 1).min(total).to_string()),
                    display_flags::NEWLINE,
                )
                .await?
        };
        if answer.eq_ignore_ascii_case("Q") || state.session.request_logoff {
            return Ok(());
        }
        let end = if answer.is_empty() {
            (start + 1).min(total)
        } else {
            answer.parse().unwrap_or(0)
        };
        if end < start || end > total {
            state.display_text(IceText::NoSuchLineNumber, display_flags::NEWLINE).await?;
        } else if !self.insert_quote(start, end) {
            state.display_text(IceText::TextEntryFull, display_flags::NEWLINE).await?;
        }
        Ok(())
    }
}
