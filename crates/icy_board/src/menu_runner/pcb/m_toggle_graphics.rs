use crate::{menu_runner::PcbBoardCommand, Res};
use icy_board_engine::icy_board::{
    icb_text::IceText,
    state::{functions::display_flags, GraphicsMode},
};

impl PcbBoardCommand {
    pub async fn toggle_graphics(&mut self) -> Res<()> {
        self.displaycmdfile("m").await?;

        if self.state.board.lock().unwrap().config.options.non_graphics {
            self.state
                .display_text(IceText::GraphicsUnavailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;

            return Ok(());
        }

        if !self.state.session.disp_options.disable_color {
            self.state.reset_color().await?;
        }

        if let Some(token) = self.state.session.tokens.pop_front() {
            let token = token.to_ascii_uppercase();

            match token.as_str() {
                "CT" => {
                    self.state.session.disp_options.disable_color = true;
                    self.state.session.disp_options.grapics_mode = GraphicsMode::Ctty;
                    self.state
                        .display_text(IceText::CTTYOn, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                }
                "AN" => {
                    self.state.session.disp_options.disable_color = true;
                    self.state.session.disp_options.grapics_mode = GraphicsMode::Ansi;
                    self.state
                        .display_text(IceText::AnsiOn, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                }
                "AV" => {
                    self.state.session.disp_options.disable_color = false;
                    self.state.session.disp_options.grapics_mode = GraphicsMode::Avatar;
                    self.state
                        .display_text(IceText::AvatarOn, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                }
                "GR" | "ON" | "YES" => {
                    self.state.session.disp_options.disable_color = false;
                    if self.state.session.disp_options.grapics_mode == GraphicsMode::Ctty {
                        self.state.session.disp_options.grapics_mode = GraphicsMode::Ansi;
                    }
                    self.state
                        .display_text(IceText::GraphicsOn, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                }
                "RI" => {
                    self.state.session.disp_options.disable_color = false;
                    self.state.session.disp_options.grapics_mode = GraphicsMode::Rip;
                    self.state
                        .display_text(IceText::RIPModeOn, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                }
                "OFF" | "NO" => {
                    self.state.session.disp_options.disable_color = true;
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
            self.state.session.disp_options.disable_color = !self.state.session.disp_options.disable_color;
            let msg = if self.state.session.disp_options.disable_color {
                IceText::GraphicsOff
            } else {
                IceText::GraphicsOn
            };
            self.state.display_text(msg, display_flags::NEWLINE | display_flags::LFAFTER).await?;
        }
        self.state.press_enter().await?;
        self.display_menu = true;
        Ok(())
    }
}
