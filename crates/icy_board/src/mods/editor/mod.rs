use crate::Res;
use chrono::Local;
use icy_board_engine::{
    icy_board::{
        icb_config::IcbColor,
        icb_text::IceText,
        state::{
            control_codes,
            functions::{display_flags, MASK_NUM},
            IcyBoardState, KeyChar,
        },
    },
    vm::TerminalTarget,
};
use icy_engine::Position;

use crate::menu_runner::MASK_COMMAND;

pub struct EditState {
    pub from: String,
    pub to: String,
    pub subj: String,

    pub msg: Vec<String>,

    pub cursor: Position,

    pub insert_mode: bool,
    pub use_fse: bool,

    pub top_line: usize,
}

pub enum EditResult {
    Abort,
    SendMessage,
}

impl EditState {
    const HEADER_SIZE: i32 = 3;

    pub(crate) fn edit_message(&mut self, state: &mut IcyBoardState) -> Res<EditResult> {
        if !self.use_fse {
            state.new_line()?;
            state.display_text(IceText::MessageEnterText, 0)?;
            // display line editor header.
            state.session.op_text = 99.to_string(); // display max lines.
            state.display_text(IceText::Columns79, display_flags::NEWLINE)?;
            self.print_divider(state)?;
        }
        loop {
            state.session.disp_options.non_stop = true;
            if self.use_fse {
                self.full_screen_edit(state)?;
            } else {
                self.insline(state)?;
            }

            loop {
                let cmd = if state.session.expert_mode {
                    state.input_field(
                        IceText::MessageCommandExpertmode,
                        30,
                        "",
                        &"hlpe",
                        None,
                        display_flags::NEWLINE | display_flags::UPCASE,
                    )?
                } else {
                    state.display_text(IceText::MessageCommandNovice1, display_flags::NEWLINE | display_flags::LFBEFORE)?;
                    state.display_text(IceText::MessageCommandNovice2, display_flags::NEWLINE)?;

                    state.input_field(
                        IceText::TextEntryCommand,
                        30,
                        &MASK_COMMAND,
                        &"hlpe",
                        None,
                        display_flags::NEWLINE | display_flags::LFAFTER | display_flags::UPCASE,
                    )?
                };

                match cmd.as_str() {
                    "A" => {
                        // Abort
                        let abort = state.input_field(
                            IceText::MessageAbort,
                            1,
                            "",
                            &"",
                            Some(state.session.no_char.to_string()),
                            display_flags::YESNO | display_flags::NEWLINE | display_flags::UPCASE | display_flags::FIELDLEN | display_flags::LFBEFORE,
                        )?;
                        if abort == state.session.yes_char.to_uppercase().to_string() {
                            state.display_text(IceText::MessageAborted, display_flags::NEWLINE | display_flags::LFBEFORE)?;
                            state.session.disp_options.non_stop = false;
                            return Ok(EditResult::Abort);
                        }
                    }
                    "C" => {
                        // line edit
                        self.use_fse = false;
                        self.print_divider(state)?;
                        break;
                    }
                    "D" => {
                        // delete line
                        let line: usize = self.read_line_number(IceText::DeleteLineNumber, state)?;
                        if line > 0 && (line as usize - 1) < self.msg.len() {
                            self.print_divider(state)?;
                            state.set_color(TerminalTarget::Both, IcbColor::Dos(11))?;
                            state.print(TerminalTarget::Both, &format!("{}: ", line))?;
                            state.reset_color()?;
                            state.println(TerminalTarget::Both, &self.msg[line - 1])?;

                            let delete_line = state.input_field(
                                IceText::WantToDeleteLine,
                                1,
                                "",
                                &"",
                                Some(state.session.no_char.to_string()),
                                display_flags::YESNO | display_flags::NEWLINE | display_flags::UPCASE | display_flags::FIELDLEN | display_flags::LFBEFORE,
                            )?;
                            if delete_line == state.session.yes_char.to_uppercase().to_string() {
                                self.msg.remove(line - 1);
                            }
                        }
                    }
                    "E" => {
                        // edit line
                        let line = self.read_line_number(IceText::EditLineNumber, state)?;
                        if line > 0 {
                            self.cursor.y = (line as i32) - 1;
                            self.print_divider(state)?;
                            self.edline(state)?;
                        }
                    }
                    "F" | "V" => {
                        // Switch to full screen editor
                        self.use_fse = true;
                        break;
                    }
                    "I" => {
                        // insert line
                        let line = self.read_line_number(IceText::InsertBeforeNumber, state)?;
                        if line > 0 {
                            self.cursor.y = (line as i32) - 1;
                            self.print_divider(state)?;
                            break;
                        }
                    }
                    "L" => {
                        self.msg_header(state)?;
                        for line in &self.msg {
                            state.println(TerminalTarget::Both, line)?;
                        }
                    }
                    "S" => {
                        // send message
                        state.session.disp_options.non_stop = false;
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

    fn msg_header(&mut self, state: &mut IcyBoardState) -> Res<()> {
        let to_txt = state.display_text.get_display_text(IceText::To)?.text.trim_start().replace("~", " ");
        let subj_txt = state.display_text.get_display_text(IceText::Subject)?.text.trim_start().replace("~", " ");

        let to_part = format!("{}{}", to_txt, self.to);
        let subj_part = format!("{}{} {}", subj_txt, self.subj, Local::now().format("%H:%M"));
        state.set_color(TerminalTarget::Both, IcbColor::Dos(14))?;
        state.println(TerminalTarget::Both, &format!("{:<38}{:<38}", to_part, subj_part))?;
        self.print_divider(state)?;

        Ok(())
    }

    fn print_divider(&mut self, state: &mut IcyBoardState) -> Res<()> {
        state.set_color(TerminalTarget::Both, IcbColor::Dos(11))?;
        state.println(TerminalTarget::Both, &str::repeat("-", 79))?;
        state.reset_color()?;
        Ok(())
    }

    fn read_line_number(&mut self, msg: IceText, state: &mut IcyBoardState) -> Res<usize> {
        let line_number = if let Some(token) = state.session.tokens.pop_front() {
            token
        } else {
            state.input_field(msg, 340, &MASK_NUM, &"", None, display_flags::NEWLINE)?
        };
        let line = line_number.parse::<usize>().unwrap_or_default();
        if line < 1 || line >= self.msg.len() + 1 {
            state.display_text(IceText::NoSuchLineNumber, display_flags::NEWLINE | display_flags::LFBEFORE)?;
            Ok(0)
        } else {
            Ok(line)
        }
    }

    fn full_screen_edit(&mut self, state: &mut IcyBoardState) -> Res<()> {
        self.redraw_fse(state)?;

        loop {
            let Some(ch) = self.get_char_edit(state)? else {
                continue;
            };
            match ch.ch {
                control_codes::ESC => {
                    state.clear_screen()?;
                    return Ok(());
                }
                control_codes::PG_UP => {
                    let pg_len = state.session.page_len as usize - Self::HEADER_SIZE as usize;
                    if self.top_line > pg_len {
                        self.top_line -= pg_len;
                    } else {
                        self.top_line = 0;
                    }
                    self.redraw_fse(state)?;
                    self.print_line_number(state)?;
                }
                control_codes::PG_DN => {
                    let pg_len = state.session.page_len as usize - Self::HEADER_SIZE as usize;
                    self.top_line += pg_len;
                    self.redraw_fse(state)?;
                    self.print_line_number(state)?;
                }
                control_codes::CTRL_B => {
                    // reformat paragraph
                }

                control_codes::CTRL_J => {
                    // join line
                }
                control_codes::CTRL_K => {
                    let o = self.cursor.x as usize;
                    if self.cur_line().len() > o {
                        self.cur_line().drain(o..);
                        state.clear_eol()?;
                    }
                }

                control_codes::CTRL_O => {
                    // Quote
                }

                control_codes::CTRL_T => {
                    // delete word
                }
                control_codes::CTRL_L => {
                    self.redraw_fse(state)?;
                }

                control_codes::CTRL_Y => {
                    if (self.cursor.y as usize) < self.msg.len() {
                        self.msg.remove(self.cursor.y as usize);
                        self.redraw_fse(state)?;
                    }
                }
                control_codes::CTRL_Z => {
                    state.session.disp_options.non_stop = false;
                    state.clear_screen()?;
                    state.show_help("hlpfscrn")?;
                    state.session.disp_options.non_stop = true;
                    state.press_enter()?;
                    self.redraw_fse(state)?;
                }
                control_codes::CTRL_LEFT => {
                    for i in (self.cursor.x..0).rev() {
                        if i == 0 || self.cur_line().chars().nth(i as usize).unwrap() == ' ' {
                            state.backward((self.cursor.x - i) as i32)?;
                            self.cursor.x = i;
                            break;
                        }
                    }
                }

                control_codes::CTRL_RIGHT => {
                    for i in self.cursor.x..=self.cur_line().len() as i32 {
                        if i == self.cur_line().len() as i32 || self.cur_line().chars().nth(i as usize).unwrap() == ' ' {
                            state.forward((i - self.cursor.x) as i32)?;
                            self.cursor.x = i;
                            break;
                        }
                    }
                }

                control_codes::LEFT => {
                    if self.cursor.x > 0 {
                        self.cursor.x -= 1;
                        state.backward(1)?;
                    }
                }
                control_codes::RIGHT => {
                    if self.cursor.x < self.cur_line().len() as i32 {
                        self.cursor.x += 1;
                        state.forward(1)?;
                    }
                }

                control_codes::UP => {
                    if self.cursor.y > self.top_line as i32 {
                        self.cursor.y -= 1;
                        state.up(1)?;
                        self.print_line_number(state)?;
                    } else if self.top_line > 0 {
                        let y = state.session.page_len as i32 - (Self::HEADER_SIZE as i32) - 1;
                        self.top_line = ((self.top_line as i32) - y).max(0) as usize;
                        self.redraw_fse(state)?;
                    }
                }
                control_codes::DOWN => {
                    if (self.cursor.y + self.top_line as i32) < 999 {
                        if (self.cursor.y - self.top_line as i32) < state.session.page_len as i32 - Self::HEADER_SIZE - 1 {
                            self.cursor.y += 1;
                            state.down(1)?;
                        } else {
                            self.cursor.y += 1;
                            self.top_line += (state.session.page_len as i32 - Self::HEADER_SIZE - 1).max(1) as usize;
                            self.redraw_fse(state)?;
                        }
                        self.print_line_number(state)?;
                    }
                }

                control_codes::HOME => {
                    if self.cursor.x > 0 {
                        state.backward(self.cursor.x as i32)?;
                        self.cursor.x = 0;
                    }
                }
                control_codes::INS => {
                    self.insert_mode = !self.insert_mode;
                    self.display_insert_mode(state)?;
                }
                control_codes::END => {
                    if self.cursor.x < self.cur_line().len() as i32 {
                        state.forward(self.cur_line().len() as i32 - self.cursor.x as i32)?;
                        self.cursor.x = self.cur_line().len() as i32;
                    }
                }

                control_codes::BS => {
                    if self.cursor.x > 0 {
                        self.cursor.x -= 1;
                        let o = self.cursor.x as usize;
                        self.cur_line().remove(o);
                        state.backward(1)?;
                        state.print(TerminalTarget::Both, &self.cur_line()[o..])?;
                        state.clear_eol()?;
                        let len = self.cur_line().len() as i32 - self.cursor.x as i32;
                        if len > 0 {
                            state.backward(len)?;
                        }
                    }
                }

                control_codes::DEL => {
                    if self.cursor.x < self.cur_line().len() as i32 {
                        let o = self.cursor.x as usize;
                        self.cur_line().remove(o);
                        state.print(TerminalTarget::Both, &self.cur_line()[o..])?;
                        let len = self.cur_line().len() as i32 - self.cursor.x as i32;
                        if len > 0 {
                            state.backward(len)?;
                        }
                    }
                }
                '\r' => {
                    if (self.cursor.y as usize) < self.msg.len() {
                        self.msg.insert(self.cursor.y as usize, String::new());
                        self.redraw_fse(state)?;
                    }
                    self.cursor.y += 1;
                    self.cursor.x = 0;
                    state.new_line()?;
                    self.print_line_number(state)?;
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
                            state.print(TerminalTarget::Both, &txt)?;
                            state.backward(txt.len() as i32)?;
                        } else {
                            self.cur_line().replace_range(o..o + 1, ch.to_string().as_str());
                        }

                        self.cursor.x += 1;
                        state.print(TerminalTarget::Both, &ch.to_string())?;
                    }
                }
            }
        }
    }
    fn redraw_fse(&mut self, state: &mut IcyBoardState) -> Res<()> {
        state.clear_screen()?;
        self.msg_header(state)?;
        state.gotoxy(TerminalTarget::Both, 1, Self::HEADER_SIZE)?;

        for i in 0..(state.session.page_len as usize) - Self::HEADER_SIZE as usize {
            let cur_line = i + self.top_line;
            if cur_line < self.msg.len() {
                state.print(TerminalTarget::Both, &self.msg[cur_line])?;
            }
            state.clear_eol()?;
            state.new_line()?;
        }
        state.display_text(IceText::EscToExit, 0)?;
        self.print_line_number(state)?;
        self.display_insert_mode(state)?;
        Ok(())
    }

    fn display_insert_mode(&self, state: &mut IcyBoardState) -> Res<()> {
        state.gotoxy(TerminalTarget::Both, 48, state.session.page_len as i32)?;
        if self.insert_mode {
            state.display_text(IceText::INSForOverwrite, 0)?;
        } else {
            state.display_text(IceText::INSForInsert, 0)?;
        }
        state.clear_eol()?;
        state.gotoxy(TerminalTarget::Both, self.cursor.x, Self::HEADER_SIZE + self.cursor.y - self.top_line as i32)?;
        Ok(())
    }

    fn insline(&mut self, state: &mut IcyBoardState) -> Res<()> {
        let mut edit_line = String::new();
        loop {
            let (new_line, next_line) = self.get_line(state, edit_line)?;
            if new_line.is_empty() && next_line.is_empty() {
                return Ok(());
            }
            self.msg.insert(self.cursor.y as usize, new_line);
            self.cursor.y += 1;
            edit_line = next_line;
        }
    }

    fn edline(&mut self, state: &mut IcyBoardState) -> Res<()> {
        let mut edit_line = self.msg.remove(self.cursor.y as usize);
        loop {
            let (new_line, next_line) = self.get_line(state, edit_line)?;
            if new_line.is_empty() && next_line.is_empty() {
                return Ok(());
            }
            self.msg.insert(self.cursor.y as usize, new_line);
            self.cursor.y += 1;
            edit_line = next_line;
        }
    }

    fn get_line(&mut self, state: &mut IcyBoardState, mut edit_line: String) -> Res<(String, String)> {
        let mut caret_x = edit_line.len();
        state.print(TerminalTarget::Both, &edit_line)?;

        loop {
            let Some(ch) = self.get_char_edit(state)? else {
                continue;
            };
            match ch.ch {
                control_codes::CTRL_LEFT => {
                    for i in (caret_x..0).rev() {
                        if i == 0 || edit_line.chars().nth(i).unwrap() == ' ' {
                            state.backward((caret_x - i) as i32)?;
                            caret_x = i;
                            break;
                        }
                    }
                }
                control_codes::CTRL_RIGHT => {
                    for i in caret_x..=edit_line.len() {
                        if i == edit_line.len() || edit_line.chars().nth(i).unwrap() == ' ' {
                            state.forward((i - caret_x) as i32)?;
                            caret_x = i;
                            break;
                        }
                    }
                }

                control_codes::LEFT => {
                    if caret_x > 0 {
                        caret_x -= 1;
                        state.backward(1)?;
                    }
                }
                control_codes::RIGHT => {
                    if caret_x < edit_line.len() {
                        caret_x += 1;
                        state.forward(1)?;
                    }
                }

                control_codes::HOME => {
                    if caret_x > 0 {
                        state.backward(caret_x as i32)?;
                        caret_x = 0;
                    }
                }

                control_codes::END => {
                    if caret_x < edit_line.len() {
                        state.forward(edit_line.len() as i32 - caret_x as i32)?;
                        caret_x = edit_line.len();
                    }
                }

                control_codes::BS => {
                    if caret_x > 0 {
                        caret_x -= 1;
                        edit_line.remove(caret_x);
                        state.print(TerminalTarget::Both, &format!("\x08 \x08{}", &edit_line[caret_x..]))?;
                    }
                }

                control_codes::DEL => {
                    if caret_x < edit_line.len() {
                        edit_line.remove(caret_x);
                        state.clear_line()?;
                        state.print(TerminalTarget::Both, &edit_line)?;
                        let len = edit_line.len() as i32 - caret_x as i32;
                        if len > 0 {
                            state.backward(len)?;
                        }
                    }
                }
                '\r' | '\n' => {
                    edit_line = edit_line.trim().to_string();
                    state.new_line()?;
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
                        state.print(TerminalTarget::Both, &ch.to_string())?;
                    }
                }
            }
        }
    }

    pub fn get_char_edit(&mut self, state: &mut IcyBoardState) -> Res<Option<KeyChar>> {
        let ch = state.get_char()?;
        if ch.is_none() {
            return Ok(None);
        }
        let mut ch: KeyChar = ch.unwrap();
        match ch.ch {
            control_codes::DEL_HIGH => {
                ch.ch = control_codes::DEL;
            }
            '\x1B' => {
                if let Some(key_char) = state.get_char()? {
                    if key_char.ch == '[' {
                        if let Some(key_char) = state.get_char()? {
                            match key_char.ch {
                                'A' => ch.ch = control_codes::UP,
                                'B' => ch.ch = control_codes::DOWN,
                                'C' => ch.ch = control_codes::RIGHT,
                                'D' => ch.ch = control_codes::LEFT,

                                'H' => ch.ch = control_codes::HOME,
                                'K' => ch.ch = control_codes::END,

                                'V' => ch.ch = control_codes::PG_UP,
                                'U' => ch.ch = control_codes::PG_DN,
                                '@' => {
                                    state.get_char()?;
                                    ch.ch = control_codes::INS;
                                }

                                '6' => {
                                    state.get_char()?;
                                    ch.ch = control_codes::PG_UP;
                                }
                                '5' => {
                                    state.get_char()?;
                                    ch.ch = control_codes::PG_DN;
                                }
                                '2' => {
                                    state.get_char()?;
                                    ch.ch = control_codes::INS;
                                }

                                'F' => ch.ch = control_codes::END,

                                _ => {
                                    // don't pass ctrl codes
                                    return Ok(None);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(Some(ch))
    }

    fn cur_line(&mut self) -> &mut String {
        while self.cursor.y as usize >= self.msg.len() {
            self.msg.push(String::new());
        }
        &mut self.msg[self.cursor.y as usize]
    }

    fn print_line_number(&self, state: &mut IcyBoardState) -> Res<()> {
        state.reset_color()?;
        state.gotoxy(TerminalTarget::Both, 79 - 2, 1)?;
        state.print(TerminalTarget::Both, &format!("{:>3}", self.top_line + self.cursor.y as usize + 1))?;
        state.clear_eol()?;
        state.gotoxy(TerminalTarget::Both, self.cursor.x, Self::HEADER_SIZE + self.cursor.y - self.top_line as i32)?;

        Ok(())
    }
}
