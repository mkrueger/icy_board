use crate::{
    menu_runner::{PcbBoardCommand, MASK_COMMAND},
    Res,
};
use icy_board_engine::icy_board::{icb_text::IceText, state::functions::display_flags};

impl PcbBoardCommand {
    pub async fn join_conference(&mut self, help: &str) -> Res<()> {
        if self.state.get_board().await.conferences.is_empty() {
            self.state
                .display_text(IceText::NoConferenceAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            self.state.press_enter().await?;
            return Ok(());
        }
        let conf_number = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            let mnu = self.state.get_board().await.config.paths.conf_join_menu.clone();
            let mnu = self.state.resolve_path(&mnu);

            self.state.display_menu(&mnu).await?;
            self.state.new_line().await?;

            self.state
                .input_field(
                    IceText::JoinConferenceNumber,
                    40,
                    MASK_COMMAND,
                    help,
                    None,
                    display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
                )
                .await?
        };
        if !conf_number.is_empty() {
            let mut joined = false;
            if let Ok(number) = conf_number.parse::<u16>() {
                if (number as usize) <= self.state.get_board().await.conferences.len() {
                    self.state.join_conference(number).await;
                    self.state.session.op_text = format!(
                        "{} ({})",
                        self.state.session.current_conference.name, self.state.session.current_conference_number
                    );
                    self.state
                        .display_text(IceText::ConferenceJoined, display_flags::NEWLINE | display_flags::NOTBLANK)
                        .await?;

                    joined = true;
                }
            }

            if !joined {
                self.state.session.op_text = conf_number;
                self.state
                    .display_text(IceText::InvalidConferenceNumber, display_flags::NEWLINE | display_flags::NOTBLANK)
                    .await?;
            }
        }

        self.state.new_line().await?;
        self.state.press_enter().await?;
        self.display_menu = true;
        Ok(())
    }
}
