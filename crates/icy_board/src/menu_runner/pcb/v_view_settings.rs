use crate::{menu_runner::PcbBoardCommand, Res};

use chrono::Datelike;
use icy_board_engine::{
    icy_board::{commands::Command, icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};

impl PcbBoardCommand {
    pub fn view_settings(&mut self, _action: &Command) -> Res<()> {
        self.displaycmdfile("prestat")?;
        if self.displaycmdfile("stat")? {
            return Ok(());
        }

        let user = self.state.current_user.clone().unwrap();
        if !(self.state.session.is_local || self.state.board.lock().unwrap().config.options.exclude_local_calls) {
            self.state.display_text(IceText::ViewSettingsCallerNumber, display_flags::DEFAULT)?;
            self.state.println(TerminalTarget::Both, &format!(" {}", self.state.session.caller_number))?;
            self.state.reset_color()?;
        }

        self.state.new_line()?;
        
        self.state.display_text(IceText::ViewSettingsLastDateOne, display_flags::DEFAULT)?;
        self.state.println(TerminalTarget::Both, &self.state.format_date(user.stats.last_on))?;
        self.state.reset_color()?;

        if user.exp_date.year() > 0 {
            self.state.display_text(IceText::ViewSettingsExpireDate, display_flags::DEFAULT)?;
            self.state.println(TerminalTarget::Both, &self.state.format_date(user.exp_date))?;
            self.state.reset_color()?;
        }
        self.state.display_text(IceText::ViewSettingsNumberTimesOn, display_flags::DEFAULT)?;
        self.state.println(TerminalTarget::Both, &format!(" {}", user.stats.num_times_on))?;
        self.state.reset_color()?;

        self.state.display_text(IceText::ViewSettingsPageLength, display_flags::DEFAULT)?;
        self.state.println(TerminalTarget::Both, &format!(" {}", self.state.session.page_len))?;
        self.state.reset_color()?;

        if self.state.session.expert_mode {
            self.state.display_text(IceText::ViewSettingsExpertModeOn, display_flags::NEWLINE)?;
        } else {
            self.state.display_text(IceText::ViewSettingsExpertModeOff, display_flags::NEWLINE)?;
        }
        self.state.reset_color()?;

        self.state.display_text(IceText::ViewSettingsSecurityLevel, display_flags::DEFAULT)?;
        self.state.println(TerminalTarget::Both, &format!(" {}", user.security_level))?;
        self.state.reset_color()?;

        self.state.display_text(IceText::ViewSettingsNumberDownloads, display_flags::DEFAULT)?;
        self.state.println(TerminalTarget::Both, &format!(" {}", user.stats.num_downloads))?;
        self.state.reset_color()?;

        self.state.display_text(IceText::ViewSettingsNumberUploads, display_flags::DEFAULT)?;
        self.state.println(TerminalTarget::Both, &format!(" {}", user.stats.num_uploads))?;
        self.state.reset_color()?;

        // MsgStats don't make any sense anymore on

        self.state.display_text(IceText::ViewSettingsTransferProtocol, display_flags::DEFAULT)?;
        let mut protocol = user.protocol.to_string();
        for p in self.state.board.lock().unwrap().protocols.iter() {
            if p.char_code == user.protocol {
                protocol = p.description.clone();
                break;
            }
        }
        self.state.println(TerminalTarget::Both, &protocol)?;
        self.state.reset_color()?;

        if self.state.session.use_alias {
            self.state.display_text(IceText::ViewSettingsAliasOn, display_flags::NEWLINE)?;
        } else {
            self.state.display_text(IceText::ViewSettingsAliasOff, display_flags::NEWLINE)?;
        }
        self.state.reset_color()?;

        self.state.display_text(IceText::ViewSettingsGraphicsMode, display_flags::NEWLINE)?;
        self.state.reset_color()?;

        self.state.new_line()?;
        self.state.press_enter()?;
        self.display_menu = true;

        Ok(())
    }
}
