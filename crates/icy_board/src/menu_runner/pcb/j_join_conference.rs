use crate::{
    menu_runner::{PcbBoardCommand, MASK_COMMAND},
    Res,
};
use icy_board_engine::icy_board::{commands::Command, icb_text::IceText, state::functions::display_flags};

impl PcbBoardCommand {
    pub fn join_conference(&mut self, action: &Command) -> Res<()> {
        if self.state.board.lock().unwrap().conferences.is_empty() {
            self.state
                .display_text(IceText::NoConferenceAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)?;
            self.state.press_enter()?;
            return Ok(());
        }
        let conf_number = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            let mnu = self.state.board.lock().unwrap().config.paths.conf_join_menu.clone();
            let mnu = self.state.resolve_path(&mnu);

            self.state.display_menu(&mnu)?;
            self.state.new_line()?;

            self.state.input_field(
                IceText::JoinConferenceNumber,
                40,
                MASK_COMMAND,
                &action.help,
                None,
                display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
            )?
        };
        if !conf_number.is_empty() {
            let mut joined = false;
            if let Ok(number) = conf_number.parse::<i32>() {
                if 0 <= number && (number as usize) <= self.state.board.lock().unwrap().conferences.len() {
                    self.state.join_conference(number);
                    self.state.session.op_text = format!(
                        "{} ({})",
                        self.state.session.current_conference.name, self.state.session.current_conference_number
                    );
                    self.state
                        .display_text(IceText::ConferenceJoined, display_flags::NEWLINE | display_flags::NOTBLANK)?;

                    joined = true;
                }
            }

            if !joined {
                self.state.session.op_text = conf_number;
                self.state
                    .display_text(IceText::InvalidConferenceNumber, display_flags::NEWLINE | display_flags::NOTBLANK)?;
            }
        }

        self.state.new_line()?;
        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }
}
