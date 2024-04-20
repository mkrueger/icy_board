use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::Res;
use codepages::tables::CP437_TO_UNICODE;
use icy_engine::IceMode;

use crate::{
    icy_board::{
        commands::CommandType,
        icb_config::IcbColor,
        icb_text::{IcbTextStyle, IceText},
        UTF8_BOM,
    },
    vm::TerminalTarget,
};

use super::{IcyBoardState, KeySource};

pub mod display_flags {
    pub const DEFAULT: i32 = 0x00000;
    pub const ECHODOTS: i32 = 0x00001;
    pub const FIELDLEN: i32 = 0x00002;
    pub const UPCASE: i32 = 0x00008;
    pub const STACKED: i32 = 0x00010;
    pub const ERASELINE: i32 = 0x00020;
    pub const NEWLINE: i32 = 0x00040;
    pub const LFBEFORE: i32 = 0x00080;
    pub const LFAFTER: i32 = 0x00100;
    pub const LOGIT: i32 = 0x08000;
    pub const LOGITLEFT: i32 = 0x10000;
    pub const GUIDE: i32 = 0x00004;
    pub const WORDWRAP: i32 = 0x00200;
    pub const YESNO: i32 = 0x04000;
    pub const NOCLEAR: i32 = 0x00400;
    pub const BELL: i32 = 0x00800;
    pub const HIGHASCII: i32 = 0x01000;
    pub const AUTO: i32 = 0x02000;
    pub const NOTBLANK: i32 = 0x02000; // same as 'AUTO'
}

pub mod pcb_colors {
    use crate::icy_board::icb_config::IcbColor;

    pub const BLUE: IcbColor = IcbColor::Dos(9);
    pub const GREEN: IcbColor = IcbColor::Dos(10);
    pub const CYAN: IcbColor = IcbColor::Dos(11);
    pub const RED: IcbColor = IcbColor::Dos(12);
    pub const MAGENTA: IcbColor = IcbColor::Dos(13);
    pub const YELLOW: IcbColor = IcbColor::Dos(14);
    pub const WHITE: IcbColor = IcbColor::Dos(15);
}
const TXT_STOPCHAR: char = '_';

lazy_static::lazy_static! {
    pub static ref MASK_PWD: String = (' '..='~').collect::<String>();
    pub static ref MASK_ALPHA: String = ('A'..='Z').collect::<String>() + ('a'..='z').collect::<String>().as_str();
    pub static ref MASK_NUM: String = ('0'..='9').collect::<String>();
    pub static ref MASK_ALNUM: String = ('A'..='Z').collect::<String>() + ('a'..='z').collect::<String>().as_str() + ('0'..='9').collect::<String>().as_str();
    pub static ref MASK_FILE: String =  ('A'..='Z').collect::<String>() + ('a'..='z').collect::<String>().as_str() + ('0'..='9').collect::<String>().as_str() + "!#$%&'()-.:[\\]^_`~";
    pub static ref MASK_PATH: String =  ('A'..='Z').collect::<String>()
    + ('a'..='z').collect::<String>().as_str()
    + ('0'..='9').collect::<String>().as_str()
    + "!#$%&'()-.:[\\]^_`~";
    pub static ref MASK_ASCII: String = (' '..='~').collect::<String>();
    pub static ref MASK_WEB: String = ('A'..='Z').collect::<String>() + ('a'..='z').collect::<String>().as_str() + ('0'..='9').collect::<String>().as_str() + "@.:!#$%&'*+-/=?^_`{|}~";

    pub static ref MASK_PHONE: String = ('0'..='9').collect::<String>() + "/()-+ ";

}

#[derive(Debug)]
pub enum PPECallType {
    PPE,
    Menu,
    File,
}
#[derive(Debug)]
pub struct PPECall {
    pub call_type: PPECallType,
    pub file: String,
    pub arguments: Vec<String>,
}

impl PPECall {
    pub fn try_parse_line(line: &str) -> Option<PPECall> {
        if line.is_empty() {
            return None;
        }
        let mut iter = line.chars();
        let first_ch = iter.next().unwrap();

        if first_ch == '!' || first_ch == '%' || first_ch == '$' {
            let call_type = match first_ch {
                '!' => PPECallType::PPE,
                '$' => PPECallType::Menu,
                _ => PPECallType::File,
            };
            let mut arguments = Vec::new();
            let mut arg = String::new();

            for ch in iter {
                if ch == ' ' || ch == '_' {
                    if !arg.is_empty() {
                        arguments.push(arg);
                        arg = String::new();
                    }
                    if ch == '_' {
                        break;
                    }
                    continue;
                }
                arg.push(ch);
            }

            if !arg.is_empty() {
                arguments.push(arg);
            }
            Some(Self {
                call_type,
                file: arguments[0].clone(),
                arguments: arguments[1..].to_vec(),
            })
        } else {
            None
        }
    }
}

impl IcyBoardState {
    pub fn display_text(&mut self, message_number: IceText, display_flags: i32) -> Res<()> {
        let txt_entry = self.display_text.get_display_text(message_number)?;
        let color = if txt_entry.style == IcbTextStyle::Plain {
            self.user_screen.caret.get_attribute().as_u8(IceMode::Blink).into()
        } else {
            txt_entry.style.to_color()
        };
        self.display_string(&txt_entry.text.replace('~', " "), color, display_flags)
    }

    pub fn display_string(&mut self, txt: &str, color: IcbColor, display_flags: i32) -> Res<()> {
        if display_flags & display_flags::NOTBLANK != 0 && txt.is_empty() {
            return Ok(());
        }

        if display_flags & display_flags::LOGIT != 0 {
            log::info!("{}", txt);
        }

        let old_color = self.user_screen.caret.get_attribute().as_u8(icy_engine::IceMode::Blink);
        if display_flags & display_flags::LFBEFORE != 0 {
            self.new_line()?;
        }
        if display_flags & display_flags::BELL != 0 {
            self.bell()?;
        }
        if self.use_graphics() {
            self.set_color(TerminalTarget::Both, color)?;
        }

        self.display_line(txt)?;

        // up to 2 new lines are correct
        if display_flags & display_flags::NEWLINE != 0 {
            self.new_line()?;
        }
        if display_flags & display_flags::LFAFTER != 0 {
            self.new_line()?;
        }
        if self.use_graphics() {
            self.set_color(TerminalTarget::Both, old_color.into())?;
        }
        Ok(())
    }

    fn display_line(&mut self, txt: &str) -> Res<()> {
        if !txt.is_empty() {
            if let Some(call) = PPECall::try_parse_line(txt) {
                for sc in call.arguments {
                    self.session.tokens.push_back(sc.to_string());
                }
                match call.call_type {
                    PPECallType::PPE => {
                        let file = self.board.lock().unwrap().resolve_file(&call.file);
                        self.run_ppe(&file, None)?;
                    }
                    PPECallType::Menu => {
                        self.display_menu(&call.file)?;
                    }
                    PPECallType::File => {
                        let file = self.board.lock().unwrap().resolve_file(&call.file);
                        self.display_file(&file)?;
                    }
                }
                return Ok(());
            } else {
                // display text
                self.print(TerminalTarget::Both, txt)?;
            }
        }
        Ok(())
    }

    pub fn display_menu<P: AsRef<Path>>(&mut self, file_name: &P) -> Res<bool> {
        let resolved_name_ppe = self.board.lock().unwrap().resolve_file(&(file_name.as_ref().with_extension("PPE")));
        let path = PathBuf::from(resolved_name_ppe);
        if path.exists() {
            self.run_ppe(&path, None)?;
            return Ok(true);
        }
        self.display_file(&file_name)
    }

    pub fn display_file<P: AsRef<Path>>(&mut self, file_name: &P) -> Res<bool> {
        let resolved_name = self.board.lock().unwrap().resolve_file(file_name);

        // lookup language/security/graphics mode
        let resolved_name = self.find_more_specific_file(resolved_name);

        let Ok(content) = fs::read(resolved_name) else {
            self.bell()?;
            self.set_color(TerminalTarget::Both, pcb_colors::RED)?;
            self.print(TerminalTarget::Both, &format!("\r\n({}) is missing!\r\n\r\n", file_name.as_ref().display()))?;
            return Ok(true);
        };
        let converted_content = if content.starts_with(&UTF8_BOM) {
            String::from_utf8_lossy(&content[3..]).to_string()
        } else {
            let mut s = String::new();
            for byte in content {
                s.push(CP437_TO_UNICODE[byte as usize]);
            }
            s
        };
        let old = self.session.disp_options.non_stop;
        self.session.disp_options.non_stop = true;
        for line in converted_content.lines() {
            self.display_line(line)?;
            self.write_raw(TerminalTarget::Both, &['\r', '\n'])?;
            self.session.disp_options.non_stop = false;
            let next = self.next_line()?;
            self.session.disp_options.non_stop = true;
            if !next {
                return Ok(false);
            }
        }
        self.session.disp_options.non_stop = old;

        Ok(true)
    }

    pub fn input_field(
        &mut self,
        message_number: IceText,
        len: i32,
        valid_mask: &str,
        help: &str,
        default_string: Option<String>,
        display_flags: i32,
    ) -> Res<String> {
        let txt_entry = self.display_text.get_display_text(message_number)?;

        self.input_string(txt_entry.style.to_color(), txt_entry.text, len, valid_mask, help, default_string, display_flags)
    }

    pub fn input_string(
        &mut self,
        color: IcbColor,
        prompt: String,
        len: i32,
        valid_mask: &str,
        help: &str,
        default_string: Option<String>,
        display_flags: i32,
    ) -> Res<String> {
        self.session.num_lines_printed = 0;

        log::warn!("{:08X}", display_flags);
        let mut prompt = prompt;
        let display_question = if prompt.ends_with(TXT_STOPCHAR) {
            prompt.pop();
            false
        } else {
            true
        };
        self.check_time_left();

        if display_flags & display_flags::LFBEFORE != 0 {
            self.new_line()?;
        }
        if display_flags & display_flags::BELL != 0 {
            self.bell()?;
        }
        if self.use_graphics() {
            self.set_color(TerminalTarget::Both, color.clone())?;
        }
        self.display_line(&prompt)?;
        if display_question {
            self.print(TerminalTarget::Both, "?")?;
        }
        self.print(TerminalTarget::Both, " ")?;

        if display_flags & display_flags::FIELDLEN != 0 {
            self.print(TerminalTarget::Both, "(")?;
            self.forward(len)?;
            self.print(TerminalTarget::Both, ")")?;
            self.backward(len + 1)?;
            self.reset_color()?;
            if let Some(default) = &default_string {
                self.print(TerminalTarget::Both, default)?;
                self.backward(default.len() as i32)?;
            }
        }
        if self.use_graphics() {
            self.reset_color()?;
        }

        let mut output = String::new();
        loop {
            let Some(mut key_char) = self.get_char()? else {
                continue;
            };
            if display_flags & display_flags::UPCASE != 0 {
                key_char.ch = key_char.ch.to_ascii_uppercase();
            }
            if key_char.ch == '\n' || key_char.ch == '\r' {
                if !help.is_empty() {
                    if let Some(cmd) = self.try_find_command(&output) {
                        if cmd.command_type == CommandType::Help {
                            self.show_help(help)?;
                            return self.input_string(color, prompt, len, valid_mask, help, default_string, display_flags);
                        }
                    }
                }

                if display_flags & display_flags::ERASELINE != 0 {
                    self.clear_line()?;
                }
                break;
            }
            if key_char.ch == '\x08' && !output.is_empty() {
                output.pop();
                if key_char.source != KeySource::StuffedHidden {
                    self.print(TerminalTarget::Both, "\x08 \x08")?;
                }
                continue;
            }

            if (output.len() as i32) < len
                && if (display_flags & display_flags::YESNO) != 0 {
                    &self.session.yes_no_mask
                } else {
                    valid_mask
                }
                .contains(key_char.ch)
            {
                output.push(key_char.ch);
                if key_char.source != KeySource::StuffedHidden {
                    if display_flags & display_flags::ECHODOTS != 0 {
                        self.print(TerminalTarget::Both, ".")?;
                    } else {
                        self.print(TerminalTarget::Both, &key_char.ch.to_string())?;
                    }
                }
            }
        }
        if display_flags & display_flags::NEWLINE != 0 {
            self.new_line()?;
        }
        if display_flags & display_flags::LFAFTER != 0 {
            self.new_line()?;
        }
        self.session.num_lines_printed = 0;

        if output.is_empty() {
            if let Some(default) = default_string {
                return Ok(default);
            }
        }

        Ok(output)
    }

    pub fn show_help(&mut self, help: &str) -> Res<()> {
        let help_loc = self.board.lock().unwrap().config.paths.help_path.clone();
        let help_loc = help_loc.join(help);
        let am = self.session.disable_auto_more;
        self.session.disable_auto_more = false;
        self.display_file(&help_loc)?;
        self.session.disable_auto_more = am;
        Ok(())
    }

    pub fn check_password<F: Fn(&str) -> bool>(&mut self, ice_text: IceText, flags: u32, call_back: F) -> Res<bool> {
        if !self.session.last_password.is_empty() && call_back(&self.session.last_password) {
            return Ok(true);
        }
        let mut tries = 0;

        while tries < 3 {
            let pwd = self.input_field(
                ice_text,
                13,
                MASK_PASSWORD,
                "",
                None,
                if (flags & pwd_flags::SHOW_WRONG_PWD_MSG) != 0 {
                    display_flags::FIELDLEN | display_flags::ECHODOTS | display_flags::NEWLINE
                } else {
                    display_flags::FIELDLEN | display_flags::ECHODOTS | display_flags::ERASELINE
                },
            )?;

            if call_back(&pwd) {
                self.session.last_password = pwd;
                return Ok(true);
            }
            if (flags & pwd_flags::SHOW_WRONG_PWD_MSG) != 0 {
                self.display_text(IceText::WrongPasswordEntered, display_flags::NEWLINE)?;
            }
            tries += 1;
        }
        if let Some(user) = &mut self.current_user {
            user.stats.num_password_failures += 1;
        }
        self.display_text(IceText::PasswordFailure, display_flags::NEWLINE | display_flags::LFAFTER)?;
        Ok(false)
    }
}

const MASK_PASSWORD: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()-_=+[]{};:'\",.<>/?\\|~`";

pub mod pwd_flags {
    pub const SHOW_WRONG_PWD_MSG: u32 = 0x00001;
}
