use crate::icy_board::{icb_text::IceText, state::functions::display_flags};
use crate::{icy_board::state::IcyBoardState, Res};

impl IcyBoardState {
    pub async fn display_news(&mut self) -> Res<()> {
        self.displaycmdfile("news").await?;
        let news_file = self.session.current_conference.news_file.clone();
        if !self.display_file(&news_file).await? {
            self.display_text(IceText::NoNews, display_flags::NEWLINE).await?;
        }
        self.new_line().await?;
        self.press_enter().await?;
        self.display_current_menu = true;
        Ok(())
    }
}
