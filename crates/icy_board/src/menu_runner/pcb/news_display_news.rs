use crate::{menu_runner::PcbBoardCommand, Res};
use icy_board_engine::icy_board::{icb_text::IceText, state::functions::display_flags};

impl PcbBoardCommand {
    pub fn display_news(&mut self) -> Res<()> {
        self.displaycmdfile("news")?;
        let news_file = self.state.session.current_conference.news_file.clone();
        let resolved_path = self.state.resolve_path(&news_file);

        if !resolved_path.exists() {
            self.state.display_text(IceText::NoNews, display_flags::NEWLINE)?;
        } else {
            self.state.display_file(&news_file)?;
        }
        self.state.new_line()?;
        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }
}
