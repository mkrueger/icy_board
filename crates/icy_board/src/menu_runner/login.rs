use crate::VERSION;

use super::{PcbBoardCommand, MASK_NUMBER};
use chrono::{Datelike, Local, Utc};
use icy_board_engine::{
    icy_board::{
        icb_config::{IcbColor, DEFAULT_PCBOARD_DATE_FORMAT},
        icb_text::IceText,
        state::{
            functions::{display_flags, pwd_flags, MASK_ALNUM, MASK_PHONE, MASK_WEB},
            UserActivity,
        },
        user_base::{Password, User},
        IcyBoardError,
    },
    vm::TerminalTarget,
};
use icy_ppe::{
    datetime::{IcbDate, IcbTime},
    Res,
};
impl PcbBoardCommand {
    pub fn login(&mut self) -> Res<bool> {
        self.state.node_state.lock().unwrap().user_activity = UserActivity::LoggingIn;

        self.state.reset_color()?;
        self.state.clear_screen()?;
        self.state.get_term_caps()?;
        self.state.session.login_date = chrono::Local::now();

        // intial_welcome
        let board_name = self.state.board.lock().unwrap().config.board.name.clone();
        self.state
            .println(TerminalTarget::Both, &format!("CONNECT {} ({})", IcbDate::today(), IcbTime::now()))?;
        self.state.new_line()?;
        self.state.println(TerminalTarget::Both, &board_name)?;
        let node_number = self.state.node_state.lock().unwrap().node_number;
        self.state
            .println(TerminalTarget::Both, &format!("IcyBoard v{} - Node {}", VERSION.to_string(), node_number))?;

        let welcome_screen = self.state.board.lock().unwrap().config.paths.welcome.clone();
        let welcome_screen = self.state.resolve_path(&welcome_screen);
        self.state.display_file(&welcome_screen)?;

        let mut tries = 0;

        loop {
            let first_name = self.state.input_field(
                IceText::YourFirstName,
                39,
                &MASK_ALNUM,
                "",
                None,
                display_flags::UPCASE | display_flags::NEWLINE | display_flags::STACKED,
            )?;

            let last_name = self.state.input_field(
                IceText::YourLastName,
                39,
                &MASK_ALNUM,
                "",
                None,
                display_flags::UPCASE | display_flags::NEWLINE | display_flags::STACKED,
            )?;
            self.state.session.user_name = format!("{} {}", first_name, last_name).trim().to_string();

            tries += 1;
            if tries > 3 {
                log::warn!("Login at {} num login tries exceeded.", Local::now().to_rfc2822());
                self.state.display_text(IceText::DeniedRefuseToRegister, display_flags::NEWLINE)?;
                self.state.hangup()?;
                return Ok(false);
            }
            let mut found_user = None;
            if let Ok(board) = self.state.board.lock() {
                for (i, user) in board.users.iter().enumerate() {
                    if user.is_valid_loginname(&self.state.session.user_name) {
                        found_user = Some(i);
                        break;
                    }
                }
            } else {
                return Err(IcyBoardError::ErrorLockingBoard.into());
            }
            if let Some(user) = found_user {
                self.state.set_current_user(user)?;
                return self.login_user();
            } else {
                self.state.session.op_text = self.state.session.user_name.clone();
                self.state.display_text(IceText::NotInUsersFile, display_flags::NEWLINE)?;
            }

            let re_enter = self.state.input_field(
                IceText::ReEnterName,
                1,
                &"RC",
                "",
                Some("C".to_string()),
                display_flags::UPCASE | display_flags::NEWLINE | display_flags::FIELDLEN,
            )?;

            if re_enter.is_empty() || re_enter == "C" {
                let new_file = self.state.board.lock().unwrap().config.paths.newuser.clone();
                self.state.display_file(&self.state.resolve_path(&new_file))?;
                self.state.new_line()?;

                let register = self.state.input_field(
                    IceText::Register,
                    1,
                    "",
                    "",
                    Some("Y".to_string()),
                    display_flags::YESNO | display_flags::NEWLINE | display_flags::FIELDLEN,
                )?;

                if register == "Y" || register.is_empty() {
                    if !self.new_user()? {
                        self.state.display_text(IceText::RefusedToRegister, display_flags::NEWLINE)?;
                        self.state.hangup()?;
                        return Ok(false);
                    }
                    return Ok(true);
                } else {
                    self.state.display_text(IceText::RefusedToRegister, display_flags::NEWLINE)?;
                    self.state.hangup()?;
                    return Ok(false);
                }
            }
        }
    }

    fn new_user(&mut self) -> Res<bool> {
        let mut tries = 0;

        let mut new_user = User::default();
        let settings = self.state.board.lock().unwrap().config.new_user_settings.clone();
        new_user.security_level = settings.sec_level;
        new_user.stats.first_dt_on = Utc::now();
        new_user.set_name(self.state.session.user_name.clone());
        loop {
            tries += 1;
            if tries > 4 {
                return Ok(false);
            }
            let Some(pw1) = self.input_required(IceText::NewPassword, &MASK_ALNUM, 20, display_flags::ECHODOTS)? else {
                return Ok(false);
            };
            let Some(pw2) = self.input_required(IceText::ReEnterPassword, &MASK_ALNUM, 20, display_flags::ECHODOTS)? else {
                return Ok(false);
            };

            if pw1 == pw2 {
                new_user.password.password = Password::PlainText(pw1);
                break;
            }
            if let Ok(board) = self.state.board.lock() {
                if board.config.user_password_policy.password_expire_days > 0 {
                    new_user.password.expire_date = Utc::now() + chrono::Duration::days(board.config.user_password_policy.password_expire_days as i64);
                }
            }
            self.state.display_text(IceText::PasswordsDontMatch, display_flags::NEWLINE)?;
        }

        if settings.ask_city_or_state {
            let Some(city_or_state) = self.input_required(IceText::CityState, &MASK_ALNUM, 24, 0)? else {
                return Ok(false);
            };
            new_user.city_or_state = city_or_state;
        }

        if settings.ask_bus_data_phone {
            let Some(bus_data_phone) = self.input_required(IceText::BusDataPhone, &MASK_PHONE, 13, 0)? else {
                return Ok(false);
            };
            new_user.bus_data_phone = bus_data_phone;
        }

        if settings.ask_voice_phone {
            let Some(home_voice_phone) = self.input_required(IceText::HomeVoicePhone, &MASK_PHONE, 13, 0)? else {
                return Ok(false);
            };
            new_user.home_voice_phone = home_voice_phone;
        }

        if settings.ask_comment {
            new_user.user_comment = self.state.input_field(
                IceText::CommentFieldPrompt,
                30,
                &MASK_ALNUM,
                "",
                None,
                display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
            )?;
        }

        if settings.ask_clr_msg {
            let msg_cls = self.state.input_field(
                IceText::CLSBetweenMessages,
                1,
                &"",
                "",
                Some(self.state.session.yes_char.to_uppercase().to_string()),
                display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::YESNO,
            )?;
            new_user.flags.msg_clear = msg_cls.is_empty() || msg_cls == self.state.session.yes_char.to_uppercase().to_string();
        }

        if settings.ask_date_format {
            new_user.date_format = DEFAULT_PCBOARD_DATE_FORMAT.to_string();
            let date_format = self.ask_date_format(&new_user.date_format)?;
            if !date_format.is_empty() {
                new_user.date_format = date_format;
            }
        }
        if settings.ask_xfer_protocol {
            let protocol = self.ask_protocols("N".to_string())?;
            if !protocol.is_empty() {
                new_user.protocol = protocol;
            } else {
                new_user.protocol = "N".to_string();
            }
        }

        if settings.ask_alias {
            new_user.alias = self.state.input_field(
                IceText::GetAliasName,
                30,
                &MASK_ALNUM,
                "",
                None,
                display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
            )?;
        }

        if settings.ask_address {
            self.state
                .display_text(IceText::EnterAddress, display_flags::NEWLINE | display_flags::LFBEFORE)?;
            new_user.street1 = self.state.input_field(
                IceText::Street1,
                50,
                &MASK_ALNUM,
                "",
                None,
                display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
            )?;

            new_user.street2 = self.state.input_field(
                IceText::Street2,
                50,
                &MASK_ALNUM,
                "",
                None,
                display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
            )?;

            new_user.city = self.state.input_field(
                IceText::City,
                25,
                &MASK_ALNUM,
                "",
                None,
                display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
            )?;

            new_user.state = self.state.input_field(
                IceText::State,
                15,
                &MASK_ALNUM,
                "",
                None,
                display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
            )?;

            new_user.zip = self.state.input_field(
                IceText::Zip,
                10,
                &MASK_ALNUM,
                "",
                None,
                display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
            )?;

            new_user.country = self.state.input_field(
                IceText::Country,
                15,
                &MASK_ALNUM,
                "",
                None,
                display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
            )?;
        }

        if settings.ask_verification {
            let Some(verify_answer) = self.input_required(IceText::EnterVerifyText, &MASK_ALNUM, 25, 0)? else {
                return Ok(false);
            };
            new_user.verify_answer = verify_answer;
        }

        if settings.ask_gender {
            new_user.gender = self.state.input_field(
                IceText::EnterGender,
                1,
                "MmFf",
                "",
                None,
                display_flags::FIELDLEN | display_flags::UPCASE | display_flags::NEWLINE | display_flags::LFBEFORE,
            )?;
        }

        if settings.ask_birthdate {
            let date = self.state.input_field(
                IceText::EnterBirthdate,
                8,
                &MASK_ALNUM,
                "",
                None,
                display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
            )?;
            new_user.birth_date = IcbDate::parse(&date).to_utc_date_time();
        }

        if settings.ask_email {
            let Some(email) = self.input_required(IceText::EnterEmail, &MASK_WEB, 30, 0)? else {
                return Ok(false);
            };
            new_user.email = email;
        }

        if settings.ask_web_address {
            new_user.web = self.state.input_field(
                IceText::EnterWebAddress,
                30,
                &MASK_WEB,
                "",
                None,
                display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
            )?;
        }

        if settings.ask_use_short_descr {
            let use_short = self.state.input_field(
                IceText::UseShortDescription,
                1,
                "",
                "",
                Some("N".to_string()),
                display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::YESNO,
            )?;
            new_user.flags.use_short_filedescr = use_short == "Y";
        }

        let id = self.state.board.lock().unwrap().users.new_user(new_user);
        self.state.board.lock().unwrap().save_userbase()?;
        self.state.set_current_user(id)?;
        self.display_news()?;
        return Ok(true);
    }

    fn login_user(&mut self) -> Res<bool> {
        let check_password = if let Some(user) = &self.state.current_user {
            if user.flags.delete_flag || user.flags.disabled_flag {
                self.state.display_text(IceText::DeniedLockedOut, display_flags::NEWLINE)?;
                self.state.hangup()?;
                return Ok(false);
            }

            let pw = user.password.password.clone();
            self.state
                .check_password(IceText::YourPassword, pwd_flags::SHOW_WRONG_PWD_MSG, |pwd| pw.is_valid(pwd))?
        } else {
            log::warn!("login_user: User missing (should never happen -> bug)");
            return Ok(false);
        };

        if !check_password {
            log::warn!("Login from {} at {} password failed", self.state.session.user_name, Local::now().to_rfc2822());
            self.state.display_text(IceText::DeniedPasswordFailed, display_flags::NEWLINE)?;
            self.state.hangup()?;
            return Ok(false);
        }

        if self.state.board.lock().unwrap().config.subscription_info.is_enabled {
            if let Some(user) = &self.state.current_user {
                if user.exp_date < Utc::now() {
                    log::warn!("Login from expired user {} at {}", self.state.session.user_name, Local::now().to_rfc2822());
                    let exp_file = self.state.board.lock().unwrap().config.paths.expired.clone();
                    self.state.display_file(&self.state.resolve_path(&exp_file))?;
                    self.state.hangup()?;
                    return Ok(false);
                }
                let warn_days = self.state.board.lock().unwrap().config.subscription_info.warning_days as i64;
                if user.exp_date + chrono::Duration::days(warn_days) < Utc::now() {
                    let exp_file = self.state.board.lock().unwrap().config.paths.expire_warning.clone();
                    self.state.display_file(&self.state.resolve_path(&exp_file))?;
                    self.state.press_enter()?;
                }
            }
        }

        if let Some(user) = &self.state.current_user {
            if !user.password.expire_date.year() > 0 {
                let today = Utc::now();
                if user.password.expire_date > today {
                    self.state
                        .display_text(IceText::PasswordExpired, display_flags::NEWLINE | display_flags::LFBEFORE)?;
                    self.change_password()?;
                    return Ok(false);
                }

                let days = self.state.board.lock().unwrap().config.user_password_policy.password_expire_warn_days as i64;

                if user.password.expire_date + chrono::Duration::days(days) > today {
                    self.state.session.op_text = (user.password.expire_date + chrono::Duration::days(days) - today).num_days().to_string();
                    self.state
                        .display_text(IceText::PasswordWillExpired, display_flags::NEWLINE | display_flags::LFBEFORE)?;
                    self.state.press_enter()?;
                    return Ok(false);
                }
            }
        }

        log::warn!("Login from {} at {}", self.state.session.user_name, Local::now().to_rfc2822());
        return Ok(true);
    }

    fn input_required(&mut self, txt: IceText, mask: &str, len: i32, flags: i32) -> Res<Option<String>> {
        let mut tries = 0;
        loop {
            tries += 1;
            if tries > 3 {
                return Ok(None);
            }

            let name = self.state.input_field(
                txt,
                len,
                mask,
                "",
                None,
                flags | display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
            )?;

            if name.is_empty() {
                self.state.display_text(IceText::ResponseRequired, display_flags::NEWLINE)?;
            } else {
                return Ok(Some(name));
            }
        }
    }

    fn change_password(&mut self) -> Res<()> {
        loop {
            let Some(pw1) = self.input_required(IceText::NewPassword, &MASK_ALNUM, 20, display_flags::ECHODOTS)? else {
                return Ok(());
            };
            let Some(pw2) = self.input_required(IceText::ReEnterPassword, &MASK_ALNUM, 20, display_flags::ECHODOTS)? else {
                return Ok(());
            };

            if pw1 == pw2 {
                if let Some(cur_user) = &mut self.state.current_user {
                    cur_user.password.password = Password::PlainText(pw1);
                    if let Ok(board) = self.state.board.lock() {
                        if board.config.user_password_policy.password_expire_days > 0 {
                            cur_user.password.expire_date = Utc::now() + chrono::Duration::days(board.config.user_password_policy.password_expire_days as i64);
                        }
                    }
                }
                self.state.board.lock().unwrap().save_userbase()?;
                return Ok(());
            }
            self.state.display_text(IceText::PasswordsDontMatch, display_flags::NEWLINE)?;
        }
    }

    pub fn ask_date_format(&mut self, cur_format: &str) -> Res<String> {
        self.state.new_line()?;
        let date_formats = if let Ok(board) = self.state.board.lock() {
            board.languages.date_formats.clone()
        } else {
            log::error!("ask_date_format: Error locking board");
            return Ok(cur_format.to_string());
        };

        self.state.set_color(TerminalTarget::Both, IcbColor::Dos(11))?;
        let mut preview = String::new();
        for (i, (disp_fmt, fmt)) in date_formats.iter().enumerate() {
            if fmt == cur_format {
                preview = (i + 1).to_string();
                self.state.println(TerminalTarget::Both, &format!("=> ({}) {}", i + 1, disp_fmt))?;
            } else {
                self.state.println(TerminalTarget::Both, &format!("   ({}) {}", i + 1, disp_fmt))?;
            }
        }
        let date_format = self.state.input_field(
            IceText::DateFormatDesired,
            1,
            &MASK_NUMBER,
            "",
            Some(preview),
            display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER | display_flags::UPCASE | display_flags::FIELDLEN,
        )?;
        if let Ok(number) = date_format.parse::<usize>() {
            if number > 0 && number <= date_formats.len() {
                return Ok(date_formats[number - 1].1.clone());
            }
        }
        return Ok(cur_format.to_string());
    }
}
