use icy_board_engine::icy_board::{icb_text::IceText, state::functions::display_flags};

use crate::{
    menu_runner::{PcbBoardCommand, MASK_COMMAND},
    Res,
};

impl PcbBoardCommand {
    pub async fn show_help(&mut self) -> Res<()> {
        self.display_menu = true;
        let help_cmd = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            self.state
                .input_field(
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
            let mut help_loc = self.state.get_board().await.config.paths.help_path.clone();
            let mut found = false;
            for action in &self.state.session.current_conference.commands {
                if action.keyword.contains(&help_cmd) && !action.help.is_empty() {
                    help_loc = help_loc.join(&action.help);
                    found = true;
                    break;
                }
            }
            if !found {
                help_loc = help_loc.join(format!("hlp{}", help_cmd).as_str());
            }
            let am = self.state.session.disable_auto_more;
            self.state.session.disable_auto_more = false;
            self.state.session.disp_options.non_stop = false;
            let res = self.state.display_file(&help_loc).await?;
            self.state.session.disable_auto_more = am;

            if res {
                self.state.press_enter().await?;
            }
        }
        Ok(())
    }
}
