use crate::{menu_runner::PcbBoardCommand, Res};
use icy_board_engine::icy_board::{icb_text::IceText, state::functions::display_flags};

impl PcbBoardCommand {
    pub async fn set_expert_mode(&mut self) -> Res<()> {
        self.displaycmdfile("x").await?;

        let mut expert_mode = !self.state.session.expert_mode;
        if let Some(token) = self.state.session.tokens.pop_front() {
            let token = token.to_ascii_uppercase();
            if token == "ON" {
                expert_mode = true;
            } else if token == "OFF" {
                expert_mode = false;
            }
        }
        self.state.session.expert_mode = expert_mode;
        if let Some(user) = &mut self.state.current_user {
            user.flags.expert_mode = expert_mode;
        }
        if expert_mode {
            self.state
                .display_text(
                    IceText::ViewSettingsExpertModeOn,
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
                )
                .await?;
        } else {
            self.state
                .display_text(
                    IceText::ViewSettingsExpertModeOff,
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
                )
                .await?;
            self.state.press_enter().await?;
        }
        self.display_menu = true;
        Ok(())
    }
}
