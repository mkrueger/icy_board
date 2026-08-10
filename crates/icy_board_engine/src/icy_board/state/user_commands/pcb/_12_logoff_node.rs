use crate::icy_board::bbs::BBSMessage;
use crate::icy_board::commands::CommandType;
use crate::icy_board::{
    icb_text::IceText,
    state::functions::{MASK_NUM, display_flags},
};
use crate::{Res, icy_board::state::IcyBoardState};

impl IcyBoardState {
    /// Sysop command 12 - force another node to log its caller off.
    pub async fn logoff_node(&mut self) -> Res<()> {
        loop {
            let answer = if let Some(token) = self.session.tokens.pop_front() {
                token
            } else {
                self.node_list().await?;
                self.input_field(
                    IceText::NodeNumberToLogoff,
                    5,
                    &MASK_NUM,
                    CommandType::LogoffNode.get_help(),
                    None,
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::FIELDLEN,
                )
                .await?
            };
            if answer.is_empty() {
                return Ok(());
            }

            let Ok(node) = answer.parse::<usize>() else {
                continue;
            };
            if node < 1 || node > self.node_state.lock().await.len() {
                continue;
            }
            if node == self.node + 1 {
                // PCBoard has no guard here, but dropping your own session from
                // the sysop menu is never what was meant.
                self.display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::BELL)
                    .await?;
                return Ok(());
            }

            let sender = self.bbs.lock().await.bbs_channels.get(node - 1).and_then(|c| c.clone());
            let Some(sender) = sender else {
                self.display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                return Ok(());
            };

            let name = self.session.user_name.clone();
            let _ = sender.send(BBSMessage::Shutdown(format!("Node {} was logged off by {}", node, name))).await;
            self.write_caller_log(&format!("Forced node {} to logoff", node)).await;
            return Ok(());
        }
    }
}
