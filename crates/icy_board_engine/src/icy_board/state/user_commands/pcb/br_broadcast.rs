use crate::{Res, icy_board::state::IcyBoardState};

impl IcyBoardState {
    pub async fn broadcast_command(&mut self) -> Res<()> {
        let mut hi = 0;
        let mut lo = 0;
        if let Some(node) = self.session.tokens.pop_front() {
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
        for tok in self.session.tokens.drain(..) {
            if !res.is_empty() {
                res.push(' ');
            }
            res.push_str(&tok);
        }
        if !res.is_empty() {
            self.broadcast(lo, hi, &res).await?;
        }

        Ok(())
    }
}
