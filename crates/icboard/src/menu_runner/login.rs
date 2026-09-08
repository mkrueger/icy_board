use crate::VERSION;

use super::PcbBoardCommand;
use crate::Res;
use chrono::{Datelike, Local, Utc};
use icy_board_engine::{
    datetime::{IcbDate, IcbTime},
    icy_board::{
        icb_config::DEFAULT_PCBOARD_DATE_FORMAT,
        icb_text::IceText,
        pcb::user_inf::AccountUserInf,
        security_expr::SecurityExpression,
        state::{
            NodeStatus,
            functions::{MASK_ASCII, MASK_DATE, MASK_MESSAGE, MASK_NAME, MASK_PHONE, MASK_WEB, display_flags, pwd_flags},
        },
        surveys::Survey,
        user_base::{ConferenceFlags, User},
    },
    vm::TerminalTarget,
};

fn assign_new_user_groups(groups: &mut icy_board_engine::icy_board::group_list::GroupList, configured: &str, user_name: &str) {
    for group in configured.split([',', ';']).map(str::trim).filter(|name| !name.is_empty()) {
        groups.add_member(group, user_name);
    }
}
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
            .println(TerminalTarget::Both, &format!("IcyBoard v{} - Node {}", *VERSION, node_number))
            .await?;

        let welcome_screen = self.state.get_board().await.config.paths.welcome.clone();
        let welcome_screen = self.state.resolve_path(&welcome_screen);
        self.state.display_file(&welcome_screen).await?;
        self.state.new_line().await?;

        if self.deny_login_for_event().await? {
            return Ok(false);
        }
        // set_current_user applies the event cap after loading the real security
        // allowance. Capping the anonymous 1000-minute default here would leave
        // EVTTIMEADJ set even for callers whose own allowance ends before the event.

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
            if self.state.session.request_logoff || self.deny_login_for_event().await? {
                return Ok(false);
            }
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
                        &MASK_ASCII,
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

            // PCBoard reads an NS stacked onto the logon prompt as "do not pause",
            // DisableQuick takes the shortcut away again.
            if let Some(pos) = self.state.session.tokens.iter().position(|t| t.eq_ignore_ascii_case("NS")) {
                self.state.session.tokens.remove(pos);
                if !self.state.get_board().await.config.system_control.disable_ns_logon {
                    self.state.session.disp_options.force_non_stop();
                }
            }

            let mut found_user = None;
            for (i, user) in self.state.get_board().await.users.iter().enumerate() {
                if user.is_valid_loginname(&first_name) {
                    found_user = Some(i);
                    break;
                }
            }

            if found_user.is_none() && !first_name.contains(' ') {
                // PCBoard caps the last name so "First Last" fits the 25 char record.
                let last_name_len = (24 - first_name.chars().count() as i32).max(0);
                let last_name = self
                    .state
                    .input_field(
                        IceText::YourLastName,
                        last_name_len,
                        &MASK_ASCII,
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
                self.state.session.user_name = if last_name.is_empty() {
                    first_name
                } else {
                    format!("{} {}", first_name, last_name.trim())
                }
                .trim()
                .to_string();
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
                if !self.confirm_caller(user).await? {
                    continue;
                }
                self.state.set_current_user(user, false).await?;
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
                    "RC",
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
        if self.state.session.request_logoff || self.deny_login_for_event().await? {
            return Ok(false);
        }
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
        let subscription = self.state.get_board().await.config.subscription_info.clone();
        new_user.security_level = settings.sec_level;
        new_user.exp_security_level = subscription.default_expired_level;
        new_user.expiration_date = icy_board_engine::icy_board::subscription::new_user_expiration(
            subscription.is_enabled,
            subscription.subscription_length,
            self.state.session.login_date,
        );
        new_user.stats.first_date_on = Utc::now();
        new_user.set_name(self.state.session.user_name.clone());
        loop {
            tries += 1;
            if tries > 4 {
                return Ok(false);
            }
            let Some(pw1) = self.input_required(IceText::NewPassword, &MASK_MESSAGE, 20, display_flags::ECHODOTS).await? else {
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

            let Some(pw2) = self
                .input_required(IceText::ReEnterPassword, &MASK_MESSAGE, 20, display_flags::ECHODOTS)
                .await?
            else {
                return Ok(false);
            };

            if pw1 == pw2 {
                new_user.password.password = self.state.create_password(pw1).await;
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
                let mask: &str = if self.state.get_board().await.config.switches.disable_registration_edits {
                    &MASK_MESSAGE
                } else {
                    &MASK_NAME
                };
                let Some(city_or_state) = self.input_required(IceText::CityState, mask, 24, display_flags::HIGHASCII).await? else {
                    return Ok(false);
                };
                new_user.city_or_state = city_or_state;
            }

            if settings.ask_business_phone && self.state.display_text.has_text(IceText::BusDataPhone) {
                let mask: &str = if self.state.get_board().await.config.switches.disable_registration_edits {
                    &MASK_MESSAGE
                } else {
                    &MASK_PHONE
                };
                let Some(bus_data_phone) = self.input_required(IceText::BusDataPhone, mask, 13, display_flags::HIGHASCII).await? else {
                    return Ok(false);
                };
                new_user.bus_data_phone = bus_data_phone;
            }

            if settings.ask_home_phone && self.state.display_text.has_text(IceText::HomeVoicePhone) {
                let mask: &str = if self.state.get_board().await.config.switches.disable_registration_edits {
                    &MASK_MESSAGE
                } else {
                    &MASK_PHONE
                };
                let Some(home_voice_phone) = self.input_required(IceText::HomeVoicePhone, mask, 13, display_flags::HIGHASCII).await? else {
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
                        &MASK_ASCII,
                        "",
                        None,
                        display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::HIGHASCII,
                    )
                    .await?;
            }

            if settings.ask_clr_msg && self.state.display_text.has_text(IceText::CLSBetweenMessages) {
                let msg_cls = self
                    .state
                    .input_field(
                        IceText::CLSBetweenMessages,
                        1,
                        "",
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
                        25,
                        &MASK_ASCII,
                        "",
                        None,
                        display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::HIGHASCII,
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
                            &MASK_ASCII,
                            "",
                            None,
                            display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::HIGHASCII,
                        )
                        .await?;
                }
                if self.state.display_text.has_text(IceText::Street2) {
                    new_user.street2 = self
                        .state
                        .input_field(
                            IceText::Street2,
                            50,
                            &MASK_ASCII,
                            "",
                            None,
                            display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::HIGHASCII,
                        )
                        .await?;
                }
                if self.state.display_text.has_text(IceText::City) {
                    new_user.city = self
                        .state
                        .input_field(
                            IceText::City,
                            25,
                            &MASK_ASCII,
                            "",
                            None,
                            display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::HIGHASCII,
                        )
                        .await?;
                }
                if self.state.display_text.has_text(IceText::State) {
                    new_user.state = self
                        .state
                        .input_field(
                            IceText::State,
                            10,
                            &MASK_ASCII,
                            "",
                            None,
                            display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::HIGHASCII,
                        )
                        .await?;
                }
                if self.state.display_text.has_text(IceText::Zip) {
                    new_user.zip = self
                        .state
                        .input_field(
                            IceText::Zip,
                            10,
                            &MASK_ASCII,
                            "",
                            None,
                            display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::HIGHASCII,
                        )
                        .await?;
                }
                if self.state.display_text.has_text(IceText::Country) {
                    new_user.country = self
                        .state
                        .input_field(
                            IceText::Country,
                            15,
                            &MASK_ASCII,
                            "",
                            None,
                            display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::HIGHASCII,
                        )
                        .await?;
                }
            }

            if settings.ask_verification && self.state.display_text.has_text(IceText::EnterVerifyText) {
                let Some(verify_answer) = self
                    .input_required(IceText::EnterVerifyText, &MASK_MESSAGE, 25, display_flags::HIGHASCII)
                    .await?
                else {
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
                        &MASK_DATE,
                        "",
                        None,
                        display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
                    )
                    .await?;
                new_user.birth_date = IcbDate::parse(&date).to_utc_date_time();
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

        // A shutdown during registration/surveys must not publish a partial account.
        if self.state.session.request_logoff || self.deny_login_for_event().await? {
            return Ok(false);
        }
        if self.state.get_board().await.config.new_user_settings.auto_register_conferences {
            self.register_public_conferences(&mut new_user).await;
        }

        // Only genuine registration receives the opening grant. Loading an
        // existing user with no account must not mint new-user credit.
        if let Some(rates) = &self.state.get_board().await.config.accounting.accounting_config {
            rates.validate()?;
            new_user.account = Some(AccountUserInf {
                starting_balance: rates.new_user_balance,
                ..Default::default()
            });
        }
        let user_name = new_user.get_name().clone();
        let id = self.state.get_board().await.users.new_user(new_user);
        {
            let mut board = self.state.get_board().await;
            let configured = board.config.new_user_settings.new_user_groups.clone();
            assign_new_user_groups(&mut board.groups, &configured, &user_name);
            board.groups.save(&board.config.paths.group_file)?;
            board.save_userbase()?;
        }
        self.state.set_current_user(id, false).await?;

        log::info!("NEW USER: '{}'", self.state.session.user_name);
        self.state.log_logon_to_caller_log().await;

        self.announce_event_time_adjustment().await?;
        if self.state.session.request_logoff {
            return Ok(false);
        }
        self.state.display_news(false).await?;
        self.logon_questions().await?;
        if self.state.session.request_logoff {
            return Ok(false);
        }
        self.start_login_accounting().await?;
        self.state.join_conference(0, false, false).await?;

        Ok(true)
    }

    /// PCBoard's AutoRegConf - a new caller starts out registered in every public
    /// conference that carries no security requirement of its own.
    async fn register_public_conferences(&self, user: &mut User) {
        let board = self.state.get_board().await;
        for (number, conference) in board.conferences.iter().enumerate() {
            if !conference.is_public || !conference.required_security.is_empty() {
                continue;
            }
            let flags = user.conference_flags.entry(number).or_insert(ConferenceFlags::None);
            *flags |= ConferenceFlags::Registered;
        }
    }

    /// PCBoard's ConfirmCaller - show the record the name matched so a caller who
    /// mistyped their name notices before a second account is created.
    async fn confirm_caller(&mut self, user_number: usize) -> Res<bool> {
        let board = self.state.get_board().await;
        if !board.config.system_control.confirm_caller_name {
            return Ok(true);
        }
        let user = board.users[user_number].clone();
        drop(board);

        self.state.new_line().await?;
        self.state.println(TerminalTarget::Both, user.get_name()).await?;
        if !user.city_or_state.is_empty() {
            self.state.println(TerminalTarget::Both, &user.city_or_state).await?;
        }
        let answer = self
            .state
            .input_field(
                IceText::IsThisCorrect,
                1,
                "",
                "",
                Some(self.state.session.yes_char.to_string()),
                display_flags::YESNO | display_flags::FIELDLEN | display_flags::UPCASE | display_flags::NEWLINE | display_flags::LFBEFORE,
            )
            .await?;
        if answer.starts_with(self.state.session.no_char) {
            self.state
                .display_text(IceText::ChangeNames, display_flags::NEWLINE | display_flags::LFAFTER)
                .await?;
            return Ok(false);
        }
        Ok(true)
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
        let _: () = if !self.state.session.is_sysop && survey.survey_file.exists() {
            // skip the survey question.
            self.state.session.tokens.push_front(self.state.session.yes_char.to_string());
            self.state.start_survey(&survey).await?;
        };
        Ok(())
    }

    async fn logon_questions(&mut self) -> Res<()> {
        let survey: Survey = {
            let board: tokio::sync::MutexGuard<'_, icy_board_engine::icy_board::IcyBoard> = self.state.get_board().await;
            Survey {
                survey_file: board.config.paths.logon_survey.clone(),
                answer_file: board.config.paths.logon_answer.clone(),
                required_security: SecurityExpression::default(),
            }
        };

        let _: () = if survey.survey_file.exists() {
            // skip the survey question.
            self.state.session.tokens.push_front(self.state.session.yes_char.to_string());
            self.state.start_survey(&survey).await?;
        };
        Ok(())
    }

    async fn login_user(&mut self) -> Res<bool> {
        let recovery_enabled = self.state.get_board().await.config.password_recovery.enabled;
        let check_password = if let Some(user) = &self.state.session.current_user {
            if user.flags.delete_flag || user.flags.disabled_flag {
                self.state.display_text(IceText::DeniedLockedOut, display_flags::NEWLINE).await?;
                self.state.hangup().await?;
                return Ok(false);
            }

            let pw = user.password.password.clone();

            let mut emsi_pw = false;
            if let Some(emsi) = &self.state.session.emsi
                && pw.is_valid(&emsi.user.password)
            {
                emsi_pw = true;
            }

            if recovery_enabled {
                use icy_board_engine::icy_board::password_recovery::LoginPassword;
                match self.recovery_login_password().await? {
                    LoginPassword::Permanent => true,
                    LoginPassword::Temporary(proof) => {
                        self.finish_password_recovery(proof).await?;
                        return Ok(false);
                    }
                    LoginPassword::Invalid => false,
                }
            } else {
                emsi_pw
                    || self
                        .state
                        .check_password(IceText::YourPassword, pwd_flags::SHOW_WRONG_PWD_MSG, |pwd| pw.is_valid(pwd))
                        .await?
            }
        } else {
            log::warn!("login_user: User missing (should never happen -> bug)");
            return Ok(false);
        };

        if !check_password {
            let offer_recovery = recovery_enabled && !self.state.session.request_logoff && {
                let board = self.state.get_board().await;
                board
                    .users
                    .get(self.state.session.cur_user_id as usize)
                    .is_some_and(icy_board_engine::icy_board::password_recovery::has_recovery_email)
            };
            if offer_recovery {
                let answer = self
                    .state
                    .input_field(
                        IceText::RecoverPasswordByEmail,
                        1,
                        "",
                        "",
                        Some("N".to_string()),
                        display_flags::YESNO | display_flags::NEWLINE | display_flags::FIELDLEN,
                    )
                    .await?;
                if answer.eq_ignore_ascii_case(&self.state.session.yes_char.to_string()) {
                    let service = self.state.get_board().await.password_recovery_service.clone();
                    // The response must not reveal eligibility, mailbox, throttling or delivery.
                    let _ = service.issue(&self.state.board, self.state.session.cur_user_id as usize, Utc::now()).await;
                    self.state.display_text(IceText::RecoveryRequestAccepted, display_flags::NEWLINE).await?;
                    self.state.session.last_password.clear();
                    self.state.session.emsi = None;
                    self.state.hangup().await?;
                    return Ok(false);
                }
                log::info!("Recovery email: user index {}: offer declined or cancelled", self.state.session.cur_user_id);
            }
            log::warn!("Login from {} at {} password failed", self.state.session.user_name, Local::now().to_rfc2822());
            if self.state.get_board().await.config.system_control.allow_password_failure_comment {
                self.state.password_failure_comment().await?;
            }
            self.state.display_text(IceText::DeniedPasswordFailed, display_flags::NEWLINE).await?;
            self.state.hangup().await?;
            return Ok(false);
        }

        if !self.state.authorize_normal_login().await? {
            self.state.hangup().await?;
            return Ok(false);
        }
        if self.state.session.request_logoff || self.deny_login_for_event().await? {
            return Ok(false);
        }

        let subscription = self.state.get_board().await.config.subscription_info.clone();
        if let Some(user) = &self.state.session.current_user {
            match icy_board_engine::icy_board::subscription::status(
                subscription.is_enabled,
                user.expiration_date,
                subscription.warning_days,
                self.state.session.login_date.date_naive(),
            ) {
                icy_board_engine::icy_board::subscription::SubscriptionStatus::Expired { .. } => {
                    log::warn!("Login from expired user {} at {}", self.state.session.user_name, Local::now().to_rfc2822());
                    let path = self.state.get_board().await.config.paths.expired.clone();
                    self.state.display_file(&self.state.resolve_path(&path)).await?;
                }
                icy_board_engine::icy_board::subscription::SubscriptionStatus::Warning { .. } => {
                    let path = self.state.get_board().await.config.paths.expire_warning.clone();
                    self.state.display_file(&self.state.resolve_path(&path)).await?;
                }
                _ => {}
            }
        }

        if let Some(user) = &self.state.session.current_user
            && !user.password.expire_date.year() > 0
        {
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

        log::warn!("Login from {} at {}", self.state.session.user_name, Local::now().to_rfc2822());
        self.state.log_logon_to_caller_log().await;
        self.announce_event_time_adjustment().await?;
        if self.state.session.request_logoff {
            return Ok(false);
        }
        self.logon_questions().await?;
        if self.state.session.request_logoff {
            return Ok(false);
        }
        self.start_login_accounting().await?;
        let last_conference = if let Some(user) = &self.state.session.current_user {
            user.last_conference
        } else {
            0
        };
        self.state.join_conference(last_conference, false, false).await?;

        Ok(true)
    }

    async fn recovery_login_password(&mut self) -> Res<icy_board_engine::icy_board::password_recovery::LoginPassword> {
        use icy_board_engine::icy_board::{password_recovery::LoginPassword, state::functions::MASK_PASSWORD};
        let service = self.state.get_board().await.password_recovery_service.clone();
        let index = self.state.session.cur_user_id as usize;
        let cached = self
            .state
            .session
            .emsi
            .as_ref()
            .map(|emsi| emsi.user.password.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| self.state.session.last_password.clone());
        if !cached.is_empty() {
            let result = service.verify(&self.state.board, index, cached, Utc::now()).await?;
            if !matches!(result, LoginPassword::Invalid) {
                return Ok(result);
            }
        }
        for _ in 0..3 {
            let pwd = self
                .state
                .input_field(
                    IceText::YourPassword,
                    13,
                    MASK_PASSWORD,
                    "",
                    None,
                    display_flags::FIELDLEN | display_flags::ECHODOTS | display_flags::NEWLINE,
                )
                .await?;
            if self.state.session.request_logoff {
                return Ok(LoginPassword::Invalid);
            }
            let result = service.verify(&self.state.board, index, pwd.clone(), Utc::now()).await?;
            match result {
                LoginPassword::Permanent => {
                    self.state.session.last_password = pwd;
                    return Ok(result);
                }
                LoginPassword::Temporary(_) => {
                    self.state.session.last_password.clear();
                    self.state.session.emsi = None;
                    return Ok(result);
                }
                LoginPassword::Invalid => self.state.display_text(IceText::WrongPasswordEntered, display_flags::NEWLINE).await?,
            }
        }
        if let Some(user) = &mut self.state.session.current_user {
            user.stats.num_password_failures += 1;
        }
        self.state.session.op_text = self.state.session.get_username_or_alias();
        self.state
            .display_text(IceText::PasswordFailure, display_flags::NEWLINE | display_flags::LFAFTER)
            .await?;
        Ok(LoginPassword::Invalid)
    }

    async fn finish_password_recovery(&mut self, proof: icy_board_engine::icy_board::password_recovery::RecoveryProof) -> Res<()> {
        use icy_board_engine::icy_board::{bbs::BBSMessage, state::functions::MASK_PASSWORD, user_base::PasswordVerdict};
        self.state.session.last_password.clear();
        self.state.session.emsi = None;
        self.state.display_text(IceText::RecoveryChangeRequired, display_flags::NEWLINE).await?;
        let service = self.state.get_board().await.password_recovery_service.clone();
        for _ in 0..3 {
            let first = self
                .state
                .input_field(
                    IceText::NewPassword,
                    12,
                    MASK_PASSWORD,
                    "",
                    None,
                    display_flags::ECHODOTS | display_flags::FIELDLEN | display_flags::NEWLINE,
                )
                .await?;
            if first.is_empty() || self.state.session.request_logoff {
                break;
            }
            let min_len = self.state.get_board().await.config.limits.min_pwd_length;
            let user = self.state.session.current_user.as_ref().unwrap();
            let verdict = user.password.check_new_password(&user.name, &first, min_len);
            let error = match verdict {
                PasswordVerdict::Ok => None,
                PasswordVerdict::TooShort => {
                    self.state.session.op_text = min_len.to_string();
                    Some(IceText::PasswordTooShort)
                }
                PasswordVerdict::PartOfName => Some(IceText::NeedUniquePassword),
                _ => Some(IceText::PreviouslyUsedPassword),
            };
            if let Some(error) = error {
                self.state.display_text(error, display_flags::NEWLINE).await?;
                continue;
            }
            let second = self
                .state
                .input_field(
                    IceText::ReEnterPassword,
                    12,
                    MASK_PASSWORD,
                    "",
                    None,
                    display_flags::ECHODOTS | display_flags::FIELDLEN | display_flags::NEWLINE,
                )
                .await?;
            if second.is_empty() || self.state.session.request_logoff {
                break;
            }
            if !first.eq_ignore_ascii_case(&second) {
                self.state.display_text(IceText::PasswordsDontMatch, display_flags::NEWLINE).await?;
                continue;
            }
            match service.complete(&self.state.board, &proof, first, Utc::now()).await {
                Ok(true) => {
                    let targets: Vec<usize> = self
                        .state
                        .node_state
                        .lock()
                        .await
                        .iter()
                        .enumerate()
                        .filter_map(|(index, node)| {
                            (index != self.state.node && node.as_ref().is_some_and(|n| n.cur_user == self.state.session.cur_user_id)).then_some(index)
                        })
                        .collect();
                    let bbs = self.state.bbs.lock().await;
                    for index in targets {
                        if let Some(Some(channel)) = bbs.bbs_channels.get(index) {
                            let _ = channel.try_send(BBSMessage::Shutdown("Credentials changed; please log in again.".to_string()));
                        }
                    }
                    drop(bbs);
                    self.state.display_text(IceText::RecoveryPasswordChanged, display_flags::NEWLINE).await?;
                    break;
                }
                Ok(false) => {
                    self.state.display_text(IceText::NeedUniquePassword, display_flags::NEWLINE).await?;
                }
                Err(_) => {
                    log::warn!("Password recovery commit failed");
                    break;
                }
            }
        }
        // Recovery proof never reaches LOGON surveys, accounting, conferences or a menu.
        self.state.hangup().await?;
        Ok(())
    }

    /// NODE/LOGIN.C: after LOGON preprocessing, before ordinary conference
    /// screens. Direct /PPE mode intentionally does not use this entry point.
    pub(crate) async fn start_login_accounting(&mut self) -> Res<()> {
        self.state.accounting_start().await?;
        if self.state.accounting_active() {
            let info = self.state.session.accounting.options.info_file.clone();
            if !info.as_os_str().is_empty() {
                self.state.display_file(&info).await?;
            }
            // The runtime owns the warning latch and enforced-vs-tracking
            // distinction; subsequent command/input checks cannot replay it.
            self.state.accounting_check_balance().await?;
        }
        Ok(())
    }

    async fn deny_login_for_event(&mut self) -> Res<bool> {
        let maintenance = self.state.bbs.lock().await.admissions_closed();
        let suspended = self.state.event_window().await.is_some_and(|window| window.is_suspended(&Local::now()));
        if !maintenance && !suspended {
            return Ok(false);
        }
        let notice = self
            .state
            .display_text(IceText::DeniedAccessForEvent, display_flags::NEWLINE | display_flags::LOGIT)
            .await;
        self.state.hangup().await?;
        notice?;
        Ok(true)
    }

    async fn announce_event_time_adjustment(&mut self) -> Res<()> {
        self.state.limit_time_for_event().await;
        if self.state.session.time_adjusted_for_event {
            // PCBoard LOGIN.C protects this warning from INTRO with an acknowledgement.
            self.state
                .display_text(IceText::TimeAdjusted, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            self.state.press_enter().await?;
        }
        Ok(())
    }

    async fn input_required(&mut self, txt: IceText, mask: &str, len: i32, flags: i32) -> Res<Option<String>> {
        let mut tries = 0;
        loop {
            if self.state.session.request_logoff {
                return Ok(None);
            }
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
            let Some(pw1) = self.input_required(IceText::NewPassword, &MASK_MESSAGE, 20, display_flags::ECHODOTS).await? else {
                return Ok(());
            };
            let Some(pw2) = self
                .input_required(IceText::ReEnterPassword, &MASK_MESSAGE, 20, display_flags::ECHODOTS)
                .await?
            else {
                return Ok(());
            };

            if pw1 == pw2 {
                let pw = self.state.create_password(pw1).await;
                let exp_days = self.state.get_board().await.config.limits.password_expire_days;
                if let Some(cur_user) = &mut self.state.session.current_user {
                    cur_user.password.password = pw;
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

#[cfg(test)]
mod option_tests {
    use super::assign_new_user_groups;
    use icy_board_engine::icy_board::group_list::GroupList;

    #[test]
    fn configured_groups_receive_the_new_user() {
        let mut groups = GroupList::new();
        groups.add_group("new_users", "New users");
        groups.add_group("trial", "Trial users");
        assign_new_user_groups(&mut groups, "new_users, trial; missing", "NEW USER");
        assert_eq!(groups.get_groups("NEW USER"), vec!["new_users", "trial"]);
    }

    #[tokio::test]
    async fn password_recovery_off_preserves_stuffed_input_and_iemsi_checks_hashes() {
        use crate::menu_runner::PcbBoardCommand;
        use icy_board_engine::icy_board::{
            IcyBoard,
            bbs::BBS,
            state::IcyBoardState,
            user_base::{Password, User},
        };
        use icy_net::{
            ConnectionType,
            channel::ChannelConnection,
            iemsi::ici::{EmsiICI, ICIUserSettings},
        };
        use std::sync::Arc;
        for (cached, accepted) in [("old-secret", true), ("******", false)] {
            let dir = tempfile::tempdir().unwrap();
            let mut board = IcyBoard::new();
            board.config.paths.user_file = dir.path().join("users.toml");
            board.config.paths.statistics_file = dir.path().join("stats.toml");
            let mut user = User {
                name: "Test Caller".into(),
                ..Default::default()
            };
            user.password.password = Password::new_argon2("old-secret");
            board.users.new_user(user);
            let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
            let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
            let nodes = bbs.lock().await.open_connections.clone();
            let (_peer, connection) = ChannelConnection::create_pair();
            let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(board)), nodes, node, Box::new(connection)).await;
            state.set_current_user(0, false).await.unwrap();
            state.session.is_local = true;
            state.session.emsi = Some(EmsiICI {
                user: ICIUserSettings {
                    password: cached.into(),
                    ..Default::default()
                },
                term: Default::default(),
                requests: Default::default(),
            });
            state.stuff_keyboard_buffer("wrong\rwrong\rwrong\rZ", false).unwrap();
            // Stop successful login before surveys; failed login still exercises all three reads.
            if accepted {
                state.session.request_logoff = true;
            }
            let mut command = PcbBoardCommand::new(state);
            assert!(!command.login_user().await.unwrap());
            assert_eq!(
                command.state.session.current_user.as_ref().unwrap().stats.num_password_failures,
                if accepted { 0 } else { 1 }
            );
            command.state.session.request_logoff = false;
            let next = command.state.get_char(icy_board_engine::vm::TerminalTarget::Both).await.unwrap().unwrap();
            assert_eq!(next.ch, if accepted { 'w' } else { 'Z' });
        }
    }
}
