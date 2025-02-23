use crate::icy_board::{
    commands::CommandType,
    icb_text::IceText,
    state::{
        functions::{display_flags, MASK_COMMAND, MASK_NUM},
        user_commands::mods::messagereader::MessageViewer,
        IcyBoardState, NodeStatus,
    },
};
use bstr::BString;
use jamjam::jam::JamMessageBase;

use crate::Res;

impl IcyBoardState {
    pub async fn text_search(&mut self) -> Res<()> {
        self.set_activity(NodeStatus::HandlingMail).await;
        let search_text = if let Some(token) = self.session.tokens.pop_front() {
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
        if search_text.is_empty() {
            return Ok(());
        }

        let message_base_file = &self.session.current_conference.areas.as_ref().unwrap()[self.session.current_message_area].filename;
        let msgbase_file_resolved = self.get_board().await.resolve_file(message_base_file);
        match JamMessageBase::open(&msgbase_file_resolved) {
            Ok(mut message_base) => {
                let msg_search_from = if let Some(token) = self.session.tokens.pop_front() {
                    token
                } else {
                    self.session.op_text = format!("{}-{}", message_base.base_messagenumber(), message_base.active_messages());
                    self.input_field(
                        IceText::MessageSearchFrom,
                        8,
                        &MASK_NUM,
                        "",
                        None,
                        display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
                    )
                    .await?
                };
                if msg_search_from.is_empty() {
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
                            self.read_message_number(&mut message_base, &viewer, start, Some(matches)).await?;
                        }
                    }
                    start += 1;
                }
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
