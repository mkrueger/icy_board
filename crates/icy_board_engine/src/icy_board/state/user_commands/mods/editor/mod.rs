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

#[cfg(test)]
mod tests;

#[derive(Default)]
pub struct EditState {
    pub from: String,
    pub to: String,
    pub subj: String,

    pub msg: Vec<String>,

    pub cursor: Position,

    pub insert_mode: bool,
    pub use_fse: bool,

    pub top_line: usize,

    pub max_line_length: usize,
}

pub enum EditResult {
    Abort,
    SendMessage,
}

impl EditState {
    const HEADER_SIZE: i32 = 3;

    pub(crate) async fn edit_message(&mut self, state: &mut IcyBoardState) -> Res<EditResult> {
        if !self.use_fse {
            state.new_line().await?;
            state.display_text(IceText::MessageEnterText, 0).await?;
            // display line editor header.
            state.session.op_text = 99.to_string(); // display max lines.
            state.display_text(IceText::Columns79, display_flags::NEWLINE).await?;
            self.print_divider(state).await?;
        }
        loop {
            if self.use_fse {
                self.full_screen_edit(state).await?;
            } else {
                self.insline(state).await?;
            }

            loop {
                let cmd = if state.session.expert_mode() {
                    state
                        .input_field(
                            IceText::MessageCommandExpertmode,
                            30,
                            "",
                            &CommandType::EnterMessage.get_help(),
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
                            &MASK_COMMAND,
                            &CommandType::EnterMessage.get_help(),
                            None,
                            display_flags::NEWLINE | display_flags::LFAFTER | display_flags::UPCASE,
                        )
                        .await?
                };

                match cmd.as_str() {
                    "A" => {
                        // Abort
                        let abort = state
                            .input_field(
                                IceText::MessageAbort,
                                1,
                                "",
                                &"",
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
                        self.print_divider(state).await?;
                        break;
                    }
                    "D" => {
                        // delete line
                        let line: usize = self.read_line_number(IceText::DeleteLineNumber, state).await?;
                        if line > 0 && (line as usize - 1) < self.msg.len() {
                            self.print_divider(state).await?;
                            state.set_color(TerminalTarget::Both, IcbColor::dos_light_cyan()).await?;
                            state.print(TerminalTarget::Both, &format!("{}: ", line)).await?;
                            state.reset_color(TerminalTarget::Both).await?;
                            state.println(TerminalTarget::Both, &self.msg[line - 1]).await?;

                            let delete_line = state
                                .input_field(
                                    IceText::WantToDeleteLine,
                                    1,
                                    "",
                                    &"",
                                    Some(state.session.no_char.to_string()),
                                    display_flags::YESNO | display_flags::NEWLINE | display_flags::UPCASE | display_flags::FIELDLEN | display_flags::LFBEFORE,
                                )
                                .await?;
                            if delete_line == state.session.yes_char.to_uppercase().to_string() {
                                self.msg.remove(line - 1);
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
                        break;
                    }
                    "I" => {
                        // insert line
                        let line = self.read_line_number(IceText::InsertBeforeNumber, state).await?;
                        if line > 0 {
                            self.cursor.y = (line as i32) - 1;
                            self.print_divider(state).await?;
                            break;
                        }
                    }
                    "L" => {
                        self.msg_header(state).await?;
                        for line in &self.msg {
                            state.println(TerminalTarget::Both, line).await?;
                        }
                    }
                    "S" => {
                        // send message
                        state.session.disp_options.force_count_lines();
                        return Ok(EditResult::SendMessage);
                    }
                    "Q" => { // quote message
                        // TODO: quote
                    }
                    "U" => { // upload message
                        // TODO: upload
                    }
                    _ => {}
                }
            }
        }
    }

    async fn msg_header(&mut self, state: &mut IcyBoardState) -> Res<()> {
        let to_txt = state.display_text.get_display_text(IceText::To)?.text.trim_start().replace("~", " ");
        let subj_txt = state.display_text.get_display_text(IceText::Subject)?.text.trim_start().replace("~", " ");

        let to_part = format!("{}{}", to_txt, self.to);
        let subj_part = format!("{}{} {}", subj_txt, self.subj, Local::now().format("%H:%M"));
        state.set_color(TerminalTarget::Both, IcbColor::dos_yellow()).await?;
        state.println(TerminalTarget::Both, &format!("{:<38}{:<38}", to_part, subj_part)).await?;
        self.print_divider(state).await?;

        Ok(())
    }

    async fn print_divider(&mut self, state: &mut IcyBoardState) -> Res<()> {
        state.set_color(TerminalTarget::Both, IcbColor::dos_light_cyan()).await?;
        state.println(TerminalTarget::Both, &str::repeat("-", 79)).await?;
        state.reset_color(TerminalTarget::Both).await?;
        Ok(())
    }

    async fn read_line_number(&mut self, msg: IceText, state: &mut IcyBoardState) -> Res<usize> {
        let line_number = if let Some(token) = state.session.tokens.pop_front() {
            token
        } else {
            state.input_field(msg, 340, &MASK_NUM, &"", None, display_flags::NEWLINE).await?
        };
        let line = line_number.parse::<usize>().unwrap_or_default();
        if line < 1 || line >= self.msg.len() + 1 {
            state
                .display_text(IceText::NoSuchLineNumber, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            Ok(0)
        } else {
            Ok(line)
        }
    }

    async fn full_screen_edit(&mut self, state: &mut IcyBoardState) -> Res<()> {
        self.redraw_fse(state).await?;

        loop {
            let Some(ch) = state.get_char_edit().await? else {
                continue;
            };
            match ch.ch {
                control_codes::ESC => {
                    state.clear_screen(TerminalTarget::Both).await?;
                    return Ok(());
                }
                control_codes::PG_UP => {
                    let pg_len = state.session.page_len as usize - Self::HEADER_SIZE as usize;
                    if self.top_line > pg_len {
                        self.top_line -= pg_len;
                    } else {
                        self.top_line = 0;
                    }
                    self.redraw_fse(state).await?;
                    self.print_line_number(state).await?;
                }
                control_codes::PG_DN => {
                    let pg_len = state.session.page_len as usize - Self::HEADER_SIZE as usize;
                    self.top_line += pg_len;
                    self.redraw_fse(state).await?;
                    self.print_line_number(state).await?;
                }
                control_codes::CTRL_B => {
                    let update = self.reformat();
                    self.update_screen(state, update).await?;
                }
                control_codes::CTRL_I => {
                    let update = self.center();
                    self.update_screen(state, update).await?;
                }
                control_codes::CTRL_J => {
                    let update = self.left_justify();
                    self.update_screen(state, update).await?;
                }
                control_codes::CTRL_K => {
                    let o = self.cursor.x as usize;
                    if self.cur_line().len() > o {
                        self.cur_line().drain(o..);
                        state.clear_eol(TerminalTarget::Both).await?;
                    }
                }

                control_codes::CTRL_O => {
                    // Quote
                }

                control_codes::CTRL_T => {
                    let update = self.delete_word();
                    self.update_screen(state, update).await?;
                }

                control_codes::CTRL_U => {
                    let update = self.delete_to_eol();
                    self.update_screen(state, update).await?;
                }
                control_codes::CTRL_L => {
                    self.redraw_fse(state).await?;
                }

                control_codes::CTRL_Y => {
                    let y = self.cursor.y as usize;
                    if y < self.msg.len() {
                        self.msg.remove(y);
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
                    for i in (self.cursor.x..0).rev() {
                        if i == 0 || self.cur_line().chars().nth(i as usize).unwrap() == ' ' {
                            state.backward((self.cursor.x - i) as i32).await?;
                            self.cursor.x = i;
                            break;
                        }
                    }
                }

                control_codes::CTRL_RIGHT => {
                    for i in self.cursor.x..=self.cur_line().len() as i32 {
                        if i == self.cur_line().len() as i32 || self.cur_line().chars().nth(i as usize).unwrap() == ' ' {
                            state.forward((i - self.cursor.x) as i32).await?;
                            self.cursor.x = i;
                            break;
                        }
                    }
                }

                control_codes::LEFT => {
                    if self.cursor.x > 0 {
                        self.cursor.x -= 1;
                        state.backward(1).await?;
                    }
                }

                control_codes::RIGHT => {
                    if self.cursor.x < self.cur_line().len() as i32 {
                        self.cursor.x += 1;
                        state.forward(1).await?;
                    }
                }

                control_codes::UP => {
                    if self.cursor.y > self.top_line as i32 {
                        self.cursor.y -= 1;
                        state.up(1).await?;
                        self.print_line_number(state).await?;
                    } else if self.top_line > 0 {
                        let y = state.session.page_len as i32 - (Self::HEADER_SIZE as i32) - 1;
                        self.top_line = ((self.top_line as i32) - y).max(0) as usize;
                        self.redraw_fse(state).await?;
                    }
                }

                control_codes::DOWN => {
                    if (self.cursor.y + self.top_line as i32) < 999 {
                        if (self.cursor.y - self.top_line as i32) < state.session.page_len as i32 - Self::HEADER_SIZE - 1 {
                            self.cursor.y += 1;
                            state.down(1).await?;
                        } else {
                            self.cursor.y += 1;
                            self.top_line += (state.session.page_len as i32 - Self::HEADER_SIZE - 1).max(1) as usize;
                            self.redraw_fse(state).await?;
                        }
                        self.print_line_number(state).await?;
                    }
                }

                control_codes::HOME => {
                    if self.cursor.x > 0 {
                        state.backward(self.cursor.x as i32).await?;
                        self.cursor.x = 0;
                    }
                }

                control_codes::INS => {
                    self.insert_mode = !self.insert_mode;
                    self.display_insert_mode(state).await?;
                }

                control_codes::END => {
                    if self.cursor.x < self.cur_line().len() as i32 {
                        state.forward(self.cur_line().len() as i32 - self.cursor.x as i32).await?;
                        self.cursor.x = self.cur_line().len() as i32;
                    }
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
                    if (self.cursor.y + self.top_line as i32) < 999 {
                        let update = self.press_enter();
                        if (self.cursor.y - self.top_line as i32) >= state.session.page_len as i32 - Self::HEADER_SIZE - 1 {
                            self.top_line += (state.session.page_len as i32 - Self::HEADER_SIZE - 1).max(1) as usize;
                            self.redraw_fse(state).await?;
                        } else {
                            self.update_screen(state, update).await?;
                        }
                        self.print_line_number(state).await?;
                    }
                }

                ch => {
                    if ch >= ' ' && ch <= '~' {
                        let o = self.cursor.x as usize;

                        while self.cursor.x >= self.cur_line().len() as i32 {
                            self.cur_line().push(' ');
                        }

                        if self.insert_mode {
                            self.cur_line().insert(o, ch);
                            let txt = format!("{}", &self.cur_line()[o..]);
                            state.print(TerminalTarget::Both, &txt).await?;
                            state.backward(txt.len() as i32).await?;
                        } else {
                            self.cur_line().replace_range(o..o + 1, ch.to_string().as_str());
                        }

                        if self.cur_line().len() > self.max_line_length {
                            let update = self.break_line(self.cursor.y);
                            self.update_screen(state, update).await?;
                        }

                        self.cursor.x += 1;
                        state.print(TerminalTarget::Both, &ch.to_string()).await?;
                    }
                }
            }
        }
    }

    async fn redraw_fse_from(&mut self, state: &mut IcyBoardState, y: usize) -> Res<()> {
        state.reset_color(TerminalTarget::Both).await?;
        state
            .gotoxy(TerminalTarget::Both, 1, y as i32 - self.top_line as i32 + Self::HEADER_SIZE)
            .await?;
        for i in y..(state.session.page_len as usize).saturating_sub(Self::HEADER_SIZE as usize) {
            let cur_line = i + self.top_line;
            if cur_line < self.msg.len() {
                state.print(TerminalTarget::Both, &self.msg[cur_line]).await?;
            }
            state.clear_eol(TerminalTarget::Both).await?;
            state.new_line().await?;
        }
        Ok(())
    }

    async fn redraw_fse(&mut self, state: &mut IcyBoardState) -> Res<()> {
        state.clear_screen(TerminalTarget::Both).await?;
        state.session.disp_options.force_non_stop();
        self.msg_header(state).await?;
        state.reset_color(TerminalTarget::Both).await?;
        state.gotoxy(TerminalTarget::Both, 1, Self::HEADER_SIZE).await?;
        for i in 0..(state.session.page_len as usize).saturating_sub(Self::HEADER_SIZE as usize) {
            let cur_line = i + self.top_line;
            if cur_line < self.msg.len() {
                state.print(TerminalTarget::Both, &self.msg[cur_line]).await?;
            }
            state.clear_eol(TerminalTarget::Both).await?;
            state.new_line().await?;
        }
        state.display_text(IceText::EscToExit, 0).await?;
        self.print_line_number(state).await?;
        self.display_insert_mode(state).await?;
        Ok(())
    }

    async fn display_insert_mode(&self, state: &mut IcyBoardState) -> Res<()> {
        state.gotoxy(TerminalTarget::Both, 48, state.session.page_len as i32).await?;
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
                self.cursor.x + 1,
                Self::HEADER_SIZE + self.cursor.y - self.top_line as i32,
            )
            .await?;
        Ok(())
    }

    async fn insline(&mut self, state: &mut IcyBoardState) -> Res<()> {
        let mut edit_line = String::new();
        loop {
            let (new_line, next_line) = self.get_line(state, edit_line).await?;
            if new_line.is_empty() && next_line.is_empty() {
                return Ok(());
            }
            self.msg.insert(self.cursor.y as usize, new_line);
            self.cursor.y += 1;
            edit_line = next_line;
        }
    }

    async fn edline(&mut self, state: &mut IcyBoardState) -> Res<()> {
        let mut edit_line = self.msg.remove(self.cursor.y as usize);
        loop {
            let (new_line, next_line) = self.get_line(state, edit_line).await?;
            if new_line.is_empty() && next_line.is_empty() {
                return Ok(());
            }
            self.msg.insert(self.cursor.y as usize, new_line);
            self.cursor.y += 1;
            edit_line = next_line;
        }
    }

    async fn get_line(&mut self, state: &mut IcyBoardState, mut edit_line: String) -> Res<(String, String)> {
        let mut caret_x = edit_line.len();
        state.print(TerminalTarget::Both, &edit_line).await?;

        loop {
            let Some(ch) = state.get_char_edit().await? else {
                continue;
            };
            match ch.ch {
                control_codes::CTRL_LEFT => {
                    for i in (caret_x..0).rev() {
                        if i == 0 || edit_line.chars().nth(i).unwrap() == ' ' {
                            state.backward((caret_x - i) as i32).await?;
                            caret_x = i;
                            break;
                        }
                    }
                }
                control_codes::CTRL_RIGHT => {
                    for i in caret_x..=edit_line.len() {
                        if i == edit_line.len() || edit_line.chars().nth(i).unwrap() == ' ' {
                            state.forward((i - caret_x) as i32).await?;
                            caret_x = i;
                            break;
                        }
                    }
                }

                control_codes::LEFT => {
                    if caret_x > 0 {
                        caret_x -= 1;
                        state.backward(1).await?;
                    }
                }
                control_codes::RIGHT => {
                    if caret_x < edit_line.len() {
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
                    if caret_x < edit_line.len() {
                        state.forward(edit_line.len() as i32 - caret_x as i32).await?;
                        caret_x = edit_line.len();
                    }
                }
                control_codes::BS => {
                    if caret_x > 0 {
                        caret_x -= 1;
                        edit_line.remove(caret_x);
                        state.print(TerminalTarget::Both, &format!("\x08 \x08{}", &edit_line[caret_x..])).await?;
                    }
                }
                control_codes::DEL => {
                    if caret_x < edit_line.len() {
                        edit_line.remove(caret_x);
                        if caret_x < edit_line.len() {
                            state.print(TerminalTarget::Both, &edit_line[caret_x..]).await?;
                            state.print(TerminalTarget::Both, " ").await?;
                            let len = edit_line.len() as i32 - caret_x as i32 + 1;
                            state.backward(len).await?;
                        }
                    }
                }
                '\r' | '\n' => {
                    edit_line = edit_line.trim().to_string();
                    state.new_line().await?;
                    return Ok((edit_line, "".to_string()));
                }
                ch => {
                    if ch >= ' ' && ch <= '~' {
                        if caret_x < edit_line.len() {
                            edit_line.replace_range(caret_x..caret_x + 1, ch.to_string().as_str());
                        } else {
                            edit_line.insert(caret_x, ch);
                        }
                        caret_x += 1;
                        state.print(TerminalTarget::Both, &ch.to_string()).await?;
                    }
                }
            }
        }
    }

    fn cur_line(&mut self) -> &mut String {
        while self.cursor.y as usize >= self.msg.len() {
            self.msg.push(String::new());
        }
        &mut self.msg[self.cursor.y as usize]
    }

    async fn print_line_number(&self, state: &mut IcyBoardState) -> Res<()> {
        state.reset_color(TerminalTarget::Both).await?;
        state.gotoxy(TerminalTarget::Both, 79 - 2, 1).await?;
        state
            .print(TerminalTarget::Both, &format!("{:>3}", self.top_line + self.cursor.y as usize + 1))
            .await?;
        state.clear_eol(TerminalTarget::Both).await?;
        state
            .gotoxy(
                TerminalTarget::Both,
                self.cursor.x + 1,
                Self::HEADER_SIZE + self.cursor.y - self.top_line as i32,
            )
            .await?;

        Ok(())
    }

    fn merge_line(&mut self, y: i32) {
        let y = y as usize;
        if y + 1 < self.msg.len() {
            let mut cur_line: Vec<char> = self.msg[y].chars().collect();
            let mut next_line: Vec<char> = self.msg[y + 1].chars().collect();

            while cur_line.len() < self.max_line_length && !next_line.is_empty() {
                let pos = next_line.iter().position(|c| *c == ' ').unwrap_or(next_line.len() - 1);
                if pos + cur_line.len() <= self.max_line_length {
                    let word = next_line.drain(0..pos + 1).collect::<String>();
                    cur_line.extend(word.chars());
                } else {
                    break;
                }
            }
            while cur_line.ends_with(&[' ']) {
                cur_line.pop();
            }
            self.msg[y] = cur_line.iter().collect();
            if next_line.is_empty() {
                self.msg.remove(y + 1);
            } else {
                self.msg[y + 1] = next_line.iter().collect();
            }
        }
    }

    async fn set_cursor_position(&self, state: &mut IcyBoardState) -> Res<()> {
        state
            .gotoxy(
                TerminalTarget::Both,
                self.cursor.x + 1,
                Self::HEADER_SIZE + self.cursor.y - self.top_line as i32,
            )
            .await?;
        Ok(())
    }

    pub fn backspace(&mut self) -> EditUpdate {
        if self.cursor.x > 0 {
            let o = self.cursor.x as usize;
            self.cur_line().remove(o - 1);
            self.cursor.x -= 1;
            return EditUpdate::UpdateCurrentLineFrom(o - 1);
        } else if self.cursor.y > 0 {
            self.cursor.y -= 1;
            self.cursor.x = self.cur_line().len() as i32;
            self.merge_line(self.cursor.y);
            return EditUpdate::UpdateLinesFrom(self.cursor.y as usize);
        }
        EditUpdate::None
    }

    pub fn delete_char(&mut self) -> EditUpdate {
        let x = self.cursor.x as usize;
        while self.cur_line().len() < x {
            self.cur_line().push(' ');
        }
        if x < self.cur_line().len() {
            let o = self.cursor.x as usize;
            self.cur_line().remove(o);
            return EditUpdate::UpdateCurrentLineFrom(o);
        } else {
            self.merge_line(self.cursor.y);
            return EditUpdate::UpdateLinesFrom(self.cursor.y as usize);
        }
    }

    async fn update_screen(&mut self, state: &mut IcyBoardState, update: EditUpdate) -> Res<()> {
        match update {
            EditUpdate::None => {}
            EditUpdate::UpdateCurrentLineFrom(pos) => {
                state.print(TerminalTarget::Both, &self.cur_line()[pos..]).await?;
                state.clear_eol(TerminalTarget::Both).await?;
                let len = self.cur_line().len() as i32 - self.cursor.x as i32 + 1;
                if len > 0 {
                    state.backward(len).await?;
                }
            }
            EditUpdate::UpdateLinesFrom(pos) => {
                self.redraw_fse_from(state, pos).await?;
                self.set_cursor_position(state).await?;
            }
        }
        Ok(())
    }

    fn press_enter(&mut self) -> EditUpdate {
        let mut y = self.cursor.y as usize;
        if y < self.msg.len() {
            let x = self.cursor.x as usize;
            if x < self.cur_line().len() {
                let new_line = self.cur_line().drain(x..).collect();
                self.msg.insert(self.cursor.y as usize + 1, new_line);
            } else {
                y += 1; // at eol, don't need to update current line.
                self.msg.insert(self.cursor.y as usize + 1, String::new());
            }
        }
        self.cursor.y += 1;
        self.cursor.x = 0;
        return EditUpdate::UpdateLinesFrom(y);
    }

    pub fn left_justify(&mut self) -> EditUpdate {
        if self.cur_line().len() > 0 && self.cur_line().chars().next().unwrap().is_whitespace() {
            *self.cur_line() = self.cur_line().trim_start().to_string();
            self.cursor.x = self.cur_line().len() as i32;
            return EditUpdate::UpdateCurrentLineFrom(0);
        }
        EditUpdate::None
    }

    pub fn center(&mut self) -> EditUpdate {
        if self.cur_line().len() > 0 {
            let len = self.cur_line().len();
            let mut line = self.cur_line().clone();
            let spaces = self.max_line_length - len;
            let left = spaces / 2;
            line.insert_str(0, &str::repeat(" ", left));
            *self.cur_line() = line.to_string();
            self.cursor.x = self.cur_line().len() as i32;
            return EditUpdate::UpdateCurrentLineFrom(0);
        }
        EditUpdate::None
    }

    pub fn delete_word(&mut self) -> EditUpdate {
        let x = self.cursor.x as usize;
        let mut line = self.cur_line().clone();
        if x < line.len() {
            let mut pos = x;
            while pos < line.len() && line.chars().nth(pos).unwrap().is_whitespace() {
                pos += 1;
            }
            while pos < line.len() && !line.chars().nth(pos).unwrap().is_whitespace() {
                pos += 1;
            }
            line.drain(x..pos);
            *self.cur_line() = line;
            return EditUpdate::UpdateCurrentLineFrom(x);
        }
        EditUpdate::None
    }

    pub fn delete_to_eol(&mut self) -> EditUpdate {
        let x = self.cursor.x as usize;
        if x < self.cur_line().len() {
            self.cur_line().drain(x..);
            return EditUpdate::UpdateCurrentLineFrom(x);
        }
        EditUpdate::None
    }

    pub fn reformat(&mut self) -> EditUpdate {
        let mut y = (self.cursor.y as usize).min(self.msg.len());
        let mut paragraph_start = 0;
        for i in (0..y).rev() {
            self.msg[i] = self.msg[i].trim_end().to_string();
            if self.msg[i].is_empty() {
                paragraph_start = i;
                break;
            }
        }
        for i in paragraph_start..y {
            if i >= self.msg.len() {
                break;
            }
            while self.msg[i].contains("  ") {
                self.msg[i] = self.msg[i].replace("  ", " ");
            }
            let line_counft = self.msg.len();
            self.msg[i].push(' ');
            self.merge_line(i as i32);
            if line_counft < self.msg.len() {
                y -= 1;
            }
        }

        EditUpdate::UpdateLinesFrom(paragraph_start)
    }

    fn break_line(&mut self, y: i32) -> EditUpdate {
        let y = y as usize;
        let mut cur_line: Vec<char> = self.msg[y].chars().collect();

        let mut pos = cur_line.len().saturating_sub(2);
        while pos > 0 && !cur_line[pos].is_whitespace() {
            pos -= 1;
        }
        if pos == 0 {
            pos = cur_line.len() / 2;
        }
        let next_line = cur_line.drain(pos..).collect::<String>().trim_start().to_string();
        self.msg.insert(y + 1, next_line);
        self.msg[y] = cur_line.iter().collect();
        self.cursor.y += 1;
        self.cursor.x = self.msg[y + 1].len() as i32;
        EditUpdate::UpdateLinesFrom(y)
    }
}

#[derive(PartialEq, Debug)]
pub enum EditUpdate {
    None,
    UpdateCurrentLineFrom(usize),
    UpdateLinesFrom(usize),
}
