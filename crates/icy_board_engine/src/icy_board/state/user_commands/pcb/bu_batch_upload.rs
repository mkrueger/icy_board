use crate::{Res, icy_board::state::IcyBoardState};

impl IcyBoardState {
    pub async fn batch_upload_command(&mut self) -> Res<()> {
        // PCBoard's BU is the U command with the batch flag forced on; the
        // upload flow already drives a batch protocol.
        self.upload_file().await
    }
}
