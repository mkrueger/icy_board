use crate::icy_board::{icb_text::IceText, state::functions::display_flags};
use crate::{icy_board::state::IcyBoardState, Res};

impl IcyBoardState {
    pub async fn set_expert_mode(&mut self) -> Res<()> {
        self.displaycmdfile("x").await?;

        let mut expert_mode = !self.session.expert_mode();
        if let Some(token) = self.session.tokens.pop_front() {
            let token = token.to_ascii_uppercase();
            if token == "ON" {
                expert_mode = true;
            } else if token == "OFF" {
                expert_mode = false;
            } else {
                return Ok(());
            }
        }
        if let Some(user) = &mut self.session.current_user {
            user.flags.expert_mode = expert_mode;
        }
        if expert_mode {
            self.display_text(IceText::ExpertmodeOn, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
        } else {
            self.display_text(IceText::ExpertmodeOff, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
        }
        Ok(())
    }
}
