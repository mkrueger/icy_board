use crate::icy_board::commands::CommandType;
use crate::icy_board::state::functions::{MASK_ALNUM, MASK_COMMAND};
use crate::{Res, icy_board::state::IcyBoardState};

use crate::icy_board::{icb_text::IceText, state::functions::display_flags};

impl IcyBoardState {
    pub async fn area_change_command(&mut self) -> Res<()> {
        let conference = self.session.current_conference_number as usize;
        let menu = self.get_board().await.conferences[conference].area_menu.clone();
        let areas = self.get_board().await.conferences[conference].areas.clone().unwrap_or_default();

        if areas.is_empty() {
            self.display_text(IceText::NoAreasAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            self.press_enter().await?;
            return Ok(());
        }

        let mut display_menu = self.session.tokens.is_empty();
        loop {
            let mut search = false;
            let mut area_num = -1;
            if self.session.tokens.is_empty() {
                if display_menu {
                    display_menu = false;
                    self.session.disp_options.no_change();
                    self.display_menu(&menu).await?;
                    self.new_line().await?;
                }

                let str = self
                    .input_field(
                        IceText::JoinAreaNumber,
                        40,
                        MASK_COMMAND,
                        CommandType::ChangeMessageArea.get_help(),
                        None,
                        display_flags::UPCASE | display_flags::STACKED | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::HIGHASCII,
                    )
                    .await?;
                self.session.push_tokens(&str);
            }
            if self.session.tokens.is_empty() {
                break;
            }
            let mut search_text = String::new();
            let mut last_token = String::new();
            let mut name = String::new();
            for token in &self.session.tokens {
                last_token = token.clone();
                match token.as_str() {
                    "S" => {
                        search = true;
                    }
                    token => {
                        if search {
                            search_text.push_str(token);
                            search_text.push(' ');
                        } else {
                            if let Ok(num) = token.parse::<i32>() {
                                area_num = num;
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
                for (i, area) in areas.iter().enumerate() {
                    if area.name.to_ascii_uppercase() == name {
                        area_num = i as i32;
                        break;
                    }
                }
            }

            self.session.tokens.clear();
            if area_num < 0 && search {
                let text = if search_text.is_empty() {
                    self.input_field(
                        IceText::TextToScanFor,
                        40,
                        &MASK_ALNUM,
                        CommandType::ChangeMessageArea.get_help(),
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
                if let Some(regex) = &self.session.search_pattern.clone() {
                    let c = areas.iter().map(|a| a.name.clone()).collect::<Vec<String>>();
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

            if area_num == self.session.current_message_area as i32 {
                return Ok(());
            }

            let Some(area) = areas.get(area_num as usize).cloned() else {
                self.session.op_text = last_token;
                self.display_text(IceText::InvalidAreaNumber, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                continue;
            };

            if !area.req_level_to_enter.user_can_access(&self.session) {
                self.session.op_text = area.name.clone();
                self.display_text(IceText::NotRegisteredInConference, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                continue;
            }

            /* Areas (not yet) have a password
            if !area.password.is_empty() {
                if !self
                    .check_password(IceText::PasswordToJoin, pwd_flags::PLAIN, |pwd| area.password.is_valid(pwd))
                    .await?
                {
                    self.display_text(IceText::DeniedWrongPassword, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                    return Ok(());
                }
            }*/

            self.session.current_message_area = area_num as usize;
            self.session.op_text = format!("{} ({})", area.name, self.session.current_message_area);
            self.display_text(IceText::AreaJoined, display_flags::NEWLINE | display_flags::LFBEFORE).await?;
            break;
        }
        Ok(())
    }
}
