use crate::icy_board::commands::CommandType;
use crate::icy_board::state::functions::MASK_COMMAND;
use crate::icy_board::state::IcyBoardState;
use crate::icy_board::{icb_text::IceText, state::functions::display_flags};
use crate::Res;

impl IcyBoardState {
    pub async fn join_conference_cmd(&mut self) -> Res<()> {
        if self.get_board().await.conferences.is_empty() {
            self.display_text(IceText::NoConferenceAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            self.press_enter().await?;
            return Ok(());
        }
        let mut text = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            let mnu = self.get_board().await.config.paths.conf_join_menu.clone();
            let mnu = self.resolve_path(&mnu);

            self.display_menu(&mnu).await?;
            self.new_line().await?;

            self.input_field(
                IceText::JoinConferenceNumber,
                40,
                MASK_COMMAND,
                CommandType::JoinConference.get_help(),
                None,
                display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
            )
            .await?
        };

        if !text.is_empty() {
            let mut joined = false;
            let conferences = self.get_board().await.conferences.clone();

            let quick_join = if text.eq_ignore_ascii_case("Q") {
                text = if let Some(token) = self.session.tokens.pop_front() {
                    token
                } else {
                    self.input_field(
                        IceText::JoinConferenceNumber,
                        40,
                        MASK_COMMAND,
                        CommandType::JoinConference.get_help(),
                        None,
                        display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
                    )
                    .await?
                };
                true
            } else {
                false
            };

            if text.eq_ignore_ascii_case("S") {
                let text_to_scan = if let Some(token) = self.session.tokens.pop_front() {
                    token
                } else {
                    self.input_field(
                        IceText::TextToScanFor,
                        40,
                        MASK_COMMAND,
                        &CommandType::TextSearch.get_help(),
                        None,
                        display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
                    )
                    .await?
                };
                let text_to_scan = text_to_scan.to_ascii_uppercase();
                for (i, c) in conferences.iter().enumerate() {
                    if c.name.to_ascii_uppercase().contains(&text_to_scan) {
                        self.println(crate::vm::TerminalTarget::Both, &format!("{}) {}", i, c.name)).await?;
                    }
                }
                text = if let Some(token) = self.session.tokens.pop_front() {
                    token
                } else {
                    self.input_field(
                        IceText::JoinConferenceNumber,
                        40,
                        MASK_COMMAND,
                        "",
                        None,
                        display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
                    )
                    .await?
                };
            }

            if let Ok(number) = text.parse::<u16>() {
                if (number as usize) <= conferences.len() {
                    self.join_conference(number, quick_join).await;

                    joined = true;
                }
            } else {
                for (i, c) in conferences.iter().enumerate() {
                    if c.name.eq_ignore_ascii_case(&text) || c.name.eq_ignore_ascii_case(&text.replace('_', " ")) {
                        self.join_conference(i as u16, quick_join).await;
                        joined = true;
                        break;
                    }
                }
            }

            if joined {
                self.session.op_text = format!("{} ({})", self.session.current_conference.name, self.session.current_conference_number);
                self.display_text(IceText::ConferenceJoined, display_flags::NEWLINE | display_flags::NOTBLANK)
                    .await?;
            } else {
                self.session.op_text = text;
                self.display_text(IceText::InvalidConferenceNumber, display_flags::NEWLINE | display_flags::NOTBLANK)
                    .await?;
            }
        }

        self.new_line().await?;
        self.press_enter().await?;
        self.display_current_menu = true;
        Ok(())
    }
}
