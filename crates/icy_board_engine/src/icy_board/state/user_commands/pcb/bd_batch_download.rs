use crate::{Res, icy_board::state::IcyBoardState};

impl IcyBoardState {
    pub async fn batch_download_command(&mut self) -> Res<()> {
        self.download_files(true, true).await
    }
}
