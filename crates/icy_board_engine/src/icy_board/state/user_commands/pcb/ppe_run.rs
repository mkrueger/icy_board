use crate::{Res, icy_board::state::IcyBoardState};

impl IcyBoardState {
    pub async fn ppe_run(&mut self) -> Res<()> {
        if let Some(token) = self.session.tokens.pop_front() {
            let mut ppe_path = self.board.lock().await.resolve_file(&token);
            if !ppe_path.exists() {
                ppe_path = ppe_path.with_extension("ppe");
            }

            self.run_ppe(&ppe_path, None).await?;
        }
        Ok(())
    }
}
