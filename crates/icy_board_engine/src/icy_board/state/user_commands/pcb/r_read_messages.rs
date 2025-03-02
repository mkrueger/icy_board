use crate::icy_board::{
    icb_text::IceText,
    state::{NodeStatus, functions::display_flags},
};
use jamjam::jam::JamMessageBase;

use crate::{Res, icy_board::state::IcyBoardState};

impl IcyBoardState {
    pub async fn read_messages(&mut self) -> Res<()> {
        self.set_activity(NodeStatus::HandlingMail).await;
        // loop for recreating the message base without async recursion problem.
        let mut tries = 0;
        while tries < 2 {
            tries += 1;
            let message_base_file = self.session.current_conference.areas.as_ref().unwrap()[self.session.current_message_area]
                .path
                .clone();
            match JamMessageBase::open(&message_base_file) {
                Ok(message_base) => {
                    self.read_msgs_from_base(message_base, false).await?;
                    return Ok(());
                }
                Err(err) => {
                    if !message_base_file.with_extension("jhr").exists() {
                        log::error!("Message index load error {}", err);
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
        return Ok(());
    }
}
