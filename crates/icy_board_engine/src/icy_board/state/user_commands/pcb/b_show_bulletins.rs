use crate::icy_board::{
    bulletins::MASK_BULLETINS,
    icb_text::IceText,
    state::{functions::display_flags, NodeStatus},
};
use crate::{icy_board::state::IcyBoardState, Res};

impl IcyBoardState {
    pub async fn show_bulletins(&mut self, help: &str) -> Res<()> {
        self.set_activity(NodeStatus::ReadBulletins).await;

        let bulletins = self.load_bullettins().await?;
        if bulletins.is_empty() {
            self.display_text(
                IceText::NoBulletinsAvailable,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER | display_flags::BELL,
            )
            .await?;
            return Ok(());
        }
        let mut display_current_menu = self.session.tokens.is_empty();
        loop {
            if display_current_menu {
                let file = self.session.current_conference.blt_menu.clone();
                self.display_file(&file).await?;
                display_current_menu = false;
            }
            let text = if let Some(token) = self.session.tokens.pop_front() {
                token
            } else {
                self.input_field(
                    if self.session.expert_mode {
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
                    self.goodbye().await?;
                    return Ok(());
                }
                "R" | "L" => {
                    display_current_menu = true;
                }
                _ => {
                    if text.is_empty() {
                        break;
                    }
                    if let Ok(number) = text.parse::<usize>() {
                        if number > 0 {
                            if let Some(b) = bulletins.get(number - 1) {
                                self.display_file(&b.file).await?;
                            } else {
                                self.display_text(
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

        self.press_enter().await?;
        self.display_current_menu = true;
        Ok(())
    }
}
