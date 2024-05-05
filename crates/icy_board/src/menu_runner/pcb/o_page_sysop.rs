use crate::{menu_runner::PcbBoardCommand, Res};
use icy_board_engine::icy_board::{commands::Command, icb_text::IceText, state::functions::display_flags};

impl PcbBoardCommand {
    pub async fn page_sysop(&mut self, action: &Command) -> Res<()> {
        if !self.state.get_board().await.config.options.page_bell {
            self.state
                .display_text(IceText::SysopUnAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;

            let comment = self
                .state
                .input_field(
                    IceText::CommentInstead,
                    1,
                    "",
                    "",
                    None,
                    display_flags::YESNO | display_flags::UPCASE | display_flags::NEWLINE | display_flags::LFAFTER | display_flags::FIELDLEN,
                )
                .await?;
            if comment == self.state.session.yes_char.to_string() {
                self.comment_to_sysop(action).await?;
            }
            self.enter_comment_to_sysop().await?;
            return Ok(());
        }

        self.state.page_sysop().await?;
        self.state.new_line().await?;
        self.state.press_enter().await?;
        self.display_menu = true;
        Ok(())
    }
}
