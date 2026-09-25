use crate::Res;
use crate::icy_board::commands::CommandType;
use crate::icy_board::state::IcyBoardState;
use crate::icy_board::state::functions::{MASK_ASCII, MASK_COMMAND, MASK_PASSWORD};
use crate::icy_board::user_base::ConferenceFlags;
use crate::icy_board::{icb_text::IceText, state::functions::display_flags};
use crate::vm::TerminalTarget;
use std::fmt::Write as _;

enum JoinSelection {
    Join { number: u16, show_news: bool },
    Stay { show_news: bool },
    Retry,
    Relist,
    Stop,
}

impl IcyBoardState {
    pub async fn join_conference_cmd(&mut self) -> Res<()> {
        if self.get_board().await.conferences.is_empty() {
            self.display_text(
                IceText::NoConferenceAvailable,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::BELL,
            )
            .await?;
            return Ok(());
        }
        let mut display_menu = self.session.tokens.is_empty();
        loop {
            if self.session.tokens.is_empty() {
                if display_menu {
                    display_menu = false;
                    self.session.disp_options.no_change();
                    let mnu = self.get_board().await.config.paths.conf_join_menu.clone();
                    let mnu = self.resolve_path(&mnu);
                    self.display_menu(&mnu).await?;
                    self.new_line().await?;
                }

                let str = self
                    .input_field(
                        IceText::JoinConferenceNumber,
                        60,
                        MASK_COMMAND,
                        CommandType::JoinConference.get_help(),
                        None,
                        display_flags::UPCASE | display_flags::STACKED | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::HIGHASCII,
                    )
                    .await?;
                if str.is_empty() {
                    return Ok(());
                }
                self.session.push_tokens(&str);
            }

            match self.select_conference().await? {
                JoinSelection::Join { number, show_news } => return self.join_selected_conference(number, show_news).await,
                JoinSelection::Stay { show_news } => return self.process_join(show_news, !show_news).await,
                JoinSelection::Retry => {}
                JoinSelection::Relist => display_menu = true,
                JoinSelection::Stop => return Ok(()),
            }
        }
    }

    async fn select_conference(&mut self) -> Res<JoinSelection> {
        let tokens: Vec<String> = std::mem::take(&mut self.session.tokens).into();
        let mut show_news = true;
        let mut search = false;
        let mut number = None;
        let mut name = String::new();
        let mut last_token = String::new();
        // A digit after Q, S or a one-letter name is part of the name.
        let mut non_digit = false;

        for token in tokens {
            last_token.clone_from(&token);
            if token.is_empty() {
                continue;
            }
            if !non_digit && token.bytes().all(|b| b.is_ascii_digit()) {
                number = Some(token.parse::<u16>().unwrap_or(u16::MAX));
                continue;
            }
            let mut chars = token.chars();
            if let (Some(ch), None) = (chars.next(), chars.next()) {
                match ch.to_ascii_uppercase() {
                    'Q' => {
                        show_news = false;
                        non_digit = true;
                        continue;
                    }
                    'S' => {
                        search = true;
                        non_digit = true;
                        continue;
                    }
                    'J' => continue,
                    'R' => return Ok(JoinSelection::Relist),
                    _ => non_digit = true,
                }
            }
            if !name.is_empty() {
                name.push(' ');
            }
            name.push_str(&token);
        }

        let conferences = self.get_board().await.conferences.clone();
        if number.is_none() && !search && !name.is_empty() {
            let upper = name.to_ascii_uppercase();
            number = if upper == "MAIN" || upper == "MAIN BOARD" {
                Some(0)
            } else {
                conferences
                    .iter()
                    .position(|conference| !conference.name.is_empty() && conference.name.eq_ignore_ascii_case(&name))
                    .map(|number| number as u16)
            };
        }

        if number.is_none() && search {
            let text = if name.is_empty() {
                self.input_field(
                    IceText::TextToScanFor,
                    60,
                    &MASK_ASCII,
                    CommandType::JoinConference.get_help(),
                    None,
                    display_flags::UPCASE | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::HIGHASCII,
                )
                .await?
            } else {
                name
            };
            if text.is_empty() {
                return Ok(JoinSelection::Stay { show_news });
            }
            self.list_matching_conferences(&conferences, &text.to_ascii_uppercase()).await?;
            return Ok(JoinSelection::Retry);
        }

        let Some(number) = number else {
            self.session.op_text = if name.is_empty() { last_token } else { name };
            self.display_text(IceText::InvalidConferenceNumber, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(JoinSelection::Retry);
        };
        if number == self.session.current_conference_number {
            return Ok(JoinSelection::Stay { show_news });
        }
        let Some(conference) = conferences.get(number as usize) else {
            self.session.op_text = if name.is_empty() { last_token } else { name };
            self.display_text(IceText::InvalidConferenceNumber, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(JoinSelection::Retry);
        };
        if conference.name.is_empty() {
            self.session.op_text = number.to_string();
            self.display_text(IceText::InvalidConferenceNumber, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(JoinSelection::Retry);
        }

        if self.registered_in_conference(number, conference) {
            return Ok(JoinSelection::Join { number, show_news });
        }
        if !conference.password.is_empty() && !self.is_lockedout(number) && self.subscription_can_access_conference(number) {
            if self.conference_password_ok(conference).await? {
                if let Some(user) = &mut self.session.current_user {
                    *user.conference_flags.entry(number as usize).or_insert(ConferenceFlags::None) |= ConferenceFlags::Registered;
                }
                return Ok(JoinSelection::Join { number, show_news });
            }
            self.session.password_failure_count = self.session.password_failure_count.saturating_add(1);
            if self.session.password_failure_count >= 4 {
                self.display_text(IceText::DeniedWrongPassword, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                self.logoff_user(crate::icy_board::state::Logoff::Abnormal).await?;
                return Ok(JoinSelection::Stop);
            }
            return Ok(JoinSelection::Retry);
        }

        self.session.op_text = number.to_string();
        self.display_text(IceText::NotRegisteredInConference, display_flags::NEWLINE | display_flags::LFBEFORE)
            .await?;
        if let Some(user) = &mut self.session.current_user {
            user.stats.num_not_reg += 1;
        }
        Ok(JoinSelection::Retry)
    }

    fn registered_in_conference(&self, number: u16, conference: &crate::icy_board::conferences::Conference) -> bool {
        if number == 0 {
            return true;
        }
        let registered = self
            .session
            .current_user
            .as_ref()
            .and_then(|user| user.conference_flags.get(&(number as usize)))
            .is_some_and(|flags| flags.contains(ConferenceFlags::Registered));
        self.subscription_can_access_conference(number)
            && !self.is_lockedout(number)
            && conference.required_security.session_can_access(&self.session)
            && (self.session.is_sysop || conference.is_public || registered)
    }

    /// Two tries; the password already given during this call is accepted without asking.
    async fn conference_password_ok(&mut self, conference: &crate::icy_board::conferences::Conference) -> Res<bool> {
        if !self.session.last_password.is_empty() && conference.password.is_valid(&self.session.last_password) {
            return Ok(true);
        }
        for _ in 0..2 {
            let password = self
                .input_field(
                    IceText::PasswordToJoin,
                    12,
                    MASK_PASSWORD,
                    "",
                    None,
                    display_flags::ECHODOTS | display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::HIGHASCII,
                )
                .await?;
            if conference.password.is_valid(&password) {
                self.session.last_password = password;
                return Ok(true);
            }
            self.display_text(IceText::WrongPasswordEntered, display_flags::NEWLINE).await?;
        }
        Ok(false)
    }

    async fn list_matching_conferences(&mut self, conferences: &[crate::icy_board::conferences::Conference], text: &str) -> Res<()> {
        let mut matches = conferences
            .iter()
            .enumerate()
            .filter(|(_, conference)| !conference.name.is_empty() && conference.name.to_ascii_uppercase().contains(text))
            .filter(|(number, conference)| self.registered_in_conference(*number as u16, conference))
            .map(|(number, conference)| (number, conference.name.clone()))
            .collect::<Vec<_>>();
        matches.sort_by_key(|(_, name)| name.to_ascii_uppercase());
        for (number, name) in matches {
            self.println(TerminalTarget::Both, &format!("{number:5}) {name}")).await?;
            if self.session.disp_options.abort_printout {
                break;
            }
        }
        Ok(())
    }

    async fn join_selected_conference(&mut self, number: u16, show_news: bool) -> Res<()> {
        self.accounting_settle_conference().await?;
        if self.session.is_logoff_requested() {
            return Ok(());
        }
        let abandoned = format!("{} ({})", self.session.current_conference.name, self.session.current_conference_number);
        if !self.set_current_conference(number).await? {
            return Ok(());
        }
        if number == 0 {
            self.session.op_text = abandoned;
            self.display_text(IceText::ConferenceAbandoned, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
        } else {
            self.session.op_text = format!("{} ({})", self.session.current_conference.name, number);
            self.display_text(IceText::ConferenceJoined, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
        }
        self.display_conference_intro(show_news).await?;
        self.process_join(show_news, !show_news).await
    }

    /// The first of the two questions `PCBoard` asks the first time a
    /// conference is entered.
    pub(crate) async fn ask_to_view_conference_members(&mut self, quick_join: bool) -> Res<()> {
        if !self.session.current_conference.allow_view_conf_members || quick_join {
            return Ok(());
        }
        if self.session.is_sysop && self.session.expert_mode() {
            return Ok(());
        }
        let answer = self
            .input_field(
                IceText::ViewConferenceMembers,
                1,
                "",
                "",
                Some(self.session.no_char.to_uppercase().to_string()),
                display_flags::YESNO | display_flags::FIELDLEN | display_flags::UPCASE | display_flags::NEWLINE,
            )
            .await?;
        if answer != self.session.yes_char.to_uppercase().to_string() {
            return Ok(());
        }
        self.list_conference_members().await
    }

    /// The user list narrowed down to the people registered in this conference.
    async fn list_conference_members(&mut self) -> Res<()> {
        self.new_line().await?;
        self.session.disp_options.no_change();
        self.display_text(IceText::UsersHeader, display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::NOTBLANK)
            .await?;
        self.display_text(IceText::UserScanLine, display_flags::NEWLINE | display_flags::NOTBLANK)
            .await?;
        self.reset_color(TerminalTarget::Both).await?;

        let conference = self.session.current_conference_number as usize;
        let mut output = String::new();
        for user in self.get_board().await.users.iter() {
            let registered = user
                .conference_flags
                .get(&conference)
                .is_some_and(|flags| flags.contains(ConferenceFlags::Registered));
            if conference == 0 || registered {
                let _ = write!(
                    output,
                    "{:<25} {:<25} {} {}\r\n",
                    user.get_name(),
                    user.city_or_state,
                    self.format_date(user.stats.last_on),
                    self.format_time(user.stats.last_on)
                );
            }
        }
        self.print(TerminalTarget::Both, &output).await
    }

    /// The second question. The answer is a scan command line,
    /// not just yes or no, and `PCBoard` appends the SINCE flag to it.
    pub(crate) async fn ask_to_scan_message_base(&mut self) -> Res<()> {
        let sec = self.session.user_command_level.cmd_y.clone();
        if self.get_board().await.config.message.disable_message_scan_prompt || !sec.session_can_access(&self.session) {
            return Ok(());
        }
        let answer = self
            .input_field(
                IceText::ScanMessageBase,
                8,
                "ACLQSWZ+-",
                CommandType::YourMailScan.get_help(),
                Some(self.session.yes_char.to_uppercase().to_string()),
                display_flags::YESNO | display_flags::UPCASE | display_flags::STACKED | display_flags::NEWLINE | display_flags::LFBEFORE,
            )
            .await?;
        if answer == self.session.no_char.to_uppercase().to_string() {
            return Ok(());
        }
        let mut command = answer;
        if answer_is_yes(&command, self.session.yes_char) {
            command.clear();
        }
        command.push_str(" S");
        let message_options = self.get_board().await.config.message.clone();
        if message_options.scan_all_mail_at_login || message_options.default_scan_all_selected_confs_at_login {
            command.push_str(" A");
        }
        self.session.push_tokens(&command);
        self.your_mail_scan().await
    }
}

/// The default answer carries no scan flags of its own.
fn answer_is_yes(answer: &str, yes_char: char) -> bool {
    answer.is_empty() || answer.eq_ignore_ascii_case(&yes_char.to_string())
}
