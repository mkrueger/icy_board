use crate::icy_board::commands::CommandType;
use crate::icy_board::state::functions::MASK_COMMAND;
use crate::icy_board::state::IcyBoardState;
use crate::Res;
use crate::{
    icy_board::{icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};
use jamjam::jam::JamMessageBase;

impl IcyBoardState {
    pub async fn delete_message(&mut self) -> Res<()> {
        let message_base_file = self.session.current_conference.areas.as_ref().unwrap()[0].filename.clone();

        match JamMessageBase::open(&message_base_file) {
            Ok(message_base) => {
                let msg = if let Some(token) = self.session.tokens.pop_front() {
                    token
                } else {
                    self.session.op_text = format!("{}-{}", message_base.base_messagenumber(), message_base.active_messages());

                    self.input_field(
                        IceText::MessageNumberToKill,
                        40,
                        MASK_COMMAND,
                        CommandType::DeleteMessage.get_help(),
                        None,
                        display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
                    )
                    .await?
                };

                if let Ok(number) = msg.parse::<u32>() {
                    self.try_to_kill_message(&message_base, number).await?;
                }
                Ok(())
            }
            Err(err) => {
                log::error!("Message index load error {}", err);
                log::error!("Creating new message index at {}", message_base_file.display());
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

    async fn try_to_kill_message(&mut self, message_base: &JamMessageBase, number: u32) -> Res<()> {
        if let Ok(header) = message_base.read_header(number) {
            if header.needs_password()
                && !self
                    .check_password(IceText::PasswordToReadMessage, 0, |pwd| header.is_password_valid(pwd))
                    .await?
            {
                return Ok(());
            }
        }

        match message_base.delete_message(number) {
            Ok(_) => {
                log::error!("Deleted message {} ({})", number, message_base.get_filename().display());
                self.display_text(IceText::MessageKilled, display_flags::DEFAULT).await?;
                self.print(TerminalTarget::Both, &format!("{}", number)).await?;
                self.new_line().await?;
                self.new_line().await?;
            }
            Err(err) => {
                log::error!("Error deleting message:{} ({})/ {}", number, message_base.get_filename().display(), err);
                self.display_text(IceText::NoSuchMessageNumber, display_flags::DEFAULT).await?;
                self.print(TerminalTarget::Both, &format!("{}", number)).await?;
                self.new_line().await?;
                self.new_line().await?;
            }
        }
        Ok(())
    }
}
