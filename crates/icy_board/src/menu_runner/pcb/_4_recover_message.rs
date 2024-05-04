use crate::{
    menu_runner::{PcbBoardCommand, MASK_COMMAND},
    Res,
};
use icy_board_engine::{
    icy_board::{commands::Command, icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};
use jamjam::jam::JamMessageBase;

impl PcbBoardCommand {
    pub async fn restore_message(&mut self, action: &Command) -> Res<()> {
        let message_base_file = &self.state.session.current_conference.areas[0].filename;
        let msgbase_file_resolved = self.state.get_board().await.resolve_file(message_base_file);

        match JamMessageBase::open(&msgbase_file_resolved) {
            Ok(message_base) => {
                let msg = if let Some(token) = self.state.session.tokens.pop_front() {
                    token
                } else {
                    self.state.session.op_text = format!("{}-{}", message_base.base_messagenumber(), message_base.active_messages());

                    self.state
                        .input_field(
                            IceText::MessageNumberToActivate,
                            40,
                            MASK_COMMAND,
                            &action.help,
                            None,
                            display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
                        )
                        .await?
                };

                if let Ok(number) = msg.parse::<u32>() {
                    self.try_to_restore_message(&message_base, number).await?;
                }

                self.state.press_enter().await?;
                self.display_menu = true;
                Ok(())
            }
            Err(err) => {
                log::error!("Message index load error {}", err);
                log::error!("Creating new message index at {}", &msgbase_file_resolved.display());
                self.state
                    .display_text(IceText::CreatingNewMessageIndex, display_flags::NEWLINE | display_flags::LFAFTER)
                    .await?;
                if JamMessageBase::create(msgbase_file_resolved).is_ok() {
                    log::error!("successfully created new message index.");
                    return self.read_messages(action).await;
                }
                log::error!("failed to create message index.");

                self.state
                    .display_text(IceText::PathErrorInSystemConfiguration, display_flags::NEWLINE | display_flags::LFAFTER)
                    .await?;

                self.state.press_enter().await?;
                self.display_menu = true;
                Ok(())
            }
        }
    }

    async fn try_to_restore_message(&mut self, message_base: &JamMessageBase, number: u32) -> Res<()> {
        match message_base.restore_message(number) {
            Ok(_) => {
                log::error!("Restore message {} ({})", number, message_base.get_filename().display());
                self.state.display_text(IceText::MessageRestored, display_flags::DEFAULT).await?;
                self.state.print(TerminalTarget::Both, &format!("{}", number)).await?;
                self.state.new_line().await?;
                self.state.new_line().await?;
            }
            Err(err) => {
                log::error!("Error restoring message:{} ({})/ {}", number, message_base.get_filename().display(), err);
                self.state.display_text(IceText::NoSuchMessageNumber, display_flags::DEFAULT).await?;
                self.state.print(TerminalTarget::Both, &format!("{}", number)).await?;
                self.state.new_line().await?;
                self.state.new_line().await?;
            }
        }
        Ok(())
    }
}
