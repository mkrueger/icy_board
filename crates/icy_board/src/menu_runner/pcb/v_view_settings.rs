use crate::{menu_runner::PcbBoardCommand, Res};

use chrono::Datelike;
use icy_board_engine::{
    icy_board::{icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};

impl PcbBoardCommand {
    pub async fn view_settings(&mut self) -> Res<()> {
        self.displaycmdfile("prestat").await?;
        if self.displaycmdfile("stat").await? {
            return Ok(());
        }

        let user = self.state.session.current_user.clone().unwrap();
        if !(self.state.session.is_local || self.state.get_board().await.config.options.exclude_local_calls) {
            self.state.display_text(IceText::ViewSettingsCallerNumber, display_flags::DEFAULT).await?;
            self.state
                .println(TerminalTarget::Both, &format!(" {}", self.state.session.caller_number))
                .await?;
            self.state.reset_color(TerminalTarget::Both).await?;
        }

        self.state.new_line().await?;

        self.state.display_text(IceText::ViewSettingsLastDateOne, display_flags::DEFAULT).await?;
        self.state.println(TerminalTarget::Both, &self.state.format_date(user.stats.last_on)).await?;
        self.state.reset_color(TerminalTarget::Both).await?;

        if user.exp_date.year() > 0 {
            self.state.display_text(IceText::ViewSettingsExpireDate, display_flags::DEFAULT).await?;
            self.state.println(TerminalTarget::Both, &self.state.format_date(user.exp_date)).await?;
            self.state.reset_color(TerminalTarget::Both).await?;
        }
        self.state.display_text(IceText::ViewSettingsNumberTimesOn, display_flags::DEFAULT).await?;
        self.state.println(TerminalTarget::Both, &format!(" {}", user.stats.num_times_on)).await?;
        self.state.reset_color(TerminalTarget::Both).await?;

        self.state.display_text(IceText::ViewSettingsPageLength, display_flags::DEFAULT).await?;
        self.state.println(TerminalTarget::Both, &format!(" {}", self.state.session.page_len)).await?;
        self.state.reset_color(TerminalTarget::Both).await?;

        if self.state.session.expert_mode {
            self.state.display_text(IceText::ViewSettingsExpertModeOn, display_flags::NEWLINE).await?;
        } else {
            self.state.display_text(IceText::ViewSettingsExpertModeOff, display_flags::NEWLINE).await?;
        }
        self.state.reset_color(TerminalTarget::Both).await?;

        self.state.display_text(IceText::ViewSettingsSecurityLevel, display_flags::DEFAULT).await?;
        self.state.println(TerminalTarget::Both, &format!(" {}", user.security_level)).await?;
        self.state.reset_color(TerminalTarget::Both).await?;

        self.state.display_text(IceText::ViewSettingsNumberDownloads, display_flags::DEFAULT).await?;
        self.state.println(TerminalTarget::Both, &format!(" {}", user.stats.num_downloads)).await?;
        self.state.reset_color(TerminalTarget::Both).await?;

        self.state.display_text(IceText::ViewSettingsNumberUploads, display_flags::DEFAULT).await?;
        self.state.println(TerminalTarget::Both, &format!(" {}", user.stats.num_uploads)).await?;
        self.state.reset_color(TerminalTarget::Both).await?;

        // MsgStats don't make any sense anymore on

        self.state.display_text(IceText::ViewSettingsTransferProtocol, display_flags::DEFAULT).await?;
        let mut protocol = user.protocol.to_string();
        for p in self.state.get_board().await.protocols.iter() {
            if p.char_code == user.protocol {
                protocol = p.description.clone();
                break;
            }
        }
        self.state.println(TerminalTarget::Both, &protocol).await?;
        self.state.reset_color(TerminalTarget::Both).await?;

        if self.state.session.use_alias {
            self.state.display_text(IceText::ViewSettingsAliasOn, display_flags::NEWLINE).await?;
        } else {
            self.state.display_text(IceText::ViewSettingsAliasOff, display_flags::NEWLINE).await?;
        }
        self.state.reset_color(TerminalTarget::Both).await?;

        self.state.display_text(IceText::ViewSettingsGraphicsMode, display_flags::NEWLINE).await?;
        self.state.reset_color(TerminalTarget::Both).await?;

        self.state.new_line().await?;
        self.state.press_enter().await?;
        self.display_menu = true;

        Ok(())
    }
}
