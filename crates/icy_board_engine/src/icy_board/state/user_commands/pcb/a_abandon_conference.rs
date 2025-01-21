use crate::icy_board::{icb_text::IceText, state::functions::display_flags};
use crate::{icy_board::state::IcyBoardState, Res};

impl IcyBoardState {
    pub async fn abandon_conference(&mut self) -> Res<()> {
        if self.get_board().await.conferences.is_empty() {
            self.display_text(IceText::NoConferenceAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            self.press_enter().await?;
            return Ok(());
        }
        if self.session.current_conference_number > 0 {
            self.session.op_text = format!("{} ({})", self.session.current_conference.name, self.session.current_conference_number);
            self.join_conference(0, false).await;
            self.display_text(IceText::ConferenceAbandoned, display_flags::NEWLINE | display_flags::NOTBLANK)
                .await?;
            self.new_line().await?;
            self.press_enter().await?;
        }
        self.display_current_menu = true;
        Ok(())
    }
}
