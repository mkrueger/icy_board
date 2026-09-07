use self::message_security::{may_read_header, requires_read_password};
use crate::Res;
use crate::icy_board::commands::CommandType;
use crate::icy_board::state::functions::{MASK_ASCII, MASK_COMMAND, MASK_NUM};
use crate::icy_board::user_base::ConferenceFlags;
use crate::{
    icy_board::{
        icb_text::{IceText, TextEntry},
        state::{IcyBoardState, functions::display_flags},
    },
    vm::TerminalTarget,
};
use bstr::BString;
use jamjam::jam::{
    JamMessage, JamMessageBase, attributes,
    msg_header::{JamMessageHeader, MessageSubfield, SubfieldType},
    raw,
};
use std::collections::VecDeque;

mod capture;
pub mod message_filter;
pub(crate) mod message_security;
pub mod read_actions;
pub mod read_command;

use message_filter::MessageFilter;
use read_actions::AfterAction;
use read_command::{AliasToggle, HeaderLength, MsgFunc, MsgRange, ParseContext, ReadCommand, ReadLoop, user_search};

/// O and header length survive subsequent commands in the same reader invocation.
struct ReaderOptions {
    update_pointers: bool,
    update_status: bool,
    header: HeaderLength,
    capture: Option<capture::ReaderCapture>,
}

impl Default for ReaderOptions {
    fn default() -> Self {
        Self {
            update_pointers: true,
            update_status: true,
            header: HeaderLength::Long,
            capture: None,
        }
    }
}

enum ReaderExit {
    Done,
    Stop,
    Skip,
    Leave,
    Command(ReadCommand),
}

fn sync_receipt_files(base: &JamMessageBase) -> jamjam::Result<()> {
    for extension in ["jdt", "jhr", "jdx"] {
        std::fs::OpenOptions::new().write(true).open(base.path().with_extension(extension))?.sync_all()?;
    }
    Ok(())
}

/// Called only after successful body output. Re-read under the writer lock so
/// two nodes cannot both create a receipt or overwrite one another's status.
fn record_recipient_read(base: &mut JamMessageBase, number: u32, user: &str, alias: &str) -> Res<bool> {
    Ok(base.transaction(|base| {
        let mut header = base.read_header(number)?;
        let to = header.to().map(|name| name.to_string()).unwrap_or_default();
        if header.is_deleted() || to.starts_with('@') || to.eq_ignore_ascii_case("ALL")
            || !(to.trim().eq_ignore_ascii_case(user) || (!alias.is_empty() && to.trim().eq_ignore_ascii_case(alias))) {
            return Ok(false);
        }
        let receipt = header.is_receipt_req();
        if header.is_read() && !receipt {
            return Ok(false);
        }
        let now = chrono::Utc::now();
        let mut received_at = now.timestamp() as u32;
        if receipt {
            // Keep a stable key on the receipt. If its append succeeds but the
            // source-header update fails, a retry finds it instead of duplicating it.
            let existing_key = header.sub_fields.iter().find(|field| field.field_type() == SubfieldType::FTSKludge
                && field.content().starts_with(b"ICYBOARD-RECEIPT-KEY: ")).map(|field| field.content().clone());
            let key = if let Some(key) = existing_key { key } else {
                let key = BString::from(format!("ICYBOARD-RECEIPT-KEY: {}:{}:{}", number, header.date_written, now.timestamp_nanos_opt().unwrap_or_default()));
                header.sub_fields.push(MessageSubfield::new(SubfieldType::FTSKludge, key.clone()));
                raw::update_header(base, number, &header)?;
                sync_receipt_files(base)?;
                key
            };
            let acknowledgment_key = BString::from(key.to_string().replacen("-KEY:", "-ACK:", 1));
            let mut exists = false;
            // Include deleted receipts: deleting an acknowledgment must not
            // re-arm a source whose request flag failed to clear.
            for candidate in raw::physical_headers(base)? {
                let candidate = candidate?;
                let is_ack = |header: &JamMessageHeader| header.sub_fields.iter().any(|field|
                    field.field_type() == SubfieldType::FTSKludge && field.content() == &acknowledgment_key);
                if is_ack(&candidate) && (candidate.is_deleted() || base.read_header(candidate.message_number).is_ok_and(|header| is_ack(&header))) {
                    exists = true;
                    received_at = candidate.date_written;
                    break;
                }
            }
            if !exists {
                let mut ack = JamMessage::default()
                    .with_from(BString::from("RETURN RECEIPT"))
                    .with_to(header.from().cloned().unwrap_or_default())
                    .with_subject(header.subject().cloned().unwrap_or_default())
                    .with_reply_to(number)
                    .with_date_time(now)
                    .with_attributes(attributes::MSG_PRIVATE | attributes::MSG_LOCAL)
                    .with_sub_field(MessageSubfield::new(SubfieldType::FTSKludge, acknowledgment_key))
                    .with_text(BString::from(format!("Your message: {number}\r\nAddressed to: {to}\r\nReceived on: {}\r\n", now.format("%m/%d/%y %H:%M"))));
                for field in &header.sub_fields {
                    if field.field_type() == SubfieldType::MsgID {
                        ack = ack.with_reply_id(field.content().clone());
                    } else if field.field_type() == SubfieldType::Address0 {
                        ack = ack.with_sub_field(MessageSubfield::new(SubfieldType::AddressD, field.content().clone()));
                    }
                }
                base.write_message(&ack)?;
                base.write_jhr_header()?;
            }
            // The acknowledgment must reach disk before the request is cleared.
            // If synchronization fails, leave the request available for retry.
            sync_receipt_files(base)?;
            header.attributes &= !attributes::MSG_RECEIPTREQ;
        }
        if !header.is_read() {
            header.attributes |= attributes::MSG_READ;
            header.date_received = received_at;
        }
        raw::update_header(base, number, &header)?;
        Ok(receipt)
    })?)
}

fn advance_read_pointer(base: &mut JamMessageBase, user: &str, user_id: u32, number: u32) -> Res<(u32, u32)> {
    Ok(base.transaction(|base| {
        let crc = JamMessageBase::crc(&BString::from(user));
        let mut last = match base.find_last_read(crc, user_id)? {
            Some(last) => last,
            None => base.create_last_read(crc, user_id)?,
        };
        last.last_read_msg = last.last_read_msg.max(number);
        last.high_read_msg = last.high_read_msg.max(number);
        base.write_last_read(&last)?;
        Ok((last.last_read_msg, last.high_read_msg))
    })?)
}

/// Owned under a shared transaction, then displayed without holding a JAM lock.
struct DisplayedMessage {
    header: JamMessageHeader,
    body: BString,
    generation: (u32, u32),
}

impl DisplayedMessage {
    fn load(base: &JamMessageBase, header: JamMessageHeader) -> jamjam::Result<Self> {
        Ok(Self {
            body: base.read_message_text(&header)?,
            header,
            generation: (base.info().date_created, base.mod_counter()),
        })
    }

    /// Compare all stored header fields and raw body bytes, not a lossy display
    /// string or a reusable message number. Even an identical packed replacement
    /// must not inherit the displayed message's read effects.
    fn unchanged(&self, base: &JamMessageBase) -> jamjam::Result<bool> {
        if self.generation != (base.info().date_created, base.mod_counter()) {
            return Ok(false);
        }
        let fresh = match base.read_header(self.header.message_number) {
            Ok(header) => header,
            Err(jamjam::Error::Jam(jamjam::jam::JamError::MessageDeleted | jamjam::jam::JamError::MessageNumberOutOfRange(..))) => return Ok(false),
            Err(error) => return Err(error),
        };
        let mut displayed_header = Vec::new();
        let mut fresh_header = Vec::new();
        self.header.write(&mut displayed_header)?;
        fresh.write(&mut fresh_header)?;
        Ok(!fresh.is_deleted() && displayed_header == fresh_header && self.body == base.read_message_text(&fresh)?)
    }
}

/// Validation, status/receipt and pointer writes share one exclusive transaction.
/// Nested JAM transactions keep the same lock; no await can split these effects.
fn commit_displayed_read(
    base: &mut JamMessageBase,
    snapshot: &DisplayedMessage,
    user: &str,
    alias: &str,
    user_id: u32,
    status: bool,
    pointer: bool,
) -> Res<(bool, Option<(u32, u32)>)> {
    if !status && !pointer {
        return Ok((false, None));
    }
    Ok(base.transaction(|base| {
        let result: Res<_> = (|| {
            if !snapshot.unchanged(base)? {
                return Ok((false, None));
            }
            let number = snapshot.header.message_number;
            let receipt = status && record_recipient_read(base, number, user, alias)?;
            let pointers = if pointer { Some(advance_read_pointer(base, user, user_id, number)?) } else { None };
            Ok((receipt, pointers))
        })();
        result.map_err(|error| std::io::Error::other(error.to_string()).into())
    })?)
}

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
        let mut result: Vec<char> = txt.chars().collect();
        result.resize(result.len().max(80), ' ');
        for (start, value) in [(self.left + 2, left), (self.right + 2, right)] {
            let value: Vec<char> = value.chars().collect();
            result.resize(result.len().max(start + value.len()), ' ');
            result[start..start + value.len()].copy_from_slice(&value);
        }
        result.into_iter().collect()
    }

    pub async fn display_header(&self, state: &mut IcyBoardState, msg_base: &JamMessageBase, header: &JamMessageHeader) -> Res<()> {
        self.display_header_length(state, msg_base, header, HeaderLength::Long).await
    }

    async fn display_header_length(&self, state: &mut IcyBoardState, msg_base: &JamMessageBase, header: &JamMessageHeader, length: HeaderLength) -> Res<()> {
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
        let txt = self.format_hdr_text(&self.to_line.text, &header.to().map(ToString::to_string).unwrap_or_default(), "");
        if state.session.search_pattern.is_some() {
            state.print_found_text(TerminalTarget::Both, &txt).await?;
        } else {
            state.print(TerminalTarget::Both, &txt).await?;
        }

        let c1 = state.get_board().await.config.color_configuration.msg_hdr_from.clone();
        state.set_color(TerminalTarget::Both, c1).await?;
        let txt = self.format_hdr_text(&self.from_line.text, &header.from().map(ToString::to_string).unwrap_or_default(), "");
        if state.session.search_pattern.is_some() {
            state.print_found_text(TerminalTarget::Both, &txt).await?;
        } else {
            state.print(TerminalTarget::Both, &txt).await?;
        }

        let c1 = state.get_board().await.config.color_configuration.msg_hdr_subj.clone();
        state.set_color(TerminalTarget::Both, c1).await?;
        let txt = self.format_hdr_text(&self.subj_line.text, &header.subject().map(ToString::to_string).unwrap_or_default(), "");
        if state.session.search_pattern.is_some() {
            state.print_found_text(TerminalTarget::Both, &txt).await?;
        } else {
            state.print(TerminalTarget::Both, &txt).await?;
        }

        let c1 = state.get_board().await.config.color_configuration.msg_hdr_read.clone();
        state.set_color(TerminalTarget::Both, c1).await?;
        if length == HeaderLength::Long {
            let received = if header.to().is_some_and(|to| to.eq_ignore_ascii_case(b"ALL")) {
                self._not_avail.text.clone()
            } else if header.is_read() {
                chrono::DateTime::from_timestamp(header.date_received as i64, 0)
                    .filter(|_| header.date_received != 0)
                    .map(|date| date.format("%m/%d/%y (%H:%M)").to_string())
                    .unwrap_or_else(|| self._not_read.text.clone())
            } else { self._not_read.text.clone() };
            let status = if header.needs_password() {
                if requires_read_password(header, false) { &self._grp_pwd.text } else { &self._snd_pwd.text }
            } else if header.is_private() { &self._rcv_only.text } else { &self._public.text };
            let txt = self.format_hdr_text(&self._read.text, &received, &format!("{} {}", self._status.text.trim(), status));
            state.print(TerminalTarget::Both, &txt).await?;
            let area = state.session.current_conference.areas.as_ref()
                .and_then(|areas| areas.get(state.session.current_message_area))
                .map(|area| area.name.as_str()).unwrap_or_default();
            let txt = self.format_hdr_text(&self.confarea.text, &state.session.current_conference.name, area);
            state.print(TerminalTarget::Both, &txt).await?;
        }
        state.reset_color(TerminalTarget::Both).await?;
        if state.session.disp_options.count_lines {
            state.session.disp_options.num_lines_printed += if length == HeaderLength::Long { 6 } else { 4 };
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
    pub async fn read_msgs_from_base(&mut self, message_base: JamMessageBase, only_personal: bool) -> Res<()> {
        let saved_search = self.session.search_pattern.clone();
        let result = self.read_messages_loop(message_base, only_personal).await;
        self.session.search_pattern = saved_search;
        result
    }

    async fn read_messages_loop(&mut self, mut message_base: JamMessageBase, only_personal: bool) -> Res<()> {
        let viewer = MessageViewer::load(&self.display_text)?;
        let mut options = ReaderOptions::default();
        let mut pending = None;
        while !self.session.disp_options.abort_printout && !self.session.request_logoff {
            message_base.read_jhr_header()?;
            let low_number = message_base.lowest_message_number();
            let high_number = message_base.highest_message_number();
            let prompt = if self.session.expert_mode() {
                IceText::MessageReadCommandExpert
            } else {
                IceText::MessageReadCommand
            };
            self.session.op_text = format!("{low_number}-{high_number}");

            if pending.is_none() && self.session.tokens.is_empty() {
                let text = self
                    .input_field(
                        prompt,
                        40,
                        MASK_COMMAND,
                        CommandType::ReadMessages.get_help(),
                        None,
                        display_flags::UPCASE | display_flags::NEWLINE | display_flags::LFBEFORE,
                    )
                    .await?;
                if text.is_empty() {
                    break;
                }
                self.session.push_tokens(&text);
            }

            let mut cmd = if let Some(cmd) = pending.take() { cmd } else {
                let tokens: Vec<String> = self.session.tokens.drain(..).collect();
                let ctx = self.read_parse_context(0).await;
                read_command::parse(&tokens, ReadLoop::Outside, &ctx)
            };
            if cmd.set_last_read {
                self.set_last_message_read(&cmd, &mut message_base).await?;
            }
            if !self.apply_reader_options(&cmd, &mut options, false).await? {
                continue;
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

            if only_personal {
                cmd.any_msgs = false;
                cmd.your_msgs = true;
                cmd.all_conf = false;
            }
            if capture::requested(&cmd) {
                self.run_reader_capture(&mut message_base, &viewer, cmd, &mut options, None).await?;
                self.stop_search();
                continue;
            }
            if cmd.func != MsgFunc::None {
                // Outer actions must execute too; never use a stale current
                // message implicitly for message-specific operations.
                let number = self.session.current_messagenumber;
                match cmd.func {
                    MsgFunc::Join => {
                        self.session.tokens.extend(cmd.action_args.clone());
                        self.join_conference_cmd().await?;
                        break;
                    }
                    MsgFunc::JumpOut => break,
                    MsgFunc::Kill if cmd.numbers.is_empty() => {
                        let answer = if let Some(answer) = cmd.action_args.first() { answer.clone() } else {
                            self.input_field(IceText::MessageNumberToKill, 10, &MASK_NUM, "hlpk", None, display_flags::NEWLINE).await?
                        };
                        if let Ok(number) = answer.parse::<u32>() {
                            self.run_read_action(&cmd, &mut message_base, number).await?;
                        }
                        continue;
                    }
                    MsgFunc::SelectConference | MsgFunc::DeselectConference => {
                        self.reader_select_conference(cmd.func == MsgFunc::SelectConference).await?;
                        break;
                    }
                    MsgFunc::EnterMessage => {
                        self.run_read_action(&cmd, &mut message_base, number).await?;
                        continue;
                    }
                    MsgFunc::Skip if !options.update_pointers || !self.get_board().await.config.message.update_last_read_pointer => continue,
                    MsgFunc::Reply | MsgFunc::ReplyOther | MsgFunc::EditHeader | MsgFunc::EditMessage
                    | MsgFunc::Copy | MsgFunc::Move | MsgFunc::Forward | MsgFunc::Protect | MsgFunc::Unprotect
                    | MsgFunc::FindTo | MsgFunc::FindFrom | MsgFunc::Export => {
                        self.display_text(IceText::InvalidEntry, display_flags::NEWLINE).await?;
                        continue;
                    }
                    _ => {}
                }
                match self.run_read_action(&cmd, &mut message_base, number).await? {
                    AfterAction::NotHandled => self.display_text(IceText::InvalidEntry, display_flags::NEWLINE).await?,
                    AfterAction::Quit => break,
                    _ => {}
                }
                continue;
            }
            let exit = if cmd.all_conf {
                self.read_all_conferences(&viewer, &cmd, &mut options).await?
            } else {
                self.read_command_from_base(&mut message_base, &viewer, cmd, &mut options, None, only_personal).await?
            };
            match exit {
                ReaderExit::Command(cmd) => pending = Some(cmd),
                ReaderExit::Stop | ReaderExit::Leave => break,
                _ => {}
            }
            self.stop_search();
        }
        Ok(())
    }

    /// Where this message base left the current user's last-read pointer.
    fn last_read_pointer(&mut self, message_base: &mut JamMessageBase) -> Res<u32> {
        let crc = JamMessageBase::crc(&BString::from(self.session.user_name.as_str()));
        let last = message_base.find_last_read(crc, self.session.cur_user_id as u32)?;
        self.session.last_msg_read = last.as_ref().map_or(0, |last| last.last_read_msg);
        self.session.highest_msg_read = last.as_ref().map_or(0, |last| last.high_read_msg);
        Ok(self.session.last_msg_read)
    }

    pub(crate) async fn read_parse_context(&mut self, reply_to: i64) -> ParseContext {
        // Use one guard: temporaries in the struct initializer live until its
        // end, so a second get_board().await would wait on our own first lock.
        let board = self.get_board().await;
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
            may_read_only: board.config.sysop_command_level.not_update_msg_read.session_can_access(&self.session),
            alias_support: self.session.current_conference.allow_aliases,
            qwk_support: true,
            reply_command: false,
            quick_scan: false,
            num_conferences: board.conferences.len() as u16,
        }
    }

    /// Turn a parsed range into the message numbers this base actually holds.
    fn clamp_range(&self, range: read_command::MsgRange, low_number: u32, high_number: u32) -> (u32, u32) {
        if range.first <= 0 || high_number == 0 || low_number > high_number || range.first.max(range.last) < low_number as i64 || range.first.min(range.last) > high_number as i64 {
            return (0, 0);
        }
        let high = high_number.max(low_number);
        let clamp = |value: i64| -> u32 { value.clamp(low_number as i64, high as i64) as u32 };
        (clamp(range.first), clamp(range.last))
    }

    /// The questions `PCBoard` asks once it has read the whole line
    /// and finds a search with nothing to search for.
    async fn resolve_read_command(&mut self, cmd: &mut ReadCommand) -> Res<Resolution> {
        if !cmd.search_text.is_empty() && !cmd.threading && !cmd.do_user_search {
            cmd.do_text_search = true;
        }
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

        if cmd.func == MsgFunc::None && cmd.numbers.is_empty() && !cmd.all_conf && !cmd.since && !cmd.new_msgs
            && !cmd.your_msgs && !cmd.from_msgs && !cmd.unread_only {
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
        first: u32,
        last: u32,
        keep_going: bool,
        filter: &MessageFilter,
    ) -> Res<()> {
        let cmd = ReadCommand { numbers: vec![MsgRange { first: first as i64, last: last as i64 }], keep_going, ..Default::default() };
        let saved_search = self.session.search_pattern.clone();
        let result = async {
            let mut options = ReaderOptions::default();
            let mut exit = self.read_command_from_base(message_base, viewer, cmd, &mut options, Some(filter.clone()), false).await?;
            while let ReaderExit::Command(cmd) = exit {
                if cmd.all_conf {
                    exit = self.read_all_conferences(viewer, &cmd, &mut options).await?;
                } else if cmd.func == MsgFunc::Join {
                    self.session.tokens.extend(cmd.action_args);
                    self.join_conference_cmd().await?;
                    break;
                } else { break; }
            }
            Ok(())
        }.await;
        self.session.search_pattern = saved_search;
        result
    }

    async fn reader_filter(&mut self, cmd: &ReadCommand) -> MessageFilter {
        self.stop_search();
        if cmd.do_text_search && !cmd.search_text.is_empty() {
            if !self.search_init(cmd.search_text.clone(), false) {
                // A malformed replacement search must not reuse the old search
                // or accidentally turn into an unfiltered scan.
                self.session.search_pattern = Some(regex::Regex::new(r"\b\B").expect("never-matching regex"));
            }
        }
        let may_read_all = self.get_board().await.config.sysop_command_level.read_all_mail.session_can_access(&self.session);
        MessageFilter::new(cmd, &self.session, may_read_all)
    }

    async fn apply_reader_options(&mut self, cmd: &ReadCommand, options: &mut ReaderOptions, inside: bool) -> Res<bool> {
        if let Some(reason) = capture::unsupported(cmd) {
            self.display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::LFBEFORE).await?;
            self.println(TerminalTarget::Both, reason).await?;
            return Ok(false);
        }
        options.update_pointers &= cmd.update_pointers;
        options.update_status &= cmd.update_msg_status;
        if let Some(length) = cmd.header_len { options.header = length; }
        if let Some(alias) = cmd.toggle_alias {
            self.session.use_alias = match alias {
                AliasToggle::On => true,
                AliasToggle::Off => false,
                AliasToggle::Flip => !self.session.use_alias,
            };
        }
        if cmd.show_help {
            let path = self.get_board().await.config.paths.help_path.join(if inside { "hlpendr" } else { CommandType::ReadMessages.get_help() });
            self.display_file(&path).await?;
        }
        Ok(true)
    }

    async fn reader_select_conference(&mut self, selected: bool) -> Res<()> {
        if let Some(user) = &mut self.session.current_user {
            let number = self.session.current_conference_number as usize;
            let mut flags = user.conference_flags.get(&number).copied().unwrap_or(ConferenceFlags::None);
            if selected { flags |= ConferenceFlags::Selected; } else { flags &= !ConferenceFlags::Selected; }
            if flags.is_empty() { user.conference_flags.remove(&number); }
            else { user.conference_flags.insert(number, flags); }
        }
        self.display_text(if selected { IceText::Selected } else { IceText::DeSelected }, display_flags::NEWLINE | display_flags::LFBEFORE).await
    }

    async fn read_all_conferences(&mut self, viewer: &MessageViewer, command: &ReadCommand, options: &mut ReaderOptions) -> Res<ReaderExit> {
        let original = self.session.current_conference_number;
        let original_conf = self.session.current_conference.clone();
        let original_area = self.session.current_message_area;
        let original_user_conf = self.session.current_user.as_ref().map(|user| user.last_conference);
        let original_security = self.session.cur_security;
        let original_message = (self.session.current_messagenumber, self.session.low_msg_num, self.session.high_msg_num,
            self.session.last_msg_read, self.session.highest_msg_read);
        let original_search = self.session.search_pattern.clone();
        let original_memorized = self.session.memorized_msg;
        let original_limits = (self.session.time_limit, self.session.batch_limit, self.session.bytes_remaining, self.session.transfer_limits.clone());
        let conferences = self.get_board().await.conferences.clone();
        let may_join = self.session.user_command_level.cmd_j.session_can_access(&self.session);
        let start = if command.stay_in_conf { original as usize } else { self.session.start_conf as usize };
        // Restore even on a corrupt base, failed output, disconnect or action error.
        let result = async {
            for (index, conf) in conferences.iter().enumerate().skip(start) {
                self.session.start_conf = index as u16;
                if self.session.request_logoff || self.session.disp_options.abort_printout { return Ok(ReaderExit::Stop); }
                // Authorization must use the original context, not a security
                // bonus acquired while visiting the previous conference.
                self.session.current_conference_number = original;
                self.session.current_conference = original_conf.clone();
                self.session.cur_security = original_security;
                let flags = self.session.current_user.as_ref().and_then(|user| user.conference_flags.get(&index)).copied().unwrap_or(ConferenceFlags::None);
                let number = index as u16;
                if index > u16::MAX as usize || !self.subscription_can_access_conference(number) || self.is_lockedout(number)
                    || !conf.required_security.session_can_access(&self.session) {
                    continue;
                }
                if number != original && (!may_join
                    || !(self.session.is_sysop || conf.is_public || flags.contains(ConferenceFlags::Registered))
                    || (command.check_user_scan && !flags.contains(ConferenceFlags::Selected))
                    || (!conf.password.is_empty() && !self.session.joined_conferences.contains(&number))) {
                    continue;
                }
                if command.mail_wait_conf && !flags.contains(ConferenceFlags::MailWaiting) { continue; }
                self.set_current_conference(number).await?;
                if !self.session.user_command_level.cmd_r.session_can_access(&self.session) { continue; }
                let Some(areas) = conf.areas.as_ref() else { continue; };
                for (area_number, area) in areas.iter().enumerate() {
                    if !area.req_level_to_list.session_can_access(&self.session) || !area.path.with_extension("jhr").exists() { continue; }
                    self.session.current_message_area = area_number;
                    self.session.memorized_msg = None;
                    let mut base = match JamMessageBase::open(&area.path) {
                        Ok(base) => base,
                        Err(error) => {
                            if options.capture.is_some() { return Err(error.into()); }
                            log::error!("Cannot scan conference {number}, area {area_number}: {error}");
                            self.display_text(IceText::MessageBaseError, display_flags::NEWLINE).await?;
                            continue;
                        }
                    };
                    let mut local = command.clone();
                    local.all_conf = false;
                    match self.read_command_from_base(&mut base, viewer, local, options, None, false).await? {
                        ReaderExit::Done => {},
                        ReaderExit::Skip => break,
                        exit => return Ok(exit),
                    }
                }
            }
            self.session.start_conf = 0;
            Ok(ReaderExit::Done)
        }.await;
        self.session.current_conference_number = original;
        self.session.current_conference = original_conf;
        self.session.current_message_area = original_area;
        self.session.cur_security = original_security;
        self.session.memorized_msg = original_memorized;
        self.session.search_pattern = original_search;
        (self.session.current_messagenumber, self.session.low_msg_num, self.session.high_msg_num,
            self.session.last_msg_read, self.session.highest_msg_read) = original_message;
        (self.session.time_limit, self.session.batch_limit, self.session.bytes_remaining, self.session.transfer_limits) = original_limits;
        if let (Some(user), Some(last)) = (&mut self.session.current_user, original_user_conf) { user.last_conference = last; }
        if let Some(state) = self.node_state.lock().await[self.node].as_mut() { state.cur_conference = original; }
        result
    }

    async fn read_command_from_base(
        &mut self,
        message_base: &mut JamMessageBase,
        viewer: &MessageViewer,
        mut command: ReadCommand,
        options: &mut ReaderOptions,
        initial_filter: Option<MessageFilter>,
        only_personal: bool,
    ) -> Res<ReaderExit> {
        // ALL uses the existing authorized conference traversal, but collection
        // never enters the interactive reader or applies premature read effects.
        if options.capture.is_some() {
            self.collect_reader_capture(message_base, viewer, &command, options, initial_filter).await?;
            return Ok(ReaderExit::Done);
        }
        self.session.low_msg_num = message_base.lowest_message_number();
        self.session.high_msg_num = message_base.highest_message_number();
        self.last_read_pointer(message_base)?;
        if command.since {
            if self.session.last_msg_read >= self.session.high_msg_num {
                self.display_text(IceText::NoMailFound, display_flags::NEWLINE | display_flags::LFAFTER).await?;
                return Ok(ReaderExit::Done);
            }
            if let Some(range) = command.numbers.first_mut() {
                range.first = self.session.last_msg_read as i64 + 1;
            }
        }
        let update_last_read = self.get_board().await.config.message.update_last_read_pointer;
        let mut filter = match initial_filter { Some(filter) => filter, None => self.reader_filter(&command).await };
        let mut ranges: VecDeque<_> = command.numbers.clone().into();
        let mut first = 0;
        let mut last = 0;
        let mut number = 0;
        let mut keep_going = command.keep_going;
        let mut reply_to = 0;
        let mut subject = String::new();
        let mut display_msg = true;
        let mut shown = 0;
        loop {
            if self.session.request_logoff || self.session.disp_options.abort_printout {
                return Ok(ReaderExit::Stop);
            }
            if number == 0 {
                let Some(range) = ranges.pop_front() else {
                    if shown == 0 {
                        self.display_text(IceText::NoMailFound, display_flags::NEWLINE | display_flags::LFAFTER).await?;
                    }
                    return Ok(ReaderExit::Done);
                };
                (first, last) = self.clamp_range(range, self.session.low_msg_num, self.session.high_msg_num);
                number = first;
                display_msg = true;
                if number == 0 { continue; }
            }
            if display_msg {
                display_msg = false;
                let may_read_all = self.get_board().await.config.sysop_command_level.read_all_mail.session_can_access(&self.session);
                let found = loop {
                    // No lock is held over terminal I/O. Header and body must
                    // nevertheless come from the same generation of the base.
                    let candidate = message_base.read_transaction(|base| {
                        let header = base.read_header(number)?;
                        // Check the actual session as well as any caller-supplied
                        // filter before loading text. A hit-dependent password
                        // prompt would disclose the contents of protected mail.
                        if !may_read_header(&header, &self.session.user_name, &self.session.alias_name, may_read_all)
                            || !filter.may_search(&header, may_read_all) {
                            return Ok(None);
                        }
                        Ok(Some(DisplayedMessage::load(base, header)?))
                    });
                    match candidate {
                        Ok(Some(snapshot)) => {
                            let text = snapshot.body.to_string();
                            if filter.matches(&snapshot.header, &text, self.session.last_msg_read) {
                                break Some((snapshot, text));
                            }
                        }
                        Ok(None) => {},
                        Err(err) => {
                            log::error!("Error reading message header: {err}");
                        }
                    }
                    match next_in_range(number, first, last) {
                        Some(next) => number = next,
                        None => break None,
                    }
                };
                let Some((snapshot, text)) = found else {
                    number = 0;
                    continue;
                };
                let header = &snapshot.header;
                self.session.current_messagenumber = number;
                reply_to = header.reply_to as i64;
                subject = header.subject().map(ToString::to_string).unwrap_or_default();
                viewer.display_header_length(self, message_base, header, options.header).await?;
                if requires_read_password(header, may_read_all) {
                    if !self
                        .check_password(IceText::PasswordToReadMessage, 0, |pwd| header.is_password_valid(pwd))
                        .await?
                    {
                        // No body, no action prompt and no persistent read effects.
                        number = next_in_range(number, first, last).unwrap_or(0);
                        display_msg = true;
                        continue;
                    }
                }
                viewer.display_body(self, &text).await?;
                self.new_line().await?;
                if self.session.request_logoff || self.session.disp_options.abort_printout {
                    return Ok(ReaderExit::Stop);
                }
                shown += 1;
                let (receipt, pointers) = commit_displayed_read(
                    message_base, &snapshot, &self.session.user_name, &self.session.alias_name, self.session.cur_user_id as u32,
                    options.update_status && !command.quick_scan,
                    options.update_pointers && update_last_read && !command.quick_scan,
                )?;
                if let Some(pointers) = pointers {
                    (self.session.last_msg_read, self.session.highest_msg_read) = pointers;
                }
                if receipt {
                    self.display_text(IceText::ReturnReceiptRequired, display_flags::LFBEFORE).await?;
                    self.display_text(IceText::GenerateReceipt, display_flags::LFBEFORE).await?;
                }
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
                    number = 0;
                    continue;
                }
            } else {
                self.session.push_tokens(&text);
                let tokens: Vec<String> = self.session.tokens.drain(..).collect();
                let ctx = self.read_parse_context(reply_to).await;
                let mut cmd = read_command::parse(&tokens, ReadLoop::Inside, &ctx);
                if cmd.set_last_read {
                    self.set_last_message_read(&cmd, message_base).await?;
                    // SET was applied to this base; never replay it after an
                    // ALL/J handoff restores a different conference.
                    cmd.set_last_read = false;
                }
                if !self.apply_reader_options(&cmd, options, true).await? { continue; }
                if cmd.threading { cmd.search_text = subject.clone(); }
                if cmd.memorize {
                    self.session.memorized_msg = Some((self.session.current_message_area, number));
                    self.display_text(IceText::MessageNumberMemorized, display_flags::LFBEFORE).await?;
                }
                if only_personal {
                    cmd.any_msgs = false;
                    cmd.your_msgs = true;
                    cmd.all_conf = false;
                }
                // An in-loop search begins after the message on screen unless
                // the caller supplies explicit ranges.
                if cmd.numbers.is_empty() && (cmd.do_text_search || cmd.do_user_search) {
                    cmd.numbers.push(MsgRange { first: number as i64 + 1, last: read_command::LAST_MESSAGE });
                    cmd.keep_going = true;
                }
                match self.resolve_read_command(&mut cmd).await? {
                    Resolution::Stop => return Ok(ReaderExit::Stop),
                    Resolution::Reprompt => continue,
                    Resolution::Run => {}
                }
                read_command::finalize(&mut cmd);
                if cmd.all_conf || cmd.func == MsgFunc::Join {
                    // Options were already applied above; do not flip ALIAS or
                    // ask the resume question twice when handing off to outer.
                    cmd.toggle_alias = None;
                    cmd.show_help = false;
                    cmd.ask_resume_all = false;
                    return Ok(ReaderExit::Command(cmd));
                }

                if capture::requested(&cmd) {
                    // A bare inner C/D/Z/QWK captures the current message only.
                    // Keep the active scan's filters for that operation, then
                    // resume its exact ranges and end-of-message prompt.
                    let inherited = if cmd.capture_single && cmd.numbers.is_empty() {
                        cmd.numbers.push(MsgRange { first: number as i64, last: number as i64 });
                        Some(filter.clone())
                    } else { None };
                    Box::pin(self.run_reader_capture(message_base, viewer, cmd, options, inherited)).await?;
                    continue;
                }

                match cmd.func {
                    MsgFunc::Stop => return Ok(ReaderExit::Stop),
                    MsgFunc::Goodbye => {
                        self.goodbye().await?;
                        return Ok(ReaderExit::Leave);
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
                    MsgFunc::SelectConference | MsgFunc::DeselectConference => {
                        self.reader_select_conference(cmd.func == MsgFunc::SelectConference).await?;
                        number = next_in_range(number, first, last).unwrap_or(0);
                        keep_going = true;
                        display_msg = true;
                        continue;
                    }
                    MsgFunc::JumpOut => return Ok(ReaderExit::Skip),
                    MsgFunc::Skip if !options.update_pointers || !update_last_read => return Ok(ReaderExit::Skip),
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
                    AfterAction::Quit => return Ok(if matches!(cmd.func, MsgFunc::Skip | MsgFunc::DeselectConference) { ReaderExit::Skip } else { ReaderExit::Leave }),
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
                if !cmd.numbers.is_empty() {
                    if cmd.since {
                        self.last_read_pointer(message_base)?;
                        if self.session.last_msg_read >= self.session.high_msg_num { return Ok(ReaderExit::Done); }
                        cmd.numbers[0].first = self.session.last_msg_read as i64 + 1;
                    }
                    filter = self.reader_filter(&cmd).await;
                    ranges = cmd.numbers.clone().into();
                    command = cmd;
                    shown = 0;
                    number = 0;
                    display_msg = true;
                    continue;
                }
            }
            if keep_going {
                match next_in_range(number, first, last) {
                    Some(next) => number = next,
                    None => number = 0,
                }
                display_msg = true;
            }
        }
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    use icy_net::{Connection, channel::ChannelConnection};
    use tokio::time::{Duration, timeout};

    async fn reader_state() -> (IcyBoardState, ChannelConnection) {
        use crate::icy_board::{IcyBoard, bbs::BBS, security_expr::SecurityExpression, user_base::User};
        use std::sync::Arc;
        use tokio::sync::Mutex;

        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        let node = bbs.lock().await.create_new_node(icy_net::ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (peer, connection) = ChannelConnection::create_pair();
        let mut board = IcyBoard::new();
        board.config.message.update_last_read_pointer = true;
        board.config.sysop_command_level.read_all_mail = SecurityExpression::from_req_security(255);
        let mut state = IcyBoardState::new(bbs, Arc::new(Mutex::new(board)), nodes, node, Box::new(connection)).await;
        state.session.current_user = Some(User { name: "READER".into(), security_level: 10, ..Default::default() });
        state.session.user_name = "READER".into();
        state.session.cur_security = 10;
        state.session.cur_user_id = 1;
        state.session.page_len = 0;
        // Reads fall back to defaults, but prompt overrides need populated records.
        state.display_text = crate::icy_board::icb_text::DEFAULT_DISPLAY_TEXT.clone();
        for (id, text) in [
            (IceText::MorePrompt, "READER-MORE"),
            (IceText::PasswordToReadMessage, "READER-PASSWORD"),
            (IceText::EndOfMessage, "READER-END"),
            (IceText::EndOfMessageExpertmode, "READER-END"),
            (IceText::MessageReadCommand, "READER-OUTER"),
            (IceText::MessageReadCommandExpert, "READER-OUTER"),
        ] {
            state.display_text.update_record_number(id as usize, text).unwrap();
        }
        (state, peer)
    }

    async fn read_until(peer: &mut ChannelConnection, marker: &str) -> String {
        let mut output = Vec::new();
        let mut bytes = [0; 4096];
        while !String::from_utf8_lossy(&output).contains(marker) {
            let count = peer.read(&mut bytes).await.unwrap();
            assert_ne!(count, 0, "reader closed before {marker}");
            output.extend_from_slice(&bytes[..count]);
        }
        String::from_utf8_lossy(&output).into_owned()
    }

    #[tokio::test]
    async fn ordinary_read_revalidates_after_password_and_pagination_waits() {
        use std::io::{Seek, SeekFrom, Write};
        use jamjam::jam::pack::PackOptions;

        // The unchanged control must still mark the message, send exactly one
        // receipt and advance both disk/session pointers. Each race must do none.
        for password in [false, true] {
            for change in ["unchanged", "header", "body", "generation", "pack"] {
                let root = tempfile::tempdir().unwrap();
                let path = root.path().join("mail");
                let mut base = JamMessageBase::create(&path).unwrap();
                let body = "DISPLAYED-BODY\n".repeat(40);
                base.write_message(&JamMessage::default().with_from("SENDER".into()).with_to("READER".into())
                    .with_subject("DISPLAYED-SUBJECT".into()).with_text(body.into())
                    .with_attributes(attributes::MSG_PRIVATE | attributes::MSG_RECEIPTREQ)).unwrap();
                if password {
                    let mut header = base.read_header(1).unwrap();
                    header.password_crc = JamMessageBase::crc(&BString::from("SECRET"));
                    raw::update_header(&mut base, 1, &header).unwrap();
                }
                mail(&mut base, "READER", attributes::MSG_PRIVATE | attributes::MSG_RECEIPTREQ);
                let (mut state, mut peer) = reader_state().await;
                state.session.page_len = if password { 0 } else { 12 };
                state.session.push_tokens("1+");
                let mut writer = JamMessageBase::open(&path).unwrap();
                let (result, ()) = timeout(Duration::from_secs(5), async {
                    tokio::join!(state.read_msgs_from_base(base, false), async {
                        let output = read_until(&mut peer, if password { "READER-PASSWORD" } else { "READER-MORE" }).await;
                        assert!(output.contains("DISPLAYED-SUBJECT"));
                        assert_eq!(output.contains("DISPLAYED-BODY"), !password);
                        // Fails promptly rather than deadlocking if a read lock
                        // accidentally survives either terminal wait.
                        assert!(writer.try_lock().unwrap(), "JAM locked during terminal wait");
                        assert!(!writer.read_header(1).unwrap().is_read());
                        assert!(writer.read_last_read_file().unwrap().is_empty());
                        match change {
                            "header" => {
                                let mut header = writer.read_header(1).unwrap();
                                header.reply_to = 42;
                                raw::update_header(&mut writer, 1, &header).unwrap();
                            }
                            "body" => {
                                // Same length/offset/header and no counter bump:
                                // raw byte equality must catch an in-place edit.
                                let header = writer.read_header(1).unwrap();
                                let mut text = std::fs::OpenOptions::new().write(true).open(path.with_extension("jdt")).unwrap();
                                text.seek(SeekFrom::Start(header.offset as u64)).unwrap();
                                text.write_all(b"REPLACED!-BODY").unwrap();
                            }
                            "generation" => writer.pack(&PackOptions::default().with_index_only(true)).map(|_| ()).unwrap(),
                            "pack" => {
                                writer.delete_message(1).unwrap();
                                writer.pack(&PackOptions::default().with_renumber_from(1)).unwrap();
                            }
                            _ => {}
                        }
                        writer.unlock();
                        peer.send(if password { b"SECRET\r" } else { b"NS\r" }).await.unwrap();
                        read_until(&mut peer, "READER-END").await;
                        // Inner N must exit the whole reader, not prompt outside
                        // or show the next message. No extra input masks a stall.
                        peer.send(b"N\r").await.unwrap();
                    })
                }).await.expect("reader stalled at password/pagination or reprompted after N");
                result.unwrap();
                let base = JamMessageBase::open(&path).unwrap();
                let header = base.read_header(1).unwrap();
                let records = base.read_last_read_file().unwrap();
                if change == "unchanged" {
                    assert!(header.is_read());
                    assert!(!header.is_receipt_req());
                    assert_ne!(header.date_received, 0);
                    assert_eq!(base.highest_message_number(), 3);
                    assert_eq!(records.len(), 1);
                    assert_eq!((records[0].last_read_msg, records[0].high_read_msg), (1, 1));
                    assert_eq!((state.session.last_msg_read, state.session.highest_msg_read), (1, 1));
                } else {
                    assert!(!header.is_read(), "{change}, password={password}");
                    assert!(header.is_receipt_req());
                    assert_eq!(header.date_received, 0);
                    assert_eq!(base.highest_message_number(), if change == "pack" { 1 } else { 2 });
                    assert!(records.is_empty());
                    assert_eq!((state.session.last_msg_read, state.session.highest_msg_read), (0, 0));
                }
                let mut remaining = [0; 4096];
                let count = peer.try_read(&mut remaining).await.unwrap();
                let output = String::from_utf8_lossy(&remaining[..count]);
                assert!(!output.contains("READER-OUTER"), "N reprompted outside: {output}");
            }
        }
    }

    #[test]
    fn displayed_read_compares_full_header_even_without_generation_change() {
        let dir = tempfile::tempdir().unwrap();
        let mut base = JamMessageBase::create(dir.path().join("mail")).unwrap();
        mail(&mut base, "READER", attributes::MSG_RECEIPTREQ);
        let mut snapshot = base.read_transaction(|base| DisplayedMessage::load(base, base.read_header(1)?)).unwrap();
        // Fields omitted by partial identity comparisons must also invalidate
        // the displayed snapshot, even with identical body and generation.
        snapshot.header.reply_to = 42;
        assert_eq!(commit_displayed_read(&mut base, &snapshot, "READER", "", 1, true, true).unwrap(), (false, None));
        assert!(!base.read_header(1).unwrap().is_read());
        assert!(base.read_header(1).unwrap().is_receipt_req());
        assert!(base.read_last_read_file().unwrap().is_empty());
        assert_eq!(base.highest_message_number(), 1);
    }

    fn mail(base: &mut JamMessageBase, to: &str, flags: u32) {
        base.write_message(&JamMessage::default().with_from(BString::from("SENDER"))
            .with_to(BString::from(to)).with_subject(BString::from("Receipt test"))
            .with_text(BString::from("body")).with_date_time(chrono::Utc::now()).with_attributes(flags)).unwrap();
        base.write_jhr_header().unwrap();
    }

    #[test]
    fn successful_read_pointer_is_monotonic_and_has_one_record() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mail");
        let mut base = JamMessageBase::create(&path).unwrap();
        assert_eq!(advance_read_pointer(&mut base, "READER", 1, 3).unwrap(), (3, 3));
        assert_eq!(advance_read_pointer(&mut base, "READER", 1, 1).unwrap(), (3, 3));
        drop(base);
        let base = JamMessageBase::open(path).unwrap();
        let records = base.read_last_read_file().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!((records[0].last_read_msg, records[0].high_read_msg), (3, 3));
    }

    #[test]
    fn recipient_read_date_and_receipt_are_persisted_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mail");
        let mut base = JamMessageBase::create(&path).unwrap();
        mail(&mut base, "READER", attributes::MSG_PRIVATE | attributes::MSG_RECEIPTREQ);
        assert!(!record_recipient_read(&mut base, 1, "SENDER", "").unwrap());
        assert!(!base.read_header(1).unwrap().is_read());
        assert!(record_recipient_read(&mut base, 1, "reader", "").unwrap());
        let received = base.read_header(1).unwrap().date_received;
        assert_ne!(received, 0);
        drop(base);
        let mut base = JamMessageBase::open(path).unwrap();
        assert!(!record_recipient_read(&mut base, 1, "reader", "").unwrap());
        let original = base.read_header(1).unwrap();
        assert!(original.is_read());
        assert!(!original.is_receipt_req());
        assert_eq!(original.date_received, received);
        assert_eq!(base.highest_message_number(), 2);
        let receipt = base.read_header(2).unwrap();
        assert!(receipt.is_private());
        assert!(!receipt.is_receipt_req());
        assert_eq!(receipt.reply_to, 1);
        assert_eq!(receipt.from().unwrap(), &BString::from("RETURN RECEIPT"));
        assert_eq!(receipt.to().unwrap(), &BString::from("SENDER"));
        assert_eq!(receipt.subject(), original.subject());
    }

    #[test]
    fn receipt_retry_does_not_duplicate_an_already_appended_or_deleted_ack() {
        let dir = tempfile::tempdir().unwrap();
        let mut base = JamMessageBase::create(dir.path().join("mail")).unwrap();
        mail(&mut base, "ALIAS", attributes::MSG_RECEIPTREQ);
        assert!(record_recipient_read(&mut base, 1, "READER", "alias").unwrap());
        // Reproduce the persistent state after ack append but failed source clear.
        raw::set_attributes(&mut base, 1, attributes::MSG_RECEIPTREQ, 0).unwrap();
        base.delete_message(2).unwrap();
        assert!(record_recipient_read(&mut base, 1, "READER", "alias").unwrap());
        assert_eq!(base.highest_message_number(), 2);
        assert!(!base.read_header(1).unwrap().is_receipt_req());
    }

    #[test]
    fn concurrent_recipient_reads_only_append_one_receipt() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mail");
        let mut base = JamMessageBase::create(&path).unwrap();
        mail(&mut base, "READER", attributes::MSG_RECEIPTREQ);
        drop(base);
        std::thread::scope(|scope| {
            for _ in 0..4 {
                let path = &path;
                scope.spawn(move || {
                    let mut base = JamMessageBase::open(path).unwrap();
                    record_recipient_read(&mut base, 1, "READER", "").unwrap();
                });
            }
        });
        let base = JamMessageBase::open(path).unwrap();
        assert_eq!(base.highest_message_number(), 2);
        assert!(base.read_header(1).unwrap().is_read());
    }
}
