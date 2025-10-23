use crate::{
    Res,
    icy_board::{
        icb_text::IceText,
        state::{IcyBoardState, functions::display_flags},
        user_base::ChatStatus,
    },
};

impl IcyBoardState {
    pub async fn group_chat_command(&mut self) -> Res<()> {
        loop {
            // Get token from command line or prompt user
            let input = if let Some(token) = self.session.tokens.pop_front() {
                token
            } else {
                let chat_status = if let Some(user) = &self.session.current_user {
                    user.chat_status
                } else {
                    ChatStatus::Available
                };

                let is_available = chat_status == ChatStatus::Available;
                let prompt = if is_available { IceText::NodeChatUPrompt } else { IceText::NodeChatAPrompt };

                self.input_field(
                    prompt,
                    1,
                    "AGU",
                    "hlpchat",
                    None,
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE | display_flags::FIELDLEN,
                )
                .await?
            };

            if input.is_empty() {
                break;
            }

            let cmd = input.to_uppercase();

            // Handle single-character commands
            if cmd.len() == 1 {
                match cmd.chars().next().unwrap() {
                    'A' => {
                        if let Some(user) = &mut self.session.current_user {
                            user.chat_status = ChatStatus::Available;
                        }
                        self.display_text(IceText::Available, display_flags::LFBEFORE | display_flags::NEWLINE).await?;
                        return Ok(());
                    }
                    'U' => {
                        if let Some(user) = &mut self.session.current_user {
                            user.chat_status = ChatStatus::Unavailable;
                        }
                        self.display_text(IceText::Unavailable, display_flags::LFBEFORE | display_flags::NEWLINE)
                            .await?;
                        return Ok(());
                    }
                    'G' => {
                        self.start_group_chat().await?;
                        return Ok(());
                    }
                    _ => {
                        // Invalid option, loop back to prompt
                        continue;
                    }
                }
            }
        }

        Ok(())
    }
}
