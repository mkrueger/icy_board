use crate::{
    icy_board::{icb_config::IcbColor, icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};

use crate::{Res, icy_board::state::IcyBoardState};

impl IcyBoardState {
    pub async fn who_display_nodes(&mut self) -> Res<()> {
        if self.displaycmdfile("who").await? {
            return Ok(());
        }

        self.display_text(IceText::UserNetHeader, display_flags::NEWLINE).await?;
        self.display_text(IceText::UsernetUnderline, display_flags::NEWLINE).await?;
        let mut lines = Vec::new();

        let include_city = self.board.lock().await.config.board.who_include_city;
        let show_alias = self.board.lock().await.config.board.who_show_alias;

        for (i, connection) in self.node_state.lock().await.iter().enumerate() {
            if let Some(connection) = connection {
                if let Some(user) = self.get_board().await.users.get(connection.cur_user as usize) {
                    let name = if show_alias {
                        if user.alias.is_empty() { user.get_name().clone() } else { user.alias.clone() }
                    } else {
                        user.get_name().clone()
                    };
                    let txt = if include_city {
                        format!("{:>4}   {:23} {} ({})", i + 1, connection.operation, name, user.city_or_state)
                    } else {
                        format!("{:>4}   {:23} {}", i + 1, connection.operation, name)
                    };
                    lines.push(txt);
                }
            }
        }
        self.set_color(TerminalTarget::Both, IcbColor::dos_light_cyan()).await?;
        self.println(TerminalTarget::Both, &lines.join("\r\n")).await?;
        self.new_line().await?;
        Ok(())
    }
}
