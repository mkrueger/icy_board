use icy_board_engine::icy_board::{
    commands::Command,
    icb_text::IceText,
    state::{functions::display_flags, UserActivity},
};
use jamjam::jam::JamMessageBase;

use crate::{menu_runner::PcbBoardCommand, Res};

impl PcbBoardCommand {
    pub async fn read_messages(&mut self, action: &Command) -> Res<()> {
        self.state.set_activity(UserActivity::ReadMessages);
        let Ok(Some(area)) = self.state.show_message_areas(self.state.session.current_conference_number, &action.help).await else {
            self.state.press_enter().await?;
            self.display_menu = true;
            return Ok(());
        };
        self.read_messages_in_area(area, action).await
    }

    async fn read_messages_in_area(&mut self, area: usize, action: &Command) -> Res<()> {
        // loop for recreating the message base without async recursion problem.
        let mut tries = 0;
        while tries < 2 {
            tries += 1;
            let message_base_file = &self.state.session.current_conference.areas[area].filename;
            let msgbase_file_resolved = self.state.get_board().await.resolve_file(message_base_file);
            match JamMessageBase::open(&msgbase_file_resolved) {
                Ok(message_base) => {
                    self.read_msgs_from_base(message_base, action).await?;
                    return Ok(());
                }
                Err(err) => {
                    if !msgbase_file_resolved.with_extension("jhr").exists() {
                        log::error!("Message index load error {}", err);
                        log::error!("Creating new message index at {}", msgbase_file_resolved.display());
                        self.state
                            .display_text(IceText::CreatingNewMessageIndex, display_flags::NEWLINE | display_flags::LFAFTER)
                            .await?;
                        if JamMessageBase::create(msgbase_file_resolved).is_ok() {
                            log::error!("successfully created new message index.");
                            continue;
                        }
                    }
                    log::error!("failed to create message index.");

                    self.state
                        .display_text(IceText::PathErrorInSystemConfiguration, display_flags::NEWLINE | display_flags::LFAFTER)
                        .await?;
                    break;
                }
            }
        }
        self.state.press_enter().await?;
        self.display_menu = true;
        return Ok(());
    }
}
