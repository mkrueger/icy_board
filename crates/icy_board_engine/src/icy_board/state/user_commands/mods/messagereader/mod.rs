use crate::Res;
use crate::icy_board::commands::CommandType;
use crate::icy_board::state::functions::{MASK_ASCII, MASK_COMMAND, MASK_NUM};
use crate::{
    icy_board::{
        icb_text::{IceText, TextEntry},
        state::{IcyBoardState, functions::display_flags},
    },
    vm::TerminalTarget,
};
use jamjam::jam::{JamMessageBase, msg_header::JamMessageHeader};

pub mod message_filter;
pub mod read_actions;
pub mod read_command;

use message_filter::MessageFilter;
use read_actions::AfterAction;
use read_command::{MsgFunc, ParseContext, ReadCommand, ReadLoop, user_search};

/// The next message number in the direction the range runs, if the range has one.
fn next_in_range(number: u32, first: u32, last: u32) -> Option<u32> {
    if last < first {
        if number <= last { None } else { Some(number - 1) }
    } else if number >= last {
        None
    } else {
        Some(number + 1)
    }
}

/// What to do with the command line once its missing pieces have been asked for.
enum Resolution {
    Run,
    Stop,
    Reprompt,
}

pub struct MessageViewer {
    date_num: TextEntry,
    to_line: TextEntry,
    _reply_line: TextEntry,
    from_line: TextEntry,
    _not_avail: TextEntry,
    _not_read: TextEntry,
    _rcv_only: TextEntry,
    _grp_pwd: TextEntry,
    _snd_pwd: TextEntry,
    _public: TextEntry,
    refer_num: TextEntry,
    _read: TextEntry,
    subj_line: TextEntry,
    _status: TextEntry,
    _file: TextEntry,
    _list: TextEntry,
    none: TextEntry,
    confarea: TextEntry,
    separator: TextEntry,
    _all_name: TextEntry,
    _ret_rec_name: TextEntry,
    _comment: TextEntry,
    _echo: TextEntry,
    _all_conf_indicator: TextEntry,
    _read_only_indicator: TextEntry,
    left: usize,
    right: usize,
}

impl MessageViewer {
    pub fn load(dt: &crate::icy_board::icb_text::IcbTextFile) -> Res<Self> {
        let date_num = dt.get_display_text(IceText::MessageDateNumber)?;
        let to_line = dt.get_display_text(IceText::MessageToLine)?;
        let reply_line = dt.get_display_text(IceText::MessageReplies)?;
        let from_line = dt.get_display_text(IceText::MessageFrom)?;
        let not_avail = dt.get_display_text(IceText::MessageNA)?;
        let not_read = dt.get_display_text(IceText::MessageNotRead)?;
        let rcv_only = dt.get_display_text(IceText::MessageReceiverOnly)?;
        let grp_pwd = dt.get_display_text(IceText::MessageGroupPassword)?;
        let snd_pwd = dt.get_display_text(IceText::MessageSenderPassword)?;
        let public = dt.get_display_text(IceText::MessagePublic)?;
        let refer_num = dt.get_display_text(IceText::MessageReferNumber)?;
        let read = dt.get_display_text(IceText::MessageRead)?;
        let subj_line = dt.get_display_text(IceText::MessageSubjectLine)?;
        let status = dt.get_display_text(IceText::MessageStatus)?;
        let file = dt.get_display_text(IceText::MessageFile)?;
        let list = dt.get_display_text(IceText::MessageList)?;
        let none = dt.get_display_text(IceText::None)?;
        let confarea = dt.get_display_text(IceText::MessagesConfArea)?;
        let separator = dt.get_display_text(IceText::Separator)?;
        let all_name = dt.get_display_text(IceText::AllName)?;
        let ret_rec_name = dt.get_display_text(IceText::FromReturnReceipt)?;
        let comment = dt.get_display_text(IceText::Comment)?;
        let echo = dt.get_display_text(IceText::Echo)?;
        let all_conf_indicator = dt.get_display_text(IceText::AllConfIndicator)?;
        let read_only_indicator = dt.get_display_text(IceText::ReadonlyIndicator)?;

        let mut left = date_num.text.find(':').unwrap_or(0);
        left = left.max(to_line.text.find(':').unwrap_or(0));
        left = left.max(from_line.text.find(':').unwrap_or(0));
        left = left.max(subj_line.text.find(':').unwrap_or(0));
        left = left.max(confarea.text.find(':').unwrap_or(0));

        let right = confarea.text.rfind(':').unwrap_or(79);

        Ok(Self {
            date_num,
            to_line,
            _reply_line: reply_line,
            from_line,
            _not_avail: not_avail,
            _not_read: not_read,
            _rcv_only: rcv_only,
            _grp_pwd: grp_pwd,
            _snd_pwd: snd_pwd,
            _public: public,
            refer_num,
            _read: read,
            subj_line,
            _status: status,
            _file: file,
            _list: list,
            none,
            confarea,
            separator,
            _all_name: all_name,
            _ret_rec_name: ret_rec_name,
            _comment: comment,
            _echo: echo,
            _all_conf_indicator: all_conf_indicator,
            _read_only_indicator: read_only_indicator,
            left,
            right,
        })
    }

    pub fn format_hdr_text(&self, txt: &str, left: &str, right: &str) -> String {
        let mut result = txt.to_string();

        while result.len() < 80 {
            result.push(' ');
        }

        let xleft = self.left + 2;
        let xright = self.right + 2;

        result.replace_range(xleft..xleft + left.len(), left);
        result.replace_range(xright..xright + right.len(), right);

        result
    }

    pub async fn display_header(&self, state: &mut IcyBoardState, msg_base: &JamMessageBase, header: &JamMessageHeader) -> Res<()> {
        state.clear_screen(TerminalTarget::Both).await?;

        let c1 = state.get_board().await.config.color_configuration.msg_hdr_date.clone();
        state.set_color(TerminalTarget::Both, c1).await?;
        let time = if let Some(dt) = chrono::DateTime::from_timestamp(header.date_written as i64, 0) {
            dt.to_string()
        } else {
            String::new()
        };
        let msg_counter = format!(
            "{} {} {} ({} {})",
            header.message_number,
            self.separator.text,
            msg_base.highest_message_number(),
            self.refer_num.text,
            if header.reply_to == 0 {
                self.none.text.clone()
            } else {
                header.reply_to.to_string()
            }
        );
        let txt = self.format_hdr_text(&self.date_num.text, &time, &msg_counter);
        state.print(TerminalTarget::Both, &txt).await?;

        let c1 = state.get_board().await.config.color_configuration.msg_hdr_to.clone();
        state.set_color(TerminalTarget::Both, c1).await?;
        let txt = self.format_hdr_text(&self.to_line.text, &header.to().unwrap().to_string(), "");
        if state.session.search_pattern.is_some() {
            state.print_found_text(TerminalTarget::Both, &txt).await?;
        } else {
            state.print(TerminalTarget::Both, &txt).await?;
        }

        let c1 = state.get_board().await.config.color_configuration.msg_hdr_from.clone();
        state.set_color(TerminalTarget::Both, c1).await?;
        let txt = self.format_hdr_text(&self.from_line.text, &header.from().unwrap().to_string(), "");
        if state.session.search_pattern.is_some() {
            state.print_found_text(TerminalTarget::Both, &txt).await?;
        } else {
            state.print(TerminalTarget::Both, &txt).await?;
        }

        let c1 = state.get_board().await.config.color_configuration.msg_hdr_subj.clone();
        state.set_color(TerminalTarget::Both, c1).await?;
        let txt = self.format_hdr_text(&self.subj_line.text, &header.subject().unwrap().to_string(), "");
        if state.session.search_pattern.is_some() {
            state.print_found_text(TerminalTarget::Both, &txt).await?;
        } else {
            state.print(TerminalTarget::Both, &txt).await?;
        }

        let c1 = state.get_board().await.config.color_configuration.msg_hdr_read.clone();
        state.set_color(TerminalTarget::Both, c1).await?;
        /*        let txt = self.format_hdr_text(&self.read.text, "", "");
                state.print(TerminalTarget::Both, &txt)?;
        */

        let area = state.session.current_message_area;
        let txt = self.format_hdr_text(
            &self.confarea.text,
            &state.session.current_conference.name,
            &state.session.current_conference.areas.as_ref().unwrap()[area].name,
        );
        state.print(TerminalTarget::Both, &txt).await?;
        state.reset_color(TerminalTarget::Both).await?;
        if state.session.disp_options.count_lines {
            state.session.disp_options.num_lines_printed += 5;
        }
        Ok(())
    }

    async fn display_body(&self, state: &mut IcyBoardState, text: &str) -> Res<()> {
        // PCBoard printed a message a line at a time, and a line is what a MORE prompt counts.
        for (i, line) in text.split('\n').enumerate() {
            if i > 0 {
                state.new_line().await?;
                if state.session.disp_options.abort_printout {
                    break;
                }
            }
            let line = line.strip_suffix('\r').unwrap_or(line);
            if state.session.search_pattern.is_some() {
                state.print_found_text(TerminalTarget::Both, line).await?;
            } else {
                state.print(TerminalTarget::Both, line).await?;
            }
        }
        Ok(())
    }
}

impl IcyBoardState {
    pub async fn read_msgs_from_base(&mut self, mut message_base: JamMessageBase, only_personal: bool) -> Res<()> {
        let viewer = MessageViewer::load(&self.display_text)?;
        let mut low_number = message_base.lowest_message_number();
        let mut high_number = message_base.highest_message_number();
        let mut messages = Vec::new();
        if only_personal {
            for msg in message_base.messages().flatten() {
                if let Some(to) = msg.to() {
                    let mut to = to.clone();
                    to.make_ascii_uppercase();
                    if to == self.session.alias_name.to_ascii_uppercase() || to == self.session.user_name.to_ascii_uppercase() {
                        messages.push(msg.message_number);
                    }
                }
            }
            if !messages.is_empty() {
                low_number = messages.first().unwrap_or(&0).to_owned();
                high_number = messages.last().unwrap_or(&0).to_owned();
            } else {
                low_number = 0;
                high_number = 0;
            }
        }
        while !self.session.disp_options.abort_printout {
            let prompt = if self.session.expert_mode() {
                IceText::MessageReadCommandExpert
            } else {
                IceText::MessageReadCommand
            };
            self.session.op_text = format!("{}-{}", low_number, high_number);

            if self.session.tokens.is_empty() {
                let text = self
                    .input_field(
                        prompt,
                        40,
                        MASK_COMMAND,
                        &CommandType::ReadMessages.get_help(),
                        None,
                        display_flags::UPCASE | display_flags::NEWLINE | display_flags::LFBEFORE,
                    )
                    .await?;
                if text.is_empty() {
                    break;
                }
                self.session.push_tokens(&text);
            }

            let tokens: Vec<String> = self.session.tokens.drain(..).collect();
            let ctx = self.read_parse_context(0).await;
            let mut cmd = read_command::parse(&tokens, ReadLoop::Outside, &ctx);
            if cmd.set_last_read {
                self.set_last_message_read(&cmd, &mut message_base).await?;
            }
            match self.resolve_read_command(&mut cmd).await? {
                Resolution::Stop => break,
                Resolution::Reprompt => continue,
                Resolution::Run => {}
            }
            read_command::finalize(&mut cmd);

            if cmd.func == MsgFunc::Goodbye {
                self.goodbye().await?;
                break;
            }
            if cmd.func == MsgFunc::Stop {
                break;
            }

            // S picks up where the last-read pointer left off rather than at the
            // bottom of the base.
            if cmd.since {
                let last_read = self.last_read_pointer(&mut message_base)?;
                if last_read >= high_number {
                    self.display_text(IceText::NoMailFound, display_flags::NEWLINE | display_flags::LFAFTER).await?;
                    continue;
                }
                if let Some(range) = cmd.numbers.first_mut() {
                    range.first = last_read as i64 + 1;
                }
            }

            if only_personal {
                cmd.any_msgs = false;
                cmd.your_msgs = true;
            }
            if cmd.do_text_search && !cmd.search_text.is_empty() {
                self.search_init(cmd.search_text.clone(), false);
            } else {
                self.stop_search();
            }
            let filter = MessageFilter::new(&cmd, &self.session);

            for range in cmd.numbers.clone() {
                let (first, last) = self.clamp_range(range, low_number, high_number);
                if first == 0 {
                    continue;
                }
                self.read_message_number(&mut message_base, &viewer, first, last, cmd.keep_going, &filter)
                    .await?;
            }
            self.stop_search();
        }
        Ok(())
    }

    /// Where this message base left the current user's last-read pointer.
    fn last_read_pointer(&mut self, message_base: &mut JamMessageBase) -> Res<u32> {
        unsafe {
            let crc = JamMessageBase::crc(&bstr::BString::new(self.session.user_name.as_mut_vec().clone()));
            let last_read = message_base
                .find_last_read(crc, self.session.cur_user_id as u32)?
                .unwrap_or(message_base.create_last_read(crc, self.session.cur_user_id as u32)?);
            Ok(last_read.last_read_msg)
        }
    }

    pub(crate) async fn read_parse_context(&mut self, reply_to: i64) -> ParseContext {
        ParseContext {
            cur_msg_number: self.session.current_messagenumber as i64,
            memorized: self
                .session
                .memorized_msg
                .filter(|(area, _)| *area == self.session.current_message_area)
                .map(|(_, num)| num as i64),
            reply_to,
            may_join: self.session.user_command_level.cmd_j.session_can_access(&self.session),
            may_quick_scan: self.session.user_command_level.cmd_q.session_can_access(&self.session),
            may_read_only: false,
            alias_support: self.session.current_conference.allow_aliases,
            qwk_support: true,
            reply_command: false,
            quick_scan: false,
            num_conferences: self.get_board().await.conferences.len() as u16,
        }
    }

    /// Turn a parsed range into the message numbers this base actually holds.
    fn clamp_range(&self, range: read_command::MsgRange, low_number: u32, high_number: u32) -> (u32, u32) {
        let high = high_number.max(low_number);
        let clamp = |value: i64| -> u32 { value.clamp(low_number as i64, high as i64) as u32 };
        (clamp(range.first), clamp(range.last))
    }

    /// The questions PCBoard asks once it has read the whole line
    /// and finds a search with nothing to search for.
    async fn resolve_read_command(&mut self, cmd: &mut ReadCommand) -> Res<Resolution> {
        if cmd.not_memorized {
            self.display_text(IceText::NotMemorized, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
        }

        // PCBoard only asked where an (A)ll scan had stopped before; without one there is
        // nothing to resume. See getallresumestatus() in MSGREAD.C.
        if cmd.ask_resume_all && self.session.start_conf != 0 {
            let answer = self
                .input_field(
                    IceText::ResumeAll,
                    1,
                    "",
                    "",
                    Some(self.session.yes_char.to_uppercase().to_string()),
                    display_flags::NEWLINE
                        | display_flags::LFBEFORE
                        | display_flags::FIELDLEN
                        | display_flags::GUIDE
                        | display_flags::UPCASE
                        | display_flags::YESNO,
                )
                .await?;
            if !answer.is_empty() && !answer.eq_ignore_ascii_case(&self.session.yes_char.to_string()) {
                self.session.start_conf = 0;
            }
        }

        if cmd.do_text_search && cmd.search_text.is_empty() {
            let text = self
                .input_field(
                    IceText::TextToScanFor,
                    40,
                    &MASK_ASCII,
                    "hlpsrch",
                    None,
                    display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE,
                )
                .await?;
            if text.is_empty() {
                return Ok(Resolution::Stop);
            }
            cmd.search_text = text;
        }

        if cmd.do_user_search {
            for (flag, text) in [
                (user_search::TO, IceText::UserSearchToName),
                (user_search::FROM, IceText::UserSearchFromName),
                (user_search::USER, IceText::UserSearchName),
            ] {
                if cmd.user_search & flag == 0 {
                    continue;
                }
                let target = if flag == user_search::FROM {
                    &mut cmd.user_name_from
                } else {
                    &mut cmd.user_name_to
                };
                if !target.is_empty() {
                    continue;
                }
                let answer = self
                    .input_field(
                        text,
                        25,
                        &MASK_ASCII,
                        "",
                        None,
                        display_flags::NEWLINE | display_flags::UPCASE | display_flags::FIELDLEN | display_flags::LFBEFORE,
                    )
                    .await?;
                if answer.is_empty() {
                    return Ok(Resolution::Stop);
                }
                if flag == user_search::FROM {
                    cmd.user_name_from = answer;
                } else {
                    cmd.user_name_to = answer;
                }
            }
        }

        if cmd.numbers.is_empty() {
            if cmd.new_msgs && cmd.new_date.is_none() {
                let date = self
                    .input_field(
                        IceText::DateToSearch,
                        6,
                        &MASK_NUM,
                        "",
                        Some(self.session.login_date.format("%m%d%y").to_string()),
                        display_flags::GUIDE | display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                    )
                    .await?;
                if date.is_empty() {
                    return Ok(Resolution::Stop);
                }
                cmd.new_date = Some(date);
            }

            if !cmd.since && cmd.new_date.is_none() && (!cmd.search_text.is_empty() || cmd.do_user_search) {
                let answer = self
                    .input_field(
                        IceText::MessageSearchFrom,
                        14,
                        MASK_COMMAND,
                        "",
                        None,
                        display_flags::UPCASE | display_flags::NEWLINE,
                    )
                    .await?;
                if answer.is_empty() {
                    return Ok(Resolution::Stop);
                }
                // a bare number here means "from there forward"
                let answer = if answer.chars().next().is_some_and(|c| c.is_ascii_digit()) && !answer.contains(['-', '+']) {
                    format!("{answer}+")
                } else {
                    answer
                };
                let tokens: Vec<String> = answer.split_whitespace().map(str::to_string).collect();
                let ctx = self.read_parse_context(0).await;
                let ranges = read_command::parse(&tokens, ReadLoop::Outside, &ctx);
                cmd.numbers = ranges.numbers;
                cmd.keep_going |= ranges.keep_going;
            }
        }

        if cmd.func == MsgFunc::None && cmd.numbers.is_empty() && !cmd.all_conf && !cmd.since && !cmd.new_msgs {
            if !cmd.valid_cmd {
                self.display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
            }
            return Ok(Resolution::Reprompt);
        }

        Ok(Resolution::Run)
    }

    pub async fn read_message_number(
        &mut self,
        message_base: &mut JamMessageBase,
        viewer: &MessageViewer,
        mut first: u32,
        mut last: u32,
        mut keep_going: bool,
        filter: &MessageFilter,
    ) -> Res<()> {
        let mut number = first;
        if number == 0 {
            return Ok(());
        }
        self.session.current_messagenumber = number;
        self.session.low_msg_num = message_base.lowest_message_number();
        self.session.high_msg_num = message_base.highest_message_number();

        // PCBoard's LastReadUpdate decides whether reading drags the pointer along.
        let update_last_read = self.get_board().await.config.message.update_last_read_pointer;
        unsafe {
            let crc = JamMessageBase::crc(&bstr::BString::new(self.session.user_name.as_mut_vec().clone()));
            let mut opt = message_base
                .find_last_read(crc, self.session.cur_user_id as u32)?
                .unwrap_or(message_base.create_last_read(crc, self.session.cur_user_id as u32)?);
            self.session.last_msg_read = opt.last_read_msg;
            self.session.highest_msg_read = opt.high_read_msg;

            if update_last_read {
                opt.last_read_msg = number;
                opt.high_read_msg = opt.high_read_msg.max(number);
                message_base.write_last_read(&opt)?;
            }
        }
        let last_read = self.session.last_msg_read;
        let mut reply_to = 0;
        let mut display_msg = true;
        let mut shown = 0;
        loop {
            if display_msg {
                display_msg = false;
                // A message the command did not ask for is skipped without a prompt.
                let found = loop {
                    match message_base.read_header(number) {
                        Ok(header) => {
                            let text = message_base.read_message_text(&header)?.to_string();
                            if filter.matches(&header, &text, last_read) {
                                break Some((header, text));
                            }
                        }
                        Err(err) => {
                            log::error!("Error reading message header: {}", err);
                        }
                    }
                    match next_in_range(number, first, last) {
                        Some(next) => number = next,
                        None => break None,
                    }
                };
                let Some((header, text)) = found else {
                    if shown == 0 {
                        self.display_text(IceText::NoMailFound, display_flags::NEWLINE | display_flags::LFAFTER).await?;
                    }
                    break;
                };
                shown += 1;
                self.session.current_messagenumber = number;
                reply_to = header.reply_to as i64;
                viewer.display_header(self, message_base, &header).await?;
                if header.needs_password() {
                    if self
                        .check_password(IceText::PasswordToReadMessage, 0, |pwd| header.is_password_valid(pwd))
                        .await?
                    {
                        viewer.display_body(self, &text).await?;
                    }
                } else {
                    viewer.display_body(self, &text).await?;
                }
                self.new_line().await?;
            }

            let prompt = if self.session.expert_mode() {
                IceText::EndOfMessageExpertmode
            } else {
                IceText::EndOfMessage
            };
            let text = self
                .input_field(
                    prompt,
                    40,
                    MASK_COMMAND,
                    "hlpendr",
                    None,
                    display_flags::UPCASE | display_flags::LFBEFORE | display_flags::NEWLINE,
                )
                .await?;

            if text.is_empty() {
                if !keep_going {
                    break;
                }
            } else {
                self.session.push_tokens(&text);
                let tokens: Vec<String> = self.session.tokens.drain(..).collect();
                let ctx = self.read_parse_context(reply_to).await;
                let mut cmd = read_command::parse(&tokens, ReadLoop::Inside, &ctx);
                if cmd.set_last_read {
                    self.set_last_message_read(&cmd, message_base).await?;
                }
                match self.resolve_read_command(&mut cmd).await? {
                    Resolution::Stop => break,
                    Resolution::Reprompt => continue,
                    Resolution::Run => {}
                }

                if cmd.memorize {
                    self.session.memorized_msg = Some((self.session.current_message_area, number));
                    self.display_text(IceText::MessageNumberMemorized, display_flags::LFBEFORE).await?;
                }

                match cmd.func {
                    MsgFunc::Stop => break,
                    MsgFunc::Goodbye => {
                        self.goodbye().await?;
                        break;
                    }
                    MsgFunc::Redisplay => {
                        display_msg = true;
                        continue;
                    }
                    MsgFunc::EditHeader => {
                        self.edit_header(message_base, number).await?;
                        display_msg = true;
                        continue;
                    }
                    _ => {}
                }

                match self.run_read_action(&cmd, message_base, number).await? {
                    AfterAction::Prompt => continue,
                    AfterAction::Redisplay => {
                        display_msg = true;
                        continue;
                    }
                    AfterAction::Next => {
                        keep_going = true;
                    }
                    AfterAction::Quit => break,
                    AfterAction::NotHandled => {
                        // A command the reader parses but cannot run must say so;
                        // silence reads as a broken board rather than a missing one.
                        if cmd.func != MsgFunc::None {
                            self.display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::LFBEFORE)
                                .await?;
                            display_msg = true;
                            continue;
                        }
                    }
                }

                keep_going |= cmd.keep_going;
                if let Some(range) = cmd.numbers.first() {
                    let (lo, hi) = self.clamp_range(*range, self.session.low_msg_num, self.session.high_msg_num);
                    if lo == 0 {
                        break;
                    }
                    number = lo;
                    first = lo;
                    last = hi;
                    display_msg = true;
                    continue;
                }
            }
            if keep_going {
                match next_in_range(number, first, last) {
                    Some(next) => number = next,
                    None => break,
                }
                display_msg = true;
            }
        }

        Ok(())
    }
}
