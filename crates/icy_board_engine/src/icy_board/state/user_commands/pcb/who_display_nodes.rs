use crate::{
    icy_board::{
        icb_config::IcbColor,
        icb_text::IceText,
        state::{NodeStatus, functions::display_flags},
    },
    vm::TerminalTarget,
};

use crate::{Res, icy_board::state::IcyBoardState};

const STATUS_WIDTH: usize = 23;
const NAME_WIDTH: usize = 48;
/// Where the extended line puts the operation, under the user column.
const OPERATION_INDENT: usize = 31;

impl IcyBoardState {
    pub async fn who_display_nodes(&mut self) -> Res<()> {
        if self.displaycmdfile("who").await? {
            return Ok(());
        }

        // Only a caller who reaches the node-list level may ask for the second line.
        let asked_extended = self.session.tokens.pop_front().is_some_and(|token| token.eq_ignore_ascii_case("X"));
        let extended = asked_extended && self.session.sysop_command_level.sec_11_view_other_nodes.session_can_access(&self.session);

        self.display_text(IceText::UserNetHeader, display_flags::NEWLINE | display_flags::LFBEFORE)
            .await?;
        self.display_text(IceText::UsernetUnderline, display_flags::NEWLINE).await?;

        let include_city = self.board.lock().await.config.board.who_include_city;
        let show_alias = self.board.lock().await.config.board.who_show_alias;

        let mut nodes = Vec::new();
        for (i, connection) in self.node_state.lock().await.iter().enumerate() {
            if let Some(connection) = connection {
                nodes.push((i + 1, connection.status, connection.operation.clone(), connection.cur_user));
            }
        }

        for (number, status, operation, cur_user) in nodes {
            let name = match self.get_board().await.users.get(cur_user as usize) {
                Some(user) => {
                    let name = if show_alias && !user.alias.is_empty() {
                        user.alias.clone()
                    } else {
                        user.get_name().clone()
                    };
                    if include_city && !user.city_or_state.is_empty() {
                        format!("{} ({})", name, user.city_or_state)
                    } else {
                        name
                    }
                }
                None => String::new(),
            };

            let status_text = self.get_display_text(status.text())?;
            self.set_color(TerminalTarget::Both, IcbColor::dos_light_cyan()).await?;
            self.print(
                TerminalTarget::Both,
                &format!("{number:>4}   {status:<width$} ", status = truncate(&status_text, STATUS_WIDTH), width = STATUS_WIDTH),
            )
            .await?;

            // A node busy with something rather than with a caller shows what it
            // is doing where the name would go.
            let shows_operation = !operation.is_empty() && matches!(status, NodeStatus::RunningDoor | NodeStatus::RunningEvent | NodeStatus::NoCaller);
            if shows_operation {
                self.println(TerminalTarget::Both, &operation).await?;
            } else if !matches!(status, NodeStatus::NoCaller) && !name.is_empty() {
                self.println(TerminalTarget::Both, truncate(&name, NAME_WIDTH)).await?;
            } else {
                self.new_line().await?;
            }

            if extended && !operation.is_empty() && !shows_operation {
                self.print(TerminalTarget::Both, &" ".repeat(OPERATION_INDENT)).await?;
                self.println(TerminalTarget::Both, &operation).await?;
            }
        }
        self.reset_color(TerminalTarget::Both).await?;
        Ok(())
    }
}

fn truncate(text: &str, len: usize) -> &str {
    match text.char_indices().nth(len) {
        Some((end, _)) => &text[..end],
        None => text,
    }
}
