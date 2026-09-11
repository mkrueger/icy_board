use bstr::BString;
use jamjam::jam::{
    JamMessage, JamMessageBase, attributes,
    msg_header::{JamMessageHeader, MessageSubfield, SubfieldType},
};

use crate::icy_board::commands::CommandType;
use crate::icy_board::state::functions::MASK_ALPHA;
use crate::icy_board::state::user_commands::mods::{editor::EditResult, messagereader::message_security::{may_read_header, requires_read_password}};
use super::e_enter_message::MessageOptions;
use crate::icy_board::{
    icb_text::IceText,
    state::{
        NodeStatus,
        functions::{MASK_ASCII, display_flags},
    },
};
use crate::{Res, icy_board::state::IcyBoardState};

fn reply_details(header: &JamMessageHeader) -> (String, String, u32, Vec<MessageSubfield>) {
    let to = header.from().map(ToString::to_string).unwrap_or_default();
    let subject = header.subject().map(ToString::to_string).unwrap_or_default();
    let mut attributes = header.attributes & attributes::MSG_PRIVATE;
    if header.attributes & attributes::MSG_TYPENET != 0 {
        attributes |= attributes::MSG_PRIVATE;
    }
    let mut fields = Vec::new();
    for field in &header.sub_fields {
        let kind = match field.field_type() {
            SubfieldType::Address0 => SubfieldType::AddressD,
            SubfieldType::MsgID => SubfieldType::ReplyID,
            _ => continue,
        };
        fields.push(MessageSubfield::new(kind, BString::from(field.content().to_vec())));
    }
    (to, subject, attributes, fields)
}

// A number/CRC alone is not an identity: pack can reuse the number, and legacy
// messages need not have a MsgID. Be conservative about edits/relocations, while
// allowing recipient read status and counters to change during composition.
pub(crate) fn same_reply_source(fresh: &JamMessageHeader, original: &JamMessageHeader) -> bool {
    !fresh.is_deleted()
        && fresh.message_number == original.message_number
        && fresh.msgid_crc == original.msgid_crc
        && fresh.reply_crc == original.reply_crc
        && fresh.reply_to == original.reply_to
        && fresh.date_written == original.date_written
        && fresh.offset == original.offset
        && fresh.txt_len == original.txt_len
        && fresh.password_crc == original.password_crc
        && fresh.attributes & !attributes::MSG_READ == original.attributes & !attributes::MSG_READ
        && fresh.attributes2 == original.attributes2
        && fresh.cost == original.cost
        && fresh.sub_fields.len() == original.sub_fields.len()
        && fresh.sub_fields.iter().zip(&original.sub_fields)
            .all(|(a, b)| a.field_type() == b.field_type() && a.content() == b.content())
}

fn reply_header(base: &JamMessageBase, number: u32) -> jamjam::Result<Option<JamMessageHeader>> {
    match base.read_header(number) {
        Ok(header) => Ok(Some(header)),
        Err(jamjam::Error::Jam(jamjam::jam::JamError::MessageDeleted | jamjam::jam::JamError::MessageNumberOutOfRange(..))) => Ok(None),
        Err(error) => Err(error),
    }
}

fn authorized_reply_header(header: &JamMessageHeader, user: &str, alias: &str, read_all: bool) -> bool {
    !header.is_deleted() && header.attributes & attributes::MSG_NODISP == 0 && may_read_header(header, user, alias, read_all)
}

/// Called only after any password prompt has completed. No body is read until
/// the fresh header's identity and authorization match the approved header.
fn read_authorized_reply(
    base: &mut JamMessageBase,
    approved: &JamMessageHeader,
    user: &str,
    alias: &str,
    read_all: bool,
) -> jamjam::Result<Option<(JamMessageHeader, BString)>> {
    base.read_transaction(|base| {
        let Some(fresh) = reply_header(base, approved.message_number)? else { return Ok(None) };
        if !same_reply_source(&fresh, approved) || !authorized_reply_header(&fresh, user, alias, read_all) {
            return Ok(None);
        }
        let body = base.read_message_text(&fresh)?;
        Ok(Some((fresh, body)))
    })
}

fn reply_link(source: &std::path::Path, destination: &std::path::Path, number: u32) -> u32 {
    // Compare the actual JAM header files, including aliases/symlinks. Failure
    // to identify the destination must not create a cross-base numeric link.
    match (source.with_extension("jhr").canonicalize(), destination.with_extension("jhr").canonicalize()) {
        (Ok(source), Ok(destination)) if source == destination => number,
        _ => 0,
    }
}

fn record_reply_date(base: &mut JamMessageBase, source: &JamMessageHeader, body: &BString) -> jamjam::Result<()> {
    base.transaction(|base| {
        let Some(mut original) = reply_header(base, source.message_number)? else { return Ok(()) };
        if !same_reply_source(&original, source) || base.read_message_text(&original)? != *body {
            return Ok(());
        }
        // JAM has no reply-date member. date_received is a receipt, not a reply.
        original.sub_fields.retain(|field| !(field.field_type() == SubfieldType::FTSKludge
            && field.content().to_string().starts_with("ICYBOARD-REPLY-DATE: ")));
        original.sub_fields.push(MessageSubfield::new(SubfieldType::FTSKludge,
            BString::from(format!("ICYBOARD-REPLY-DATE: {}", chrono::Utc::now().to_rfc3339()))));
        jamjam::jam::raw::update_header(base, source.message_number, &original)
    })
}

impl IcyBoardState {
    pub(crate) fn message_reply_defaults(&self, header: &JamMessageHeader) -> crate::icy_board::state::ppl_message::MessageHeader {
        let (mut to, subject, attributes, _) = reply_details(header);
        if to.eq_ignore_ascii_case(&self.session.user_name) || (!self.session.alias_name.is_empty() && to.eq_ignore_ascii_case(&self.session.alias_name)) {
            to = header.to().map(ToString::to_string).unwrap_or_default();
        }
        crate::icy_board::state::ppl_message::MessageHeader {
            from: self.session.get_username_or_alias(), to, subject, is_private: attributes & attributes::MSG_PRIVATE != 0,
        }
    }

    pub(crate) async fn authorized_message_snapshot(&mut self, source: &crate::icy_board::state::ppl_message::PplMessage) -> Res<Option<JamMessage>> {
        let expected = source.stored_header.as_ref().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid MSG"))?;
        let mut base = JamMessageBase::open(&source.path)?;
        let header = base.read_transaction(|base| base.read_header(source.number))?;
        if !same_reply_source(&header, expected) {
            return Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "Message changed since it was read").into());
        }
        let read_all = self.get_board().await.config.sysop_command_level.read_all_mail.session_can_access(&self.session);
        if !authorized_reply_header(&header, &self.session.user_name, &self.session.alias_name, read_all) {
            return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Message access denied").into());
        }
        if requires_read_password(&header, read_all)
            && !self.check_password(IceText::PasswordToReadMessage, 0, |password| header.is_password_valid(password)).await? {
            if self.session.request_logoff { return Ok(None); }
            return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Message password denied").into());
        }
        if self.session.request_logoff { return Ok(None); }
        let Some((header, body)) = read_authorized_reply(&mut base, &header, &self.session.user_name, &self.session.alias_name, read_all)? else {
            return Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "Message changed during authorization").into());
        };
        Ok(Some(JamMessage::from_stored(header, body)))
    }

    pub async fn get_ret_receipt(&mut self) -> Res<bool> {
        let input = self
            .input_field(
                IceText::RequireReturnReceipt,
                1,
                &MASK_ALPHA,
                "",
                Some(self.session.no_char.to_string()),
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::FIELDLEN | display_flags::YESNO,
            )
            .await?;
        Ok(input == self.session.yes_char.to_uppercase().to_string())
    }

    pub async fn reply_message_command(&mut self) -> Res<()> {
        if self.session.current_conference.is_read_only {
            self.display_text(
                IceText::ConferenceIsReadOnly,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::BELL,
            )
            .await?;
            return Ok(());
        }
        loop {
            self.set_activity(NodeStatus::EnterMessage).await;
            let msg_num = if let Some(token) = self.session.tokens.pop_front() {
                token
            } else {
                self.input_field(
                    IceText::ReplyToMessages,
                    54,
                    &MASK_ASCII,
                    CommandType::ReplyMessage.get_help(),
                    None,
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::STACKED,
                )
                .await?
            };

            if msg_num.is_empty() {
                self.display_text(IceText::MessageAborted, display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::BELL)
                    .await?;
                return Ok(());
            }

            let Ok(msg_number) = msg_num.parse::<u32>() else {
                self.display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                continue;
            };

            self.reply_current_message(msg_number, false).await?;
            return Ok(());
        }
    }

    /// Compatibility entry point for callers of the standalone reply command.
    pub async fn reply_command(&mut self) -> Res<()> {
        self.reply_message_command().await
    }

    /// Reply/RO to a selected area message. Rechecks read/write/password access
    /// before exposing either its header defaults or its quote text. SendKill
    /// confirms deletion here; the reader owns navigation only.
    pub(crate) async fn reply_current_message(&mut self, number: u32, ask_other: bool) -> Res<EditResult> {
        let Some(area) = self.session.current_conference.areas.as_ref().and_then(|areas| areas.get(self.session.current_message_area)) else {
            self.display_text(IceText::NoMailFound, display_flags::NEWLINE).await?;
            return Ok(EditResult::Abort);
        };
        let path = self.resolve_path(&area.path);
        self.reply_from_base(&path, number, ask_other, false).await
    }

    /// Reply to a number in the reader's actual source base. Email replies go to
    /// the shared email base, not the currently selected conference/area.
    pub(crate) async fn reply_from_base(
        &mut self,
        base_path: &std::path::Path,
        number: u32,
        ask_other: bool,
        email: bool,
    ) -> Res<EditResult> {
        let mut saved = None;
        match self.reply_from_base_with_defaults(base_path, number, ask_other, email, None, "", None, &mut saved).await {
            Err(error) if saved.is_none() && error.is::<super::message_attachment::MessageCreditDenied>() => Ok(EditResult::Abort),
            result => result,
        }
    }

    pub(crate) async fn reply_from_base_with_defaults(
        &mut self,
        base_path: &std::path::Path,
        number: u32,
        ask_other: bool,
        email: bool,
        defaults: Option<&crate::icy_board::state::ppl_message::MessageHeader>,
        initial_text: &str,
        expected: Option<&JamMessageHeader>,
        saved: &mut Option<crate::icy_board::state::ppl_message::PplMessage>,
    ) -> Res<EditResult> {
        let conference = self.session.current_conference.clone();
        if !email && conference.is_read_only {
            self.display_text(IceText::ConferenceIsReadOnly, display_flags::NEWLINE | display_flags::BELL).await?;
            return Ok(EditResult::Abort);
        }
        let command_sec = self.session.user_command_level.cmd_e.clone();
        if !self.check_sec("REPLY", &command_sec).await? || (!email && !self.check_sec("REPLY", &conference.sec_write_message).await?) {
            return Ok(EditResult::Abort);
        }
        let (destination_conf, destination_area, destination_path) = if email {
            (-1, 0, self.email_msgbase_path().await)
        } else {
            let Some(area) = conference.areas.as_ref().and_then(|areas| areas.get(self.session.current_message_area)) else {
                self.display_text(IceText::NoMailFound, display_flags::NEWLINE).await?;
                return Ok(EditResult::Abort);
            };
            if area.is_read_only {
                self.display_text(IceText::ConferenceIsReadOnly, display_flags::NEWLINE).await?;
                return Ok(EditResult::Abort);
            }
            if !self.check_sec("REPLY", &area.req_level_to_enter).await? || !self.check_sec("REPLY", &area.req_level_to_list).await? {
                return Ok(EditResult::Abort);
            }
            (self.session.current_conference_number as i32, self.session.current_message_area as i32, self.resolve_path(&area.path))
        };
        let may_read_all = self.get_board().await.config.sysop_command_level.read_all_mail.session_can_access(&self.session);
        let mut base = match JamMessageBase::open(base_path) {
            Ok(base) => base,
            Err(error) => {
                if expected.is_some() { return Err(error.into()); }
                self.display_text(IceText::MessageBaseError, display_flags::NEWLINE).await?;
                return Ok(EditResult::Abort);
            }
        };
        let header = base.read_transaction(|base| {
            Ok(reply_header(base, number)?.filter(|header|
                authorized_reply_header(header, &self.session.user_name, &self.session.alias_name, may_read_all)))
        })?;
        let Some(header) = header else {
            if expected.is_some() { return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Message unavailable or access denied").into()); }
            self.display_text(IceText::NoMailFound, display_flags::NEWLINE).await?;
            return Ok(EditResult::Abort);
        };
        if expected.is_some_and(|expected| !same_reply_source(&header, expected)) {
            return Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "Reply source changed since it was read").into());
        }
        if requires_read_password(&header, may_read_all)
            && !self.check_password(IceText::PasswordToReadMessage, 0, |password| header.is_password_valid(password)).await? {
            if expected.is_some() && !self.session.request_logoff { return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Message password denied").into()); }
            return Ok(EditResult::Abort);
        }
        if self.session.request_logoff { return Ok(EditResult::Abort); }
        let Some((header, body)) = read_authorized_reply(&mut base, &header, &self.session.user_name, &self.session.alias_name, may_read_all)? else {
            if expected.is_some() { return Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "Reply source changed during authorization").into()); }
            self.display_text(IceText::NoMailFound, display_flags::NEWLINE).await?;
            return Ok(EditResult::Abort);
        };
        // Cached by the same read transaction as header/body, before any await.
        let source_generation = base.mod_counter();
        let numeric_link = reply_link(base_path, &destination_path, number);
        drop(base);
        let from = if let Some(defaults) = defaults { self.get_message_sender(&defaults.from).await? }
            else { self.session.get_username_or_alias() };
        if self.session.request_logoff { return Ok(EditResult::Abort); }
        let (mut to, mut subject, inherited_attributes, mut fields) = reply_details(&header);
        let own = to.eq_ignore_ascii_case(&self.session.user_name)
            || (!self.session.alias_name.is_empty() && to.eq_ignore_ascii_case(&self.session.alias_name));
        if own || ask_other {
            to = header.to().map(ToString::to_string).unwrap_or_default();
            fields.retain(|field| field.field_type() != SubfieldType::AddressD);
            if let Some(address) = header.sub_fields.iter().find(|field| field.field_type() == SubfieldType::AddressD) {
                fields.push(address.clone());
            }
        }
        if let Some(defaults) = defaults {
            if defaults.to != to {
                fields.retain(|field| field.field_type() != SubfieldType::AddressD);
                if defaults.to.contains('@') { fields.push(MessageSubfield::new(SubfieldType::AddressD, defaults.to.clone().into())); }
            }
            to = defaults.to.clone();
            subject = defaults.subject.clone();
        }
        if ask_other || defaults.is_some() {
            let old_to = to.clone();
            let Some(recipient) = self.get_message_recipient(IceText::MessageTo, to, false).await? else { return Ok(EditResult::Abort) };
            to = recipient;
            if to != old_to {
                fields.retain(|field| field.field_type() != SubfieldType::AddressD);
                if to.contains('@') { fields.push(MessageSubfield::new(SubfieldType::AddressD, BString::from(to.clone()))); }
            }
            // Carbon lists are collected by E, not by reader RO header changes.
            if to.eq_ignore_ascii_case("@LIST@") {
                self.display_text(IceText::InvalidEntry, display_flags::NEWLINE).await?;
                return Ok(EditResult::Abort);
            }
        }
        if !conference.long_to_names || ask_other || defaults.is_some() {
            let answer = self.input_field(IceText::NewSubject, if subject.chars().count() > 60 { 120 } else { 60 }, &MASK_ASCII, "", Some(subject.clone()),
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::FIELDLEN).await?;
            if !answer.is_empty() { subject = answer; }
        }
        let group_password = header.needs_password() && requires_read_password(&header, false) && !ask_other;
        let mut options = if group_password {
            let private = email || defaults.is_some_and(|header| header.is_private && !to.eq_ignore_ascii_case("ALL") && !conference.disallow_private_msgs);
            let mut options = MessageOptions { attributes: if private { attributes::MSG_PRIVATE } else { 0 }, password: None, packout_date: None, sub_fields: Vec::new() };
            self.get_message_delivery_options(to.eq_ignore_ascii_case("ALL"), private, &mut options).await?;
            options
        } else {
            self.get_message_options_with_default(to.eq_ignore_ascii_case("ALL"), email || (defaults.is_none() && !ask_other && inherited_attributes & attributes::MSG_PRIVATE != 0), defaults.map(|header| header.is_private)).await?
        };
        if options.sub_fields.iter().any(|field| field.field_type() == SubfieldType::AddressD) {
            fields.retain(|field| field.field_type() != SubfieldType::AddressD);
        }
        fields.append(&mut options.sub_fields);
        let mut message = JamMessage::default()
            .with_from(BString::from(from))
            .with_to(BString::from(to))
            .with_subject(BString::from(subject))
            .with_reply_to(numeric_link)
            .with_date_time(chrono::Utc::now())
            .with_attributes(options.attributes | attributes::MSG_LOCAL);
        if let Some(password) = options.password { message = message.with_password(&BString::from(password)); }
        if let Some(date) = options.packout_date { message = message.with_packout_date(date); }
        for field in fields {
            if field.field_type() == SubfieldType::ReplyID { message = message.with_reply_id(field.content().clone()); }
            else { message = message.with_sub_field(field); }
        }
        if group_password {
            let mut reply_header = message.header().clone();
            reply_header.password_crc = header.password_crc;
            message = JamMessage::from_stored(reply_header, BString::default());
        }
        message = message.with_text(initial_text.into());
        self.set_activity(NodeStatus::EnterMessage).await;
        let result = self.write_message_with_result(destination_conf, destination_area,
            message, body.to_string().replace("\r\n", "\n").replace('\r', "\n").lines().map(str::to_string).collect(), IceText::SavingMessage, true, saved).await?;
        if result == EditResult::SendKill {
            // SK saves one ordinary reply: send_message writes the JAM base
            // header once for append and once explicitly. Successful attachment
            // saves return SendMessage, and carbon copies return CarbonCopy.
            // Admit only our own writes, never refresh away a concurrent pack
            // or edit. Do not stamp reply-date before comparing the snapshot.
            let generation = source_generation.wrapping_add(if numeric_link != 0 { 2 } else { 0 });
            let mut base = JamMessageBase::open(base_path)?;
            let killed = self.try_to_kill_reply_source(&mut base, JamMessage::from_stored(header, body), generation).await?;
            return Ok(if killed { EditResult::SendKill } else { EditResult::SendMessage });
        }
        if result != EditResult::Abort {
            let mut base = JamMessageBase::open(base_path)?;
            record_reply_date(&mut base, &header, &body)?;
        }
        Ok(result)
    }
}

#[cfg(test)]
#[path = "e_enter_message_tests.rs"]
mod entry_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_netmail_reply_goes_back_to_its_sender_and_keeps_the_network_thread() {
        let mut header = JamMessageHeader {
            attributes: attributes::MSG_TYPENET | attributes::MSG_PRIVATE,
            ..Default::default()
        };
        header
            .sub_fields
            .push(MessageSubfield::new(SubfieldType::SenderName, BString::from("Remote Sysop")));
        header
            .sub_fields
            .push(MessageSubfield::new(SubfieldType::RecvName, BString::from("Local Sysop")));
        header.sub_fields.push(MessageSubfield::new(SubfieldType::Subject, BString::from("Hello")));
        header.sub_fields.push(MessageSubfield::new(SubfieldType::Address0, BString::from("21:1/2")));
        header
            .sub_fields
            .push(MessageSubfield::new(SubfieldType::MsgID, BString::from("21:1/2 abcdef01")));

        let (to, subject, reply_attributes, fields) = reply_details(&header);

        assert_eq!(to, "Remote Sysop");
        assert_eq!(subject, "Hello");
        assert_ne!(reply_attributes & attributes::MSG_PRIVATE, 0);
        assert_eq!(reply_attributes & attributes::MSG_TYPENET, 0);
        assert!(
            fields
                .iter()
                .any(|field| field.field_type() == SubfieldType::AddressD && field.content() == "21:1/2")
        );
        assert!(
            fields
                .iter()
                .any(|field| field.field_type() == SubfieldType::ReplyID && field.content() == "21:1/2 abcdef01")
        );
    }
}
