use crate::{icy_board::state::IcyBoardState, Res};

use crate::icy_board::{
    icb_text::IceText,
    state::{
        functions::{display_flags, MASK_ASCII},
        NodeStatus,
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
        let conf = self.session.current_conference_number;
        let Ok(Some(area)) = self.show_message_areas(conf).await else {
            self.press_enter().await?;
            self.display_current_menu = true;
            return Ok(());
        };

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
            self.new_line().await?;
            self.press_enter().await?;
            self.display_current_menu = true;
            return Ok(());
        };

        self.write_message(
            self.session.current_conference_number as i32,
            area as i32,
            &to,
            &subject,
            false,
            IceText::SavingMessage,
        )
        .await?;

        self.press_enter().await?;
        self.display_current_menu = true;
        Ok(())
    }
}
