use crate::{icy_board::state::IcyBoardState, Res};

use crate::{
    icy_board::{icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};

impl IcyBoardState {
    pub async fn view_settings(&mut self) -> Res<()> {
        self.displaycmdfile("prestat").await?;
        if self.displaycmdfile("stat").await? {
            return Ok(());
        }

        let user = self.session.current_user.clone().unwrap();
        if !(self.session.is_local || self.get_board().await.config.switches.exclude_local_calls_stats) {
            self.display_text(IceText::ViewSettingsCallerNumber, display_flags::DEFAULT).await?;
            self.println(TerminalTarget::Both, &format!(" {}", self.session.caller_number)).await?;
            self.reset_color(TerminalTarget::Both).await?;
        }

        self.new_line().await?;

        self.display_text(IceText::ViewSettingsLastDateOne, display_flags::DEFAULT).await?;
        self.println(TerminalTarget::Both, &self.format_date(user.stats.last_on)).await?;
        self.reset_color(TerminalTarget::Both).await?;

        if user.exp_date.year() > 0 {
            self.display_text(IceText::ViewSettingsExpireDate, display_flags::DEFAULT).await?;
            self.println(TerminalTarget::Both, &self.format_date(user.exp_date.to_utc_date_time())).await?;
            self.reset_color(TerminalTarget::Both).await?;
        }
        self.display_text(IceText::ViewSettingsNumberTimesOn, display_flags::DEFAULT).await?;
        self.println(TerminalTarget::Both, &format!(" {}", user.stats.num_times_on)).await?;
        self.reset_color(TerminalTarget::Both).await?;

        self.display_text(IceText::ViewSettingsPageLength, display_flags::DEFAULT).await?;
        self.println(TerminalTarget::Both, &format!(" {}", self.session.page_len)).await?;
        self.reset_color(TerminalTarget::Both).await?;

        if self.session.expert_mode {
            self.display_text(IceText::ViewSettingsExpertModeOn, display_flags::NEWLINE).await?;
        } else {
            self.display_text(IceText::ViewSettingsExpertModeOff, display_flags::NEWLINE).await?;
        }
        self.reset_color(TerminalTarget::Both).await?;

        self.display_text(IceText::ViewSettingsSecurityLevel, display_flags::DEFAULT).await?;
        self.println(TerminalTarget::Both, &format!(" {}", user.security_level)).await?;
        self.reset_color(TerminalTarget::Both).await?;

        self.display_text(IceText::ViewSettingsNumberDownloads, display_flags::DEFAULT).await?;
        self.println(TerminalTarget::Both, &format!(" {}", user.stats.num_downloads)).await?;
        self.reset_color(TerminalTarget::Both).await?;

        self.display_text(IceText::ViewSettingsNumberUploads, display_flags::DEFAULT).await?;
        self.println(TerminalTarget::Both, &format!(" {}", user.stats.num_uploads)).await?;
        self.reset_color(TerminalTarget::Both).await?;

        // MsgStats don't make any sense anymore on

        self.display_text(IceText::ViewSettingsTransferProtocol, display_flags::DEFAULT).await?;
        let mut protocol = user.protocol.to_string();
        for p in self.get_board().await.protocols.iter() {
            if p.char_code == user.protocol {
                protocol = p.description.clone();
                break;
            }
        }
        self.println(TerminalTarget::Both, &protocol).await?;
        self.reset_color(TerminalTarget::Both).await?;

        if self.session.use_alias {
            self.display_text(IceText::ViewSettingsAliasOn, display_flags::NEWLINE).await?;
        } else {
            self.display_text(IceText::ViewSettingsAliasOff, display_flags::NEWLINE).await?;
        }
        self.reset_color(TerminalTarget::Both).await?;

        self.display_text(IceText::ViewSettingsGraphicsMode, display_flags::NEWLINE).await?;
        self.reset_color(TerminalTarget::Both).await?;
        Ok(())
    }
}
