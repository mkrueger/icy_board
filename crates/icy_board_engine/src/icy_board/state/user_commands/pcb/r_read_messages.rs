use crate::icy_board::{
    icb_text::IceText,
    state::{functions::display_flags, NodeStatus},
};
use jamjam::jam::JamMessageBase;

use crate::{icy_board::state::IcyBoardState, Res};

impl IcyBoardState {
    pub async fn read_messages(&mut self) -> Res<()> {
        self.set_activity(NodeStatus::HandlingMail).await;
        // loop for recreating the message base without async recursion problem.
        let mut tries = 0;
        while tries < 2 {
            tries += 1;
            let message_base_file = &self.session.current_conference.areas.as_ref().unwrap()[self.session.current_message_area].filename;
            let msgbase_file_resolved = self.get_board().await.resolve_file(message_base_file);
            match JamMessageBase::open(&msgbase_file_resolved) {
                Ok(message_base) => {
                    self.read_msgs_from_base(message_base, false).await?;
                    return Ok(());
                }
                Err(err) => {
                    if !msgbase_file_resolved.with_extension("jhr").exists() {
                        log::error!("Message index load error {}", err);
                        log::error!("Creating new message index at {}", msgbase_file_resolved.display());
                        self.display_text(IceText::CreatingNewMessageIndex, display_flags::NEWLINE | display_flags::LFAFTER)
                            .await?;
                        if JamMessageBase::create(msgbase_file_resolved).is_ok() {
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
