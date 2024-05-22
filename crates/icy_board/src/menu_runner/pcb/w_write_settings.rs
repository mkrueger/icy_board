use crate::{menu_runner::PcbBoardCommand, Res};
use icy_board_engine::{
    datetime::IcbDate,
    icy_board::{
        icb_text::IceText,
        state::functions::{display_flags, MASK_ALNUM, MASK_PHONE, MASK_WEB},
        user_base::FSEMode,
    },
};

impl PcbBoardCommand {
    pub async fn write_settings(&mut self) -> Res<()> {
        self.state
            .display_text(IceText::EnterNoChange, display_flags::LFBEFORE | display_flags::NEWLINE)
            .await?;
        let settings = self.state.get_board().await.config.new_user_settings.clone();
        let Some(mut new_user) = self.state.session.current_user.clone() else {
            return Ok(());
        };

        if settings.ask_city_or_state {
            new_user.city_or_state = self
                .state
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

        if settings.ask_bus_data_phone {
            new_user.bus_data_phone = self
                .state
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

        if settings.ask_voice_phone {
            new_user.home_voice_phone = self
                .state
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
                .state
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
                .state
                .input_field(
                    IceText::CLSBetweenMessages,
                    1,
                    &"",
                    "",
                    Some(
                        if new_user.flags.msg_clear {
                            self.state.session.yes_char
                        } else {
                            self.state.session.no_char
                        }
                        .to_uppercase()
                        .to_string(),
                    ),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::YESNO,
                )
                .await?;
            new_user.flags.msg_clear = msg_cls.is_empty() || msg_cls == self.state.session.yes_char.to_uppercase().to_string();
        }

        if settings.ask_fse {
            let str = match new_user.flags.fse_mode {
                FSEMode::Yes => "Y",
                FSEMode::No => "N",
                FSEMode::Ask => "A",
            };
            let fse_default = self
                .state
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
            let protocol = self.ask_protocols("N".to_string()).await?;
            if !protocol.is_empty() {
                new_user.protocol = protocol;
            } else {
                new_user.protocol = "N".to_string();
            }
        }

        if settings.ask_alias {
            new_user.alias = self
                .state
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
            self.state
                .display_text(IceText::EnterAddress, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            new_user.street1 = self
                .state
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
                .state
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
                .state
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
                .state
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
                .state
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
                .state
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
                .state
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
                .state
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
                .state
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
            new_user.email = self
                .state
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
                .state
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
                .state
                .input_field(
                    IceText::UseShortDescription,
                    1,
                    "",
                    "",
                    Some(
                        if new_user.flags.use_short_filedescr {
                            self.state.session.yes_char
                        } else {
                            self.state.session.no_char
                        }
                        .to_uppercase()
                        .to_string(),
                    ),
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::YESNO,
                )
                .await?;
            new_user.flags.use_short_filedescr = use_short == self.state.session.yes_char.to_ascii_uppercase().to_string();
        }
        self.state.session.current_user = Some(new_user);
        self.state.save_current_user().await?;

        self.state
            .display_text(
                IceText::UserRecordUpdated,
                display_flags::NEWLINE | display_flags::LFAFTER | display_flags::LFBEFORE,
            )
            .await?;
        self.state.press_enter().await?;
        self.display_menu = true;
        Ok(())
    }
}
