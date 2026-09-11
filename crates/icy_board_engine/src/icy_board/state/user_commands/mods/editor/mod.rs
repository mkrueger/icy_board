use crate::icy_board::commands::CommandType;
use crate::{Res, icy_board::state::functions::MASK_COMMAND};
use crate::{
    icy_board::{
        icb_config::IcbColor,
        icb_text::IceText,
        state::{
            IcyBoardState, control_codes,
            functions::{MASK_NUM, display_flags},
        },
    },
    vm::TerminalTarget,
};
use chrono::Local;
use icy_engine::Position;

pub(crate) mod external;
mod operations;
mod upload;

#[cfg(test)]
mod tests;

#[derive(Default)]
pub struct EditState {
    pub from: String,
    pub to: String,
    pub subj: String,

    pub editor_details: Option<String>,

    pub msg: Vec<String>,

    /// Original body supplied by the caller after read-access checks, for Q/Ctrl-O.
    pub quote_text: Vec<String>,

    pub cursor: Position,

    pub insert_mode: bool,
    pub use_fse: bool,

    pub top_line: usize,

    pub max_line_length: usize,
    pub max_lines: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditResult {
    Abort,
    SendMessage,
    CarbonCopy,
    AttachFile,
    SendNext,
    SendKill,
}

impl EditState {
    const HEADER_SIZE: i32 = 3;
    const MAX_VISIBLE_LINES: usize = 20;

    fn visible_line_count(page_len: u16) -> usize {
        if page_len == 0 || page_len >= 22 {
            Self::MAX_VISIBLE_LINES
        } else {
            (page_len as usize).saturating_sub(2).max(1)
        }
    }

    fn footer_row(page_len: u16) -> i32 {
        Self::HEADER_SIZE + Self::visible_line_count(page_len) as i32
    }

    pub(crate) async fn edit_message(&mut self, state: &mut IcyBoardState) -> Res<EditResult> {
        if state.session.request_logoff {
            return Ok(EditResult::Abort);
        }
        if self.max_lines == 0 || self.max_line_length == 0 {
            return Ok(EditResult::Abort);
        }
        self.max_line_length = self.max_line_length.min(79);
        if !self.use_fse {
            state.new_line().await?;
            state.display_text(IceText::MessageEnterText, 0).await?;
            // display line editor header.
            state.session.op_text = self.max_lines.to_string();
            state
                .display_text(
                    if self.max_line_length == 72 { IceText::Columns72 } else { IceText::Columns79 },
                    display_flags::NEWLINE,
                )
                .await?;
            self.print_divider(state).await?;
        }
        loop {
            if self.use_fse {
                self.full_screen_edit(state).await?;
            } else {
                self.insline(state).await?;
            }

            loop {
                // EOF/time-limit expiry must abandon the draft, not repeatedly
                // ask a disconnected caller for an editor command.
                if state.session.request_logoff {
                    return Ok(EditResult::Abort);
                }
                let cmd = if let Some(command) = state.session.tokens.pop_front() {
                    command
                } else if state.session.expert_mode() {
                    state
                        .input_field(
                            IceText::MessageCommandExpertmode,
                            30,
                            MASK_COMMAND,
                            CommandType::EnterMessage.get_help(),
                            None,
                            display_flags::NEWLINE | display_flags::UPCASE,
                        )
                        .await?
                } else {
                    state
                        .display_text(IceText::MessageCommandNovice1, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                    state.display_text(IceText::MessageCommandNovice2, display_flags::NEWLINE).await?;

                    state
                        .input_field(
                            IceText::TextEntryCommand,
                            30,
                            MASK_COMMAND,
                            CommandType::EnterMessage.get_help(),
                            None,
                            display_flags::NEWLINE | display_flags::LFAFTER | display_flags::UPCASE,
                        )
                        .await?
                };

                if state.session.request_logoff {
                    return Ok(EditResult::Abort);
                }
                // Only split command arguments on whitespace: ';' belongs to
                // the old-text/new-text substitution dialog, not this lexer.
                let (cmd, arguments) = Self::parse_command(&cmd);
                for argument in arguments.into_iter().rev() {
                    state.session.tokens.push_front(argument);
                }
                match cmd.as_str() {
                    "A" => {
                        // Abort
                        let abort = state
                            .input_field(
                                IceText::MessageAbort,
                                1,
                                "",
                                "",
                                Some(state.session.no_char.to_string()),
                                display_flags::YESNO | display_flags::NEWLINE | display_flags::UPCASE | display_flags::FIELDLEN,
                            )
                            .await?;
                        if abort == state.session.yes_char.to_uppercase().to_string() {
                            state
                                .display_text(IceText::MessageAborted, display_flags::NEWLINE | display_flags::LFBEFORE)
                                .await?;
                            state.session.disp_options.force_count_lines();
                            return Ok(EditResult::Abort);
                        }
                    }
                    "C" => {
                        // line edit
                        self.use_fse = false;
                        self.cursor = (0, self.msg.len()).into();
                        self.print_divider(state).await?;
                        break;
                    }
                    "D" => {
                        // delete line
                        let line: usize = self.read_line_number(IceText::DeleteLineNumber, state).await?;
                        if line > 0 && (line as usize - 1) < self.msg.len() {
                            self.print_divider(state).await?;
                            state.set_color(TerminalTarget::Both, IcbColor::dos_light_cyan()).await?;
                            state.print(TerminalTarget::Both, &format!("{line}: ")).await?;
                            state.reset_color(TerminalTarget::Both).await?;
                            state.println(TerminalTarget::Both, &self.msg[line - 1]).await?;

                            let delete_line = state
                                .input_field(
                                    IceText::WantToDeleteLine,
                                    1,
                                    "",
                                    "",
                                    Some(state.session.no_char.to_string()),
                                    display_flags::YESNO | display_flags::NEWLINE | display_flags::UPCASE | display_flags::FIELDLEN | display_flags::LFBEFORE,
                                )
                                .await?;
                            if delete_line == state.session.yes_char.to_uppercase().to_string() {
                                self.msg.remove(line - 1);
                                self.cursor.y = self.cursor.y.min(self.msg.len() as i32);
                            }
                        }
                    }
                    "E" => {
                        // edit line
                        let line = self.read_line_number(IceText::EditLineNumber, state).await?;
                        if line > 0 {
                            self.cursor.y = (line as i32) - 1;
                            self.print_divider(state).await?;
                            self.edline(state).await?;
                        }
                    }
                    "F" | "V" => {
                        // Switch to full screen editor
                        self.use_fse = true;
                        self.insert_mode = true;
                        break;
                    }
                    "I" => {
                        // insert line
                        let line = self.read_line_number(IceText::InsertBeforeNumber, state).await?;
                        if line > 0 {
                            self.cursor.y = (line as i32) - 1;
                            self.use_fse = false;
                            self.print_divider(state).await?;
                            break;
                        }
                    }
                    "L" => {
                        let start = if state.session.tokens.front().is_some_and(|token| token.parse::<usize>().is_ok()) {
                            self.read_line_number(IceText::NoSuchLineNumber, state).await?.saturating_sub(1)
                        } else {
                            0
                        };
                        self.msg_header(state).await?;
                        for line in self.msg.iter().skip(start) {
                            state.println(TerminalTarget::Both, line).await?;
                        }
                    }
                    "H" => state.show_help(CommandType::EnterMessage.get_help()).await?,
                    "Q" => {
                        self.quote(state).await?;
                    }
                    "U" => self.upload_text(state).await?,
                    "S" | "SC" | "SA" | "SN" | "SK" => {
                        state.session.disp_options.force_count_lines();
                        return Ok(self.save_result(&cmd));
                    }
                    _ => {}
                }
            }
        }
    }

    async fn msg_header(&mut self, state: &mut IcyBoardState) -> Res<()> {
        let to_txt = state.get_display_text(IceText::To)?;
        let subj_txt = state.get_display_text(IceText::Subject)?;

        let to_part = format!("{}{}", to_txt, self.to);
        let subj_part = format!("{}{} {}", subj_txt, self.subj, Local::now().format("%H:%M"));
        state.set_color(TerminalTarget::Both, IcbColor::dos_yellow()).await?;
        let header: String = format!("{to_part:<38}{subj_part:<38}").chars().filter(|c| !c.is_control()).take(79).collect();
        state.println(TerminalTarget::Both, &header).await?;
        self.print_divider(state).await?;

        Ok(())
    }

    async fn print_divider(&mut self, state: &mut IcyBoardState) -> Res<()> {
        state.set_color(TerminalTarget::Both, IcbColor::dos_light_cyan()).await?;
        let divider = if self.max_line_length == 72 {
            format!("    ({})", "-".repeat(72))
        } else {
            "-".repeat(79)
        };
        state.println(TerminalTarget::Both, &divider).await?;
        state.reset_color(TerminalTarget::Both).await?;
        Ok(())
    }

    async fn read_line_number(&mut self, msg: IceText, state: &mut IcyBoardState) -> Res<usize> {
        let line_number = if let Some(token) = state.session.tokens.pop_front() {
            token
        } else {
            state.input_field(msg, 340, &MASK_NUM, "", None, display_flags::NEWLINE).await?
        };
        if line_number.is_empty() {
            return Ok(0);
        }
        let line = line_number.parse::<usize>().unwrap_or_default();
        let limit = if msg == IceText::InsertBeforeNumber {
            self.msg.len() + 1
        } else {
            self.msg.len()
        };
        if line < 1 || line > limit {
            state
                .display_text(IceText::NoSuchLineNumber, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            Ok(0)
        } else {
            Ok(line)
        }
    }

    async fn full_screen_edit(&mut self, state: &mut IcyBoardState) -> Res<()> {
        self.bound_viewport(state.session.page_len);
        self.redraw_fse(state).await?;

        loop {
            if state.session.request_logoff {
                return Ok(());
            }
            let Some(ch) = state.get_char_edit().await? else {
                continue;
            };
            match ch.ch {
                control_codes::ESC | control_codes::CTRL_U => {
                    state.clear_screen(TerminalTarget::Both).await?;
                    return Ok(());
                }
                control_codes::PG_UP => {
                    self.move_page(false, state.session.page_len);
                    self.redraw_fse(state).await?;
                    self.print_line_number(state).await?;
                }
                control_codes::PG_DN => {
                    self.move_page(true, state.session.page_len);
                    self.redraw_fse(state).await?;
                    self.print_line_number(state).await?;
                }
                control_codes::CTRL_B => {
                    let update = self.reformat();
                    self.update_screen(state, update).await?;
                }
                control_codes::CTRL_I => {
                    let update = self.tab();
                    self.update_screen(state, update).await?;
                }
                control_codes::CTRL_J => {
                    self.merge_line(self.cursor.y);
                    let update = EditUpdate::UpdateLinesFrom(self.cursor.y as usize);
                    self.update_screen(state, update).await?;
                }
                control_codes::CTRL_K => {
                    let update = self.delete_to_eol();
                    self.update_screen(state, update).await?;
                }

                control_codes::CTRL_O => {
                    state.clear_screen(TerminalTarget::Both).await?;
                    self.quote(state).await?;
                    self.bound_viewport(state.session.page_len);
                    self.redraw_fse(state).await?;
                }

                control_codes::CTRL_T => {
                    let update = self.delete_word();
                    self.update_screen(state, update).await?;
                }

                control_codes::CTRL_L => {
                    self.redraw_fse(state).await?;
                }
                '\x1f' => {
                    self.toggle_width();
                    self.bound_viewport(state.session.page_len);
                    self.redraw_fse(state).await?;
                }

                control_codes::CTRL_N => {
                    if self.cursor.y < self.max_lines.saturating_sub(1) as i32 {
                        let update = self.force_new_line();
                        self.update_screen(state, update).await?;
                        self.print_line_number(state).await?;
                    }
                }

                control_codes::CTRL_Y => {
                    let y = self.cursor.y as usize;
                    if y < self.msg.len() {
                        self.msg.remove(y);
                        self.cursor.x = 0;
                        self.redraw_fse_from(state, y).await?;
                    }
                }
                control_codes::CTRL_Z => {
                    state.session.disp_options.force_count_lines();
                    state.clear_screen(TerminalTarget::Both).await?;
                    state.show_help("hlpfscrn").await?;
                    state.session.disp_options.force_count_lines();
                    state.press_enter().await?;
                    self.redraw_fse(state).await?;
                }

                control_codes::CTRL_LEFT => {
                    let x = self.cursor.x.max(0) as usize;
                    self.cursor.x = Self::word_left(self.cur_line(), x) as i32;
                    self.set_cursor_position(state).await?;
                }

                control_codes::CTRL_RIGHT => {
                    let x = self.cursor.x.max(0) as usize;
                    self.cursor.x = Self::word_right(self.cur_line(), x) as i32;
                    self.set_cursor_position(state).await?;
                }

                control_codes::LEFT => {
                    if self.cursor.x > 0 {
                        self.cursor.x -= 1;
                        state.backward(1).await?;
                    }
                }

                control_codes::RIGHT => {
                    if self.cursor.x < self.max_line_length.saturating_sub(1) as i32 {
                        self.cursor.x += 1;
                        state.forward(1).await?;
                    }
                }

                control_codes::UP => {
                    self.cursor.y = self.cursor.y.saturating_sub(1).max(0);
                    self.bound_viewport(state.session.page_len);
                    self.redraw_fse(state).await?;
                }

                control_codes::DOWN => {
                    self.cursor.y = self.cursor.y.saturating_add(1);
                    self.bound_viewport(state.session.page_len);
                    self.redraw_fse(state).await?;
                }

                control_codes::HOME => {
                    if self.cursor.x > 0 {
                        state.backward(self.cursor.x).await?;
                        self.cursor.x = 0;
                    }
                }

                control_codes::INS => {
                    self.insert_mode = !self.insert_mode;
                    self.display_insert_mode(state).await?;
                }

                control_codes::END => {
                    self.cursor.x = self.cur_line().chars().count() as i32;
                    self.set_cursor_position(state).await?;
                }

                control_codes::BS => {
                    let update = self.backspace();
                    self.update_screen(state, update).await?;
                }

                control_codes::DEL => {
                    let update = self.delete_char();
                    self.update_screen(state, update).await?;
                }

                '\r' => {
                    if self.cursor.y < self.max_lines.saturating_sub(1) as i32 {
                        let update = self.press_enter();
                        self.update_screen(state, update).await?;
                        self.print_line_number(state).await?;
                    }
                }

                ch => {
                    if !ch.is_control() {
                        let update = self.type_char(ch);
                        self.update_screen(state, update).await?;
                    }
                }
            }
        }
    }

    async fn redraw_fse_from(&mut self, state: &mut IcyBoardState, y: usize) -> Res<()> {
        state.reset_color(TerminalTarget::Both).await?;
        let visible_lines = Self::visible_line_count(state.session.page_len);
        for screen_line in y.saturating_sub(self.top_line)..visible_lines {
            let cur_line = screen_line + self.top_line;
            state.gotoxy(TerminalTarget::Both, 1, Self::HEADER_SIZE + screen_line as i32).await?;
            state.print(TerminalTarget::Both, &self.screen_line(cur_line)).await?;
            state.clear_eol(TerminalTarget::Both).await?;
        }
        self.display_fse_footer(state).await?;
        Ok(())
    }

    async fn redraw_fse(&mut self, state: &mut IcyBoardState) -> Res<()> {
        state.clear_screen(TerminalTarget::Both).await?;
        state.session.disp_options.force_non_stop();
        self.msg_header(state).await?;
        state.reset_color(TerminalTarget::Both).await?;
        for i in 0..Self::visible_line_count(state.session.page_len) {
            let cur_line = i + self.top_line;
            state.gotoxy(TerminalTarget::Both, 1, Self::HEADER_SIZE + i as i32).await?;
            state.print(TerminalTarget::Both, &self.screen_line(cur_line)).await?;
            state.clear_eol(TerminalTarget::Both).await?;
        }
        self.display_fse_footer(state).await?;
        Ok(())
    }

    async fn display_fse_footer(&self, state: &mut IcyBoardState) -> Res<()> {
        state.gotoxy(TerminalTarget::Both, 1, Self::footer_row(state.session.page_len)).await?;
        state.clear_eol(TerminalTarget::Both).await?;
        state.display_text(IceText::EscToExit, 0).await?;
        self.print_line_number(state).await?;
        self.display_insert_mode(state).await?;
        Ok(())
    }

    async fn display_insert_mode(&self, state: &mut IcyBoardState) -> Res<()> {
        state.gotoxy(TerminalTarget::Both, 48, Self::footer_row(state.session.page_len)).await?;
        if self.insert_mode {
            state.display_text(IceText::INSForOverwrite, 0).await?;
        } else {
            state.display_text(IceText::INSForInsert, 0).await?;
        }
        state.reset_color(TerminalTarget::Both).await?;
        state.clear_eol(TerminalTarget::Both).await?;
        state
            .gotoxy(
                TerminalTarget::Both,
                self.cursor.x + 1 + self.left_margin(),
                Self::HEADER_SIZE + self.cursor.y - self.top_line as i32,
            )
            .await?;
        Ok(())
    }

    async fn insline(&mut self, state: &mut IcyBoardState) -> Res<()> {
        self.cursor.y = self.cursor.y.clamp(0, self.msg.len() as i32);
        let mut edit_line = String::new();
        loop {
            if self.msg.len() >= self.max_lines {
                state.display_text(IceText::TextEntryFull, display_flags::NEWLINE).await?;
                return Ok(());
            }
            let (new_line, next_line) = self.get_line(state, edit_line).await?;
            if state.session.request_logoff {
                return Ok(());
            }
            if new_line.is_empty() && next_line.is_empty() {
                return Ok(());
            }
            self.msg.insert(self.cursor.y as usize, new_line);
            self.cursor.y += 1;
            edit_line = next_line;
        }
    }

    async fn edline(&mut self, state: &mut IcyBoardState) -> Res<()> {
        let y = self.cursor.y.max(0) as usize;
        if y >= self.msg.len() {
            return Ok(());
        }
        loop {
            state.println(TerminalTarget::Both, &format!("{}: {}", y + 1, self.msg[y])).await?;
            state.display_text(IceText::OldTextNewText, display_flags::NEWLINE).await?;
            let replacement = if let Some(token) = state.session.tokens.pop_front() {
                token
            } else {
                self.read_editor_line(state, String::new(), 127, false).await?.0
            };
            if state.session.request_logoff || replacement.is_empty() {
                return Ok(());
            }
            if !self.substitute(y, &replacement) {
                state.display_text(IceText::WasNotFoundInLine, display_flags::NEWLINE).await?;
            }
        }
    }

    async fn get_line(&mut self, state: &mut IcyBoardState, edit_line: String) -> Res<(String, String)> {
        self.read_editor_line(state, edit_line, self.max_line_length, true).await
    }

    async fn read_editor_line(&mut self, state: &mut IcyBoardState, mut edit_line: String, width: usize, wrap: bool) -> Res<(String, String)> {
        let mut caret_x = edit_line.chars().count();
        state.print(TerminalTarget::Both, &edit_line).await?;

        loop {
            if state.session.request_logoff {
                return Ok((String::new(), String::new()));
            }
            let Some(ch) = state.get_char_edit().await? else {
                continue;
            };
            match ch.ch {
                control_codes::CTRL_LEFT => {
                    let x = Self::word_left(&edit_line, caret_x);
                    state.backward((caret_x - x) as i32).await?;
                    caret_x = x;
                }
                control_codes::CTRL_RIGHT => {
                    let x = Self::word_right(&edit_line, caret_x);
                    state.forward((x - caret_x) as i32).await?;
                    caret_x = x;
                }

                control_codes::LEFT => {
                    if caret_x > 0 {
                        caret_x -= 1;
                        state.backward(1).await?;
                    }
                }
                control_codes::RIGHT => {
                    if caret_x < edit_line.chars().count() {
                        caret_x += 1;
                        state.forward(1).await?;
                    }
                }
                control_codes::HOME => {
                    if caret_x > 0 {
                        state.backward(caret_x as i32).await?;
                        caret_x = 0;
                    }
                }
                control_codes::END => {
                    if caret_x < edit_line.chars().count() {
                        state.forward(edit_line.chars().count() as i32 - caret_x as i32).await?;
                        caret_x = edit_line.chars().count();
                    }
                }
                control_codes::BS => {
                    if caret_x > 0 {
                        caret_x -= 1;
                        edit_line.remove(Self::byte_index(&edit_line, caret_x));
                        state.backward(1).await?;
                        let tail = &edit_line[Self::byte_index(&edit_line, caret_x)..];
                        state.print(TerminalTarget::Both, &format!("{tail} ")).await?;
                        state.backward(tail.chars().count() as i32 + 1).await?;
                    }
                }
                control_codes::DEL => {
                    if caret_x < edit_line.chars().count() {
                        edit_line.remove(Self::byte_index(&edit_line, caret_x));
                        let tail = &edit_line[Self::byte_index(&edit_line, caret_x)..];
                        state.print(TerminalTarget::Both, &format!("{tail} ")).await?;
                        state.backward(tail.chars().count() as i32 + 1).await?;
                    }
                }
                control_codes::CTRL_I => {
                    let count = (8 - caret_x % 8).min(width.saturating_sub(caret_x));
                    for _ in 0..count {
                        Self::put_char(&mut edit_line, caret_x, ' ', false);
                        caret_x += 1;
                    }
                    state.print(TerminalTarget::Both, &" ".repeat(count)).await?;
                }
                '\r' | '\n' => {
                    if wrap {
                        edit_line = edit_line.trim_end().to_string();
                    }
                    state.new_line().await?;
                    return Ok((edit_line, String::new()));
                }
                ch => {
                    if !ch.is_control() {
                        let mut candidate = edit_line.clone();
                        Self::put_char(&mut candidate, caret_x, ch, false);
                        if candidate.chars().count() > width {
                            // The final available line cannot spill into another.
                            if !wrap || self.msg.len() + 1 >= self.max_lines {
                                continue;
                            }
                            let (line, rest) = Self::wrap_once(&candidate, width);
                            // Remove the overflow word from the displayed line;
                            // get_line will echo it at the next prompt instead.
                            let shown = line.chars().count();
                            if caret_x > shown {
                                state.backward((caret_x - shown) as i32).await?;
                                state.print(TerminalTarget::Both, &" ".repeat(caret_x - shown)).await?;
                            }
                            state.new_line().await?;
                            return Ok((line, rest));
                        }
                        edit_line = candidate;
                        caret_x += 1;
                        state.print(TerminalTarget::Both, &ch.to_string()).await?;
                    }
                }
            }
        }
    }

    fn cur_line(&mut self) -> &mut String {
        self.cursor.y = self.cursor.y.clamp(0, self.max_lines.saturating_sub(1) as i32);
        while self.cursor.y as usize >= self.msg.len() {
            self.msg.push(String::new());
        }
        &mut self.msg[self.cursor.y as usize]
    }

    async fn print_line_number(&self, state: &mut IcyBoardState) -> Res<()> {
        state.reset_color(TerminalTarget::Both).await?;
        state.gotoxy(TerminalTarget::Both, 79 - 2, 1).await?;
        state.print(TerminalTarget::Both, &format!("{:>3}", self.cursor.y as usize + 1)).await?;
        state.clear_eol(TerminalTarget::Both).await?;
        state
            .gotoxy(
                TerminalTarget::Both,
                self.cursor.x + 1 + self.left_margin(),
                Self::HEADER_SIZE + self.cursor.y - self.top_line as i32,
            )
            .await?;

        Ok(())
    }

    fn merge_line(&mut self, y: i32) {
        if y < 0 {
            return;
        }
        let y = y as usize;
        if y + 1 < self.msg.len() {
            let mut line = self.msg[y].trim_end().to_string();
            let x = self.cursor.x.max(0) as usize;
            while line.chars().count() < x.min(self.max_line_length) {
                line.push(' ');
            }
            let room = self.max_line_length.saturating_sub(line.chars().count());
            let next = &self.msg[y + 1];
            if next.chars().count() <= room {
                line.push_str(next);
                self.msg[y] = line;
                self.msg.remove(y + 1);
            } else if let Some(split) = next.chars().take(room + 1).collect::<Vec<_>>().iter().rposition(|c| c.is_whitespace()) {
                let byte = Self::byte_index(next, split);
                line.push_str(&next[..byte]);
                let remainder = next[byte..].trim_start().to_string();
                self.msg[y] = line;
                self.msg[y + 1] = remainder;
            }
        }
    }

    async fn set_cursor_position(&self, state: &mut IcyBoardState) -> Res<()> {
        state
            .gotoxy(
                TerminalTarget::Both,
                self.cursor.x + 1 + self.left_margin(),
                Self::HEADER_SIZE + self.cursor.y - self.top_line as i32,
            )
            .await?;
        Ok(())
    }

    pub fn backspace(&mut self) -> EditUpdate {
        if self.max_lines == 0 {
            return EditUpdate::None;
        }
        if self.cursor.x > 0 {
            let o = self.cursor.x as usize;
            if o <= self.cur_line().chars().count() {
                let byte = Self::byte_index(self.cur_line(), o - 1);
                self.cur_line().remove(byte);
            }
            self.cursor.x -= 1;
            return EditUpdate::UpdateCurrentLineFrom(o - 1);
        } else if self.cursor.y > 0 {
            self.cursor.y -= 1;
            self.cursor.x = self.cur_line().chars().count() as i32;
            self.merge_line(self.cursor.y);
            return EditUpdate::UpdateLinesFrom(self.cursor.y as usize);
        }
        EditUpdate::None
    }

    pub fn delete_char(&mut self) -> EditUpdate {
        if self.max_lines == 0 {
            return EditUpdate::None;
        }
        let x = self.cursor.x.max(0) as usize;
        if x < self.cur_line().chars().count() {
            let byte = Self::byte_index(self.cur_line(), x);
            self.cur_line().remove(byte);
            EditUpdate::UpdateCurrentLineFrom(x)
        } else {
            self.merge_line(self.cursor.y);
            EditUpdate::UpdateLinesFrom(self.cursor.y as usize)
        }
    }

    async fn update_screen(&mut self, state: &mut IcyBoardState, update: EditUpdate) -> Res<()> {
        let previous_top = self.top_line;
        self.bound_viewport(state.session.page_len);
        if previous_top != self.top_line {
            return self.redraw_fse(state).await;
        }
        match update {
            EditUpdate::None => {}
            EditUpdate::UpdateCurrentLineFrom(_) => {
                self.redraw_fse_from(state, self.cursor.y.max(0) as usize).await?;
                self.set_cursor_position(state).await?;
            }
            EditUpdate::UpdateLinesFrom(pos) => {
                self.redraw_fse_from(state, pos).await?;
                self.set_cursor_position(state).await?;
            }
        }
        Ok(())
    }

    fn press_enter(&mut self) -> EditUpdate {
        if self.cursor.y >= self.max_lines.saturating_sub(1) as i32 {
            return EditUpdate::None;
        }
        if !self.insert_mode {
            if let Some(line) = self.msg.get_mut(self.cursor.y as usize) {
                line.truncate(line.trim_end().len());
            }
            self.cursor.y += 1;
            self.cursor.x = 0;
            return EditUpdate::UpdateLinesFrom(self.cursor.y as usize);
        }
        self.force_new_line()
    }

    fn force_new_line(&mut self) -> EditUpdate {
        if self.msg.len() >= self.max_lines || self.cursor.y >= self.max_lines.saturating_sub(1) as i32 {
            return EditUpdate::None;
        }
        self.cur_line();
        let mut y = self.cursor.y as usize;
        if y < self.msg.len() {
            let x = self.cursor.x as usize;
            if x < self.cur_line().chars().count() {
                let byte = Self::byte_index(self.cur_line(), x);
                let new_line = self.cur_line().drain(byte..).collect();
                self.msg.insert(self.cursor.y as usize + 1, new_line);
            } else {
                y += 1; // at eol, don't need to update current line.
                self.msg.insert(self.cursor.y as usize + 1, String::new());
            }
        }
        self.cursor.y += 1;
        self.cursor.x = 0;
        EditUpdate::UpdateLinesFrom(y)
    }

    pub fn left_justify(&mut self) -> EditUpdate {
        if self.max_lines == 0 {
            return EditUpdate::None;
        }
        if !self.cur_line().is_empty() && self.cur_line().chars().next().unwrap().is_whitespace() {
            *self.cur_line() = self.cur_line().trim_start().to_string();
            self.cursor.x = self.cur_line().chars().count() as i32;
            return EditUpdate::UpdateCurrentLineFrom(0);
        }
        EditUpdate::None
    }

    pub fn center(&mut self) -> EditUpdate {
        if self.max_lines == 0 {
            return EditUpdate::None;
        }
        if !self.cur_line().is_empty() {
            let len = self.cur_line().chars().count();
            let mut line = self.cur_line().clone();
            let spaces = self.max_line_length.saturating_sub(len);
            let left = spaces / 2;
            line.insert_str(0, &str::repeat(" ", left));
            self.cur_line().clone_from(&line);
            self.cursor.x = self.cur_line().chars().count() as i32;
            return EditUpdate::UpdateCurrentLineFrom(0);
        }
        EditUpdate::None
    }

    pub fn delete_word(&mut self) -> EditUpdate {
        if self.max_lines == 0 {
            return EditUpdate::None;
        }
        let x = self.cursor.x.max(0) as usize;
        let mut line = self.cur_line().clone();
        if x < line.chars().count() {
            let mut pos = x;
            while pos < line.chars().count() && line.chars().nth(pos).is_some_and(char::is_whitespace) {
                pos += 1;
            }
            while pos < line.chars().count() && line.chars().nth(pos).is_some_and(|c| !c.is_whitespace()) {
                pos += 1;
            }
            line.drain(Self::byte_index(&line, x)..Self::byte_index(&line, pos));
            *self.cur_line() = line;
            return EditUpdate::UpdateCurrentLineFrom(x);
        }
        EditUpdate::None
    }

    pub fn delete_to_eol(&mut self) -> EditUpdate {
        if self.max_lines == 0 {
            return EditUpdate::None;
        }
        let x = self.cursor.x.max(0) as usize;
        if x < self.cur_line().chars().count() {
            let byte = Self::byte_index(self.cur_line(), x);
            self.cur_line().truncate(byte);
            return EditUpdate::UpdateCurrentLineFrom(x);
        }
        EditUpdate::None
    }

    pub fn reformat(&mut self) -> EditUpdate {
        let y = self.cursor.y.max(0) as usize;
        if self.max_line_length == 0 || y >= self.msg.len() || self.msg[y].trim().is_empty() {
            return EditUpdate::None;
        }
        let mut start = y;
        while start > 0 && !self.msg[start - 1].trim().is_empty() {
            start -= 1;
        }
        let mut end = y + 1;
        while end < self.msg.len() && !self.msg[end].trim().is_empty() {
            end += 1;
        }
        let indent: String = self.msg[start].chars().take_while(|ch| ch.is_whitespace()).collect();
        let mut text = indent;
        text.push_str(
            &self.msg[start..end]
                .iter()
                .flat_map(|line| line.split_whitespace())
                .collect::<Vec<_>>()
                .join(" "),
        );
        let mut lines = Vec::new();
        loop {
            let (line, rest) = Self::wrap_once(&text, self.max_line_length);
            lines.push(line);
            if self.msg.len() - (end - start) + lines.len() > self.max_lines {
                return EditUpdate::None;
            }
            if rest.is_empty() {
                break;
            }
            text = rest;
        }
        let last = start + lines.len() - 1;
        self.msg.splice(start..end, lines);
        self.cursor.y = y.min(last) as i32;
        self.cursor.x = self.cursor.x.min(self.msg[self.cursor.y as usize].chars().count() as i32);
        EditUpdate::UpdateLinesFrom(start)
    }

    fn break_line(&mut self, y: i32) -> EditUpdate {
        let y = y.max(0) as usize;
        if self.max_line_length == 0 || self.msg.len() >= self.max_lines || y >= self.msg.len() {
            return EditUpdate::None;
        }
        let original_len = self.msg[y].chars().count();
        let (line, next_line) = Self::wrap_once(&self.msg[y], self.max_line_length);
        if next_line.is_empty() {
            self.msg[y] = line;
            self.cursor.x = self.cursor.x.min(self.msg[y].chars().count() as i32);
            return EditUpdate::UpdateCurrentLineFrom(0);
        }
        let consumed = original_len - next_line.chars().count();
        if self.cursor.y == y as i32 && self.cursor.x as usize >= consumed {
            self.cursor.y += 1;
            self.cursor.x = self.cursor.x.saturating_sub(consumed as i32);
        }
        self.msg[y] = line;
        self.msg.insert(y + 1, next_line);
        EditUpdate::UpdateLinesFrom(y)
    }
}

#[derive(PartialEq, Debug)]
pub enum EditUpdate {
    None,
    UpdateCurrentLineFrom(usize),
    UpdateLinesFrom(usize),
}
