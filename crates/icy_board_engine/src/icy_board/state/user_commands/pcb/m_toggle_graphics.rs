use crate::{Res, icy_board::state::IcyBoardState};
use crate::{
    icy_board::{
        icb_text::IceText,
        state::{GraphicsMode, functions::display_flags},
    },
    vm::TerminalTarget,
};

impl IcyBoardState {
    pub async fn toggle_graphics(&mut self) -> Res<()> {
        self.displaycmdfile("m").await?;

        if self.get_board().await.config.switches.non_graphics {
            self.display_text(IceText::GraphicsUnavailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;

            return Ok(());
        }

        self.reset_color(TerminalTarget::Both).await?;

        if let Some(token) = self.session.tokens.pop_front() {
            let token = token.to_ascii_uppercase();

            match token.as_str() {
                "CT" | "CTT" | "CTTY" => {
                    self.set_grapics_mode(GraphicsMode::Ctty).await;
                    self.display_text(IceText::CTTYOn, display_flags::NEWLINE | display_flags::LFBEFORE).await?;
                }
                "AN" | "ANS" | "ANSI" => {
                    self.set_grapics_mode(GraphicsMode::Ansi).await;
                    self.display_text(IceText::AnsiOn, display_flags::NEWLINE | display_flags::LFBEFORE).await?;
                }
                "AV" | "AVT" | "AVATAR" => {
                    self.set_grapics_mode(GraphicsMode::Avatar).await;
                    self.display_text(IceText::AvatarOn, display_flags::NEWLINE | display_flags::LFBEFORE).await?;
                }
                "GR" | "GRAPH" | "ON" | "YES" => {
                    self.set_grapics_mode(GraphicsMode::Graphics).await;
                    self.display_text(IceText::GraphicsOn, display_flags::NEWLINE | display_flags::LFBEFORE).await?;
                }
                "RI" | "RIP" => {
                    self.set_grapics_mode(GraphicsMode::Rip).await;
                    self.display_text(IceText::RIPModeOn, display_flags::NEWLINE | display_flags::LFBEFORE).await?;
                }
                "OFF" | "NO" => {
                    self.set_grapics_mode(GraphicsMode::Ansi).await;
                    self.display_text(IceText::GraphicsOff, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                }
                _ => {
                    self.display_text(IceText::GraphicsUnavailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                    return Ok(());
                }
            }
        } else {
            if self.session.disp_options.grapics_mode == GraphicsMode::Graphics {
                self.set_grapics_mode(GraphicsMode::Ansi).await;
            } else {
                self.set_grapics_mode(GraphicsMode::Graphics).await;
            }
            let msg = if self.session.disp_options.grapics_mode == GraphicsMode::Graphics {
                IceText::GraphicsOn
            } else {
                IceText::GraphicsOff
            };
            self.display_text(msg, display_flags::NEWLINE | display_flags::LFBEFORE).await?;
        }
        Ok(())
    }
}
