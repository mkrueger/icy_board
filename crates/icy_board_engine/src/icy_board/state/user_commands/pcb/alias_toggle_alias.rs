use crate::icy_board::{icb_text::IceText, state::functions::display_flags};
use crate::{icy_board::state::IcyBoardState, Res};

impl IcyBoardState {
    pub async fn toggle_alias(&mut self) -> Res<()> {
        self.displaycmdfile("alias").await?;

        let mut new_alias = !self.session.use_alias;
        if let Some(token) = self.session.tokens.pop_front() {
            let token = token.to_ascii_uppercase();
            match token.as_str() {
                "ON" => {
                    new_alias = true;
                }
                "OFF" => {
                    new_alias = false;
                }
                _ => {}
            }
        }
        if self.session.use_alias == new_alias {
            return Ok(());
        }
        self.session.use_alias = new_alias;

        if let Some(user) = &mut self.session.current_user {
            user.flags.use_alias = self.session.use_alias;
        }

        self.display_text(IceText::HidingIdentity, display_flags::NEWLINE | display_flags::LFBEFORE)
            .await?;

        self.session.op_text = self.session.get_username_or_alias();
        self.display_text(IceText::ChangedNameTo, display_flags::NEWLINE | display_flags::LFBEFORE)
            .await?;

        if self.session.use_alias {
            self.display_text(IceText::IdentityProtected, display_flags::NEWLINE | display_flags::LFAFTER)
                .await?;
        }
        Ok(())
    }
}
