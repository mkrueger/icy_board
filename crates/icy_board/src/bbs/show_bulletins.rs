use icy_board_engine::icy_board::{bulletins::MASK_BULLETINS, commands::Command, icb_text::IceText, state::functions::display_flags};
use icy_ppe::Res;

use super::PcbBoardCommand;

impl PcbBoardCommand {
    pub fn show_bulletins(&mut self, action: &Command) -> Res<()> {
        let bulletins = self.state.load_bullettins()?;
        if bulletins.is_empty() {
            self.state.display_text(
                IceText::NoBulletinsAvailable,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER | display_flags::BELL,
            )?;
            return Ok(());
        }
        let mut display_menu = self.state.session.tokens.is_empty();
        loop {
            if display_menu {
                let file = self.state.session.current_conference.blt_menu.clone();
                self.state.display_file(&file)?;
                display_menu = false;
            }
            let text = if let Some(token) = self.state.session.tokens.pop_front() {
                token
            } else {
                self.state.input_field(
                    if self.state.session.expert_mode {
                        IceText::BulletinListCommandExpertmode
                    } else {
                        IceText::BulletinListCommand
                    },
                    12,
                    MASK_BULLETINS,
                    &action.help,
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE,
                )?
            };
            match text.as_str() {
                "G" => {
                    self.state.hangup(icy_board_engine::vm::HangupType::Goodbye)?;
                    return Ok(());
                }
                "R" | "L" => {
                    display_menu = true;
                }
                _ => {
                    if text.is_empty() {
                        break;
                    }
                    if let Ok(number) = text.parse::<usize>() {
                        if number > 0 {
                            if let Some(b) = bulletins.get(number - 1) {
                                self.state.display_file(&b.file)?;
                            } else {
                                self.state.display_text(
                                    IceText::InvalidBulletinNumber,
                                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
                                )?;
                            }
                        }
                    }
                }
            }
        }

        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }
}