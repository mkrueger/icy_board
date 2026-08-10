use crate::icy_board::{icb_config::IcbColor, icb_text::IceText, state::functions::display_flags};
use crate::{Res, icy_board::state::IcyBoardState, vm::TerminalTarget};

impl IcyBoardState {
    /// Sysop command 11 - the node list WHO shows, including the nodes that carry
    /// no caller, which is what PCBoard's expanded display adds.
    pub async fn node_list(&mut self) -> Res<()> {
        self.display_text(IceText::UserNetHeader, display_flags::NEWLINE).await?;
        self.display_text(IceText::UsernetUnderline, display_flags::NEWLINE).await?;

        let include_city = self.board.lock().await.config.board.who_include_city;

        // Read the nodes out first - the user lookup takes the board lock.
        let nodes: Vec<Option<(String, usize)>> = self
            .node_state
            .lock()
            .await
            .iter()
            .map(|state| state.as_ref().map(|state| (state.operation.clone(), state.cur_user as usize)))
            .collect();

        let mut lines = Vec::new();
        for (i, node) in nodes.into_iter().enumerate() {
            let Some((operation, cur_user)) = node else {
                lines.push(format!("{:>4}   Available", i + 1));
                continue;
            };
            let board = self.get_board().await;
            let line = match board.users.get(cur_user) {
                Some(user) if include_city => {
                    format!("{:>4}   {:23} {} ({})", i + 1, operation, user.get_name(), user.city_or_state)
                }
                Some(user) => format!("{:>4}   {:23} {}", i + 1, operation, user.get_name()),
                None => format!("{:>4}   {}", i + 1, operation),
            };
            drop(board);
            lines.push(line);
        }

        self.set_color(TerminalTarget::Both, IcbColor::dos_light_cyan()).await?;
        self.println(TerminalTarget::Both, &lines.join("\r\n")).await?;
        self.reset_color(TerminalTarget::Both).await?;
        self.new_line().await?;
        Ok(())
    }
}
