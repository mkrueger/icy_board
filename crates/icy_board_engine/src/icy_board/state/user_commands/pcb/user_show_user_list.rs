use crate::Res;
use crate::icy_board::commands::CommandType;
use crate::icy_board::state::IcyBoardState;
use crate::icy_board::state::functions::MASK_COMMAND;
use crate::{
    icy_board::{icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};

impl IcyBoardState {
    pub async fn show_user_list_cmd(&mut self) -> Res<()> {
        self.new_line().await?;
        let text = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            self.input_field(
                IceText::UserScan,
                40,
                MASK_COMMAND,
                CommandType::UserList.get_help(),
                None,
                display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
            )
            .await?
        };
        self.session.disp_options.no_change();
        self.display_text(IceText::UsersHeader, display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::NOTBLANK)
            .await?;
        self.display_text(IceText::UserScanLine, display_flags::NOTBLANK).await?;
        self.reset_color(TerminalTarget::Both).await?;
        self.new_line().await?;
        let mut output = String::new();
        for u in self.get_board().await.users.iter() {
            if text.is_empty() || u.get_name().to_ascii_uppercase().contains(&text.to_ascii_uppercase()) {
                output.push_str(&format!(
                    "{:<25} {:<25} {} {}\r\n",
                    u.get_name(),
                    u.city_or_state,
                    self.format_date(u.stats.last_on),
                    self.format_time(u.stats.last_on)
                ));
            }
        }
        self.print(TerminalTarget::Both, &output).await?;
        Ok(())
    }
}
