use jamjam::jam::JamMessageBase;

use crate::icy_board::commands::CommandType;
use crate::icy_board::state::functions::{MASK_ALNUM, MASK_ALPHA};
use crate::{Res, icy_board::state::IcyBoardState};

use crate::icy_board::{
    icb_text::IceText,
    state::{
        NodeStatus,
        functions::{MASK_ASCII, display_flags},
    },
};

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
                    &MASK_ALNUM,
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
            };

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
                self.session.current_conference.areas.as_ref().unwrap()[self.session.current_message_area as usize].name,
                self.session.current_message_area
            );
            self.println(crate::vm::TerminalTarget::Both, &area_str).await?;
            let msg_base = self.get_board().await.conferences[conf as usize].areas.as_ref().unwrap()[area as usize]
                .filename
                .clone();

            let mut subject = String::new();
            let mut to = String::new();

            if let Ok(base) = JamMessageBase::open(msg_base) {
                if let Ok(msg) = base.read_header(msg_number) {
                    if let Some(s) = msg.get_to() {
                        to = s.to_string();
                    }
                    if let Some(s) = msg.get_subject() {
                        subject = s.to_string();
                    }
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
            };
            let ret_receipt = self.get_ret_receipt().await?;

            self.write_message(
                self.session.current_conference_number as i32,
                self.session.current_message_area as i32,
                &to,
                &new_subject,
                ret_receipt,
                IceText::SavingMessage,
            )
            .await?;
            return Ok(());
        }
    }
}
