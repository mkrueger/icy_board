use crate::icy_board::commands::CommandType;
use crate::icy_board::state::functions::MASK_NUM;
use crate::icy_board::state::IcyBoardState;
use crate::Res;
use crate::{
    icy_board::{icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};

impl IcyBoardState {
    pub async fn set_page_len_command(&mut self) -> Res<()> {
        let page_len = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            self.display_text(IceText::CurrentPageLength, display_flags::LFBEFORE).await?;
            self.print(TerminalTarget::Both, &format!(" {}\r\n", self.session.page_len)).await?;
            self.input_field(
                IceText::EnterPageLength,
                2,
                &MASK_NUM,
                CommandType::SetPageLength.get_help(),
                Some(self.session.page_len.to_string()),
                display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
            )
            .await?
        };

        if !page_len.is_empty() {
            let page_len = page_len.parse::<u16>().unwrap_or_default();
            self.session.page_len = page_len;
            if let Some(user) = &mut self.session.current_user {
                user.page_len = page_len;
            }
        }
        self.press_enter().await?;
        self.display_current_menu = true;
        Ok(())
    }
}
