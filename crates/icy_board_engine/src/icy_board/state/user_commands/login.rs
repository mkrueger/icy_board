use crate::icy_board::state::functions::MASK_NUM;
use crate::icy_board::state::IcyBoardState;
use crate::VERSION;

use crate::Res;
use crate::{
    datetime::{IcbDate, IcbTime},
    icy_board::{
        icb_config::{IcbColor, DEFAULT_PCBOARD_DATE_FORMAT},
        icb_text::IceText,
        security_expr::SecurityExpression,
        state::{
            functions::{display_flags, pwd_flags, MASK_ALNUM, MASK_PHONE, MASK_WEB},
            NodeStatus,
        },
        surveys::Survey,
        user_base::{Password, User},
    },
    vm::TerminalTarget,
};
use chrono::{Datelike, Local, Utc};
use icy_net::iemsi::try_iemsi;
impl IcyBoardState {
    pub async fn login(&mut self) -> Res<bool> {
        self.set_activity(NodeStatus::LogIntoSystem).await;

        self.reset_color(TerminalTarget::Both).await?;
        self.clear_screen(TerminalTarget::Both).await?;
        self.get_term_caps()?;
        self.session.login_date = chrono::Local::now();

        // intial_welcome
        let board_name = self.get_board().await.config.board.name.clone();
        self.println(TerminalTarget::Both, &format!("CONNECT {} ({})", IcbDate::today(), IcbTime::now()))
            .await?;
        self.new_line().await?;
        self.println(TerminalTarget::Both, &board_name).await?;
        let node_number = self.node;
        self.println(TerminalTarget::Both, &format!("IcyBoard v{} - Node {}", VERSION.to_string(), node_number))
            .await?;

        let welcome_screen = self.get_board().await.config.paths.welcome.clone();
        let welcome_screen = self.resolve_path(&welcome_screen);
        self.display_file(&welcome_screen).await?;
        self.new_line().await?;

        let mut tries = 0;
        if self.get_board().await.config.options.allow_iemsi {
            let (name, location, operator, notice, caps) = {
                let board = self.get_board().await;
                (
                    board.config.board.name.clone(),
                    board.config.board.location.clone(),
                    board.config.board.operator.clone(),
                    board.config.board.notice.clone(),
                    board.config.board.capabilities.clone(),
                )
            };

            if let Some(settings) = try_iemsi(&mut self.connection, name, location, operator, notice, caps).await? {
                self.session.emsi = Some(settings);
            }
        }

        loop {
            tries += 1;
            if tries > 3 {
                log::warn!("Login at {} num login tries exceeded.", Local::now().to_rfc2822());
                self.display_text(IceText::DeniedRefuseToRegister, display_flags::NEWLINE).await?;
                self.hangup().await?;
                return Ok(false);
            }

            let first_name = if let Some(ici) = &self.session.emsi {
                ici.user.name.clone()
            } else {
                self.input_field(
                    IceText::YourFirstName,
                    39,
                    &MASK_ALNUM,
                    "",
                    None,
                    display_flags::UPCASE | display_flags::NEWLINE | display_flags::STACKED,
                )
                .await?
                .trim()
                .to_string()
            };

            if first_name.is_empty() {
                continue;
            }

            let mut found_user = None;
            for (i, user) in self.get_board().await.users.iter().enumerate() {
                if user.is_valid_loginname(&first_name) {
                    found_user = Some(i);
                    break;
                }
            }

            if found_user.is_none() && !first_name.contains(' ') {
                let last_name = self
                    .input_field(
                        IceText::YourLastName,
                        39,
                        &MASK_ALNUM,
                        "",
                        None,
                        display_flags::UPCASE | display_flags::NEWLINE | display_flags::STACKED,
                    )
                    .await?;
                if !self.get_board().await.config.new_user_settings.allow_one_name_users && last_name.is_empty() {
                    self.display_text(
                        IceText::RequireTwoNames,
                        display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
                    )
                    .await?;
                    continue;
                }
                self.session.user_name = format!("{} {}", first_name, last_name.trim()).trim().to_string();

                for (i, user) in self.get_board().await.users.iter().enumerate() {
                    if user.is_valid_loginname(&self.session.user_name) {
                        found_user = Some(i);
                        break;
                    }
                }
            } else {
                self.session.user_name = first_name.to_string();
            }
            if let Some(user) = found_user {
                self.set_current_user(user).await?;
                return self.login_user().await;
            } else {
                self.session.op_text = self.session.user_name.clone();
                self.display_text(IceText::NotInUsersFile, display_flags::NEWLINE).await?;
            }

            let re_enter = self
                .input_field(
                    IceText::ReEnterName,
                    1,
                    &"RC",
                    "",
                    Some("C".to_string()),
                    display_flags::UPCASE | display_flags::NEWLINE | display_flags::FIELDLEN,
                )
                .await?;

            if re_enter.trim().is_empty() || re_enter == "C" {
                let new_file = self.get_board().await.config.paths.newuser.clone();
                self.display_file(&self.resolve_path(&new_file)).await?;
                self.new_line().await?;

                let register = self
                    .input_field(
                        IceText::Register,
                        1,
                        "",
                        "",
                        Some("Y".to_string()),
                        display_flags::YESNO | display_flags::NEWLINE | display_flags::FIELDLEN,
                    )
                    .await?;
                if register == "Y" || register.trim().is_empty() {
                    if !self.new_user().await? {
                        self.display_text(IceText::RefusedToRegister, display_flags::NEWLINE).await?;
                        self.hangup().await?;
                        log::info!("'{}' refused to register.", self.session.user_name);
                        return Ok(false);
                    }
                    return Ok(true);
                } else {
                    self.display_text(IceText::RefusedToRegister, display_flags::NEWLINE).await?;
                    self.hangup().await?;
                    log::info!("'{}' refused to register.", self.session.user_name);
                    return Ok(false);
                }
            }
            // clear emsi data
            self.session.emsi = None;
        }
    }

    async fn new_user(&mut self) -> Res<bool> {
        let mut tries = 0;

        if self.get_board().await.config.options.is_closed_board {
            self.newask_questions().await?;
            log::info!("New user registration for {} attempted on closed board.", self.session.user_name);
            self.display_text(IceText::ClosedBoard, display_flags::NEWLINE).await?;
            self.hangup().await?;
            return Ok(false);
        }

        let mut new_user = User::default();
        let settings = self.get_board().await.config.new_user_settings.clone();
        new_user.security_level = settings.sec_level;
        new_user.stats.first_date_on = Utc::now();
        new_user.set_name(self.session.user_name.clone());
        loop {
            tries += 1;
            if tries > 4 {
                return Ok(false);
            }
            let Some(pw1) = self.input_required(IceText::NewPassword, &MASK_ALNUM, 20, display_flags::ECHODOTS).await? else {
                return Ok(false);
            };
            let Some(pw2) = self.input_required(IceText::ReEnterPassword, &MASK_ALNUM, 20, display_flags::ECHODOTS).await? else {
                return Ok(false);
            };

            if pw1 == pw2 {
                new_user.password.password = Password::PlainText(pw1);
                break;
            }
            let exp_days = self.get_board().await.config.user_password_policy.password_expire_days;
            if exp_days > 0 {
                new_user.password.expire_date = Utc::now() + chrono::Duration::days(exp_days as i64);
            }
            self.display_text(IceText::PasswordsDontMatch, display_flags::NEWLINE).await?;
        }

        if !self.newask_exists().await || self.get_board().await.config.new_user_settings.use_newask_and_builtin {
            if settings.ask_city_or_state && self.display_text.has_text(IceText::CityState) {
                let Some(city_or_state) = self.input_required(IceText::CityState, &MASK_ALNUM, 24, 0).await? else {
                    return Ok(false);
                };
                new_user.city_or_state = city_or_state;
            }

            if settings.ask_bus_data_phone && self.display_text.has_text(IceText::BusDataPhone) {
                let Some(bus_data_phone) = self.input_required(IceText::BusDataPhone, &MASK_PHONE, 13, 0).await? else {
                    return Ok(false);
                };
                new_user.bus_data_phone = bus_data_phone;
            }

            if settings.ask_voice_phone && self.display_text.has_text(IceText::HomeVoicePhone) {
                let Some(home_voice_phone) = self.input_required(IceText::HomeVoicePhone, &MASK_PHONE, 13, 0).await? else {
                    return Ok(false);
                };
                new_user.home_voice_phone = home_voice_phone;
            }

            if settings.ask_comment && self.display_text.has_text(IceText::CommentFieldPrompt) {
                new_user.user_comment = self
                    .input_field(
                        IceText::CommentFieldPrompt,
                        30,
                        &MASK_ALNUM,
                        "",
                        None,
                        display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                    )
                    .await?;
            }

            if settings.ask_clr_msg && self.display_text.has_text(IceText::CLSBetweenMessages) {
                let msg_cls = self
                    .input_field(
                        IceText::CLSBetweenMessages,
                        1,
                        &"",
                        "",
                        Some(self.session.yes_char.to_uppercase().to_string()),
                        display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::YESNO,
                    )
                    .await?;
                new_user.flags.msg_clear = msg_cls.is_empty() || msg_cls == self.session.yes_char.to_uppercase().to_string();
            }

            if settings.ask_date_format && self.display_text.has_text(IceText::DateFormatDesired) {
                new_user.date_format = DEFAULT_PCBOARD_DATE_FORMAT.to_string();
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

            if settings.ask_alias && self.display_text.has_text(IceText::GetAliasName) {
                new_user.alias = self
                    .input_field(
                        IceText::GetAliasName,
                        30,
                        &MASK_ALNUM,
                        "",
                        None,
                        display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                    )
                    .await?;
            }

            if settings.ask_address && self.display_text.has_text(IceText::EnterAddress) {
                self.display_text(IceText::EnterAddress, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;

                if self.display_text.has_text(IceText::Street1) {
                    new_user.street1 = self
                        .input_field(
                            IceText::Street1,
                            50,
                            &MASK_ALNUM,
                            "",
                            None,
                            display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                        )
                        .await?;
                }
                if self.display_text.has_text(IceText::Street2) {
                    new_user.street2 = self
                        .input_field(
                            IceText::Street2,
                            50,
                            &MASK_ALNUM,
                            "",
                            None,
                            display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                        )
                        .await?;
                }
                if self.display_text.has_text(IceText::City) {
                    new_user.city = self
                        .input_field(
                            IceText::City,
                            25,
                            &MASK_ALNUM,
                            "",
                            None,
                            display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                        )
                        .await?;
                }
                if self.display_text.has_text(IceText::State) {
                    new_user.state = self
                        .input_field(
                            IceText::State,
                            15,
                            &MASK_ALNUM,
                            "",
                            None,
                            display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                        )
                        .await?;
                }
                if self.display_text.has_text(IceText::Zip) {
                    new_user.zip = self
                        .input_field(
                            IceText::Zip,
                            10,
                            &MASK_ALNUM,
                            "",
                            None,
                            display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                        )
                        .await?;
                }
                if self.display_text.has_text(IceText::Country) {
                    new_user.country = self
                        .input_field(
                            IceText::Country,
                            15,
                            &MASK_ALNUM,
                            "",
                            None,
                            display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                        )
                        .await?;
                }
            }

            if settings.ask_verification && self.display_text.has_text(IceText::EnterVerifyText) {
                let Some(verify_answer) = self.input_required(IceText::EnterVerifyText, &MASK_ALNUM, 25, 0).await? else {
                    return Ok(false);
                };
                new_user.verify_answer = verify_answer;
            }

            if settings.ask_gender && self.display_text.has_text(IceText::EnterGender) {
                new_user.gender = self
                    .input_field(
                        IceText::EnterGender,
                        1,
                        "MmFf",
                        "",
                        None,
                        display_flags::FIELDLEN | display_flags::UPCASE | display_flags::NEWLINE | display_flags::LFBEFORE,
                    )
                    .await?;
            }

            if settings.ask_birthdate && self.display_text.has_text(IceText::EnterBirthdate) {
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
                new_user.birth_date = IcbDate::parse(&date).to_utc_date_time();
            }

            if settings.ask_email && self.display_text.has_text(IceText::EnterEmail) {
                let Some(email) = self.input_required(IceText::EnterEmail, &MASK_WEB, 30, 0).await? else {
                    return Ok(false);
                };
                new_user.email = email;
            }

            if settings.ask_web_address && self.display_text.has_text(IceText::EnterWebAddress) {
                new_user.web = self
                    .input_field(
                        IceText::EnterWebAddress,
                        30,
                        &MASK_WEB,
                        "",
                        None,
                        display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                    )
                    .await?;
            }

            if settings.ask_use_short_descr && self.display_text.has_text(IceText::UseShortDescription) {
                let use_short = self
                    .input_field(
                        IceText::UseShortDescription,
                        1,
                        "",
                        "",
                        Some("N".to_string()),
                        display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::YESNO,
                    )
                    .await?;
                new_user.flags.use_short_filedescr = use_short == "Y";
            }
        }
        self.newask_questions().await?;

        let id = self.get_board().await.users.new_user(new_user);
        self.get_board().await.save_userbase()?;
        self.set_current_user(id).await?;

        log::info!("NEW USER: '{}'", self.session.user_name);

        self.display_news().await?;
        return Ok(true);
    }

    async fn newask_exists(&self) -> bool {
        let board = self.get_board().await;
        board.resolve_file(&board.config.paths.newask_survey).exists()
    }

    async fn newask_questions(&mut self) -> Res<()> {
        let survey = {
            let board = self.get_board().await;
            Survey {
                survey_file: board.resolve_file(&board.config.paths.newask_survey),
                answer_file: board.resolve_file(&board.config.paths.newask_answer),
                required_security: SecurityExpression::default(),
            }
        };
        Ok(if !self.session.is_sysop && survey.survey_file.exists() {
            // skip the survey question.
            self.session.tokens.push_front(self.session.yes_char.to_string());
            self.start_survey(&survey).await?;
        })
    }

    async fn login_user(&mut self) -> Res<bool> {
        let check_password = if let Some(user) = &self.session.current_user {
            if user.flags.delete_flag || user.flags.disabled_flag {
                self.display_text(IceText::DeniedLockedOut, display_flags::NEWLINE).await?;
                self.hangup().await?;
                return Ok(false);
            }

            let pw = user.password.password.clone();

            let mut emsi_pw = false;
            if let Some(emsi) = &self.session.emsi {
                if emsi.user.password.eq_ignore_ascii_case(pw.to_string().as_str()) {
                    emsi_pw = true;
                }
            }

            emsi_pw
                || self
                    .check_password(IceText::YourPassword, pwd_flags::SHOW_WRONG_PWD_MSG, |pwd| pw.is_valid(pwd))
                    .await?
        } else {
            log::warn!("login_user: User missing (should never happen -> bug)");
            return Ok(false);
        };

        if !check_password {
            log::warn!("Login from {} at {} password failed", self.session.user_name, Local::now().to_rfc2822());
            self.display_text(IceText::DeniedPasswordFailed, display_flags::NEWLINE).await?;
            self.hangup().await?;
            return Ok(false);
        }

        if self.get_board().await.config.subscription_info.is_enabled {
            if let Some(user) = &self.session.current_user {
                if user.exp_date < Utc::now() {
                    log::warn!("Login from expired user {} at {}", self.session.user_name, Local::now().to_rfc2822());
                    let exp_file = self.get_board().await.config.paths.expired.clone();
                    self.display_file(&self.resolve_path(&exp_file)).await?;
                    self.hangup().await?;
                    return Ok(false);
                }
                let warn_days = self.get_board().await.config.subscription_info.warning_days as i64;
                if user.exp_date + chrono::Duration::days(warn_days) < Utc::now() {
                    let exp_file = self.get_board().await.config.paths.expire_warning.clone();
                    self.display_file(&self.resolve_path(&exp_file)).await?;
                    self.press_enter().await?;
                }
            }
        }

        if let Some(user) = &self.session.current_user {
            if !user.password.expire_date.year() > 0 {
                let today = Utc::now();
                if user.password.expire_date > today {
                    self.display_text(IceText::PasswordExpired, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                    self.request_change_password().await?;
                    return Ok(false);
                }

                let days = self.get_board().await.config.user_password_policy.password_expire_warn_days as i64;

                if user.password.expire_date + chrono::Duration::days(days) > today {
                    self.session.op_text = (user.password.expire_date + chrono::Duration::days(days) - today).num_days().to_string();
                    self.display_text(IceText::PasswordWillExpired, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                    self.press_enter().await?;
                    return Ok(false);
                }
            }
        }

        log::warn!("Login from {} at {}", self.session.user_name, Local::now().to_rfc2822());
        return Ok(true);
    }

    async fn input_required(&mut self, txt: IceText, mask: &str, len: i32, flags: i32) -> Res<Option<String>> {
        let mut tries = 0;
        loop {
            tries += 1;
            if tries > 3 {
                return Ok(None);
            }

            let name = self
                .input_field(
                    txt,
                    len,
                    mask,
                    "",
                    None,
                    flags | display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?;

            if name.is_empty() {
                self.display_text(IceText::ResponseRequired, display_flags::NEWLINE).await?;
            } else {
                return Ok(Some(name));
            }
        }
    }

    async fn request_change_password(&mut self) -> Res<()> {
        loop {
            let Some(pw1) = self.input_required(IceText::NewPassword, &MASK_ALNUM, 20, display_flags::ECHODOTS).await? else {
                return Ok(());
            };

            if !self.is_valid_password(&pw1).await? {
                self.display_text(IceText::PasswordTooShort, display_flags::NEWLINE).await?;
                continue;
            }

            let Some(pw2) = self.input_required(IceText::ReEnterPassword, &MASK_ALNUM, 20, display_flags::ECHODOTS).await? else {
                return Ok(());
            };

            if pw1 == pw2 {
                self.change_password(&pw1).await?;
                return Ok(());
            }
            self.display_text(IceText::PasswordsDontMatch, display_flags::NEWLINE).await?;
        }
    }

    pub async fn ask_date_format(&mut self, cur_format: &str) -> Res<String> {
        self.new_line().await?;
        let date_formats = self.get_board().await.languages.date_formats.clone();

        self.set_color(TerminalTarget::Both, IcbColor::Dos(11)).await?;
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
