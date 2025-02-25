use crate::{Res, icy_board::state::IcyBoardState};

use crate::icy_board::{
    icb_text::IceText,
    state::{
        NodeStatus,
        functions::{MASK_ASCII, display_flags},
    },
};

impl IcyBoardState {
    pub async fn enter_message(&mut self) -> Res<()> {
        if self.session.current_conference.is_read_only {
            self.display_text(
                IceText::ConferenceIsReadOnly,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::BELL,
            )
            .await?;
            return Ok(());
        }
        self.set_activity(NodeStatus::EnterMessage).await;
        let mut to = self
            .input_field(
                IceText::MessageTo,
                54,
                &MASK_ASCII,
                "",
                Some("ALL".to_string()),
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::FIELDLEN,
            )
            .await?;

        if to.is_empty() {
            to = "ALL".to_string();
        };

        let subject = self
            .input_field(
                IceText::MessageSubject,
                54,
                &MASK_ASCII,
                "",
                None,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::FIELDLEN,
            )
            .await?;

        if subject.is_empty() {
            return Ok(());
        };

        self.write_message(
            self.session.current_conference_number as i32,
            self.session.current_message_area as i32,
            &to,
            &subject,
            false,
            IceText::SavingMessage,
        )
        .await?;
        Ok(())
    }
}
