use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::Res;
use async_recursion::async_recursion;
use codepages::tables::CP437_TO_UNICODE;
use icy_engine::IceMode;
use jamjam::jam::{JamMessage, JamMessageBase};

use crate::{
    icy_board::{
        commands::CommandType,
        icb_config::IcbColor,
        icb_text::{IcbTextStyle, IceText},
        UTF8_BOM,
    },
    vm::TerminalTarget,
};

use super::{IcyBoardState, KeySource, NodeStatus};

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

pub const MASK_COMMAND: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+-=[]{}|;':,.<>?/\\\" ";

#[derive(Debug, PartialEq)]
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
        let first_ch = iter.next().unwrap_or_default();

        let call_type = match first_ch {
            '!' => PPECallType::PPE,
            '$' => PPECallType::Menu,
            '%' => PPECallType::File,
            _ => return None,
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
    }
}

impl IcyBoardState {
    #[async_recursion(?Send)]
    pub async fn display_text(&mut self, message_number: IceText, display_flags: i32) -> Res<()> {
        let txt_entry = self.display_text.get_display_text(message_number)?;
        let color = if txt_entry.style == IcbTextStyle::Plain {
            self.user_screen.caret.get_attribute().as_u8(IceMode::Blink).into()
        } else {
            txt_entry.style.to_color()
        };
        self.display_string(&txt_entry.text.replace('~', " "), color, display_flags).await
    }

    pub async fn display_string(&mut self, txt: &str, color: IcbColor, display_flags: i32) -> Res<()> {
        if display_flags & display_flags::NOTBLANK != 0 && txt.is_empty() {
            return Ok(());
        }

        if display_flags & display_flags::LOGIT != 0 {
            log::info!("{}", txt);
        }

        // let old_color = self.user_screen.caret.get_attribute().as_u8(icy_engine::IceMode::Blink);
        if display_flags & display_flags::LFBEFORE != 0 {
            self.new_line().await?;
        }
        if display_flags & display_flags::BELL != 0 {
            self.bell().await?;
        }
        if self.use_graphics() {
            self.set_color(TerminalTarget::Both, color).await?;
        }

        self.display_line(txt).await?;

        // up to 2 new lines are correct
        if display_flags & display_flags::NEWLINE != 0 {
            self.new_line().await?;
        }
        if display_flags & display_flags::LFAFTER != 0 {
            self.new_line().await?;
        }
        Ok(())
    }

    #[async_recursion(?Send)]
    async fn display_line(&mut self, txt: &str) -> Res<()> {
        if !txt.is_empty() {
            if let Some(call) = PPECall::try_parse_line(txt) {
                for sc in call.arguments {
                    self.session.tokens.push_back(sc.to_string());
                }
                match call.call_type {
                    PPECallType::PPE => {
                        let file = self.get_board().await.resolve_file(&call.file);
                        self.run_ppe(&file, None).await?;
                    }
                    PPECallType::Menu => {
                        self.display_menu(&call.file).await?;
                    }
                    PPECallType::File => {
                        let file = self.get_board().await.resolve_file(&call.file);
                        self.display_file(&file).await?;
                    }
                }
                return Ok(());
            } else {
                // display text
                self.print(TerminalTarget::Both, txt).await?;
            }
        }
        Ok(())
    }

    pub async fn display_menu<P: AsRef<Path>>(&mut self, file_name: &P) -> Res<bool> {
        let resolved_name_ppe = self.get_board().await.resolve_file(&(file_name.as_ref().with_extension("ppe")));
        let path = PathBuf::from(resolved_name_ppe);
        if path.exists() {
            self.run_ppe(&path, None).await?;
            return Ok(true);
        }
        self.display_file(&file_name).await
    }

    pub async fn display_file<P: AsRef<Path>>(&mut self, file_name: &P) -> Res<bool> {
        self.display_file_with_error(file_name, true).await
    }

    pub async fn display_file_with_error<P: AsRef<Path>>(&mut self, file_name: &P, display_error: bool) -> Res<bool> {
        self.session.disp_options.abort_printout = false;
        let resolved_name = self.get_board().await.resolve_file(file_name);
        // lookup language/security/graphics mode
        let resolved_name = self.find_more_specific_file(resolved_name.to_string_lossy().to_string());

        let Ok(content) = fs::read(resolved_name) else {
            if display_error {
                self.bell().await?;
                self.set_color(TerminalTarget::Both, IcbColor::dos_light_red()).await?;
                self.print(TerminalTarget::Both, &format!("\r\n({}) is missing!\r\n\r\n", file_name.as_ref().display()))
                    .await?;
            }
            return Ok(true);
        };
        let converted_content = if content.starts_with(&UTF8_BOM) {
            String::from_utf8_lossy(&content[3..]).to_string()
        } else {
            let mut s: String = String::new();
            for byte in content {
                if byte == 0x1A {
                    break;
                }
                s.push(CP437_TO_UNICODE[byte as usize]);
            }
            s
        };
        for (i, line) in converted_content.lines().enumerate() {
            if i > 0 {
                self.new_line().await?;
                if self.session.disp_options.abort_printout {
                    break;
                }
            }
            self.display_line(line).await?;
        }

        // .lines() not recognizes last empty line.
        if converted_content.ends_with('\n') {
            self.new_line().await?;
        }
        Ok(true)
    }

    pub async fn input_field(
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
            .await
    }

    #[async_recursion(?Send)]
    pub async fn input_string(
        &mut self,
        color: IcbColor,
        prompt: String,
        len: i32,
        valid_mask: &str,
        help: &str,
        default_string: Option<String>,
        mut display_flags: i32,
    ) -> Res<String> {
        if self.session.request_logoff {
            return Ok(String::new());
        }

        self.session.disp_options.no_change();

        // we've data from a PPE here, so take that input and return it.
        // ignoring all other settings.
        if let Some(front) = self.char_buffer.front() {
            if front.source == KeySource::StuffedHidden {
                let mut result = String::new();
                while let Some(key) = self.char_buffer.pop_front() {
                    if key.ch == '\n' || key.ch == '\r' {
                        break;
                    }
                    result.push(key.ch);
                }
                log::info!("PPE stuffed input: {}", result);
                self.session.push_tokens(&result);
                if let Some(token) = self.session.tokens.pop_front() {
                    return Ok(token);
                }
            }
        }

        let mut prompt = prompt;

        let display_question = if prompt.ends_with(TXT_STOPCHAR) {
            display_flags &= !(display_flags::FIELDLEN | display_flags::GUIDE);
            prompt.pop();
            false
        } else {
            true
        };
        self.check_time_left();

        if display_flags & display_flags::LFBEFORE != 0 {
            self.new_line().await?;
        }
        if display_flags & display_flags::BELL != 0 {
            self.bell().await?;
        }
        if self.use_graphics() {
            self.set_color(TerminalTarget::Both, color.clone()).await?;
        }
        self.display_line(&prompt).await?;

        // we've data from a PPE here, so take that input and return it.
        // ignoring all other settings.
        if let Some(front) = self.char_buffer.front() {
            if front.source == KeySource::StuffedHidden {
                let mut result = String::new();
                while let Some(key) = self.char_buffer.pop_front() {
                    if key.ch == '\n' || key.ch == '\r' {
                        break;
                    }
                    result.push(key.ch);
                }
                log::info!("PPE stuffed input: {}", result);
                return Ok(result);
            }
        }

        if display_question {
            self.print(TerminalTarget::Both, "? ").await?;
        }

        if display_flags & display_flags::FIELDLEN != 0 {
            self.print(TerminalTarget::Both, "(").await?;
            self.forward(len).await?;
            self.print(TerminalTarget::Both, ")").await?;
            self.backward(len + 1).await?;
            self.reset_color(TerminalTarget::Both).await?;
            if let Some(default) = &default_string {
                self.print(TerminalTarget::Both, default).await?;
                self.backward(default.len() as i32).await?;
            }
        }
        if self.use_graphics() {
            self.reset_color(TerminalTarget::Both).await?;
        }

        let mut output = String::new();
        loop {
            if self.session.request_logoff {
                return Ok(String::new());
            }

            let Some(mut key_char) = self.get_char(TerminalTarget::Both).await? else {
                continue;
            };
            if display_flags & display_flags::UPCASE != 0 {
                key_char.ch = key_char.ch.to_ascii_uppercase();
            }
            if key_char.ch == '\n' || key_char.ch == '\r' {
                if !help.is_empty() {
                    if let Some(cmd) = self.try_find_command(&output).await {
                        if !cmd.actions.is_empty() && cmd.actions[0].command_type == CommandType::Help {
                            self.show_help(help).await?;
                            return self.input_string(color, prompt, len, valid_mask, help, default_string, display_flags).await;
                        }
                    }
                }

                if display_flags & display_flags::ERASELINE != 0 {
                    self.clear_line(TerminalTarget::Both).await?;
                }
                break;
            }
            if key_char.ch == '\x08' && !output.is_empty() {
                output.pop();
                if key_char.source != KeySource::StuffedHidden {
                    self.print(TerminalTarget::Both, "\x08 \x08").await?;
                }
                continue;
            }

            if (output.len() as i32) < len
                && (if (display_flags & display_flags::YESNO) != 0 {
                    &self.session.yes_no_mask
                } else {
                    valid_mask
                }
                .contains(key_char.ch)
                    || (display_flags & display_flags::STACKED) != 0 && " ;".contains(key_char.ch))
            {
                output.push(key_char.ch);
                if key_char.source != KeySource::StuffedHidden {
                    if display_flags & display_flags::ECHODOTS != 0 {
                        self.print(TerminalTarget::Both, ".").await?;
                    } else {
                        self.print(TerminalTarget::Both, &key_char.ch.to_string()).await?;
                    }
                }
            }
        }
        if display_flags & display_flags::NEWLINE != 0 {
            self.new_line().await?;
        }
        if display_flags & display_flags::LFAFTER != 0 {
            self.new_line().await?;
        }

        if output.is_empty() {
            if let Some(default) = default_string {
                return Ok(default);
            }
        }

        Ok(output)
    }

    pub async fn show_help(&mut self, help: &str) -> Res<()> {
        // hardcoded help file.
        if help == "HLPMORE" || help == "HLPXFRMORE" {
            self.display_text(IceText::MorehelpEnter, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            self.display_text(IceText::MorehelpYes, display_flags::NEWLINE).await?;
            self.display_text(IceText::MorehelpNo, display_flags::NEWLINE).await?;
            self.display_text(IceText::MorehelpNonstop, display_flags::NEWLINE).await?;
            if help == "HLPXFRMORE" {
                self.display_text(IceText::MorehelpView, display_flags::NEWLINE).await?;
                self.display_text(IceText::MorehelpFlag, display_flags::NEWLINE).await?;
            }
            return Ok(());
        }

        let help_loc = self.get_board().await.config.paths.help_path.clone();
        let help_loc = help_loc.join(help);

        let tmp = self.session.disp_options.count_lines;
        self.session.disp_options.no_change();
        self.display_file(&help_loc).await?;
        self.session.disp_options.count_lines = tmp;
        Ok(())
    }

    pub async fn check_password<F: Fn(&str) -> bool>(&mut self, ice_text: IceText, flags: u32, call_back: F) -> Res<bool> {
        if !self.session.last_password.is_empty() && call_back(&self.session.last_password) {
            return Ok(true);
        }
        let mut tries = 0;

        while tries < 3 {
            let pwd = self
                .input_field(
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
                )
                .await?;

            if call_back(&pwd) {
                self.session.last_password = pwd;
                return Ok(true);
            }
            if (flags & pwd_flags::PLAIN) == 0 && (flags & pwd_flags::SHOW_WRONG_PWD_MSG) != 0 {
                self.display_text(IceText::WrongPasswordEntered, display_flags::NEWLINE).await?;
            }
            tries += 1;
        }
        if let Some(user) = &mut self.session.current_user {
            user.stats.num_password_failures += 1;
        }
        if (flags & pwd_flags::PLAIN) == 0 {
            self.session.op_text = self.session.get_username_or_alias();
            self.display_text(IceText::PasswordFailure, display_flags::NEWLINE | display_flags::LFAFTER)
                .await?;
        }
        Ok(false)
    }

    #[async_recursion(?Send)]
    pub async fn show_message_areas(&mut self, conference: u16) -> Res<Option<usize>> {
        let menu = self.get_board().await.conferences[conference as usize].area_menu.clone();
        let areas = self.get_board().await.conferences[conference as usize].areas.clone().unwrap_or_default();

        self.set_activity(NodeStatus::EnterMessage).await;
        self.session.disp_options.no_change();
        self.session.more_requested = false;

        if areas.is_empty() {
            self.display_text(IceText::NoAreasAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            self.press_enter().await?;
            return Ok(None);
        }
        let area_number = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            let mnu = self.resolve_path(&menu);
            self.display_menu(&mnu).await?;
            self.new_line().await?;

            self.input_field(
                if self.session.expert_mode {
                    IceText::AreaListCommandExpert
                } else {
                    IceText::AreaListCommand
                },
                40,
                &MASK_NUM,
                CommandType::ReadMessages.get_help(),
                None,
                display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
            )
            .await?
        };

        if !area_number.is_empty() {
            if let Ok(number) = area_number.parse::<i32>() {
                if 1 <= number && (number as usize) <= areas.len() {
                    let area = &areas[number as usize - 1];
                    if area.req_level_to_list.user_can_access(&self.session) {
                        return Ok(Some(number as usize - 1));
                    }
                }
            }

            self.session.op_text = area_number;
            self.display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::NOTBLANK)
                .await?;
        }
        Ok(None)
    }

    pub async fn send_message(&mut self, conf: i32, area: i32, msg: JamMessage, text: IceText) -> Res<()> {
        let msg_base = if conf < 0 {
            let user_name = msg.get_to().unwrap().to_string();
            self.get_email_msgbase(&user_name).await
        } else {
            let msg_base = self.get_board().await.conferences[conf as usize].areas.as_ref().unwrap()[area as usize]
                .filename
                .clone();
            let msg_base: PathBuf = self.resolve_path(&msg_base);
            if msg_base.with_extension("jhr").exists() {
                JamMessageBase::open(msg_base)
            } else {
                JamMessageBase::create(msg_base)
            }
        };

        match msg_base {
            Ok(mut msg_base) => {
                let number = msg_base.write_message(&msg)?;
                msg_base.write_jhr_header()?;

                self.display_text(text, display_flags::DEFAULT).await?;
                self.println(TerminalTarget::Both, &number.to_string()).await?;
                self.new_line().await?;
            }
            Err(err) => {
                log::error!("while opening message base: {}", err.to_string());
                self.display_text(IceText::MessageBaseError, display_flags::NEWLINE).await?;
            }
        }

        Ok(())
    }

    pub async fn get_email_msgbase(&mut self, user_name: &str) -> Res<JamMessageBase> {
        let name = self.get_board().await.config.paths.email_msgbase.clone();
        let mut msg_base = self.resolve_path(&name);
        if msg_base.is_dir() {
            msg_base = msg_base.join("email");
        }
        Ok(if msg_base.with_extension("jhr").exists() {
            JamMessageBase::open(msg_base)?
        } else {
            log::info!("Creating new email message base for user {}", user_name);
            JamMessageBase::create(msg_base)?
        })
    }
}

const MASK_PASSWORD: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()-_=+[]{};:'\",.<>/?\\|~`";

pub mod pwd_flags {
    pub const SHOW_WRONG_PWD_MSG: u32 = 0x00001;
    /// Don't show any text
    pub const PLAIN: u32 = 0x00002;
}
