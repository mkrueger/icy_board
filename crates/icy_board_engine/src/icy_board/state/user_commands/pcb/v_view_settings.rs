use bstr::BString;
use jamjam::jam::JamMessageBase;

use crate::{Res, icy_board::state::IcyBoardState};

use crate::{
    icy_board::{icb_text::IceText, limits, state::functions::display_flags},
    vm::TerminalTarget,
};

/// Lines the block below takes, so a screen that no longer holds it gets a MORE
/// first instead of scrolling the top away.
const SETTINGS_LINES: usize = 18;

impl IcyBoardState {
    pub async fn view_settings(&mut self) -> Res<()> {
        self.displaycmdfile("prestat").await?;
        if self.displaycmdfile("stat").await? {
            return Ok(());
        }

        self.new_line().await?;
        if self.session.page_len > 0 && self.session.disp_options.num_lines_printed + SETTINGS_LINES > self.session.page_len as usize {
            self.press_enter().await?;
        }

        let user = self.session.current_user.clone().unwrap();
        if !(self.session.is_local || self.get_board().await.config.switches.exclude_local_calls_stats) {
            self.show_setting_number(IceText::ViewSettingsCallerNumber, self.session.caller_number as i64)
                .await?;
        }

        self.show_setting(IceText::ViewSettingsLastDateOne, &self.format_date(user.stats.last_on))
            .await?;

        // The line is printed either way - PCBoard says "None" when nothing expires.
        let expires = if user.expiration_date == chrono::DateTime::<chrono::Utc>::default() {
            self.get_display_text(IceText::None)?
        } else {
            self.format_date(user.expiration_date)
        };
        self.show_setting(IceText::ViewSettingsExpireDate, &expires).await?;

        self.show_setting_number(IceText::ViewSettingsNumberTimesOn, user.stats.num_times_on as i64)
            .await?;
        self.show_setting_number(IceText::ViewSettingsPageLength, self.session.page_len as i64).await?;

        if self.session.expert_mode() {
            self.display_text(IceText::ViewSettingsExpertModeOn, display_flags::NEWLINE).await?;
        } else {
            self.display_text(IceText::ViewSettingsExpertModeOff, display_flags::NEWLINE).await?;
        }
        self.reset_color(TerminalTarget::Both).await?;

        self.show_setting_number(IceText::ViewSettingsSecurityLevel, user.security_level as i64).await?;
        self.show_setting_number(IceText::ViewSettingsNumberDownloads, user.stats.num_downloads as i64)
            .await?;
        self.show_setting_number(IceText::ViewSettingsNumberUploads, user.stats.num_uploads as i64)
            .await?;

        // -1 stands for "no limit" and comes back out as the Unlimited text.
        self.show_setting_number(IceText::ViewSettingsBytesAvailable, self.bytes_available().unwrap_or(-1))
            .await?;

        if self.session.transfer_limits.byte_ratio_tenths != 0 {
            let ratio = limits::format_ratio(user.stats.total_dnld_bytes, user.stats.total_upld_bytes);
            self.show_setting(IceText::ShowByteRatio, &ratio).await?;
        }
        if self.session.transfer_limits.file_ratio_tenths != 0 {
            let ratio = limits::format_ratio(user.stats.num_downloads, user.stats.num_uploads);
            self.show_setting(IceText::ShowFileRatio, &ratio).await?;
        }

        self.show_message_base_stats().await?;

        let mut protocol = None;
        for p in self.get_board().await.protocols.iter() {
            if p.char_code == user.protocol {
                protocol = Some(p.description.clone());
                break;
            }
        }
        let protocol = match protocol {
            Some(description) => description,
            None => self.get_display_text(IceText::None)?,
        };
        self.show_setting(IceText::ViewSettingsTransferProtocol, &protocol).await?;

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

    /// Where the caller stands in the message base of the current area.
    async fn show_message_base_stats(&mut self) -> Res<()> {
        let Some(path) = self.message_area_path(self.session.current_message_area) else {
            return Ok(());
        };
        let Ok(message_base) = JamMessageBase::open(&path) else {
            log::error!("View settings could not open the message base {}", path.display());
            return Ok(());
        };

        let low = message_base.lowest_message_number();
        let high = message_base.highest_message_number();
        let crc = JamMessageBase::crc(&BString::from(self.session.user_name.as_bytes()));
        let last_read = match message_base.find_last_read(crc, self.session.cur_user_id as u32) {
            Ok(Some(entry)) => entry.last_read_msg,
            _ => 0,
        };
        // A pointer outside the base is what packing the base leaves behind.
        let last_read = last_read.clamp(low.saturating_sub(1), high);
        let active = message_base.active_messages();

        self.show_setting_number(IceText::ViewSettingsLastMessageRead, last_read as i64).await?;
        self.show_setting_number(IceText::ViewSettingsHighMessageNumber, high as i64).await?;
        self.show_setting_number(IceText::ViewSettingsNumberActiveMessages, active as i64).await?;
        Ok(())
    }

    async fn show_setting(&mut self, label: IceText, value: &str) -> Res<()> {
        self.display_text(label, display_flags::DEFAULT).await?;
        self.println(TerminalTarget::Both, value).await?;
        self.reset_color(TerminalTarget::Both).await
    }

    /// `PCBoard` leads the number with a blank and prints -1 as the Unlimited text.
    async fn show_setting_number(&mut self, label: IceText, value: i64) -> Res<()> {
        let value = if value == -1 { self.unlimited_text() } else { value.to_string() };
        self.show_setting(label, &format!(" {value}")).await
    }
}
