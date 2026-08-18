//! The commands that act on the message in front of the reader.

use bstr::BString;
use jamjam::jam::msg_header::{JamMessageHeader, MessageSubfield, SubfieldType};
use jamjam::jam::{JamMessage, JamMessageBase, attributes, raw};

use crate::Res;
use crate::icy_board::icb_text::IceText;
use crate::icy_board::state::IcyBoardState;
use crate::icy_board::state::functions::{MASK_ASCII, MASK_NUM, display_flags};
use crate::icy_board::state::user_commands::pcb::select_conferences::SelectMode;
use crate::vm::TerminalTarget;

use super::read_command::{MsgFunc, ReadCommand};

/// Swaps one variable length header field for a new value.
fn replace_sub_field(header: &mut JamMessageHeader, field: SubfieldType, value: &str) {
    header.sub_fields.retain(|sub_field| sub_field.field_type() != field);
    header.sub_fields.push(MessageSubfield::new(field, BString::from(value)));
}

/// What the read loop should do once the command has run.
pub(super) enum AfterAction {
    /// Command not handled here.
    NotHandled,
    /// Ask for the next command without re-showing the message.
    Prompt,
    /// Show the message again (PCBoard's REREAD).
    Redisplay,
    /// Move on to the next message (PCBoard's READNEXT).
    Next,
    /// Leave the read loop (PCBoard's QUITREAD/QUITLOOP/SKIPNEXT).
    Quit,
}

impl IcyBoardState {
    pub(super) async fn run_read_action(&mut self, cmd: &ReadCommand, message_base: &mut JamMessageBase, number: u32) -> Res<AfterAction> {
        match cmd.func {
            MsgFunc::Kill => {
                self.new_line().await?;
                let sec = self.session.user_command_level.cmd_k.clone();
                if self.check_sec("K", &sec).await? {
                    self.try_to_kill_message(message_base, number).await?;
                }
                Ok(AfterAction::Next)
            }
            MsgFunc::Protect | MsgFunc::Unprotect => {
                self.new_line().await?;
                let protect = cmd.func == MsgFunc::Protect;
                let sec = self.get_board().await.config.sysop_command_level.protect_unprotect_messages.clone();
                if self.check_sec(if protect { "P" } else { "U" }, &sec).await? {
                    let (set, clear) = if protect {
                        (attributes::MSG_PRIVATE, 0)
                    } else {
                        (0, attributes::MSG_PRIVATE)
                    };
                    if let Err(err) = raw::set_attributes(message_base, number, set, clear) {
                        log::error!("Error changing the protection of message {number}: {err}");
                        self.display_text(IceText::MessageBaseError, display_flags::NEWLINE).await?;
                    }
                }
                Ok(AfterAction::Redisplay)
            }
            MsgFunc::Move | MsgFunc::Copy => {
                self.new_line().await?;
                let moving = cmd.func == MsgFunc::Move;
                let sec = self.get_board().await.config.sysop_command_level.copy_move_messages.clone();
                if !self.check_sec(if moving { "MOVE" } else { "COPY" }, &sec).await? {
                    return Ok(AfterAction::Prompt);
                }
                let Some(conference) = self.ask_target_conference(cmd, moving).await? else {
                    return Ok(AfterAction::Prompt);
                };
                let Some(area) = self.ask_target_area(conference).await? else {
                    return Ok(AfterAction::Prompt);
                };
                if !self.copy_message_to_conference(message_base, number, conference, area, moving).await? {
                    return Ok(AfterAction::Prompt);
                }
                if moving {
                    // The move already reported itself, so kill without a second message.
                    let _ = message_base.delete_message(number);
                    return Ok(AfterAction::Next);
                }
                Ok(AfterAction::Redisplay)
            }
            // J remembers nothing here: PCBoard leaves the reader and lets the
            // main prompt run the join with the tokens that follow.
            MsgFunc::Join | MsgFunc::JumpOut => Ok(AfterAction::Quit),
            MsgFunc::Skip => {
                // SKIPEND drags the pointer to the end before leaving, so the
                // conference counts as read, and says where it left it.
                let high = message_base.highest_message_number();
                self.session.last_msg_read = high;
                self.session.highest_msg_read = self.session.highest_msg_read.max(high);
                self.store_last_read(message_base, high).await?;
                self.session.op_text = high.to_string();
                self.display_text(IceText::LastMessageReadSetTo, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                Ok(AfterAction::Quit)
            }
            MsgFunc::EnterMessage => {
                let sec = self.session.user_command_level.cmd_e.clone();
                if self.check_sec("E", &sec).await? {
                    self.enter_message().await?;
                }
                Ok(AfterAction::Quit)
            }
            MsgFunc::Reply | MsgFunc::ReplyOther => {
                self.new_line().await?;
                let sec = self.session.user_command_level.cmd_e.clone();
                if self.check_sec("REPLY", &sec).await? {
                    // The reply command reads the number it answers from the tokens.
                    self.session.tokens.push_front(number.to_string());
                    self.reply_message_command().await?;
                }
                Ok(AfterAction::Redisplay)
            }
            MsgFunc::QuickScan => {
                self.quick_message_scan().await?;
                Ok(AfterAction::Redisplay)
            }
            MsgFunc::SelectConference | MsgFunc::DeselectConference => {
                self.select_conferences(SelectMode::SelectCmd).await?;
                Ok(AfterAction::Next)
            }
            MsgFunc::Chat => {
                let sec = self.session.user_command_level.cmd_chat.clone();
                if self.check_sec("CHAT", &sec).await? {
                    self.group_chat_command().await?;
                    // The message is redrawn over the top, so PCBoard waits here.
                    self.press_enter().await?;
                }
                Ok(AfterAction::Redisplay)
            }
            MsgFunc::Who => {
                let sec = self.session.user_command_level.cmd_who.clone();
                if self.check_sec("WHO", &sec).await? {
                    self.who_display_nodes().await?;
                    self.press_enter().await?;
                }
                Ok(AfterAction::Redisplay)
            }
            MsgFunc::FlagFile => {
                self.flag_files_cmd(true).await?;
                Ok(AfterAction::Redisplay)
            }
            // PCBoard answers the sender or recipient of the message in front of
            // the reader by handing the name to user maintenance.
            MsgFunc::FindTo | MsgFunc::FindFrom => {
                self.new_line().await?;
                let sec = self.get_board().await.config.sysop_command_level.sec_7_user_maint.clone();
                if self.check_sec("F", &sec).await? {
                    let Ok(header) = message_base.read_header(number) else {
                        return Ok(AfterAction::Redisplay);
                    };
                    let name = if cmd.func == MsgFunc::FindTo { header.to() } else { header.from() };
                    if let Some(name) = name {
                        self.session.tokens.push_front(name.to_string());
                    }
                    self.user_maintenance().await?;
                }
                Ok(AfterAction::Redisplay)
            }
            _ => Ok(AfterAction::NotHandled),
        }
    }

    /// Moves this user's last-read pointer for the base in front of the reader.
    async fn store_last_read(&mut self, message_base: &mut JamMessageBase, number: u32) -> Res<()> {
        unsafe {
            let crc = JamMessageBase::crc(&BString::new(self.session.user_name.as_mut_vec().clone()));
            let user_id = self.session.cur_user_id as u32;
            let mut last_read = message_base
                .find_last_read(crc, user_id)?
                .unwrap_or(message_base.create_last_read(crc, user_id)?);
            last_read.last_read_msg = number;
            last_read.high_read_msg = last_read.high_read_msg.max(number);
            message_base.write_last_read(&last_read)?;
        }
        Ok(())
    }

    /// PCBoard asks for the conference only when the command line did not carry one.
    async fn ask_target_conference(&mut self, cmd: &ReadCommand, moving: bool) -> Res<Option<u16>> {
        let num_conferences = self.get_board().await.conferences.len() as u16;
        if let Some(conference) = cmd.move_conf {
            return Ok(if conference < num_conferences { Some(conference) } else { None });
        }
        let prompt = if moving {
            IceText::MovedMessageToConference
        } else {
            IceText::CopyMessageToConference
        };
        let answer = self
            .input_field(prompt, 5, &MASK_NUM, "hlpendr", None, display_flags::NEWLINE | display_flags::LFBEFORE)
            .await?;
        if answer.is_empty() {
            return Ok(None);
        }
        let Ok(conference) = answer.parse::<u16>() else {
            return Ok(None);
        };
        Ok(if conference < num_conferences { Some(conference) } else { None })
    }

    /// A conference is split into message areas in icy_board, which PCBoard has
    /// no notion of. A board shaped the way PCBoard expects has one area per
    /// conference, so nothing extra is asked and a PPE stuffing the keyboard
    /// still sees the prompts it was written for.
    async fn ask_target_area(&mut self, conference: u16) -> Res<Option<usize>> {
        let areas = self
            .get_board()
            .await
            .conferences
            .get(conference as usize)
            .and_then(|conference| conference.areas.clone())
            .unwrap_or_default();
        if areas.len() <= 1 {
            return Ok(Some(0));
        }
        for (i, area) in areas.iter().enumerate() {
            self.print(TerminalTarget::Both, &format!("{:>3}) {}", i + 1, area.name)).await?;
            self.new_line().await?;
        }
        let answer = self
            .input_field(
                IceText::JoinAreaNumber,
                5,
                &MASK_NUM,
                "",
                Some("1".to_string()),
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::FIELDLEN | display_flags::GUIDE,
            )
            .await?;
        if answer.is_empty() {
            return Ok(Some(0));
        }
        match answer.parse::<usize>() {
            Ok(number) if number >= 1 && number <= areas.len() => Ok(Some(number - 1)),
            _ => {
                self.session.op_text = answer;
                self.display_text(IceText::InvalidAreaNumber, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                Ok(None)
            }
        }
    }

    async fn copy_message_to_conference(&mut self, message_base: &JamMessageBase, number: u32, conference: u16, area: usize, moving: bool) -> Res<bool> {
        let Ok(header) = message_base.read_header(number) else {
            self.display_text(IceText::NoSuchMessageNumber, display_flags::NEWLINE).await?;
            return Ok(false);
        };
        let text = message_base.read_message_text(&header)?;

        let mut msg = JamMessage::default()
            .with_from(header.from().cloned().unwrap_or_default())
            .with_to(header.to().cloned().unwrap_or_default())
            .with_subject(header.subject().cloned().unwrap_or_default())
            .with_date_time(chrono::Utc::now())
            .with_attributes(header.attributes)
            .with_text(BString::from(text));
        if header.reply_to != 0 {
            msg = msg.with_reply_to(header.reply_to);
        }

        let text = if moving { IceText::MessageMoved } else { IceText::MessageCopied };
        self.send_message(conference as i32, area as i32, msg, text).await?;
        Ok(true)
    }

    /// E inside the read loop: change one field of the header
    /// in front of the reader. The option letter and the follow-up question are
    /// what a stuffing PPE counts on, so they come in PCBoard's order.
    pub(super) async fn edit_header(&mut self, message_base: &mut JamMessageBase, number: u32) -> Res<()> {
        self.new_line().await?;
        let sec = self.session.user_command_level.cmd_e.clone();
        if !self.check_sec("E", &sec).await? {
            return Ok(());
        }
        let Ok(mut header) = message_base.read_header(number) else {
            self.display_text(IceText::NoSuchMessageNumber, display_flags::NEWLINE).await?;
            return Ok(());
        };

        let from = header.from().map(|f| f.to_string()).unwrap_or_default();
        let edit_all = self.get_board().await.config.sysop_command_level.edit_any_message.clone();
        let own = from.eq_ignore_ascii_case(&self.session.user_name) || from.eq_ignore_ascii_case(&self.session.alias_name);
        if !own && !edit_all.session_can_access(&self.session) {
            self.display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(());
        }

        let to = header.to().map(|f| f.to_string()).unwrap_or_default();
        let subject = header.subject().map(|f| f.to_string()).unwrap_or_default();
        self.display_text(IceText::To, display_flags::DEFAULT).await?;
        self.println(TerminalTarget::Both, &to).await?;
        self.display_text(IceText::From, display_flags::DEFAULT).await?;
        self.println(TerminalTarget::Both, &from).await?;
        self.display_text(IceText::Subject, display_flags::DEFAULT).await?;
        self.println(TerminalTarget::Both, &subject).await?;

        let echo_mail = self.session.current_conference.echo_mail_in_conference;
        let option = self
            .input_field(
                if echo_mail { IceText::EditHeaderEcho } else { IceText::EditHeader },
                1,
                if echo_mail { "EFNPRST" } else { "FNPRST" },
                "",
                None,
                display_flags::FIELDLEN | display_flags::UPCASE | display_flags::NEWLINE | display_flags::LFBEFORE,
            )
            .await?;

        let (old, len) = match option.as_str() {
            "" => return Ok(()),
            "E" => {
                header.attributes ^= attributes::MSG_TYPEECHO;
                return self.write_header(message_base, number, &header).await;
            }
            "R" => {
                let sec = self.get_board().await.config.sysop_command_level.edit_message_headers.clone();
                if !self.check_sec("R", &sec).await? {
                    return Ok(());
                }
                if header.attributes & attributes::MSG_READ == 0 {
                    return Ok(());
                }
                header.attributes &= !attributes::MSG_READ;
                return self.write_header(message_base, number, &header).await;
            }
            "P" => {
                let sec = self.get_board().await.config.sysop_command_level.protect_unprotect_messages.clone();
                if !self.check_sec("P", &sec).await? {
                    return Ok(());
                }
                let current = if header.needs_password() {
                    "S"
                } else if header.attributes & attributes::MSG_PRIVATE != 0 {
                    "R"
                } else {
                    "N"
                };
                let answer = self
                    .input_field(
                        IceText::MessageSecurity,
                        1,
                        "NRSG",
                        "hlpsec",
                        Some(current.to_string()),
                        display_flags::UPCASE | display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                    )
                    .await?;
                match answer.as_str() {
                    "N" => {
                        header.attributes &= !attributes::MSG_PRIVATE;
                        header.password_crc = JamMessageBase::crc(&BString::from(""));
                    }
                    "R" => header.attributes |= attributes::MSG_PRIVATE,
                    "S" | "G" => {
                        header.attributes |= attributes::MSG_PRIVATE;
                        let password = self
                            .input_field(
                                IceText::SecurityPassword,
                                12,
                                &MASK_ASCII,
                                "hlpe",
                                None,
                                display_flags::FIELDLEN | display_flags::UPCASE | display_flags::NEWLINE | display_flags::HIGHASCII,
                            )
                            .await?;
                        header.password_crc = JamMessageBase::crc(&BString::from(password.as_str()));
                    }
                    _ => return Ok(()),
                }
                return self.write_header(message_base, number, &header).await;
            }
            "N" => (header.reply_to.to_string(), 9),
            "T" => (to.clone(), 25),
            "F" => {
                let sec = self.get_board().await.config.sysop_command_level.edit_message_headers.clone();
                if !self.check_sec("F", &sec).await? {
                    return Ok(());
                }
                (from.clone(), 25)
            }
            "S" => (subject.clone(), 60),
            _ => return Ok(()),
        };

        let answer = self
            .input_field(
                IceText::NewInfo,
                len,
                &MASK_ASCII,
                "",
                Some(old.clone()),
                display_flags::FIELDLEN | display_flags::HIGHASCII | display_flags::NEWLINE | display_flags::LFBEFORE,
            )
            .await?;
        let answer = answer.trim().to_string();
        if answer.is_empty() || answer == old {
            return Ok(());
        }

        match option.as_str() {
            "N" => match answer.parse::<u32>() {
                Ok(reply_to) => header.reply_to = reply_to,
                Err(_) => return Ok(()),
            },
            "T" => replace_sub_field(&mut header, SubfieldType::RecvName, &answer),
            "F" => {
                let answer = answer.to_ascii_uppercase();
                if answer.contains("@USER@") {
                    self.display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                    return Ok(());
                }
                replace_sub_field(&mut header, SubfieldType::SenderName, &answer);
            }
            "S" => replace_sub_field(&mut header, SubfieldType::Subject, &answer),
            _ => return Ok(()),
        }
        self.write_header(message_base, number, &header).await
    }

    async fn write_header(&mut self, message_base: &mut JamMessageBase, number: u32, header: &JamMessageHeader) -> Res<()> {
        if let Err(err) = raw::update_header(message_base, number, header) {
            log::error!("Error writing the header of message {number}: {err}");
            self.display_text(IceText::MessageBaseError, display_flags::NEWLINE).await?;
        }
        Ok(())
    }

    /// R's SET command: move this conference's last-read pointer.
    pub(super) async fn set_last_message_read(&mut self, cmd: &ReadCommand, message_base: &mut JamMessageBase) -> Res<()> {
        let low = message_base.lowest_message_number();
        let high = message_base.highest_message_number();

        let number = match cmd.new_last_read {
            Some(number) => number,
            None => {
                self.session.op_text = format!("{low}-{high}");
                let answer = self
                    .input_field(
                        IceText::SetLastMessageReadPointer,
                        9,
                        &MASK_NUM,
                        "",
                        Some(self.session.last_msg_read.to_string()),
                        display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::FIELDLEN | display_flags::GUIDE,
                    )
                    .await?;
                if answer.is_empty() {
                    return Ok(());
                }
                match answer.parse::<i64>() {
                    Ok(number) => number,
                    Err(_) => return Ok(()),
                }
            }
        };

        let number = number.clamp(0, high as i64) as u32;
        unsafe {
            let crc = JamMessageBase::crc(&BString::new(self.session.user_name.as_mut_vec().clone()));
            let mut last_read = message_base
                .find_last_read(crc, self.session.cur_user_id as u32)?
                .unwrap_or(message_base.create_last_read(crc, self.session.cur_user_id as u32)?);
            last_read.last_read_msg = number;
            message_base.write_last_read(&last_read)?;
        }
        self.session.last_msg_read = number;

        self.session.op_text = number.to_string();
        self.display_text(IceText::LastMessageReadSetTo, display_flags::NEWLINE | display_flags::LFBEFORE)
            .await?;
        Ok(())
    }
}
