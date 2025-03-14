use crate::VERSION;

use super::PcbBoardCommand;
use crate::Res;
use chrono::{Datelike, Local, Utc};
use icy_board_engine::{
    datetime::{IcbDate, IcbTime},
    icy_board::{
        icb_config::DEFAULT_PCBOARD_DATE_FORMAT,
        icb_text::IceText,
        security_expr::SecurityExpression,
        state::{
            NodeStatus,
            functions::{MASK_ALNUM, MASK_PHONE, MASK_WEB, display_flags, pwd_flags},
        },
        surveys::Survey,
        user_base::{Password, User},
    },
    vm::TerminalTarget,
};
use icy_net::iemsi::try_iemsi;
use tokio::fs;
impl PcbBoardCommand {
    pub async fn login(&mut self, is_local: bool) -> Res<bool> {
        self.state.set_activity(NodeStatus::LogIntoSystem).await;

        self.state.reset_color(TerminalTarget::Both).await?;
        self.state.clear_screen(TerminalTarget::Both).await?;
        self.state.session.disp_options.count_lines = false;
        self.state.session.login_date = chrono::Utc::now();

        // intial_welcome
        let board_name = self.state.get_board().await.config.board.name.clone();
        self.state
            .println(TerminalTarget::Both, &format!("CONNECT {} ({})", IcbDate::today(), IcbTime::now()))
            .await?;
        self.state.new_line().await?;
        self.state.println(TerminalTarget::Both, &board_name).await?;
        let node_number = self.state.node;
        self.state
            .println(TerminalTarget::Both, &format!("IcyBoard v{} - Node {}", VERSION.to_string(), node_number))
            .await?;

        let welcome_screen = self.state.get_board().await.config.paths.welcome.clone();
        let welcome_screen = self.state.resolve_path(&welcome_screen);
        self.state.display_file(&welcome_screen).await?;
        self.state.new_line().await?;

        let mut tries = 0;
        if !is_local && self.state.get_board().await.config.board.allow_iemsi {
            let (name, location, operator, notice, caps) = {
                let board = self.state.get_board().await;
                (
                    board.config.board.name.clone(),
                    board.config.board.location.clone(),
                    board.config.board.operator.clone(),
                    board.config.board.notice.clone(),
                    board.config.board.capabilities.clone(),
                )
            };

            if let Some(settings) = try_iemsi(&mut self.state.connection, name, location, operator, notice, caps).await? {
                self.state.session.emsi = Some(settings);
            }
        }

        loop {
            tries += 1;
            if tries > 3 {
                log::warn!("Login at {} num login tries exceeded.", Local::now().to_rfc2822());
                self.state.display_text(IceText::DeniedRefuseToRegister, display_flags::NEWLINE).await?;
                self.state.hangup().await?;
                return Ok(false);
            }

            let first_name = if let Some(ici) = &self.state.session.emsi {
                ici.user.name.clone()
            } else {
                self.state
                    .input_field(
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
            for (i, user) in self.state.get_board().await.users.iter().enumerate() {
                if user.is_valid_loginname(&first_name) {
                    found_user = Some(i);
                    break;
                }
            }

            if found_user.is_none() && !first_name.contains(' ') {
                let last_name = self
                    .state
                    .input_field(
                        IceText::YourLastName,
                        39,
                        &MASK_ALNUM,
                        "",
                        None,
                        display_flags::UPCASE | display_flags::NEWLINE | display_flags::STACKED,
                    )
                    .await?;
                if !self.state.get_board().await.config.new_user_settings.allow_one_name_users && last_name.is_empty() {
                    self.state
                        .display_text(
                            IceText::RequireTwoNames,
                            display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
                        )
                        .await?;
                    continue;
                }
                self.state.session.user_name = format!("{} {}", first_name, last_name.trim()).trim().to_string();

                for (i, user) in self.state.get_board().await.users.iter().enumerate() {
                    if user.is_valid_loginname(&self.state.session.user_name) {
                        found_user = Some(i);
                        break;
                    }
                }
            } else {
                self.state.session.user_name = first_name.to_string();
            }
            if let Some(user) = found_user {
                self.state.set_current_user(user).await?;
                return self.login_user().await;
            } else {
                self.state.session.op_text = self.state.session.user_name.clone();
                self.state.display_text(IceText::NotInUsersFile, display_flags::NEWLINE).await?;
            }

            let re_enter = self
                .state
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
                let new_file = self.state.get_board().await.config.paths.newuser.clone();
                self.state.display_file(&self.state.resolve_path(&new_file)).await?;
                self.state.new_line().await?;

                let register = self
                    .state
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
                        self.state.display_text(IceText::RefusedToRegister, display_flags::NEWLINE).await?;
                        self.state.hangup().await?;
                        log::info!("'{}' refused to register.", self.state.session.user_name);
                        return Ok(false);
                    }
                    return Ok(true);
                } else {
                    self.state.display_text(IceText::RefusedToRegister, display_flags::NEWLINE).await?;
                    self.state.hangup().await?;
                    log::info!("'{}' refused to register.", self.state.session.user_name);
                    return Ok(false);
                }
            }
            // clear emsi data
            self.state.session.emsi = None;
        }
    }

    async fn new_user(&mut self) -> Res<bool> {
        let mut tries = 0;

        if self.state.get_board().await.config.system_control.is_closed_board {
            self.newask_questions().await?;
            log::info!("New user registration for {} attempted on closed board.", self.state.session.user_name);

            let closed_path = self.state.resolve_path(&self.state.get_board().await.config.paths.closed);
            if closed_path.is_file() {
                self.state.display_file(&closed_path).await?;
            }
            self.state.display_text(IceText::ClosedBoard, display_flags::NEWLINE).await?;
            self.state.hangup().await?;
            return Ok(false);
        }

        let trashcan_user = self.state.resolve_path(&self.state.get_board().await.config.paths.trashcan_user);
        if trashcan_user.is_file() {
            let users = fs::read_to_string(trashcan_user).await?;
            for line in users.lines().filter(|p| !p.is_empty() && !p.starts_with('#')) {
                if line.eq_ignore_ascii_case(&self.state.session.user_name) {
                    self.state.display_text(IceText::RealNamesOnly, display_flags::NEWLINE).await?;
                    self.state.hangup().await?;
                    return Ok(false);
                }
            }
        }

        let mut new_user = User::default();
        let settings = self.state.get_board().await.config.new_user_settings.clone();
        new_user.security_level = settings.sec_level;
        new_user.stats.first_date_on = Utc::now();
        new_user.set_name(self.state.session.user_name.clone());
        loop {
            tries += 1;
            if tries > 4 {
                return Ok(false);
            }
            let Some(pw1) = self.input_required(IceText::NewPassword, &MASK_ALNUM, 20, display_flags::ECHODOTS).await? else {
                return Ok(false);
            };
            if !self.state.is_valid_password(&pw1).await? {
                self.state.display_text(IceText::PasswordTooShort, display_flags::NEWLINE).await?;
                continue;
            }

            let trashcan_passwords = self.state.resolve_path(&self.state.get_board().await.config.paths.trashcan_passwords);
            if trashcan_passwords.is_file() {
                let users = fs::read_to_string(trashcan_passwords).await?;
                if users
                    .lines()
                    .filter(|p| !p.is_empty() && !p.starts_with('#'))
                    .any(|p| p.eq_ignore_ascii_case(&pw1))
                {
                    self.state.display_text(IceText::PasswordTooWeak, display_flags::NEWLINE).await?;
                    continue;
                }
            }

            let Some(pw2) = self.input_required(IceText::ReEnterPassword, &MASK_ALNUM, 20, display_flags::ECHODOTS).await? else {
                return Ok(false);
            };

            if pw1 == pw2 {
                new_user.password.password = Password::PlainText(pw1);
                break;
            }
            let exp_days = self.state.get_board().await.config.limits.password_expire_days;
            if exp_days > 0 {
                new_user.password.expire_date = Utc::now() + chrono::Duration::days(exp_days as i64);
            }
            self.state.display_text(IceText::PasswordsDontMatch, display_flags::NEWLINE).await?;
        }

        if !self.newask_exists().await || self.state.get_board().await.config.new_user_settings.use_newask_and_builtin {
            if settings.ask_city_or_state && self.state.display_text.has_text(IceText::CityState) {
                let Some(city_or_state) = self.input_required(IceText::CityState, &MASK_ALNUM, 24, 0).await? else {
                    return Ok(false);
                };
                new_user.city_or_state = city_or_state;
            }

            if settings.ask_business_phone && self.state.display_text.has_text(IceText::BusDataPhone) {
                let Some(bus_data_phone) = self.input_required(IceText::BusDataPhone, &MASK_PHONE, 13, 0).await? else {
                    return Ok(false);
                };
                new_user.bus_data_phone = bus_data_phone;
            }

            if settings.ask_home_phone && self.state.display_text.has_text(IceText::HomeVoicePhone) {
                let Some(home_voice_phone) = self.input_required(IceText::HomeVoicePhone, &MASK_PHONE, 13, 0).await? else {
                    return Ok(false);
                };
                new_user.home_voice_phone = home_voice_phone;
            }

            if settings.ask_comment && self.state.display_text.has_text(IceText::CommentFieldPrompt) {
                new_user.user_comment = self
                    .state
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

            if settings.ask_clr_msg && self.state.display_text.has_text(IceText::CLSBetweenMessages) {
                let msg_cls = self
                    .state
                    .input_field(
                        IceText::CLSBetweenMessages,
                        1,
                        &"",
                        "",
                        Some(self.state.session.yes_char.to_uppercase().to_string()),
                        display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::YESNO,
                    )
                    .await?;
                new_user.flags.msg_clear = msg_cls.is_empty() || msg_cls == self.state.session.yes_char.to_uppercase().to_string();
            }

            if settings.ask_date_format && self.state.display_text.has_text(IceText::DateFormatDesired) {
                new_user.date_format = DEFAULT_PCBOARD_DATE_FORMAT.to_string();
                let date_format = self.state.ask_date_format(&new_user.date_format).await?;
                if !date_format.is_empty() {
                    new_user.date_format = date_format;
                }
            }
            if settings.ask_xfer_protocol {
                let protocol = self.state.ask_protocols("N").await?;
                self.state.new_line().await?;
                if !protocol.is_empty() {
                    new_user.protocol = protocol;
                } else {
                    new_user.protocol = "N".to_string();
                }
            }

            if settings.ask_alias && self.state.display_text.has_text(IceText::GetAliasName) {
                new_user.alias = self
                    .state
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

            if settings.ask_address && self.state.display_text.has_text(IceText::EnterAddress) {
                self.state
                    .display_text(IceText::EnterAddress, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;

                if self.state.display_text.has_text(IceText::Street1) {
                    new_user.street1 = self
                        .state
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
                if self.state.display_text.has_text(IceText::Street2) {
                    new_user.street2 = self
                        .state
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
                if self.state.display_text.has_text(IceText::City) {
                    new_user.city = self
                        .state
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
                if self.state.display_text.has_text(IceText::State) {
                    new_user.state = self
                        .state
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
                if self.state.display_text.has_text(IceText::Zip) {
                    new_user.zip = self
                        .state
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
                if self.state.display_text.has_text(IceText::Country) {
                    new_user.country = self
                        .state
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

            if settings.ask_verification && self.state.display_text.has_text(IceText::EnterVerifyText) {
                let Some(verify_answer) = self.input_required(IceText::EnterVerifyText, &MASK_ALNUM, 25, 0).await? else {
                    return Ok(false);
                };
                new_user.verify_answer = verify_answer;
            }

            if settings.ask_gender && self.state.display_text.has_text(IceText::EnterGender) {
                new_user.gender = self
                    .state
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

            if settings.ask_birthdate && self.state.display_text.has_text(IceText::EnterBirthdate) {
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
                new_user.birth_day = Some(IcbDate::parse(&date));
            }

            if settings.ask_email && self.state.display_text.has_text(IceText::EnterEmail) {
                let Some(email) = self.input_required(IceText::EnterEmail, &MASK_WEB, 30, 0).await? else {
                    return Ok(false);
                };
                new_user.email = email;
            }

            if settings.ask_web_address && self.state.display_text.has_text(IceText::EnterWebAddress) {
                new_user.web = self
                    .state
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

            if settings.ask_use_short_descr && self.state.display_text.has_text(IceText::UseShortDescription) {
                let use_short = self
                    .state
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

        let id = self.state.get_board().await.users.new_user(new_user);
        self.state.get_board().await.save_userbase()?;
        self.state.set_current_user(id).await?;

        log::info!("NEW USER: '{}'", self.state.session.user_name);

        self.state.display_news().await?;
        return Ok(true);
    }

    async fn newask_exists(&self) -> bool {
        let board = self.state.get_board().await;
        board.resolve_file(&board.config.paths.newask_survey).exists()
    }

    async fn newask_questions(&mut self) -> Res<()> {
        let survey = {
            let board = self.state.get_board().await;
            Survey {
                survey_file: board.resolve_file(&board.config.paths.newask_survey),
                answer_file: board.resolve_file(&board.config.paths.newask_answer),
                required_security: SecurityExpression::default(),
            }
        };
        Ok(if !self.state.session.is_sysop && survey.survey_file.exists() {
            // skip the survey question.
            self.state.session.tokens.push_front(self.state.session.yes_char.to_string());
            self.state.start_survey(&survey).await?;
        })
    }

    async fn logon_questions(&mut self) -> Res<()> {
        let survey: Survey = {
            let board = self.state.get_board().await;
            Survey {
                survey_file: board.config.paths.logon_survey.clone(),
                answer_file: board.config.paths.logon_answer.clone(),
                required_security: SecurityExpression::default(),
            }
        };
        Ok(if survey.survey_file.exists() {
            // skip the survey question.
            self.state.session.tokens.push_front(self.state.session.yes_char.to_string());
            self.state.start_survey(&survey).await?;
        })
    }

    async fn login_user(&mut self) -> Res<bool> {
        let check_password = if let Some(user) = &self.state.session.current_user {
            if user.flags.delete_flag || user.flags.disabled_flag {
                self.state.display_text(IceText::DeniedLockedOut, display_flags::NEWLINE).await?;
                self.state.hangup().await?;
                return Ok(false);
            }

            let pw = user.password.password.clone();

            let mut emsi_pw = false;
            if let Some(emsi) = &self.state.session.emsi {
                if emsi.user.password.eq_ignore_ascii_case(pw.to_string().as_str()) {
                    emsi_pw = true;
                }
            }

            emsi_pw
                || self
                    .state
                    .check_password(IceText::YourPassword, pwd_flags::SHOW_WRONG_PWD_MSG, |pwd| pw.is_valid(pwd))
                    .await?
        } else {
            log::warn!("login_user: User missing (should never happen -> bug)");
            return Ok(false);
        };

        if !check_password {
            log::warn!("Login from {} at {} password failed", self.state.session.user_name, Local::now().to_rfc2822());
            self.state.display_text(IceText::DeniedPasswordFailed, display_flags::NEWLINE).await?;
            self.state.hangup().await?;
            return Ok(false);
        }

        if self.state.get_board().await.config.subscription_info.is_enabled {
            if let Some(user) = &self.state.session.current_user {
                if user.exp_date.to_utc_date_time() < Utc::now() {
                    log::warn!("Login from expired user {} at {}", self.state.session.user_name, Local::now().to_rfc2822());
                    let exp_file = self.state.get_board().await.config.paths.expired.clone();
                    self.state.display_file(&self.state.resolve_path(&exp_file)).await?;
                    self.state.hangup().await?;
                    return Ok(false);
                }
                let warn_days = self.state.get_board().await.config.subscription_info.warning_days as i64;
                if user.exp_date.to_utc_date_time() + chrono::Duration::days(warn_days) < Utc::now() {
                    let exp_file = self.state.get_board().await.config.paths.expire_warning.clone();
                    self.state.display_file(&self.state.resolve_path(&exp_file)).await?;
                    self.state.press_enter().await?;
                }
            }
        }

        if let Some(user) = &self.state.session.current_user {
            if !user.password.expire_date.year() > 0 {
                let today = Utc::now();
                if user.password.expire_date > today {
                    self.state
                        .display_text(IceText::PasswordExpired, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                    self.change_password().await?;
                    return Ok(false);
                }

                let days = self.state.get_board().await.config.limits.password_expire_warn_days as i64;

                if days > 0 && user.password.expire_date + chrono::Duration::days(days) > today {
                    self.state.session.op_text = (user.password.expire_date + chrono::Duration::days(days) - today).num_days().to_string();
                    self.state
                        .display_text(IceText::PasswordWillExpired, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                    self.state.press_enter().await?;
                    return Ok(false);
                }
            }
        }

        log::warn!("Login from {} at {}", self.state.session.user_name, Local::now().to_rfc2822());
        self.logon_questions().await?;
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
                .state
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
                self.state.display_text(IceText::ResponseRequired, display_flags::NEWLINE).await?;
            } else {
                return Ok(Some(name));
            }
        }
    }

    async fn change_password(&mut self) -> Res<()> {
        loop {
            let Some(pw1) = self.input_required(IceText::NewPassword, &MASK_ALNUM, 20, display_flags::ECHODOTS).await? else {
                return Ok(());
            };
            let Some(pw2) = self.input_required(IceText::ReEnterPassword, &MASK_ALNUM, 20, display_flags::ECHODOTS).await? else {
                return Ok(());
            };

            if pw1 == pw2 {
                let exp_days = self.state.get_board().await.config.limits.password_expire_days;
                if let Some(cur_user) = &mut self.state.session.current_user {
                    cur_user.password.password = Password::PlainText(pw1);
                    if exp_days > 0 {
                        cur_user.password.expire_date = Utc::now() + chrono::Duration::days(exp_days as i64);
                    }
                }
                self.state.get_board().await.save_userbase()?;
                return Ok(());
            }
            self.state.display_text(IceText::PasswordsDontMatch, display_flags::NEWLINE).await?;
        }
    }
}
