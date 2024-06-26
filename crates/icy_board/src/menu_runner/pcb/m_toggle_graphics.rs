use crate::{menu_runner::PcbBoardCommand, Res};
use icy_board_engine::{
    icy_board::{
        icb_text::IceText,
        state::{functions::display_flags, GraphicsMode},
    },
    vm::TerminalTarget,
};

impl PcbBoardCommand {
    pub async fn toggle_graphics(&mut self) -> Res<()> {
        self.displaycmdfile("m").await?;

        if self.state.get_board().await.config.options.non_graphics {
            self.state
                .display_text(IceText::GraphicsUnavailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;

            return Ok(());
        }

        self.state.reset_color(TerminalTarget::Both).await?;

        if let Some(token) = self.state.session.tokens.pop_front() {
            let token = token.to_ascii_uppercase();

            match token.as_str() {
                "CT" | "CTT" | "CTTY" => {
                    self.state.set_grapics_mode(GraphicsMode::Ctty).await;
                    self.state
                        .display_text(IceText::CTTYOn, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                }
                "AN" | "ANS" | "ANSI" => {
                    self.state.set_grapics_mode(GraphicsMode::Ansi).await;
                    self.state
                        .display_text(IceText::AnsiOn, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                }
                "AV" | "AVT" | "AVATAR" => {
                    self.state.set_grapics_mode(GraphicsMode::Avatar).await;
                    self.state
                        .display_text(IceText::AvatarOn, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                }
                "GR" | "GRAPH" | "ON" | "YES" => {
                    self.state.set_grapics_mode(GraphicsMode::Graphics).await;
                    self.state
                        .display_text(IceText::GraphicsOn, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                }
                "RI" | "RIP" => {
                    self.state.set_grapics_mode(GraphicsMode::Rip).await;
                    self.state
                        .display_text(IceText::RIPModeOn, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                }
                "OFF" | "NO" => {
                    self.state.set_grapics_mode(GraphicsMode::Ansi).await;
                    self.state
                        .display_text(IceText::GraphicsOff, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                }
                _ => {
                    self.state
                        .display_text(IceText::GraphicsUnavailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                    return Ok(());
                }
            }
        } else {
            if self.state.session.disp_options.grapics_mode == GraphicsMode::Graphics {
                self.state.set_grapics_mode(GraphicsMode::Ansi).await;
            } else {
                self.state.set_grapics_mode(GraphicsMode::Graphics).await;
            }
            let msg = if self.state.session.disp_options.grapics_mode == GraphicsMode::Graphics {
                IceText::GraphicsOn
            } else {
                IceText::GraphicsOff
            };
            self.state.display_text(msg, display_flags::NEWLINE | display_flags::LFAFTER).await?;
        }
        self.state.press_enter().await?;
        self.display_menu = true;
        Ok(())
    }
}
