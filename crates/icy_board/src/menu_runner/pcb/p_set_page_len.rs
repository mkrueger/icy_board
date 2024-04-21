use crate::{
    menu_runner::{PcbBoardCommand, MASK_NUMBER},
    Res,
};
use icy_board_engine::{
    icy_board::{commands::Command, icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};

impl PcbBoardCommand {
    pub fn set_page_len(&mut self, action: &Command) -> Res<()> {
        let page_len = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            self.state.display_text(IceText::CurrentPageLength, display_flags::LFBEFORE)?;
            self.state.print(TerminalTarget::Both, &format!(" {}\r\n", self.state.session.page_len))?;
            self.state.input_field(
                IceText::EnterPageLength,
                2,
                MASK_NUMBER,
                &action.help,
                Some(self.state.session.page_len.to_string()),
                display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
            )?
        };

        if !page_len.is_empty() {
            let page_len = page_len.parse::<u16>().unwrap_or_default();
            self.state.session.page_len = page_len;
            if let Some(user) = &mut self.state.current_user {
                user.page_len = page_len;
            }
        }
        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }
}
