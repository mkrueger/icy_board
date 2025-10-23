use crate::icy_board::{icb_text::IceText, state::functions::display_flags};
use crate::{Res, icy_board::state::IcyBoardState};

impl IcyBoardState {
    pub async fn display_news(&mut self, only_newer: bool) -> Res<()> {
        self.displaycmdfile("news").await?;
        let news_file = self.session.current_conference.news_file.clone();

        if only_newer {
            if let Some(user) = &self.session.current_user {
                if news_file.exists() {
                    if let Ok(metadata) = std::fs::metadata(&news_file) {
                        if let Ok(modified) = metadata.modified() {
                            let modified_time: chrono::DateTime<chrono::Utc> = modified.into();
                            if modified_time <= user.stats.last_on {
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }

        self.session.disp_options.no_change();
        if !self.display_file(&news_file).await? {
            self.display_text(IceText::NoNews, display_flags::NEWLINE).await?;
        }
        Ok(())
    }
}
