use std::{
    fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    time::Instant,
};

use crate::Res;
use async_recursion::async_recursion;
use chrono::{DateTime, Local};
use codepages::tables::CP437_TO_UNICODE;
use icy_engine::{AnsiCompatibilityLevel, FileFormat, FormatOptions, IceMode, LineBreakBehavior, LineLength, SaveOptions, Screen};
use icy_engine_scripting::Animator;
use jamjam::jam::{JamMessage, JamMessageBase};
use tokio::time::{Duration, sleep};

use crate::{
    icy_board::{
        UTF8_BOM,
        commands::CommandType,
        icb_config::IcbColor,
        icb_text::{IcbTextStyle, IceText},
    },
    vm::TerminalTarget,
};

use super::IcyBoardState;

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

pub static MASK_PWD: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| (' '..='~').collect::<String>());
pub static MASK_ALPHA: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| ('A'..='Z').collect::<String>() + ('a'..='z').collect::<String>().as_str());
pub static MASK_NUM: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| ('0'..='9').collect::<String>());
/// PPL's `MASK_ALNUM()`, which is narrower than the name suggests elsewhere:
/// `PCBoard`'s own "alphanumeric" mask is all of printable ASCII.
pub static MASK_ALNUM: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(|| ('A'..='Z').collect::<String>() + ('a'..='z').collect::<String>().as_str() + ('0'..='9').collect::<String>().as_str());
pub static MASK_FILE: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(|| ('@'..='z').collect::<String>() + ('0'..=':').collect::<String>().as_str() + "!#$%&'()-.~");
pub static MASK_PATH: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(|| ('@'..='z').collect::<String>() + ('0'..=':').collect::<String>().as_str() + "!#$%&'()-.~:\\");
pub static MASK_ASCII: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| (' '..='~').collect::<String>());
/// What `PCBoard` lets through where line noise is not a concern: printable
/// ASCII plus the high half, so an accented name survives.
pub static MASK_MESSAGE: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(|| (' '..='~').collect::<String>() + ('\u{80}'..='\u{FE}').collect::<String>().as_str());
pub static MASK_WEB: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    ('A'..='Z').collect::<String>() + ('a'..='z').collect::<String>().as_str() + ('0'..='9').collect::<String>().as_str() + "@.:!#$%&'*+-/=?^_`{|}~"
});
pub static MASK_PHONE: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| ('0'..='9').collect::<String>() + "/()-+ ");
pub static MASK_NAME: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(|| ('A'..='Z').collect::<String>() + ('a'..='z').collect::<String>().as_str() + " .,-'");
pub static MASK_DATE: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| ('0'..='9').collect::<String>() + "./");

pub const MASK_COMMAND: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+-=[]{}|;':,.<>?/\\\" ";

/// A name a display file carries still looks the way it did on the sysop's DOS drive.
fn dos_path(file: &str) -> PathBuf {
    let mut file = file.replace('\\', "/");
    if file.len() > 2 && file.as_bytes()[1] == b':' && file.as_bytes()[0].is_ascii_alphabetic() {
        file = file[2..].trim_start_matches('/').to_string();
    }
    PathBuf::from(file)
}

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
            // A space separates arguments.
            if ch == ' ' {
                if !arg.is_empty() {
                    arguments.push(arg);
                    arg = String::new();
                }
                continue;
            }
            // '_' acts as a terminator only at a token boundary (start of a token),
            // so underscores inside a file name or path are kept as-is.
            if ch == '_' && arg.is_empty() {
                break;
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
            self.user_screen.buffer.caret.attribute.as_u8(IceMode::Blink).into()
        } else {
            txt_entry.style.to_color()
        };
        self.display_string(&txt_entry.text.replace('~', " "), color, display_flags).await
    }

    pub fn get_display_text(&mut self, message_number: IceText) -> Res<String> {
        let txt_entry = self.display_text.get_display_text(message_number)?;
        Ok(txt_entry.text.replace('~', " "))
    }

    pub async fn display_string(&mut self, txt: &str, color: IcbColor, display_flags: i32) -> Res<()> {
        if display_flags & display_flags::NOTBLANK != 0 && txt.is_empty() {
            return Ok(());
        }

        if display_flags & display_flags::LOGIT != 0 {
            log::info!("{txt}");
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
    pub async fn display_line(&mut self, txt: &str) -> Res<()> {
        if !txt.is_empty() {
            if let Some(call) = PPECall::try_parse_line(txt) {
                let file = self.get_board().await.resolve_file(&dos_path(&call.file));
                let found = match call.call_type {
                    PPECallType::Menu => file.exists() || self.get_board().await.resolve_file(&file.with_extension("ppe")).exists(),
                    _ => file.exists(),
                };
                // A line naming something that is not there is a line, the way PCBoard
                // printed it when runscriptwithparams() came back empty. See FILES.C.
                if !found {
                    self.print(TerminalTarget::Both, txt).await?;
                    return Ok(());
                }
                for sc in call.arguments {
                    self.session.tokens.push_back(sc.clone());
                }
                match call.call_type {
                    PPECallType::PPE => {
                        self.run_ppe(&file, None).await?;
                    }
                    PPECallType::Menu => {
                        let _ = self.display_menu(&file).await?;
                    }
                    PPECallType::File => {
                        let _ = self.display_file(&file).await?;
                    }
                }
                return Ok(());
            }
            // display text
            self.print(TerminalTarget::Both, txt).await?;
        }
        Ok(())
    }

    pub async fn display_menu<P: AsRef<Path>>(&mut self, file_name: &P) -> Res<bool> {
        let resolved_name_ppe = self.get_board().await.resolve_file(&(file_name.as_ref().with_extension("ppe")));
        let path = resolved_name_ppe;
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

        // PCBoard left a file alone that it was already displaying, so a file including itself
        // or a PPE started from it displaying it again ends here. See displayfile() in FILES.C.
        if self.displayed_files.iter().any(|open| open == &resolved_name) {
            return Ok(true);
        }

        if resolved_name.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("icyanim")) {
            let Ok(script) = fs::read_to_string(&resolved_name) else {
                if display_error {
                    self.bell().await?;
                }
                return Ok(false);
            };
            self.displayed_files.push(resolved_name.clone());
            let result = self.display_icy_animation(&resolved_name, script).await;
            self.displayed_files.pop();
            result?;
            return Ok(true);
        }

        if resolved_name.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("icy")) {
            let mut document = match FileFormat::IcyDraw.load(&resolved_name, None) {
                Ok(document) => document,
                Err(_) if !resolved_name.exists() => {
                    if display_error {
                        self.bell().await?;
                    }
                    return Ok(false);
                }
                Err(error) => return Err(error.into()),
            };
            let user_bytes = screen_as_ansi_bytes(&mut document.screen, self.session.term_caps.is_utf8)?;
            let sysop_bytes = screen_as_ansi_bytes(&mut document.screen, false)?;
            self.write_terminal_bytes(TerminalTarget::Both, &user_bytes, &sysop_bytes).await?;
            return Ok(true);
        }

        let Ok(content) = fs::read(&resolved_name) else {
            if display_error {
                self.bell().await?;
                self.set_color(TerminalTarget::Both, IcbColor::dos_light_red()).await?;
                self.print(TerminalTarget::Both, &format!("\r\n({}) is missing!\r\n\r\n", file_name.as_ref().display()))
                    .await?;
            }
            return Ok(false);
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

        self.displayed_files.push(resolved_name);
        let result = self.display_file_content(&converted_content).await;
        self.displayed_files.pop();
        result?;
        Ok(true)
    }

    async fn display_icy_animation(&mut self, path: &Path, script: String) -> Res<()> {
        let parent = path.parent().map(Path::to_path_buf);
        let animator = Animator::run(&parent, script);
        animator.lock().set_is_playing(true);

        self.write_terminal_bytes(TerminalTarget::Both, b"\x1b[?25l", b"\x1b[?25l").await?;
        let result: Res<()> = async {
            while animator.lock().is_playing() && !self.session.disp_options.abort_printout && !self.session.request_logoff {
                let frame = {
                    let mut animator = animator.lock();
                    animator.get_cur_frame_buffer_mut().map(|(screen, _, delay)| {
                        (
                            screen_as_ansi_bytes(screen.as_mut(), self.session.term_caps.is_utf8),
                            screen_as_ansi_bytes(screen.as_mut(), false),
                            *delay,
                        )
                    })
                };

                let Some((user_bytes, sysop_bytes, delay)) = frame else {
                    if !animator.lock().is_thread_running() {
                        break;
                    }
                    sleep(Duration::from_millis(10)).await;
                    continue;
                };

                self.write_terminal_bytes(TerminalTarget::Both, &user_bytes?, &sysop_bytes?).await?;
                sleep(Duration::from_millis(u64::from(delay))).await;
                while !animator.lock().next_frame() {
                    sleep(Duration::from_millis(10)).await;
                }
            }

            let animator = animator.lock();
            if !animator.error.is_empty() {
                return Err(format!("Error playing {}: {}", path.display(), animator.error).into());
            }
            Ok(())
        }
        .await;
        let restore_result = self.write_terminal_bytes(TerminalTarget::Both, b"\x1b[?25h", b"\x1b[?25h").await;
        result?;
        restore_result
    }

    async fn display_file_content(&mut self, converted_content: &str) -> Res<()> {
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
        Ok(())
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
        mut len: i32,
        valid_mask: &str,
        help: &str,
        default_answer: Option<String>,
        mut display_flags: i32,
    ) -> Res<String> {
        if self.session.request_logoff {
            return Ok(String::new());
        }
        self.session.default_answer.clone_from(&default_answer);
        self.session.disp_options.no_change();

        // we've data from a PPE here, so take that input and return it.
        // ignoring all other settings.
        if let Some(front) = self.char_buffer.front()
            && front.source.is_hidden()
        {
            let mut result = String::new();
            while let Some(key) = self.char_buffer.pop_front() {
                if key.ch == '\n' || key.ch == '\r' {
                    break;
                }
                result.push(key.ch);
            }
            if result.is_empty() {
                return Ok(String::new());
            }
            // Return the whole stuffed line so the caller can tokenize it
            // (the same way typed input is handled). Returning only the
            // first token here corrupted the token order when the caller
            // re-pushed the returned value.
            let result = if display_flags & display_flags::UPCASE != 0 {
                result.to_uppercase()
            } else {
                result
            };
            self.session.last_answer = Some(result.clone());
            return Ok(result);
        }
        if let Some(token) = self.session.tokens.pop_front() {
            self.session.last_answer = Some(token.clone());
            return Ok(token);
        }

        let mut prompt = prompt;

        let display_question = if prompt.ends_with(TXT_STOPCHAR) {
            display_flags &= !(display_flags::FIELDLEN | display_flags::GUIDE);
            prompt.pop();
            false
        } else {
            true
        };
        self.check_time_left().await;

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
        if let Some(front) = self.char_buffer.front()
            && front.source.is_hidden()
        {
            let mut result = String::new();
            while let Some(key) = self.char_buffer.pop_front() {
                if key.ch == '\n' || key.ch == '\r' {
                    break;
                }
                result.push(key.ch);
            }
            log::info!("PPE stuffed input: {result}");
            let result = if display_flags & display_flags::UPCASE != 0 {
                result.to_uppercase()
            } else {
                result
            };
            self.session.last_answer = Some(result.clone());
            return Ok(result);
        }

        let mut show_field_len = display_flags & display_flags::FIELDLEN != 0 && self.use_ansi();
        if display_question {
            self.print(TerminalTarget::Both, "?").await?;
            if show_field_len {
                let x = self.session.cursor_pos.x;
                let before_wrap_len = 79 - 3 - x;
                if before_wrap_len < len && before_wrap_len <= len / 2 {
                    if x < 70 {
                        show_field_len = false;
                    } else {
                        self.new_line().await?;
                    }
                }
            }
        }

        let mut default_answer = default_answer;
        if show_field_len {
            self.print(TerminalTarget::Both, " (").await?;
            let x = self.session.cursor_pos.x;
            if x + 1 + len > 79 {
                len = 79 - 1 - x;
                if len < 1 {
                    return Ok(String::new());
                }
                // A default longer than the field it sits in would overwrite the
                // closing delimiter, so it is cut to what the field can hold.
                if let Some(default) = &mut default_answer
                    && default.chars().count() as i32 > len
                {
                    *default = default.chars().take(len as usize).collect();
                }
            }
            self.forward(len).await?;
            self.print(TerminalTarget::Both, ")").await?;
            self.backward(len + 1).await?;
            self.reset_color(TerminalTarget::Both).await?;
            if let Some(default) = &default_answer {
                self.print(TerminalTarget::Both, default).await?;
                self.backward(default.chars().count() as i32).await?;
            }
        } else if display_question {
            self.print(TerminalTarget::Both, " ").await?;
        }
        if self.use_graphics() {
            self.reset_color(TerminalTarget::Both).await?;
        }

        // PCBoard only opens the high half when the sysop has turned the line noise
        // filter off, so a noisy modem cannot type its way into a user record.
        let high_ascii = display_flags & display_flags::HIGHASCII != 0 && self.get_board().await.config.switches.disable_high_ascii_filter;

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
                if !help.is_empty()
                    && let Some(cmd) = self.try_find_command(&output, true).await
                    && !cmd.actions.is_empty()
                    && cmd.actions[0].command_type == CommandType::Help
                {
                    self.show_help(help).await?;
                    return self.input_string(color, prompt, len, valid_mask, help, default_answer, display_flags).await;
                }

                if display_flags & display_flags::ERASELINE != 0 {
                    self.clear_line(TerminalTarget::Both).await?;
                }
                break;
            }
            if key_char.ch == '\x08' && !output.is_empty() {
                output.pop();
                if !key_char.source.is_hidden() {
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
                    || high_ascii && key_char.ch >= '\u{80}'
                    || (display_flags & display_flags::STACKED) != 0 && " ;".contains(key_char.ch))
            {
                output.push(key_char.ch);
                if !key_char.source.is_hidden() {
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

        if output.is_empty()
            && let Some(default) = default_answer
        {
            self.session.last_answer = Some(default.clone());
            return Ok(default);
        }
        self.session.last_answer = Some(output.clone());
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

    pub async fn send_message(&mut self, conf: i32, area: i32, msg: JamMessage, text: IceText) -> Res<()> {
        let msg_base = if conf < 0 {
            let user_name = msg.to().unwrap().to_string();
            self.get_email_msgbase(&user_name).await
        } else {
            let msg_base = self.get_board().await.conferences[conf as usize].areas.as_ref().unwrap()[area as usize]
                .path
                .clone();
            if msg_base.with_extension("jhr").exists() {
                JamMessageBase::open(msg_base)
            } else {
                JamMessageBase::create(msg_base)
            }
            .map_err(Into::into)
        };

        match msg_base {
            Ok(mut msg_base) => {
                let number = msg_base.write_message(&msg)?;
                msg_base.write_jhr_header()?;

                if let Some(user) = &mut self.session.current_user {
                    user.stats.messages_left += 1;
                }
                self.get_board().await.statistics.add_message();
                self.get_board().await.save_statistics()?;

                self.display_text(text, display_flags::DEFAULT).await?;
                self.println(TerminalTarget::Both, &number.to_string()).await?;
                self.new_line().await?;
            }
            Err(err) => {
                log::error!("while opening message base: {err}");
                self.display_text(IceText::MessageBaseError, display_flags::NEWLINE).await?;
            }
        }

        Ok(())
    }

    /// Where the private mail everyone shares lives.
    pub async fn email_msgbase_path(&mut self) -> std::path::PathBuf {
        let name = self.get_board().await.config.paths.email_msgbase.clone();
        let msg_base = self.resolve_path(&name);
        if msg_base.is_dir() { msg_base.join("email") } else { msg_base }
    }

    pub async fn get_email_msgbase(&mut self, user_name: &str) -> Res<JamMessageBase> {
        let msg_base = self.email_msgbase_path().await;
        Ok(if msg_base.with_extension("jhr").exists() {
            JamMessageBase::open(msg_base)?
        } else {
            log::info!("Creating new email message base for user {user_name}");
            if let Some(parent) = msg_base.parent() {
                std::fs::create_dir_all(parent)?;
            }
            JamMessageBase::create(msg_base)?
        })
    }

    /// Appends one line per transferred file to the transfer log.
    pub async fn log_transfer(&mut self, upload: bool, file_names: &[String], protocol: &str, errors: usize, cps: usize) -> Res<()> {
        let (log_file, exclude_locals) = {
            let board = self.get_board().await;
            (board.config.paths.transfer_log.clone(), board.config.switches.exclude_local_calls_stats)
        };
        if log_file.as_os_str().is_empty() || file_names.is_empty() || (self.session.is_local && exclude_locals) {
            return Ok(());
        }
        let log_file = self.resolve_path(&log_file);
        let user_name = self.session.user_name.clone();
        let now = Local::now();
        let mut text = String::new();
        for file_name in file_names {
            text.push_str(&transfer_log_line(upload, &user_name, now, file_name, protocol, errors, cps));
        }
        let mut file = OpenOptions::new().create(true).append(true).open(&log_file)?;
        file.write_all(text.as_bytes())?;
        Ok(())
    }
}

fn screen_as_ansi_bytes(screen: &mut dyn Screen, utf8: bool) -> Res<Vec<u8>> {
    let level = if utf8 {
        AnsiCompatibilityLevel::Utf8Terminal
    } else {
        AnsiCompatibilityLevel::Vt100
    };
    let mut options = SaveOptions::ansi(level);
    options.preprocess.optimize_colors = false;
    options.sauce = None;
    if let FormatOptions::Ansi(ansi_options) = &mut options.format {
        ansi_options.line_break = LineBreakBehavior::GotoXY;
        ansi_options.line_length = LineLength::Minimum(80);
    }
    Ok(screen.to_bytes("ans", &options)?)
}

fn transfer_log_line(upload: bool, user_name: &str, time: DateTime<Local>, file_name: &str, protocol: &str, errors: usize, cps: usize) -> String {
    format!(
        "({}),{},{},{},{},{},{},{}\r\n",
        if upload { 'U' } else { 'D' },
        user_name,
        time.format("%m-%d-%Y"),
        time.format("%H:%M"),
        file_name,
        protocol,
        errors,
        cps
    )
}

pub fn transfer_cps(bytes: u64, started: Instant) -> usize {
    let elapsed = started.elapsed().as_secs_f64();
    if elapsed <= 0.0 { 0 } else { (bytes as f64 / elapsed) as usize }
}

const MASK_PASSWORD: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()-_=+[]{};:'\",.<>/?\\|~`";

pub mod pwd_flags {
    pub const SHOW_WRONG_PWD_MSG: u32 = 0x00001;
    /// Don't show any text
    pub const PLAIN: u32 = 0x00002;
}

#[cfg(test)]
mod tests {
    use super::{PPECall, PPECallType, transfer_log_line};
    use chrono::TimeZone;

    #[test]
    fn a_transfer_log_line_names_direction_user_file_and_speed() {
        let time = chrono::Local.with_ymd_and_hms(2026, 8, 14, 21, 5, 0).unwrap();
        assert_eq!(
            transfer_log_line(false, "JOHN DOE", time, "GAME.ZIP", "Z", 2, 1150),
            "(D),JOHN DOE,08-14-2026,21:05,GAME.ZIP,Z,2,1150\r\n"
        );
        assert!(transfer_log_line(true, "JOHN DOE", time, "GAME.ZIP", "Z", 0, 0).starts_with("(U),"));
    }

    #[test]
    fn test_parse_simple_ppe_call() {
        let call = PPECall::try_parse_line("!FOO.PPE").unwrap();
        assert_eq!(call.call_type, PPECallType::PPE);
        assert_eq!(call.file, "FOO.PPE");
        assert!(call.arguments.is_empty());
    }

    #[test]
    fn test_parse_arguments_separated_by_space() {
        let call = PPECall::try_parse_line("!FOO.PPE arg1 arg2").unwrap();
        assert_eq!(call.file, "FOO.PPE");
        assert_eq!(call.arguments, vec!["arg1".to_string(), "arg2".to_string()]);
    }

    #[test]
    fn test_underscore_in_file_name_is_kept() {
        let call = PPECall::try_parse_line("!foo_bar.ppe arg1").unwrap();
        assert_eq!(call.file, "foo_bar.ppe");
        assert_eq!(call.arguments, vec!["arg1".to_string()]);
    }

    #[test]
    fn test_underscore_in_absolute_path_is_kept() {
        let call = PPECall::try_parse_line("!/home/my_user/my_ppe.ppe").unwrap();
        assert_eq!(call.file, "/home/my_user/my_ppe.ppe");
        assert!(call.arguments.is_empty());
    }

    #[test]
    fn test_underscore_at_token_boundary_terminates() {
        let call = PPECall::try_parse_line("!SUBSCR.PPE _some trailing prompt").unwrap();
        assert_eq!(call.file, "SUBSCR.PPE");
        assert!(call.arguments.is_empty());
    }

    #[test]
    fn test_non_call_line_returns_none() {
        assert!(PPECall::try_parse_line("just some text").is_none());
        assert!(PPECall::try_parse_line("").is_none());
    }
}
