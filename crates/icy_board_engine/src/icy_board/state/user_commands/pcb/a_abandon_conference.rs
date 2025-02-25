use crate::{Res, icy_board::state::IcyBoardState};

impl IcyBoardState {
    pub async fn abandon_conference(&mut self) -> Res<()> {
        self.session.push_tokens("0");
        self.join_conference_cmd().await
    }
}
