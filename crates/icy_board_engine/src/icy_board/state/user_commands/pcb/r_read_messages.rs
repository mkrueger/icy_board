use crate::icy_board::{
    icb_text::IceText,
    state::{NodeStatus, functions::display_flags},
};
use jamjam::jam::JamMessageBase;

use crate::{Res, icy_board::state::IcyBoardState};

impl IcyBoardState {
    pub async fn read_messages(&mut self) -> Res<()> {
        self.read_messages_in_area(self.session.current_message_area).await?;
        Ok(())
    }

    pub async fn read_messages_in_area(&mut self, msg_area: usize) -> Res<()> {
        self.set_activity(NodeStatus::HandlingMail).await;
        if !self.session.user_command_level.cmd_r.session_can_access(&self.session)
            || self
                .session
                .current_conference
                .areas
                .as_ref()
                .and_then(|areas| areas.get(msg_area))
                .is_some_and(|area| !area.req_level_to_list.session_can_access(&self.session))
        {
            return Ok(());
        }
        let Some(message_base_file) = self.message_area_path(msg_area) else {
            self.display_text(IceText::PathErrorInSystemConfiguration, display_flags::NEWLINE | display_flags::LFAFTER)
                .await?;
            return Ok(());
        };
        // loop for recreating the message base without async recursion problem.
        let mut tries = 0;
        while tries < 2 {
            tries += 1;
            let message_base_file = message_base_file.clone();
            match JamMessageBase::open(&message_base_file) {
                Ok(message_base) => {
                    let previous_area = self.session.current_message_area;
                    self.session.current_message_area = msg_area;
                    let conference = self.session.current_conference_number;
                    let result = self.read_msgs_from_base(message_base, false).await;
                    if self.session.current_conference_number == conference {
                        self.session.current_message_area = previous_area;
                    }
                    return result;
                }
                Err(err) => {
                    if !message_base_file.with_extension("jhr").exists() {
                        log::error!("Message index load error {err}");
                        log::error!("Creating new message index at {}", message_base_file.display());
                        self.display_text(IceText::CreatingNewMessageIndex, display_flags::NEWLINE | display_flags::LFAFTER)
                            .await?;
                        if JamMessageBase::create(message_base_file).is_ok() {
                            log::error!("successfully created new message index.");
                            continue;
                        }
                    }
                    log::error!("failed to create message index.");

                    self.display_text(IceText::PathErrorInSystemConfiguration, display_flags::NEWLINE | display_flags::LFAFTER)
                        .await?;
                    break;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn message_area_path(&self, msg_area: usize) -> Option<std::path::PathBuf> {
        Some(self.session.current_conference.areas.as_ref()?.get(msg_area)?.path.clone())
    }
}
