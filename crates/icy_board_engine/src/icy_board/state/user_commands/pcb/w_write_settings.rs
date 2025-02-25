use crate::{Res, icy_board::state::IcyBoardState};
use crate::{
    datetime::IcbDate,
    icy_board::{
        icb_config::IcbColor,
        icb_text::IceText,
        state::functions::{MASK_ALNUM, MASK_NUM, MASK_PHONE, MASK_WEB, display_flags},
        user_base::FSEMode,
    },
    vm::TerminalTarget,
};

impl IcyBoardState {
    pub async fn write_settings(&mut self) -> Res<()> {
        self.display_text(IceText::EnterNoChange, display_flags::LFBEFORE | display_flags::NEWLINE)
            .await?;
        let settings = self.get_board().await.config.new_user_settings.clone();
        let Some(mut new_user) = self.session.current_user.clone() else {
            return Ok(());
        };

        if settings.ask_city_or_state {
            new_user.city_or_state = self
                .input_field(
                    IceText::CityState,
                    24,
                    &MASK_ALNUM,
                    "",
                    Some(new_user.city_or_state.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
        }

        if settings.ask_business_phone {
            new_user.bus_data_phone = self
                .input_field(
                    IceText::BusDataPhone,
                    13,
                    &MASK_PHONE,
                    "",
                    Some(new_user.bus_data_phone.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
        }

        if settings.ask_home_phone {
            new_user.home_voice_phone = self
                .input_field(
                    IceText::HomeVoicePhone,
                    13,
                    &MASK_PHONE,
                    "",
                    Some(new_user.home_voice_phone.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
        }

        if settings.ask_comment {
            new_user.user_comment = self
                .input_field(
                    IceText::CommentFieldPrompt,
                    30,
                    &MASK_ALNUM,
                    "",
                    Some(new_user.user_comment.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
        }

        if settings.ask_clr_msg {
            let msg_cls = self
                .input_field(
                    IceText::CLSBetweenMessages,
                    1,
                    &"",
                    "",
                    Some(
                        if new_user.flags.msg_clear {
                            self.session.yes_char
                        } else {
                            self.session.no_char
                        }
                        .to_uppercase()
                        .to_string(),
                    ),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::YESNO,
                )
                .await?;
            new_user.flags.msg_clear = msg_cls.is_empty() || msg_cls == self.session.yes_char.to_uppercase().to_string();
        }

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

        if settings.ask_alias {
            new_user.alias = self
                .input_field(
                    IceText::GetAliasName,
                    30,
                    &MASK_ALNUM,
                    "",
                    Some(new_user.alias.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
        }

        if settings.ask_address {
            self.display_text(IceText::EnterAddress, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            new_user.street1 = self
                .input_field(
                    IceText::Street1,
                    50,
                    &MASK_ALNUM,
                    "",
                    Some(new_user.street1.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;

            new_user.street2 = self
                .input_field(
                    IceText::Street2,
                    50,
                    &MASK_ALNUM,
                    "",
                    Some(new_user.street2.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;

            new_user.city = self
                .input_field(
                    IceText::City,
                    25,
                    &MASK_ALNUM,
                    "",
                    Some(new_user.city.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;

            new_user.state = self
                .input_field(
                    IceText::State,
                    15,
                    &MASK_ALNUM,
                    "",
                    Some(new_user.state.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;

            new_user.zip = self
                .input_field(
                    IceText::Zip,
                    10,
                    &MASK_ALNUM,
                    "",
                    Some(new_user.zip.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;

            new_user.country = self
                .input_field(
                    IceText::Country,
                    15,
                    &MASK_ALNUM,
                    "",
                    Some(new_user.country.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
        }

        if settings.ask_verification {
            new_user.verify_answer = self
                .input_field(
                    IceText::EnterVerifyText,
                    25,
                    &MASK_ALNUM,
                    "",
                    Some(new_user.verify_answer.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
        }

        if settings.ask_gender {
            new_user.gender = self
                .input_field(
                    IceText::EnterGender,
                    1,
                    "MmFf",
                    "",
                    Some(new_user.gender.clone()),
                    display_flags::FIELDLEN | display_flags::UPCASE | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
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
                new_user.birth_day = Some(IcbDate::parse(&date));
            }
        }

        if settings.ask_email {
            new_user.email = self
                .input_field(
                    IceText::EnterEmail,
                    30,
                    &MASK_WEB,
                    "",
                    Some(new_user.email.clone()),
                    display_flags::FIELDLEN | display_flags::UPCASE | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
        }

        if settings.ask_web_address {
            new_user.web = self
                .input_field(
                    IceText::EnterWebAddress,
                    30,
                    &MASK_WEB,
                    "",
                    Some(new_user.web.clone()),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;
        }

        if settings.ask_use_short_descr {
            let use_short = self
                .input_field(
                    IceText::UseShortDescription,
                    1,
                    "",
                    "",
                    Some(
                        if new_user.flags.use_short_filedescr {
                            self.session.yes_char
                        } else {
                            self.session.no_char
                        }
                        .to_uppercase()
                        .to_string(),
                    ),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::YESNO,
                )
                .await?;
            new_user.flags.use_short_filedescr = use_short == self.session.yes_char.to_ascii_uppercase().to_string();
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

    pub async fn ask_date_format(&mut self, cur_format: &str) -> Res<String> {
        self.new_line().await?;
        let date_formats = self.get_board().await.languages.date_formats.clone();

        self.set_color(TerminalTarget::Both, IcbColor::dos_cyan()).await?;
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
