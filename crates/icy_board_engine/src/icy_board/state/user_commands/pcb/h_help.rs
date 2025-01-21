use crate::icy_board::state::functions::MASK_COMMAND;
use crate::icy_board::state::IcyBoardState;
use crate::icy_board::{icb_text::IceText, state::functions::display_flags};

use crate::Res;

impl IcyBoardState {
    pub async fn show_help_cmd(&mut self) -> Res<()> {
        self.display_current_menu = true;
        let help_cmd = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            self.input_field(
                IceText::HelpPrompt,
                8,
                MASK_COMMAND,
                "",
                None,
                display_flags::UPCASE | display_flags::NEWLINE | display_flags::HIGHASCII,
            )
            .await?
        };
        if !help_cmd.is_empty() {
            let mut help_loc = self.get_board().await.config.paths.help_path.clone();
            let mut found = false;
            for action in &self.session.current_conference.commands {
                if action.keyword.contains(&help_cmd) && !action.help.is_empty() {
                    help_loc = help_loc.join(&action.help);
                    found = true;
                    break;
                }
            }
            if !found {
                help_loc = help_loc.join(format!("hlp{}", help_cmd).as_str());
            }
            let am = self.session.is_non_stop;
            self.session.is_non_stop = false;
            self.session.disp_options.non_stop = false;
            let res = self.display_file(&help_loc).await?;
            self.session.is_non_stop = am;

            if res {
                self.press_enter().await?;
            }
        }
        Ok(())
    }
}
