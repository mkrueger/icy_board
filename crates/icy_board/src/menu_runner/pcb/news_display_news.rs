use crate::{menu_runner::PcbBoardCommand, Res};
use icy_board_engine::icy_board::{icb_text::IceText, state::functions::display_flags};

impl PcbBoardCommand {
    pub async fn display_news(&mut self) -> Res<()> {
        self.displaycmdfile("news").await?;
        let news_file = self.state.session.current_conference.news_file.clone();
        if !self.state.display_file(&news_file).await? {
            self.state.display_text(IceText::NoNews, display_flags::NEWLINE).await?;
        }
        self.state.new_line().await?;
        self.state.press_enter().await?;
        self.display_menu = true;
        Ok(())
    }
}
