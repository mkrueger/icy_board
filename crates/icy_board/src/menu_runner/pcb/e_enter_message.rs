use crate::{menu_runner::PcbBoardCommand, Res};

use icy_board_engine::icy_board::{
    icb_text::IceText,
    state::{
        functions::{display_flags, MASK_ASCII},
        UserActivity,
    },
};

impl PcbBoardCommand {
    pub async fn enter_message(&mut self, help: &str) -> Res<()> {
        if self.state.session.current_conference.is_read_only {
            self.state
                .display_text(
                    IceText::ConferenceIsReadOnly,
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::BELL,
                )
                .await?;
            return Ok(());
        }
        self.state.set_activity(UserActivity::EnterMessage).await;
        let conf = self.state.session.current_conference_number;
        let Ok(Some(area)) = self.state.show_message_areas(conf, help).await else {
            self.state.press_enter().await?;
            self.display_menu = true;
            return Ok(());
        };

        let mut to = self
            .state
            .input_field(
                IceText::MessageTo,
                54,
                &MASK_ASCII,
                help,
                Some("ALL".to_string()),
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::FIELDLEN,
            )
            .await?;

        if to.is_empty() {
            to = "ALL".to_string();
        };

        let subject = self
            .state
            .input_field(
                IceText::MessageSubject,
                54,
                &MASK_ASCII,
                help,
                None,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::FIELDLEN,
            )
            .await?;

        if subject.is_empty() {
            self.state.new_line().await?;
            self.state.press_enter().await?;
            self.display_menu = true;
            return Ok(());
        };

        self.write_message(
            self.state.session.current_conference_number as i32,
            area as i32,
            &to,
            &subject,
            false,
            IceText::SavingMessage,
        )
        .await?;

        self.state.press_enter().await?;
        self.display_menu = true;
        Ok(())
    }
}
