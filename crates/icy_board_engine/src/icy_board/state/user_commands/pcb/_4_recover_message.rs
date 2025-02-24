use crate::icy_board::commands::CommandType;
use crate::icy_board::state::functions::MASK_COMMAND;
use crate::{icy_board::state::IcyBoardState, Res};
use crate::{
    icy_board::{icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};
use jamjam::jam::JamMessageBase;

impl IcyBoardState {
    pub async fn restore_message(&mut self) -> Res<()> {
        let Some(areas) = &self.session.current_conference.areas else {
            return Ok(());
        };
        let message_base_file = areas[0].filename.clone();

        match JamMessageBase::open(&message_base_file) {
            Ok(message_base) => {
                let msg = if let Some(token) = self.session.tokens.pop_front() {
                    token
                } else {
                    self.session.op_text = format!("{}-{}", message_base.base_messagenumber(), message_base.active_messages());

                    self.input_field(
                        IceText::MessageNumberToActivate,
                        40,
                        MASK_COMMAND,
                        &CommandType::RestoreMessage.get_help(),
                        None,
                        display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
                    )
                    .await?
                };

                if let Ok(number) = msg.parse::<u32>() {
                    self.try_to_restore_message(&message_base, number).await?;
                }

                Ok(())
            }
            Err(err) => {
                log::error!("Message index load error {}", err);
                log::error!("Creating new message index at {}", &message_base_file.display());
                self.display_text(IceText::CreatingNewMessageIndex, display_flags::NEWLINE | display_flags::LFAFTER)
                    .await?;
                if JamMessageBase::create(message_base_file).is_ok() {
                    log::error!("successfully created new message index.");
                    return self.read_messages().await;
                }
                log::error!("failed to create message index.");

                self.display_text(IceText::PathErrorInSystemConfiguration, display_flags::NEWLINE | display_flags::LFAFTER)
                    .await?;

                Ok(())
            }
        }
    }

    async fn try_to_restore_message(&mut self, message_base: &JamMessageBase, number: u32) -> Res<()> {
        match message_base.restore_message(number) {
            Ok(_) => {
                log::error!("Restore message {} ({})", number, message_base.get_filename().display());
                self.display_text(IceText::MessageRestored, display_flags::DEFAULT).await?;
                self.print(TerminalTarget::Both, &format!("{}", number)).await?;
                self.new_line().await?;
                self.new_line().await?;
            }
            Err(err) => {
                log::error!("Error restoring message:{} ({})/ {}", number, message_base.get_filename().display(), err);
                self.display_text(IceText::NoSuchMessageNumber, display_flags::DEFAULT).await?;
                self.print(TerminalTarget::Both, &format!("{}", number)).await?;
                self.new_line().await?;
                self.new_line().await?;
            }
        }
        Ok(())
    }
}
