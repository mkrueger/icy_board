use crate::icy_board::state::user_commands::mods::editor::{EditResult, EditState};
use crate::{Res, icy_board::state::IcyBoardState};

use crate::icy_board::user_base::FSEMode;
use crate::{
    datetime::IcbTime,
    icy_board::{
        icb_text::IceText,
        state::{GraphicsMode, NodeStatus, functions::display_flags},
    },
};
use bstr::BString;
use chrono::{DateTime, Utc};
use icy_engine::Position;
use jamjam::jam::{JamMessage, attributes, msg_header::{MessageSubfield, SubfieldType}};

fn message_text(lines: &[String], allow_esc_codes: bool) -> String {
    let mut text = lines.join("\n");
    if !allow_esc_codes {
        text.retain(|ch| ch != '\u{1b}' && ch != '\u{1d}');
    }
    text
}

fn new_message_id() -> BString {
    BString::from(format!("icyboard {:032x}", fastrand::u128(..)))
}

/// A copy has independent storage/identity and never inherits another recipient's
/// network address or receipt state. Thread and security metadata are retained.
fn carbon_copy(message: &JamMessage, recipient: &str) -> JamMessage {
    let mut header = message.header().clone();
    header.set_to(BString::from(recipient));
    header.message_number = 0;
    header.offset = 0;
    header.txt_len = 0;
    header.times_read = 0;
    header.date_received = 0;
    header.date_processed = 0;
    header.reply_first = 0;
    header.reply_next = 0;
    header.msgid_crc = 0;
    header.attributes &= !(attributes::MSG_READ | attributes::MSG_SENT | attributes::MSG_DELETED);
    if recipient.eq_ignore_ascii_case("ALL") {
        header.attributes &= !(attributes::MSG_PRIVATE | attributes::MSG_RECEIPTREQ);
    }
    header.sub_fields.retain(|field| !matches!(field.field_type(), SubfieldType::AddressD | SubfieldType::MsgID));
    if recipient.contains('@') {
        header.sub_fields.push(MessageSubfield::new(SubfieldType::AddressD, BString::from(recipient)));
    }
    JamMessage::from_stored(header, message.text().clone()).with_msg_id(new_message_id())
}

impl IcyBoardState {
    /// A negative conference number selects the recipient's mailbox instead of a message area.
    async fn comment_target(&mut self) -> (i32, i32) {
        if self.get_board().await.config.message.force_comments_to_main {
            (0, 0)
        } else {
            (-1, 0)
        }
    }

    pub async fn password_failure_comment(&mut self) -> Res<()> {
        let answer = self
            .input_field(
                IceText::WrongPasswordComment,
                1,
                "",
                "",
                Some(self.session.no_char.to_string()),
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::FIELDLEN | display_flags::YESNO,
            )
            .await?;
        if answer != self.session.yes_char.to_uppercase().to_string() {
            return Ok(());
        }

        let to = self.get_board().await.config.sysop.name.clone();
        let subject = self.get_display_text(IceText::WrongPasswordSubject)?;
        let (conf, area) = self.comment_target().await;
        self.write_message(
            conf,
            area,
            &to,
            subject.trim(),
            attributes::MSG_PRIVATE,
            None,
            None,
            Vec::new(),
            IceText::SavingComment,
        )
        .await
    }

    pub async fn comment_to_sysop(&mut self) -> Res<()> {
        let leave_comment = self
            .input_field(
                IceText::LeaveComment,
                1,
                "",
                "",
                Some(self.session.no_char.to_string()),
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::FIELDLEN | display_flags::YESNO,
            )
            .await?;

        if leave_comment.is_empty() || leave_comment.chars().next().unwrap() == self.session.no_char {
            return Ok(());
        }

        self.enter_comment_to_sysop().await?;

        Ok(())
    }

    pub async fn enter_comment_to_sysop(&mut self) -> Res<()> {
        let to = self.get_board().await.config.sysop.name.clone();
        let (conf, area) = self.comment_target().await;
        let subj = format!("COMMENT {}", IcbTime::now());
        self.set_activity(NodeStatus::HandlingMail).await;
        let mut msg_attributes = attributes::MSG_PRIVATE;
        if self.session.current_conference.sec_request_rr.session_can_access(&self.session) && self.get_ret_receipt().await? {
            msg_attributes |= attributes::MSG_RECEIPTREQ;
        }
        self.write_message(conf, area, &to, &subj, msg_attributes, None, None, Vec::new(), IceText::SavingComment)
            .await?;

        Ok(())
    }

    pub async fn write_message(
        &mut self,
        conf: i32,
        area: i32,
        to: &str,
        subj: &str,
        attributes: u32,
        password: Option<String>,
        packout_date: Option<DateTime<Utc>>,
        sub_fields: Vec<MessageSubfield>,
        text: IceText,
    ) -> Res<()> {
        let mut message = JamMessage::default()
            .with_from(BString::from(self.session.get_username_or_alias()))
            .with_to(BString::from(to))
            .with_subject(BString::from(subj))
            .with_date_time(Utc::now())
            .with_attributes(attributes | attributes::MSG_LOCAL);
        if let Some(password) = password {
            message = message.with_password(&BString::from(password));
        }
        if let Some(date) = packout_date {
            message = message.with_packout_date(date);
        }
        for field in sub_fields {
            message = message.with_sub_field(field);
        }
        if to.contains('@') && !to.eq_ignore_ascii_case("@LIST@")
            && !message.header().sub_fields.iter().any(|field| field.field_type() == SubfieldType::AddressD) {
            message = message.with_sub_field(MessageSubfield::new(SubfieldType::AddressD, BString::from(to)));
        }
        if let Err(error) = self.write_message_context(conf, area, message, Vec::new(), text, true).await {
            if error.is::<super::message_attachment::MessageCreditDenied>() { return Ok(()); }
            if error.is::<super::message_attachment::MessagePersistedError>() { return Err(error); }
            // The interactive compose command acknowledges a failed save;
            // storage callers such as MOVE must never wait here or see success.
            self.press_enter().await?;
            return Err(error);
        }
        Ok(())
    }

    /// Edit in place without writing a base. Callers editing an existing message
    /// own authorization and replacement; all header metadata is preserved.
    /// Quote text must have passed the caller's read/password checks.
    pub(crate) async fn edit_message_context(&mut self, message: &mut JamMessage, quote_text: Vec<String>) -> Res<EditResult> {
        self.displaycmdfile("preedit").await?;

        // PCBoard asks whether to use the full screen editor unless the user has a
        // fixed preference (msgeditor()/TXT_USEFULLSCREEN). Skipping this prompt made
        // stuffed answers (e.g. from a PPE) leak into the message body.
        let use_fse = match self.session.fse_mode {
            FSEMode::Yes => true,
            FSEMode::No => false,
            FSEMode::Ask => self.prompt_use_fse().await?,
        };

        let mut editor = EditState {
            from: message.from().map(ToString::to_string).unwrap_or_default(),
            to: message.to().map(ToString::to_string).unwrap_or_default(),
            subj: message.header().subject().map(ToString::to_string).unwrap_or_default(),
            editor_details: None,
            msg: message.text().to_string().lines().map(str::to_string).collect(),
            quote_text,
            cursor: Position::new(0, 0),
            use_fse,
            insert_mode: use_fse,
            top_line: 0,
            max_line_length: 79,
            max_lines: self.get_board().await.config.message.max_msg_lines.max(1) as usize,
        };

        let external = self.get_board().await.config.message.external_editor.clone();
        loop {
            let result = if use_fse && external.mode != crate::icy_board::icb_config::ExternalEditorMode::Internal {
                let area = self.session.current_conference.areas.as_ref()
                    .and_then(|areas| areas.get(self.session.current_message_area as usize)).map(|area| area.name.clone()).unwrap_or_default();
                self.run_external_editor(&external, &mut editor, &area, message.header().attributes & attributes::MSG_PRIVATE != 0).await?
            } else {
                editor.edit_message(self).await?
            };
            if result != EditResult::Abort {
                let body = message_text(&editor.msg, self.get_board().await.config.message.allow_esc_codes);
                let mut header = message.header().clone();
                header.set_from(BString::from(editor.from.clone()));
                header.set_to(BString::from(editor.to.clone()));
                header.set_subject(BString::from(editor.subj.clone()));
                if let Some(details) = &editor.editor_details {
                    header.sub_fields.retain(|field| field.field_type() != SubfieldType::PID);
                    header.sub_fields.push(MessageSubfield::new(SubfieldType::PID, BString::from(details.clone())));
                }
                *message = JamMessage::from_stored(header, BString::from(body));
            }
            if result != EditResult::AttachFile { return Ok(result); }
            if self.attach_message_file(message).await? { return Ok(EditResult::SendMessage); }
            if self.session.request_logoff { return Ok(EditResult::Abort); }
            // Denial/cancellation returns to composition with the SAME editor,
            // including its unsaved body, header fields, and cursor position.
        }
    }

    /// Compose and persist a NEW message, returning SN/SK unchanged for the reader.
    /// Set allow_carbon_copy=false for forwarding/editing contexts. Comments always
    /// suppress copies, irrespective of this argument or the board setting.
    pub(crate) async fn write_message_context(
        &mut self,
        conf: i32,
        area: i32,
        message: JamMessage,
        quote_text: Vec<String>,
        text: IceText,
        allow_carbon_copy: bool,
    ) -> Res<EditResult> {
        let mut saved = None;
        match self.write_message_with_result(conf, area, message, quote_text, text, allow_carbon_copy, &mut saved).await {
            Err(error) if saved.is_none() && error.is::<super::message_attachment::MessageCreditDenied>() => Ok(EditResult::Abort),
            result => result,
        }
    }

    pub(crate) async fn write_message_with_result(
        &mut self,
        conf: i32,
        area: i32,
        mut message: JamMessage,
        quote_text: Vec<String>,
        text: IceText,
        allow_carbon_copy: bool,
        saved: &mut Option<crate::icy_board::state::ppl_message::PplMessage>,
    ) -> Res<EditResult> {
        if !self.message_write_allowed(conf, &message).await? {
            if self.session.request_logoff { return Ok(EditResult::Abort); }
            return Err(super::message_attachment::MessageCreditDenied.into());
        }
        let mut attachments = self.message_attachment_cleanup(&message);
        let result = self.edit_message_context(&mut message, quote_text).await?;
        attachments.track(&message)?;
        if result == EditResult::Abort || self.session.request_logoff {
            return Ok(EditResult::Abort);
        }
        if !message.header().sub_fields.iter().any(|field| field.field_type() == SubfieldType::MsgID) {
            message = message.with_msg_id(new_message_id());
        }
        let copies = result == EditResult::CarbonCopy
            && allow_carbon_copy
            && !matches!(text, IceText::SavingComment)
            && message.to().is_none_or(|to| to.to_string().chars().count() <= 25)
            && self.get_board().await.config.message.allow_carbon_copy;
        let mut header = message.header().clone();
        let mut list = Vec::new();
        header.sub_fields.retain(|field| {
            if field.field_type() == SubfieldType::FTSKludge && let Some(to) = field.content().to_string().strip_prefix("ICYBOARD-CARBON-TO: ") {
                list.push(to.to_string());
                false
            } else { true }
        });
        message = JamMessage::from_stored(header, message.text().clone());
        if message.to().is_some_and(|to| to.eq_ignore_ascii_case(b"@LIST@")) {
            if list.is_empty() || !allow_carbon_copy || matches!(text, IceText::SavingComment)
                || !self.session.current_conference.sec_carbon_copy.session_can_access(&self.session) {
                return Ok(EditResult::Abort);
            }
            for to in list.into_iter().take(self.session.current_conference.carbon_list_limit as usize) {
                self.send_message_with_result(conf, area, carbon_copy(&message, &to), text, &mut attachments, saved).await?;
            }
        } else {
            let original = JamMessage::from_stored(message.header().clone(), message.text().clone());
            self.send_message_with_result(conf, area, original, text, &mut attachments, saved).await?;
        }
        if copies {
            while let Some(recipient) = self.get_message_recipient(IceText::CarbonCopyTo, String::new(), true).await? {
                let copy = carbon_copy(&message, &recipient);
                self.send_message_with_result(conf, area, copy, text, &mut attachments, saved).await?;
                if recipient.chars().count() > 25 { break; }
            }
        }
        Ok(result)
    }

    /// Asks the user whether to use the full screen editor (mirrors `PCBoard`'s
    /// `msgeditor()` `TXT_USEFULLSCREEN` prompt). Only called when the user's editor
    /// preference is "ask".
    async fn prompt_use_fse(&mut self) -> Res<bool> {
        let ansi = self.session.disp_options.grapics_mode != GraphicsMode::Ctty;
        let default = if self.session.expert_mode() && ansi {
            self.session.yes_char
        } else {
            self.session.no_char
        };
        let mut answer = self
            .input_field(
                IceText::UseFullScreen,
                1,
                "",
                "",
                Some(default.to_string()),
                display_flags::YESNO | display_flags::NEWLINE | display_flags::UPCASE | display_flags::FIELDLEN,
            )
            .await?;

        // The full screen editor requires ANSI - re-ask if selected without it.
        let yes = self.session.yes_char.to_uppercase().to_string();
        if answer == yes && !ansi {
            self.display_text(IceText::RequiresAnsi, display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER)
                .await?;
            answer = self
                .input_field(
                    IceText::UseFullScreen,
                    1,
                    "",
                    "",
                    Some(self.session.no_char.to_string()),
                    display_flags::YESNO | display_flags::NEWLINE | display_flags::UPCASE | display_flags::FIELDLEN,
                )
                .await?;
        }
        Ok(answer == yes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jamjam::jam::JamMessageBase;

    #[test]
    fn escape_codes_are_removed_when_disabled() {
        assert_eq!(message_text(&["A\u{1b}[31mB\u{1d}C".to_string()], false), "A[31mBC");
    }

    #[test]
    fn escape_codes_are_kept_when_enabled() {
        assert_eq!(message_text(&["A\u{1b}[31mB\u{1d}C".to_string()], true), "A\u{1b}[31mB\u{1d}C");
    }

    #[test]
    fn carbon_copy_keeps_security_thread_expiry_but_resets_storage_and_recipient_state() {
        let original = JamMessage::default()
            .with_from(BString::from("AUTHOR"))
            .with_to(BString::from("FIRST"))
            .with_subject(BString::from("Subject"))
            .with_reply_to(42)
            .with_reply_id(BString::from("parent-id"))
            .with_msg_id(BString::from("original-id"))
            .with_password(&BString::from("SECRET"))
            .with_packout_date(Utc::now())
            .with_attributes(attributes::MSG_LOCAL | attributes::MSG_PRIVATE | attributes::MSG_RECEIPTREQ | attributes::MSG_READ | attributes::MSG_SENT)
            .with_sub_field(MessageSubfield::new(SubfieldType::AddressD, BString::from("old@example.org")))
            .with_sub_field(MessageSubfield::new(SubfieldType::FTSKludge, BString::from("ICYBOARD-SECURITY: S")))
            .with_text(BString::from("The complete body"));
        let copy = carbon_copy(&original, "new@example.org");
        assert_eq!(copy.text(), original.text());
        assert_eq!(copy.reply_to(), 42);
        assert_eq!(copy.header().reply_crc, original.header().reply_crc);
        assert!(copy.header().is_password_valid("SECRET"));
        assert!(copy.header().is_private());
        assert!(copy.header().is_receipt_req());
        assert_eq!(copy.header().attributes & (attributes::MSG_READ | attributes::MSG_SENT), 0);
        assert_ne!(copy.msgid_crc(), original.msgid_crc());
        assert_eq!(copy.header().message_number, 0);
        assert_eq!(copy.header().times_read, 0);
        assert_eq!(copy.header().date_received, 0);
        assert!(copy.header().sub_fields.iter().any(|field| field.field_type() == SubfieldType::PackoutDate));
        assert!(copy.header().sub_fields.iter().any(|field| field.content() == "ICYBOARD-SECURITY: S"));
        assert!(!copy.header().sub_fields.iter().any(|field| field.content() == "old@example.org"));
        assert!(copy.header().sub_fields.iter().any(|field| field.field_type() == SubfieldType::AddressD && field.content() == "new@example.org"));
        assert_eq!(original.to().unwrap().to_string(), "FIRST");
        assert!(original.header().is_read());
    }

    #[test]
    fn carbon_copy_to_all_does_not_create_private_or_receipt_requested_broadcast() {
        let message = JamMessage::default().with_attributes(attributes::MSG_PRIVATE | attributes::MSG_RECEIPTREQ);
        let copy = carbon_copy(&message, "ALL");
        assert!(!copy.header().is_private());
        assert!(!copy.header().is_receipt_req());
    }

    #[test]
    fn carbon_copies_have_independent_persistent_indexes_bodies_and_ids() {
        let temp = tempfile::tempdir().unwrap();
        let mut base = JamMessageBase::create(temp.path().join("copies")).unwrap();
        let message = JamMessage::default().with_from(BString::from("AUTHOR")).with_text(BString::from("body"));
        for to in ["ALICE", "BOB", "CAROL"] {
            base.write_message(&carbon_copy(&message, to)).unwrap();
        }
        let first = base.read_header(1).unwrap();
        let second = base.read_header(2).unwrap();
        let third = base.read_header(3).unwrap();
        assert_ne!(first.offset, second.offset);
        assert_ne!(second.offset, third.offset);
        assert_ne!(first.msgid_crc, second.msgid_crc);
        assert_ne!(second.msgid_crc, third.msgid_crc);
        base.delete_message(2).unwrap();
        assert!(!base.read_header(1).unwrap().is_deleted());
        assert!(base.read_header(2).is_err());
        assert!(!base.read_header(3).unwrap().is_deleted());
        assert_eq!(base.read_message_text(&first).unwrap(), BString::from("body"));
        assert_eq!(base.search_to(&BString::from("CAROL")).unwrap(), vec![3]);
    }
}
