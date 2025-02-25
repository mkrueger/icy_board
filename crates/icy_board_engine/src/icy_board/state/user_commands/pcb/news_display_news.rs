use crate::icy_board::{icb_text::IceText, state::functions::display_flags};
use crate::{Res, icy_board::state::IcyBoardState};

impl IcyBoardState {
    pub async fn display_news(&mut self) -> Res<()> {
        self.displaycmdfile("news").await?;
        let news_file = self.session.current_conference.news_file.clone();
        self.session.disp_options.no_change();
        if !self.display_file(&news_file).await? {
            self.display_text(IceText::NoNews, display_flags::NEWLINE).await?;
        }
        Ok(())
    }
}
