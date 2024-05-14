use crate::{menu_runner::PcbBoardCommand, Res};
use icy_board_engine::icy_board::{
    bulletins::MASK_BULLETINS,
    icb_text::IceText,
    state::{functions::display_flags, UserActivity},
};

impl PcbBoardCommand {
    pub async fn show_bulletins(&mut self, help: &str) -> Res<()> {
        self.state.set_activity(UserActivity::ReadBulletins).await;

        let bulletins = self.state.load_bullettins().await?;
        if bulletins.is_empty() {
            self.state
                .display_text(
                    IceText::NoBulletinsAvailable,
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER | display_flags::BELL,
                )
                .await?;
            return Ok(());
        }
        let mut display_menu = self.state.session.tokens.is_empty();
        loop {
            if display_menu {
                let file = self.state.session.current_conference.blt_menu.clone();
                self.state.display_file(&file).await?;
                display_menu = false;
            }
            let text = if let Some(token) = self.state.session.tokens.pop_front() {
                token
            } else {
                self.state
                    .input_field(
                        if self.state.session.expert_mode {
                            IceText::BulletinListCommandExpertmode
                        } else {
                            IceText::BulletinListCommand
                        },
                        12,
                        MASK_BULLETINS,
                        help,
                        None,
                        display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE,
                    )
                    .await?
            };
            match text.as_str() {
                "G" => {
                    self.state.goodbye().await?;
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
                                self.state.display_file(&b.file).await?;
                            } else {
                                self.state
                                    .display_text(
                                        IceText::InvalidBulletinNumber,
                                        display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
                                    )
                                    .await?;
                            }
                        }
                    }
                }
            }
        }

        self.state.press_enter().await?;
        self.display_menu = true;
        Ok(())
    }
}
