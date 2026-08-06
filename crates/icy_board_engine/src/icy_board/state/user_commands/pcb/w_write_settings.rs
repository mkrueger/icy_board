use chrono::Utc;

use crate::{Res, icy_board::state::IcyBoardState};
use crate::{
    datetime::IcbDate,
    icy_board::{
        icb_config::IcbColor,
        icb_text::IceText,
        state::{
            functions::{MASK_ALNUM, MASK_NUM, MASK_PHONE, MASK_WEB, display_flags},
            user_commands::pcb::select_conferences::SelectMode,
        },
        user_base::FSEMode,
    },
    vm::TerminalTarget,
};

impl IcyBoardState {
    /// The W command.
    ///
    /// The prompt sequence is a compatibility contract: PPEs pre-answer it with
    /// KBDSTUFF, so a missing or extra question shifts every following stuffed
    /// answer onto the wrong field. The order below is the one PCBoard walks,
    /// verified against a live board. Anything icy_board asks on top of that is
    /// appended after the last PCBoard question so the indices stay stable.
    pub async fn write_settings(&mut self) -> Res<()> {
        // PCBoard ignores any argument to W.
        self.session.tokens.clear();

        self.display_text(IceText::EnterNoChange, display_flags::LFBEFORE | display_flags::NEWLINE)
            .await?;
        let settings = self.get_board().await.config.new_user_settings.clone();
        let Some(mut new_user) = self.session.current_user.clone() else {
            return Ok(());
        };

        // An empty answer returns straight away and skips the confirmation.
        loop {
            let pw1 = self
                .input_field(
                    IceText::NewPassword,
                    12,
                    &MASK_ALNUM,
                    "",
                    None,
                    display_flags::ECHODOTS
                        | display_flags::FIELDLEN
                        | display_flags::UPCASE
                        | display_flags::GUIDE
                        | display_flags::NEWLINE
                        | display_flags::LFBEFORE
                        | display_flags::LFAFTER,
                )
                .await?;
            if pw1.is_empty() {
                break;
            }
            let pw2 = self
                .input_field(
                    IceText::ReEnterPassword,
                    12,
                    &MASK_ALNUM,
                    "",
                    None,
                    display_flags::ECHODOTS
                        | display_flags::FIELDLEN
                        | display_flags::UPCASE
                        | display_flags::GUIDE
                        | display_flags::NEWLINE
                        | display_flags::LFAFTER,
                )
                .await?;
            if pw1 != pw2 {
                self.display_text(IceText::PasswordsDontMatch, display_flags::NEWLINE).await?;
                continue;
            }
            new_user.password.password = self.create_password(pw1).await;
            new_user.password.last_change = Utc::now();
            new_user.password.times_changed += 1;
            let exp_days = self.get_board().await.config.limits.password_expire_days;
            if exp_days > 0 {
                new_user.password.expire_date = Utc::now() + chrono::Duration::days(exp_days as i64);
            }
            break;
        }

        if settings.ask_city_or_state {
            let answer = self
                .input_field(
                    IceText::CityState,
                    24,
                    &MASK_ALNUM,
                    "",
                    Some(new_user.city_or_state.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
            if !answer.is_empty() {
                new_user.city_or_state = answer;
            }
        }

        if settings.ask_business_phone {
            let answer = self
                .input_field(
                    IceText::BusDataPhone,
                    13,
                    &MASK_PHONE,
                    "",
                    Some(new_user.bus_data_phone.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
            if !answer.is_empty() {
                new_user.bus_data_phone = answer;
            }
        }

        if settings.ask_home_phone {
            let answer = self
                .input_field(
                    IceText::HomeVoicePhone,
                    13,
                    &MASK_PHONE,
                    "",
                    Some(new_user.home_voice_phone.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
            if !answer.is_empty() {
                new_user.home_voice_phone = answer;
            }
        }

        if settings.ask_comment {
            let answer = self
                .input_field(
                    IceText::CommentFieldPrompt,
                    30,
                    &MASK_ALNUM,
                    "",
                    Some(new_user.user_comment.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
            if !answer.is_empty() {
                new_user.user_comment = answer;
            }
        }

        if settings.ask_clr_msg {
            new_user.flags.msg_clear = self.ask_yes_no(IceText::CLSBetweenMessages, new_user.flags.msg_clear).await?;
        }

        new_user.flags.scroll_msg_body = self.ask_yes_no(IceText::ScrollMessageBody, new_user.flags.scroll_msg_body).await?;
        new_user.flags.long_msg_header = self.ask_yes_no(IceText::UseBigHeaders, new_user.flags.long_msg_header).await?;

        if settings.ask_fse {
            let str = match new_user.flags.fse_mode {
                FSEMode::Yes => "Y",
                FSEMode::No => "N",
                FSEMode::Ask => "A",
            };
            let fse_default = self
                .input_field(
                    IceText::SetFSEDefault,
                    1,
                    &"YNA",
                    "",
                    Some(str.to_string()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE,
                )
                .await?;
            match fse_default.as_str() {
                "Y" => {
                    new_user.flags.fse_mode = FSEMode::Yes;
                }
                "N" => {
                    new_user.flags.fse_mode = FSEMode::No;
                }
                "A" => {
                    new_user.flags.fse_mode = FSEMode::Ask;
                }
                _ => {}
            }
        }

        new_user.flags.wide_editor = self.ask_yes_no(IceText::DefaultWideMessages, new_user.flags.wide_editor).await?;

        if settings.ask_alias {
            let answer = self
                .input_field(
                    IceText::GetAliasName,
                    25,
                    &MASK_ALNUM,
                    "",
                    Some(new_user.alias.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
            if !answer.is_empty() {
                new_user.alias = answer;
            }
        }

        if settings.ask_use_short_descr {
            new_user.flags.use_short_filedescr = self
                .ask_yes_no(IceText::UseShortDescription, new_user.flags.use_short_filedescr)
                .await?;
        }

        // Only offered to users who may join conferences, and suppressed when
        // the prompt was blanked out in ICBTEXT: PCBoard gates this on SEC_J
        // plus a non-empty TXT_SELECTCONFS.
        let select_confs_text = self.get_display_text(IceText::SelectConferences)?;
        let may_join = self.session.user_command_level.cmd_j.session_can_access(&self.session);
        if may_join && !select_confs_text.trim().is_empty() && self.ask_yes_no(IceText::SelectConferences, false).await? {
            // select_conferences edits session.current_user in place, so the
            // record has to be handed over and taken back around the call.
            self.session.current_user = Some(new_user);
            self.select_conferences(SelectMode::Register).await?;
            let Some(updated) = self.session.current_user.clone() else {
                return Ok(());
            };
            new_user = updated;
        }

        if settings.ask_address {
            self.display_text(IceText::EnterAddress, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            let answer = self
                .input_field(
                    IceText::Street1,
                    50,
                    &MASK_ALNUM,
                    "",
                    Some(new_user.street1.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
            if !answer.is_empty() {
                new_user.street1 = answer;
            }

            let answer = self
                .input_field(
                    IceText::Street2,
                    50,
                    &MASK_ALNUM,
                    "",
                    Some(new_user.street2.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
            if !answer.is_empty() {
                new_user.street2 = answer;
            }

            let answer = self
                .input_field(
                    IceText::City,
                    25,
                    &MASK_ALNUM,
                    "",
                    Some(new_user.city.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
            if !answer.is_empty() {
                new_user.city = answer;
            }

            let answer = self
                .input_field(
                    IceText::State,
                    10,
                    &MASK_ALNUM,
                    "",
                    Some(new_user.state.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE,
                )
                .await?;
            if !answer.is_empty() {
                new_user.state = answer;
            }

            let answer = self
                .input_field(
                    IceText::Zip,
                    10,
                    &MASK_ALNUM,
                    "",
                    Some(new_user.zip.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE,
                )
                .await?;
            if !answer.is_empty() {
                new_user.zip = answer;
            }

            let answer = self
                .input_field(
                    IceText::Country,
                    15,
                    &MASK_ALNUM,
                    "",
                    Some(new_user.country.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE,
                )
                .await?;
            if !answer.is_empty() {
                new_user.country = answer;
            }
        }

        // getqwklimits(). Each question is skipped when its ICBTEXT entry is
        // blank, which is how a sysop turns an individual limit off.
        let qwk_settings = self.get_board().await.config.qwk_settings.clone();
        let mut qwk_config = new_user.qwk_config.clone().unwrap_or_default();

        if !self.get_display_text(IceText::PersonalMessageLimit)?.trim().is_empty() {
            self.session.op_text = qwk_settings.max_msgs.to_string();
            let answer = self
                .input_field(
                    IceText::PersonalMessageLimit,
                    5,
                    &MASK_NUM,
                    "",
                    Some(qwk_config.max_msgs.to_string()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
            if let Ok(value) = answer.parse::<u16>() {
                qwk_config.max_msgs = value.min(qwk_settings.max_msgs);
            }
        }

        if !self.get_display_text(IceText::PersonalConferenceLimit)?.trim().is_empty() {
            self.session.op_text = qwk_settings.max_msgs_per_conf.to_string();
            let answer = self
                .input_field(
                    IceText::PersonalConferenceLimit,
                    5,
                    &MASK_NUM,
                    "",
                    Some(qwk_config.max_msgs_per_conf.to_string()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
            if let Ok(value) = answer.parse::<u16>() {
                qwk_config.max_msgs_per_conf = value.min(qwk_settings.max_msgs_per_conf);
            }
        }

        if !self.get_display_text(IceText::PersonalQWKLimit)?.trim().is_empty() {
            let answer = self
                .input_field(
                    IceText::PersonalQWKLimit,
                    9,
                    &MASK_NUM,
                    "",
                    Some(qwk_config.personal_attach_limit.to_string()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
            if let Ok(value) = answer.parse::<i32>() {
                qwk_config.personal_attach_limit = value;
            }
        }

        if !self.get_display_text(IceText::PublicQWKLimit)?.trim().is_empty() {
            let answer = self
                .input_field(
                    IceText::PublicQWKLimit,
                    9,
                    &MASK_NUM,
                    "",
                    Some(qwk_config.public_attach_limit.to_string()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
            if let Ok(value) = answer.parse::<i32>() {
                qwk_config.public_attach_limit = value;
            }
        }
        new_user.qwk_config = Some(qwk_config);

        // Nothing below this line has a PCBoard counterpart, so it has to stay
        // behind the last question PCBoard asks.
        if settings.ask_date_format {
            let date_format = self.ask_date_format(&new_user.date_format).await?;
            if !date_format.is_empty() {
                new_user.date_format = date_format;
            }
        }

        if settings.ask_xfer_protocol {
            let protocol = self.ask_protocols("N").await?;
            self.new_line().await?;
            if !protocol.is_empty() {
                new_user.protocol = protocol;
            } else {
                new_user.protocol = "N".to_string();
            }
        }

        if settings.ask_verification {
            let answer = self
                .input_field(
                    IceText::EnterVerifyText,
                    25,
                    &MASK_ALNUM,
                    "",
                    Some(new_user.verify_answer.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
            if !answer.is_empty() {
                new_user.verify_answer = answer;
            }
        }

        if settings.ask_gender {
            let answer = self
                .input_field(
                    IceText::EnterGender,
                    1,
                    "MmFf",
                    "",
                    Some(new_user.gender.clone()),
                    display_flags::FIELDLEN | display_flags::UPCASE | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
            if !answer.is_empty() {
                new_user.gender = answer;
            }
        }

        if settings.ask_birthdate {
            let date = self
                .input_field(
                    IceText::EnterBirthdate,
                    8,
                    &MASK_ALNUM,
                    "",
                    None,
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
            if !date.is_empty() {
                new_user.birth_date = IcbDate::parse(&date).to_utc_date_time();
            }
        }

        if settings.ask_email {
            let answer = self
                .input_field(
                    IceText::EnterEmail,
                    30,
                    &MASK_WEB,
                    "",
                    Some(new_user.email.clone()),
                    display_flags::FIELDLEN | display_flags::UPCASE | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
            if !answer.is_empty() {
                new_user.email = answer;
            }
        }

        if settings.ask_web_address {
            let answer = self
                .input_field(
                    IceText::EnterWebAddress,
                    30,
                    &MASK_WEB,
                    "",
                    Some(new_user.web.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
            if !answer.is_empty() {
                new_user.web = answer;
            }
        }

        self.session.current_user = Some(new_user);
        self.save_current_user().await?;

        self.display_text(
            IceText::UserRecordUpdated,
            display_flags::NEWLINE | display_flags::LFAFTER | display_flags::LFBEFORE,
        )
        .await?;
        Ok(())
    }

    /// A yes/no question that keeps the current setting when the user just
    /// presses Enter, the way PCBoard's `getfield` pre-loads the answer buffer.
    pub(crate) async fn ask_yes_no(&mut self, text: IceText, current: bool) -> Res<bool> {
        let yes_char = self.session.yes_char.to_ascii_uppercase();
        let default = if current { yes_char } else { self.session.no_char.to_ascii_uppercase() };
        let answer = self
            .input_field(
                text,
                1,
                "",
                "",
                Some(default.to_string()),
                display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE | display_flags::YESNO,
            )
            .await?;
        let Some(ch) = answer.chars().next() else {
            return Ok(current);
        };
        Ok(ch.to_ascii_uppercase() == yes_char)
    }

    pub async fn ask_date_format(&mut self, cur_format: &str) -> Res<String> {
        self.new_line().await?;
        let date_formats = self.get_board().await.languages.date_formats.clone();

        self.set_color(TerminalTarget::Both, IcbColor::dos_light_cyan()).await?;
        let mut preview = String::new();
        for (i, (disp_fmt, fmt)) in date_formats.iter().enumerate() {
            if fmt == cur_format {
                preview = (i + 1).to_string();
                self.println(TerminalTarget::Both, &format!("=> ({}) {}", i + 1, disp_fmt)).await?;
            } else {
                self.println(TerminalTarget::Both, &format!("   ({}) {}", i + 1, disp_fmt)).await?;
            }
        }
        let date_format = self
            .input_field(
                IceText::DateFormatDesired,
                1,
                &MASK_NUM,
                "",
                Some(preview),
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER | display_flags::UPCASE | display_flags::FIELDLEN,
            )
            .await?;
        if let Ok(number) = date_format.parse::<usize>() {
            if number > 0 && number <= date_formats.len() {
                return Ok(date_formats[number - 1].1.clone());
            }
        }
        return Ok(cur_format.to_string());
    }
}
