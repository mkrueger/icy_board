use bstr::BString;
use jamjam::jam::{
    JamMessageBase, attributes,
    msg_header::{JamMessageHeader, MessageSubfield, SubfieldType},
};

use crate::icy_board::commands::CommandType;
use crate::icy_board::state::functions::MASK_ALPHA;
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

impl IcyBoardState {
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

            let conf = self.session.current_conference_number;
            let area = self.session.current_message_area;
            self.display_text(IceText::Scanning, display_flags::DEFAULT).await?;
            let area_str = format!(
                "{} ({})",
                self.session.current_conference.areas.as_ref().unwrap()[self.session.current_message_area].name,
                self.session.current_message_area
            );
            self.println(crate::vm::TerminalTarget::Both, &area_str).await?;
            let msg_base = self.get_board().await.conferences[conf as usize].areas.as_ref().unwrap()[area].path.clone();

            let mut subject = String::new();
            let mut to = String::new();
            let mut msg_attributes = 0;
            let mut sub_fields = Vec::new();

            if let Ok(base) = JamMessageBase::open(msg_base) {
                if let Ok(msg) = base.read_header(msg_number) {
                    (to, subject, msg_attributes, sub_fields) = reply_details(&msg);
                } else {
                    self.display_text(IceText::NoMailFound, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                    continue;
                }
            }
            let mut new_subject = self
                .input_field(
                    IceText::NewSubject,
                    50,
                    &MASK_ASCII,
                    "",
                    Some(subject.clone()),
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::FIELDLEN,
                )
                .await?;

            if new_subject.is_empty() {
                new_subject = subject;
            }
            let ret_receipt = self.get_ret_receipt().await?;
            if ret_receipt {
                msg_attributes |= attributes::MSG_RECEIPTREQ;
            }

            self.write_message(
                self.session.current_conference_number as i32,
                self.session.current_message_area as i32,
                &to,
                &new_subject,
                msg_attributes,
                None,
                None,
                sub_fields,
                IceText::SavingMessage,
            )
            .await?;
            return Ok(());
        }
    }
}

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
