use crate::{menu_runner::PcbBoardCommand, Res};
use icy_board_engine::icy_board::{icb_text::IceText, state::functions::display_flags};

impl PcbBoardCommand {
    pub async fn abandon_conference(&mut self) -> Res<()> {
        if self.state.get_board().await.conferences.is_empty() {
            self.state
                .display_text(IceText::NoConferenceAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            self.state.press_enter().await?;
            return Ok(());
        }
        if self.state.session.current_conference_number > 0 {
            self.state.session.op_text = format!(
                "{} ({})",
                self.state.session.current_conference.name, self.state.session.current_conference_number
            );
            self.state.join_conference(0).await;
            self.state
                .display_text(IceText::ConferenceAbandoned, display_flags::NEWLINE | display_flags::NOTBLANK)
                .await?;
            self.state.new_line().await?;
            self.state.press_enter().await?;
        }
        self.display_menu = true;
        Ok(())
    }
}
