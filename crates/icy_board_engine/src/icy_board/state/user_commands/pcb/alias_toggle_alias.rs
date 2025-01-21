use crate::icy_board::{icb_text::IceText, state::functions::display_flags};
use crate::{icy_board::state::IcyBoardState, Res};

impl IcyBoardState {
    pub async fn toggle_alias(&mut self) -> Res<()> {
        self.displaycmdfile("alias").await?;

        self.session.use_alias = !self.session.use_alias;
        if let Some(token) = self.session.tokens.pop_front() {
            let token = token.to_ascii_uppercase();
            match token.as_str() {
                "ON" => {
                    self.session.use_alias = true;
                }
                "OFF" => {
                    self.session.use_alias = false;
                }
                _ => {}
            }
        }

        if let Some(user) = &mut self.session.current_user {
            user.flags.use_alias = self.session.use_alias;
        }

        let msg = if self.session.use_alias { IceText::AliasOn } else { IceText::AliasOff };
        self.display_text(msg, display_flags::NEWLINE | display_flags::LFAFTER).await?;
        self.press_enter().await?;
        self.display_current_menu = true;
        Ok(())
    }
}
