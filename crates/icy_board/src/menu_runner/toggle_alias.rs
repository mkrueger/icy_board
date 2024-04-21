use crate::Res;
use icy_board_engine::icy_board::{commands::Command, icb_text::IceText, state::functions::display_flags};

use super::PcbBoardCommand;

impl PcbBoardCommand {
    pub fn toggle_alias(&mut self, _action: &Command) -> Res<()> {
        self.displaycmdfile("alias")?;

        self.state.session.use_alias = !self.state.session.use_alias;
        if let Some(token) = self.state.session.tokens.pop_front() {
            let token = token.to_ascii_uppercase();
            match token.as_str() {
                "ON" => {
                    self.state.session.use_alias = true;
                }
                "OFF" => {
                    self.state.session.use_alias = false;
                }
                _ => {}
            }
        }

        if let Some(user) = &mut self.state.current_user {
            user.flags.use_alias = self.state.session.use_alias;
        }

        let msg = if self.state.session.use_alias { IceText::AliasOn } else { IceText::AliasOff };
        self.state.display_text(msg, display_flags::NEWLINE | display_flags::LFAFTER)?;

        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }
}
