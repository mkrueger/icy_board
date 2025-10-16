use crate::Res;
use crate::icy_board::commands::CommandType;
use crate::icy_board::state::IcyBoardState;
use crate::icy_board::state::functions::{MASK_ALNUM, MASK_COMMAND, pwd_flags};
use crate::icy_board::{icb_text::IceText, state::functions::display_flags};

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
                        } else {
                            if let Ok(num) = token.parse::<i32>() {
                                conf_num = num;
                            } else {
                                if name.len() > 0 {
                                    name.push(' ');
                                }
                                let token = token.to_ascii_uppercase();
                                name.push_str(&token);
                            }
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
                        &MASK_ALNUM,
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
                        if let Some(_) = regex.find(c) {
                            self.print(crate::vm::TerminalTarget::Both, &format!("{}) ", i)).await?;
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

            if !conference.required_security.session_can_access(&self.session) {
                self.session.op_text = conference.name.clone();
                self.display_text(IceText::NotRegisteredInConference, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                continue;
            }

            if !conference.password.is_empty() {
                if !self
                    .check_password(IceText::PasswordToJoin, pwd_flags::PLAIN, |pwd| conference.password.is_valid(pwd))
                    .await?
                {
                    self.display_text(IceText::DeniedWrongPassword, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                    return Ok(());
                }
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
}
