use crate::{menu_runner::PcbBoardCommand, Res};

use icy_board_engine::icy_board::{
    commands::Command,
    icb_text::IceText,
    state::{
        functions::{display_flags, MASK_ASCII},
        UserActivity,
    },
};

impl PcbBoardCommand {
    pub fn enter_message(&mut self, action: &Command) -> Res<()> {
        if self.state.session.current_conference.is_read_only {
            self.state.display_text(
                IceText::ConferenceIsReadOnly,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::BELL,
            )?;
            return Ok(());
        }

        self.state.node_state.lock().unwrap().user_activity = UserActivity::EnterMessage;
        let Ok(Some(area)) = self.show_message_areas(action) else {
            self.state.press_enter()?;
            self.display_menu = true;
            return Ok(());
        };

        let mut to = self.state.input_field(
            IceText::MessageTo,
            54,
            &MASK_ASCII,
            &action.help,
            Some("ALL".to_string()),
            display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::FIELDLEN,
        )?;

        if to.is_empty() {
            to = "ALL".to_string();
        };

        let subject = self.state.input_field(
            IceText::MessageSubject,
            54,
            &MASK_ASCII,
            &action.help,
            None,
            display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::FIELDLEN,
        )?;

        if subject.is_empty() {
            self.state.new_line()?;
            self.state.press_enter()?;
            self.display_menu = true;
            return Ok(());
        };

        self.write_message(
            self.state.session.current_conference_number,
            area as i32,
            &to,
            &subject,
            false,
            IceText::SavingMessage,
        )?;

        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }
}
