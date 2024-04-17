use super::{PcbBoardCommand, MASK_COMMAND, MASK_NUMBER};

use bstr::BString;
use chrono::Utc;
use icy_board_engine::{
    icy_board::{
        commands::Command,
        icb_config::IcbColor,
        icb_text::IceText,
        state::{control_codes, functions::display_flags, KeyChar, UserActivity},
    },
    vm::TerminalTarget,
};
use icy_ppe::Res;
use jamjam::jam::JamMessage;

impl PcbBoardCommand {
    pub fn comment_to_sysop(&mut self, action: &Command) -> Res<()> {
        let leave_comment = self.state.input_field(
            IceText::LeaveComment,
            1,
            "",
            &action.help,
            Some(self.state.session.no_char.to_string()),
            display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::FIELDLEN | display_flags::YESNO,
        )?;

        if leave_comment.is_empty() || leave_comment.chars().next().unwrap() == self.state.session.no_char {
            return Ok(());
        };

        let to = self.state.board.lock().unwrap().config.sysop.name.clone();
        let subj: &str = "comment";

        let receipt = self.state.input_field(
            IceText::RequireReturnReceipt,
            1,
            "",
            &"",
            Some(self.state.session.no_char.to_string()),
            display_flags::NEWLINE | display_flags::UPCASE | display_flags::YESNO | display_flags::FIELDLEN,
        )?;
        self.state.node_state.lock().unwrap().user_activity = UserActivity::CommentToSysop;
        self.write_message(-1, -1, &to, &subj, receipt == self.state.session.yes_char.to_uppercase().to_string())?;

        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }

    fn write_message(&mut self, conf: i32, area: i32, to: &str, subj: &str, _ret_receipt: bool) -> Res<()> {
        self.displaycmdfile("preedit")?;
        self.state.new_line()?;

        self.state.display_text(IceText::MessageEnterText, 0)?;

        // display line editor header.
        self.state.session.op_text = 99.to_string(); // display max lines.
        self.state.display_text(IceText::Columns79, display_flags::NEWLINE)?;
        self.print_divider()?;
        let mut msg = Vec::new();
        let mut cur_line = 0;
        loop {
            self.insline(&mut msg, &mut cur_line)?;

            loop {
                let cmd = if self.state.session.expert_mode {
                    self.state.input_field(
                        IceText::MessageCommandExpertmode,
                        30,
                        "",
                        &"hlpe",
                        None,
                        display_flags::NEWLINE | display_flags::UPCASE,
                    )?
                } else {
                    self.state
                        .display_text(IceText::MessageCommandNovice1, display_flags::NEWLINE | display_flags::LFBEFORE)?;
                    self.state.display_text(IceText::MessageCommandNovice2, display_flags::NEWLINE)?;

                    self.state.input_field(
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
                        let abort = self.state.input_field(
                            IceText::MessageAbort,
                            1,
                            "",
                            &"",
                            Some(self.state.session.no_char.to_string()),
                            display_flags::YESNO | display_flags::NEWLINE | display_flags::UPCASE | display_flags::FIELDLEN | display_flags::LFBEFORE,
                        )?;
                        if abort == self.state.session.yes_char.to_uppercase().to_string() {
                            self.state
                                .display_text(IceText::MessageAborted, display_flags::NEWLINE | display_flags::LFBEFORE)?;

                            return Ok(());
                        }
                    }
                    "C" => {
                        // continue edit
                        self.print_divider()?;
                        cur_line = msg.len();
                        break;
                    }
                    "D" => {
                        // delete line
                        let line = self.read_line_number(IceText::DeleteLineNumber, msg.len())?;
                        if line > 0 {
                            self.print_divider()?;
                            self.state.set_color(TerminalTarget::Both, IcbColor::Dos(11))?;
                            self.state.print(TerminalTarget::Both, &format!("{}: ", line))?;
                            self.state.reset_color()?;
                            self.state.println(TerminalTarget::Both, &msg[line - 1])?;

                            let delete_line = self.state.input_field(
                                IceText::WantToDeleteLine,
                                1,
                                "",
                                &"",
                                Some(self.state.session.no_char.to_string()),
                                display_flags::YESNO | display_flags::NEWLINE | display_flags::UPCASE | display_flags::FIELDLEN | display_flags::LFBEFORE,
                            )?;
                            if delete_line == self.state.session.yes_char.to_uppercase().to_string() {
                                msg.remove(line - 1);
                            }
                        }
                    }
                    "E" => {
                        // edit line
                        let line = self.read_line_number(IceText::EditLineNumber, msg.len())?;
                        if line > 0 {
                            cur_line = line - 1;
                            self.print_divider()?;
                            self.edline(&mut msg, &mut cur_line)?;
                        }
                    }
                    "F" | "V" => { // Switch to full screen editor
                         // TODO: FSE
                    }
                    "I" => {
                        // insert line
                        let line = self.read_line_number(IceText::InsertBeforeNumber, msg.len())?;
                        if line > 0 {
                            cur_line = line - 1;
                            self.print_divider()?;
                            break;
                        }
                    }
                    "L" => {
                        let to_txt = self.state.display_text.get_display_text(IceText::To)?.text.trim_start().replace("~", " ");
                        let subj_txt = self.state.display_text.get_display_text(IceText::Subject)?.text.trim_start().replace("~", " ");

                        let to_part = format!("{}{}", to_txt, to);
                        let subj_part = format!("{}{}", subj_txt, subj);
                        self.state.set_color(TerminalTarget::Both, IcbColor::Dos(14))?;
                        self.state.println(TerminalTarget::Both, &format!("{:<39}{}", to_part, subj_part))?;
                        self.print_divider()?;
                        for line in &msg {
                            self.state.println(TerminalTarget::Both, line)?;
                        }
                    }
                    "S" => {
                        // send message
                        let msg = msg.join("\n");
                        let msg = JamMessage::default()
                            .with_from(BString::from(self.state.session.user_name.clone()))
                            .with_to(BString::from(self.state.session.sysop_name.clone()))
                            .with_subject(BString::from(subj))
                            .with_date_time(Utc::now())
                            .with_text(BString::from(msg));

                        self.send_message(conf, area, msg)?;
                        return Ok(());
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

    fn print_divider(&mut self) -> Res<()> {
        self.state.set_color(TerminalTarget::Both, IcbColor::Dos(11))?;
        self.state.println(TerminalTarget::Both, &str::repeat("-", 79))?;
        self.state.reset_color()?;
        Ok(())
    }

    fn read_line_number(&mut self, msg: IceText, line_count: usize) -> Res<usize> {
        let line_number = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            self.state.input_field(msg, 340, MASK_NUMBER, &"", None, display_flags::NEWLINE)?
        };
        let line = line_number.parse::<usize>().unwrap_or_default();
        if line < 1 || line >= line_count + 1 {
            self.state
                .display_text(IceText::NoSuchLineNumber, display_flags::NEWLINE | display_flags::LFBEFORE)?;
            Ok(0)
        } else {
            Ok(line)
        }
    }

    fn insline(&mut self, msg: &mut Vec<String>, line_number: &mut usize) -> Res<()> {
        let mut edit_line = String::new();
        loop {
            let (new_line, next_line) = self.get_line(edit_line)?;
            if new_line.is_empty() && next_line.is_empty() {
                return Ok(());
            }
            msg.insert(*line_number, new_line);
            *line_number += 1;
            edit_line = next_line;
        }
    }

    fn edline(&mut self, msg: &mut Vec<String>, line_number: &mut usize) -> Res<()> {
        let mut edit_line = msg.remove(*line_number);
        loop {
            let (new_line, next_line) = self.get_line(edit_line)?;
            if new_line.is_empty() && next_line.is_empty() {
                return Ok(());
            }
            msg.insert(*line_number, new_line);
            *line_number += 1;
            edit_line = next_line;
        }
    }

    fn get_line(&mut self, mut edit_line: String) -> Res<(String, String)> {
        let mut caret_x = edit_line.len();
        self.state.print(TerminalTarget::Both, &edit_line)?;

        loop {
            let Some(ch) = self.get_char_edit()? else {
                continue;
            };
            match ch.ch {
                control_codes::CTRL_LEFT => {
                    for i in (caret_x..0).rev() {
                        if i == 0 || edit_line.chars().nth(i).unwrap() == ' ' {
                            self.state.backward((caret_x - i) as i32)?;
                            caret_x = i;
                            break;
                        }
                    }
                }
                control_codes::CTRL_RIGHT => {
                    for i in caret_x..=edit_line.len() {
                        if i == edit_line.len() || edit_line.chars().nth(i).unwrap() == ' ' {
                            self.state.forward((i - caret_x) as i32)?;
                            caret_x = i;
                            break;
                        }
                    }
                }

                control_codes::LEFT => {
                    if caret_x > 0 {
                        caret_x -= 1;
                        self.state.backward(1)?;
                    }
                }
                control_codes::RIGHT => {
                    if caret_x < edit_line.len() {
                        caret_x += 1;
                        self.state.forward(1)?;
                    }
                }

                control_codes::HOME => {
                    if caret_x > 0 {
                        self.state.backward(caret_x as i32)?;
                        caret_x = 0;
                    }
                }

                control_codes::END => {
                    if caret_x < edit_line.len() {
                        self.state.forward(edit_line.len() as i32 - caret_x as i32)?;
                        caret_x = edit_line.len();
                    }
                }

                control_codes::BS => {
                    if caret_x > 0 {
                        caret_x -= 1;
                        edit_line.remove(caret_x);
                        self.state.print(TerminalTarget::Both, &format!("\x08 \x08{}", &edit_line[caret_x..]))?;
                    }
                }

                control_codes::DEL => {
                    if caret_x < edit_line.len() {
                        edit_line.remove(caret_x);
                        self.state.clear_line()?;
                        self.state.print(TerminalTarget::Both, &edit_line)?;
                        let len = edit_line.len() as i32 - caret_x as i32;
                        if len > 0 {
                            self.state.backward(len)?;
                        }
                    }
                }
                '\r' | '\n' => {
                    edit_line = edit_line.trim().to_string();
                    self.state.new_line()?;
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
                        self.state.print(TerminalTarget::Both, &ch.to_string())?;
                    }
                }
            }
        }
    }

    pub fn get_char_edit(&mut self) -> Res<Option<KeyChar>> {
        let ch = self.state.get_char()?;
        if ch.is_none() {
            return Ok(None);
        }
        let mut ch: KeyChar = ch.unwrap();
        match ch.ch {
            control_codes::DEL_HIGH => {
                ch.ch = control_codes::DEL;
            }
            '\x1B' => {
                if let Some(key_char) = self.state.get_char()? {
                    if key_char.ch == '[' {
                        if let Some(key_char) = self.state.get_char()? {
                            match key_char.ch {
                                'A' => ch.ch = control_codes::UP,
                                'B' => ch.ch = control_codes::DOWN,
                                'C' => ch.ch = control_codes::RIGHT,
                                'D' => ch.ch = control_codes::LEFT,

                                'H' => ch.ch = control_codes::HOME,
                                'K' => ch.ch = control_codes::END,

                                'V' => ch.ch = control_codes::PG_UP,
                                'U' => ch.ch = control_codes::PG_DN,
                                '@' => ch.ch = control_codes::INS,

                                'F' => ch.ch = control_codes::END,

                                _ => {}
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(Some(ch))
    }
}
