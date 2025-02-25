use crate::icy_board::{icb_text::IceText, state::functions::display_flags};
use crate::{Res, icy_board::state::IcyBoardState};

impl IcyBoardState {
    pub async fn page_sysop_command(&mut self) -> Res<()> {
        if !self.get_board().await.config.options.page_bell {
            self.display_text(IceText::SysopUnAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;

            let comment = self
                .input_field(
                    IceText::CommentInstead,
                    1,
                    "",
                    "",
                    None,
                    display_flags::YESNO | display_flags::UPCASE | display_flags::NEWLINE | display_flags::LFAFTER | display_flags::FIELDLEN,
                )
                .await?;
            if comment == self.session.yes_char.to_string() {
                self.comment_to_sysop().await?;
            }
            self.enter_comment_to_sysop().await?;
            return Ok(());
        }

        self.page_sysop().await?;
        Ok(())
    }
}
