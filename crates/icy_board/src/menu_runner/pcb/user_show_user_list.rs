use crate::{
    menu_runner::{PcbBoardCommand, MASK_COMMAND},
    Res,
};
use icy_board_engine::{
    icy_board::{commands::Command, icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};

impl PcbBoardCommand {
    pub async fn show_user_list(&mut self, action: &Command) -> Res<()> {
        self.state.new_line().await?;
        let text = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            self.state
                .input_field(
                    IceText::UserScan,
                    40,
                    MASK_COMMAND,
                    &action.help,
                    None,
                    display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
                )
                .await?
        };

        self.state
            .display_text(IceText::UsersHeader, display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::NOTBLANK)
            .await?;
        self.state.display_text(IceText::UserScanLine, display_flags::NOTBLANK).await?;
        self.state.reset_color().await?;
        self.state.new_line().await?;
        let mut output = String::new();
        for u in self.state.get_board().await.users.iter() {
            if text.is_empty() || u.get_name().to_ascii_uppercase().contains(&text.to_ascii_uppercase()) {
                output.push_str(&format!(
                    "{:<24} {:<30} {} {}\r\n",
                    u.get_name(),
                    u.city_or_state,
                    self.state.format_date(u.stats.last_on),
                    self.state.format_time(u.stats.last_on)
                ));
            }
        }
        self.state.print(TerminalTarget::Both, &output).await?;

        self.state.new_line().await?;
        self.state.press_enter().await?;
        self.display_menu = true;
        Ok(())
    }
}
