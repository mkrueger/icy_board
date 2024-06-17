use crate::{
    menu_runner::{PcbBoardCommand, MASK_COMMAND},
    Res,
};
use icy_board_engine::icy_board::{icb_text::IceText, state::functions::display_flags};

impl PcbBoardCommand {
    pub async fn join_conference(&mut self, help: &str) -> Res<()> {
        if self.state.get_board().await.conferences.is_empty() {
            self.state
                .display_text(IceText::NoConferenceAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            self.state.press_enter().await?;
            return Ok(());
        }
        let mut text = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            let mnu = self.state.get_board().await.config.paths.conf_join_menu.clone();
            let mnu = self.state.resolve_path(&mnu);

            self.state.display_menu(&mnu).await?;
            self.state.new_line().await?;

            self.state
                .input_field(
                    IceText::JoinConferenceNumber,
                    40,
                    MASK_COMMAND,
                    help,
                    None,
                    display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
                )
                .await?
        };

        if !text.is_empty() {
            let mut joined = false;
            let conferences = self.state.get_board().await.conferences.clone();

            let quick_join = if text.eq_ignore_ascii_case("Q") {
                text = if let Some(token) = self.state.session.tokens.pop_front() {
                    token
                } else {
                    self.state
                        .input_field(
                            IceText::JoinConferenceNumber,
                            40,
                            MASK_COMMAND,
                            help,
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
                let text_to_scan = if let Some(token) = self.state.session.tokens.pop_front() {
                    token
                } else {
                    self.state
                        .input_field(
                            IceText::TextToScanFor,
                            40,
                            MASK_COMMAND,
                            help,
                            None,
                            display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
                        )
                        .await?
                };
                let text_to_scan = text_to_scan.to_ascii_uppercase();
                for (i, c) in conferences.iter().enumerate() {
                    if c.name.to_ascii_uppercase().contains(&text_to_scan) {
                        self.state
                            .println(icy_board_engine::vm::TerminalTarget::Both, &format!("{}) {}", i, c.name))
                            .await?;
                    }
                }
                text = if let Some(token) = self.state.session.tokens.pop_front() {
                    token
                } else {
                    self.state
                        .input_field(
                            IceText::JoinConferenceNumber,
                            40,
                            MASK_COMMAND,
                            help,
                            None,
                            display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
                        )
                        .await?
                };
            }

            if let Ok(number) = text.parse::<u16>() {
                if (number as usize) <= conferences.len() {
                    self.state.join_conference(number, quick_join).await;

                    joined = true;
                }
            } else {
                for (i, c) in conferences.iter().enumerate() {
                    if c.name.eq_ignore_ascii_case(&text) || c.name.eq_ignore_ascii_case(&text.replace('_', " ")) {
                        self.state.join_conference(i as u16, quick_join).await;
                        joined = true;
                        break;
                    }
                }
            }

            if joined {
                self.state.session.op_text = format!(
                    "{} ({})",
                    self.state.session.current_conference.name, self.state.session.current_conference_number
                );
                self.state
                    .display_text(IceText::ConferenceJoined, display_flags::NEWLINE | display_flags::NOTBLANK)
                    .await?;
            } else {
                self.state.session.op_text = text;
                self.state
                    .display_text(IceText::InvalidConferenceNumber, display_flags::NEWLINE | display_flags::NOTBLANK)
                    .await?;
            }
        }

        self.state.new_line().await?;
        self.state.press_enter().await?;
        self.display_menu = true;
        Ok(())
    }
}
