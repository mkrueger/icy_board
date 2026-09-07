use crate::{Res, icy_board::state::IcyBoardState};

impl IcyBoardState {
    pub async fn batch_upload_command(&mut self) -> Res<()> {
        // BU entry is independent of U's automatic promotion predicate.
        self.upload_files(true).await
    }
}
