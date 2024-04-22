use icy_board_engine::icy_board::{
    commands::Command,
    icb_text::IceText,
    state::{functions::display_flags, UserActivity},
};
use jamjam::jam::JamMessageBase;

use crate::{menu_runner::PcbBoardCommand, Res};

impl PcbBoardCommand {
    pub fn read_messages(&mut self, action: &Command) -> Res<()> {
        self.state.node_state.lock().unwrap().user_activity = UserActivity::ReadMessages;
        let Ok(Some(area)) = self.show_message_areas(action) else {
            self.state.press_enter()?;
            self.display_menu = true;
            return Ok(());
        };
        self.read_messages_in_area(area, action)
    }

    fn read_messages_in_area(&mut self, area: usize, action: &Command) -> Res<()> {
        let message_base_file = &self.state.session.current_conference.areas[area].filename;
        let msgbase_file_resolved = self.state.board.lock().unwrap().resolve_file(message_base_file);
        match JamMessageBase::open(&msgbase_file_resolved) {
            Ok(message_base) => {
                self.read_msgs_from_base(message_base, action)?;
                Ok(())
            }
            Err(err) => {
                log::error!("Message index load error {}", err);
                log::error!("Creating new message index at {}", msgbase_file_resolved.display());
                self.state
                    .display_text(IceText::CreatingNewMessageIndex, display_flags::NEWLINE | display_flags::LFAFTER)?;
                if JamMessageBase::create(msgbase_file_resolved).is_ok() {
                    log::error!("successfully created new message index.");
                    return self.read_messages_in_area(area, action);
                }
                log::error!("failed to create message index.");

                self.state
                    .display_text(IceText::PathErrorInSystemConfiguration, display_flags::NEWLINE | display_flags::LFAFTER)?;

                self.state.press_enter()?;
                self.display_menu = true;
                Ok(())
            }
        }
    }
}
