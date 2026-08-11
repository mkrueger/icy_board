use crate::{Res, icy_board::state::IcyBoardState};

impl IcyBoardState {
    pub async fn batch_download_command(&mut self) -> Res<()> {
        // PCBoard's BD is the D command with the batch flag forced on; the
        // download flow already works on the flagged-file batch.
        self.download(true).await
    }
}
