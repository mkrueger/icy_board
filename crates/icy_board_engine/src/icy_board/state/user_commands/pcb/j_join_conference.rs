use crate::Res;
use crate::icy_board::commands::CommandType;
use crate::icy_board::state::IcyBoardState;
use crate::icy_board::state::functions::{MASK_ASCII, MASK_COMMAND, pwd_flags};
use crate::icy_board::user_base::ConferenceFlags;
use crate::icy_board::{icb_text::IceText, state::functions::display_flags};
use crate::vm::TerminalTarget;
use std::fmt::Write as _;

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
            let mut quick_join = false;
            let mut search = false;
            let mut conf_num = -1;
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
                        40,
                        MASK_COMMAND,
                        CommandType::JoinConference.get_help(),
                        None,
                        display_flags::UPCASE | display_flags::STACKED | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::HIGHASCII,
                    )
                    .await?;
                if str.is_empty() {
                    break;
                }
                self.session.push_tokens(&str);
            }

            let mut search_text = String::new();
            let mut last_token = String::new();
            let mut name = String::new();
            for token in &self.session.tokens {
                last_token = token.clone();
                match token.as_str() {
                    "Q" => {
                        quick_join = true;
                    }
                    "S" => {
                        search = true;
                    }
                    token => {
                        if search || quick_join {
                            search_text.push_str(token);
                            search_text.push(' ');
                        } else if let Ok(num) = token.parse::<i32>() {
                            conf_num = num;
                        } else {
                            if !name.is_empty() {
                                name.push(' ');
                            }
                            let token = token.to_ascii_uppercase();
                            name.push_str(&token);
                        }
                    }
                }
            }
            if !name.is_empty() {
                if name == "MAIN" || name == "MAIN BOARD" {
                    conf_num = 0;
                } else {
                    for (i, conf) in self.get_board().await.conferences.iter().enumerate() {
                        if conf.name.to_ascii_uppercase() == name {
                            conf_num = i as i32;
                            break;
                        }
                    }
                }
            }

            self.session.tokens.clear();
            if conf_num < 0 && search {
                let text = if search_text.is_empty() {
                    self.input_field(
                        IceText::TextToScanFor,
                        40,
                        &MASK_ASCII,
                        CommandType::JoinConference.get_help(),
                        None,
                        display_flags::UPCASE | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::HIGHASCII,
                    )
                    .await?
                } else {
                    search_text.pop();
                    search_text
                };
                if text.is_empty() {
                    break;
                }
                self.search_init(text, false);
                let c = self.get_board().await.conferences.iter().map(|c| c.name.clone()).collect::<Vec<String>>();
                if let Some(regex) = &self.session.search_pattern.clone() {
                    for (i, c) in c.iter().enumerate() {
                        if regex.find(c).is_some() {
                            self.print(crate::vm::TerminalTarget::Both, &format!("{i}) ")).await?;
                            self.print_found_text(crate::vm::TerminalTarget::Both, c).await?;
                            self.new_line().await?;

                            if self.session.disp_options.abort_printout {
                                break;
                            }
                        }
                    }
                }
                self.stop_search();
                continue;
            }

            if conf_num == self.session.current_conference_number as i32 {
                return Ok(());
            }

            let Some(conference) = self.get_board().await.conferences.get(conf_num as usize).cloned() else {
                self.session.op_text = last_token;
                self.display_text(IceText::InvalidConferenceNumber, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                continue;
            };

            if !self.subscription_can_access_conference(conf_num as u16) {
                self.session.op_text.clone_from(&conference.name);
                self.display_text(IceText::NotRegisteredInConference, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                continue;
            }

            if !conference.required_security.session_can_access(&self.session) {
                self.session.op_text.clone_from(&conference.name);
                self.display_text(IceText::NotRegisteredInConference, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                continue;
            }

            if !conference.password.is_empty()
                && !self
                    .check_password(IceText::PasswordToJoin, pwd_flags::PLAIN, |pwd| conference.password.is_valid(pwd))
                    .await?
            {
                self.display_text(IceText::DeniedWrongPassword, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                return Ok(());
            }

            if conf_num == 0 {
                self.session.op_text = format!("{} ({})", self.session.current_conference.name, self.session.current_conference_number);
                self.join_conference(conf_num as u16, quick_join, true).await?;
                self.display_text(IceText::ConferenceAbandoned, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
            } else {
                self.join_conference(conf_num as u16, quick_join, true).await?;
                self.session.op_text = format!("{} ({})", self.session.current_conference.name, self.session.current_conference_number);
                self.display_text(IceText::ConferenceJoined, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
            }
            break;
        }
        Ok(())
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
