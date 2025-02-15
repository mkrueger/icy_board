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
            let help_cmd = help_cmd.to_ascii_uppercase();
            for action in self
                .session
                .current_conference
                .commands
                .iter()
                .chain(self.board.lock().await.commands.commands.iter())
            {
                if action.keyword.to_ascii_uppercase().starts_with(&help_cmd) {
                    if !action.help.is_empty() {
                        help_loc = help_loc.join(&action.help);
                        found = true;
                        break;
                    } else if let Some(first) = action.actions.first() {
                        let hlp = first.command_type.get_help();
                        if !hlp.is_empty() {
                            help_loc = help_loc.join(hlp);
                            found = true;
                            break;
                        }
                    }
                }
            }
            if !found {
                help_loc = help_loc.join(help_cmd.to_ascii_lowercase());
            }
            let am = self.session.disp_options.count_lines;
            self.session.disp_options.force_count_lines();
            self.display_file(&help_loc).await?;
            self.session.disp_options.count_lines = am;
        }
        Ok(())
    }
}
