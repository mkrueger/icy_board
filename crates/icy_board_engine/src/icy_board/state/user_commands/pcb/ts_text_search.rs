use crate::icy_board::{
    icb_text::IceText,
    state::{
        functions::{display_flags, MASK_COMMAND, MASK_NUM},
        user_commands::message_reader::MessageViewer,
        IcyBoardState, NodeStatus,
    },
};
use bstr::BString;
use jamjam::jam::JamMessageBase;

use crate::Res;

impl IcyBoardState {
    pub async fn text_search(&mut self, help: &str) -> Res<()> {
        self.set_activity(NodeStatus::HandlingMail).await;
        let Ok(Some(area)) = self.show_message_areas(self.session.current_conference_number, help).await else {
            self.press_enter().await?;
            self.display_current_menu = true;
            return Ok(());
        };

        let search_text = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            self.input_field(
                IceText::TextToScanFor,
                40,
                MASK_COMMAND,
                help,
                None,
                display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
            )
            .await?
        };
        if search_text.is_empty() {
            self.press_enter().await?;
            self.display_current_menu = true;
            return Ok(());
        }
        self.text_search_in_area(&search_text, area, help).await
    }

    async fn text_search_in_area(&mut self, search_text: &str, area: usize, help: &str) -> Res<()> {
        let message_base_file = &self.session.current_conference.areas[area].filename;
        let msgbase_file_resolved = self.get_board().await.resolve_file(message_base_file);
        match JamMessageBase::open(&msgbase_file_resolved) {
            Ok(message_base) => {
                let msg_search_from = if let Some(token) = self.session.tokens.pop_front() {
                    token
                } else {
                    self.session.op_text = format!("{}-{}", message_base.base_messagenumber(), message_base.active_messages());
                    self.input_field(
                        IceText::MessageSearchFrom,
                        8,
                        &MASK_NUM,
                        help,
                        None,
                        display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
                    )
                    .await?
                };
                if msg_search_from.is_empty() {
                    self.press_enter().await?;
                    self.display_current_menu = true;
                    return Ok(());
                }
                let mut start = msg_search_from.parse::<u32>()?.max(message_base.base_messagenumber());
                let search_text = BString::from(search_text);
                let viewer = MessageViewer::load(&self.display_text)?;
                while start < message_base.active_messages() {
                    if let Ok(msg) = message_base.read_header(start) {
                        let txt = message_base.read_msg_text(&msg)?;
                        let matches = get_matches(&txt, &search_text);
                        if !matches.is_empty() {
                            self.read_message_number(&message_base, &viewer, start, Some(matches)).await?;
                        }
                    }
                    start += 1;
                }
                self.press_enter().await?;
                self.display_current_menu = true;
                Ok(())
            }
            Err(err) => {
                log::error!("Message index load error {}", err);
                log::error!("Creating new message index at {}", msgbase_file_resolved.display());
                self.display_text(IceText::CreatingNewMessageIndex, display_flags::NEWLINE | display_flags::LFAFTER)
                    .await?;
                if JamMessageBase::create(msgbase_file_resolved).is_ok() {
                    log::error!("successfully created new message index.");
                }
                log::error!("failed to create message index.");

                self.display_text(IceText::PathErrorInSystemConfiguration, display_flags::NEWLINE | display_flags::LFAFTER)
                    .await?;

                self.press_enter().await?;
                self.display_current_menu = true;
                Ok(())
            }
        }
    }
}

fn get_matches(txt: &BString, search_text: &BString) -> Vec<(usize, usize)> {
    let mut matches = Vec::new();
    for i in 0..txt.len() - search_text.len() {
        if txt[i..i + search_text.len()] == *search_text {
            matches.push((i, i + search_text.len()));
        }
    }
    matches
}
