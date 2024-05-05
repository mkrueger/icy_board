use crate::{menu_runner::PcbBoardCommand, Res};
use icy_board_engine::icy_board::commands::Command;

impl PcbBoardCommand {
    pub async fn broadcast(&mut self, _action: &Command) -> Res<()> {
        let mut hi = 0;
        let mut lo = 0;
        if let Some(node) = self.state.session.tokens.pop_front() {
            if node.eq_ignore_ascii_case("all") {
                lo = 0;
                hi = u16::MAX;
            } else {
                let node = node.parse::<u16>().unwrap_or_default();
                hi = node;
                lo = node;
            }
        }

        let mut res = String::new();
        for tok in self.state.session.tokens.drain(..) {
            if !res.is_empty() {
                res.push(' ');
            }
            res.push_str(&tok);
        }
        if !res.is_empty() {
            self.state.broadcast(lo, hi, &res).await?;
        }

        Ok(())
    }
}
