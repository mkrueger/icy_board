use crate::{menu_runner::PcbBoardCommand, Res};
use icy_board_engine::icy_board::{icb_text::IceText, state::functions::display_flags};

impl PcbBoardCommand {
    pub fn abandon_conference(&mut self) -> Res<()> {
        if self.state.board.lock().unwrap().conferences.is_empty() {
            self.state
                .display_text(IceText::NoConferenceAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)?;
            self.state.press_enter()?;
            return Ok(());
        }
        if self.state.session.current_conference_number > 0 {
            self.state.session.op_text = format!(
                "{} ({})",
                self.state.session.current_conference.name, self.state.session.current_conference_number
            );
            self.state.join_conference(0);
            self.state
                .display_text(IceText::ConferenceAbandoned, display_flags::NEWLINE | display_flags::NOTBLANK)?;
            self.state.new_line()?;
            self.state.press_enter()?;
        }
        self.display_menu = true;
        Ok(())
    }
}
