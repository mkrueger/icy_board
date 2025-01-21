use crate::{icy_board::state::IcyBoardState, Res};

impl IcyBoardState {
    pub fn redisplay_cmd(&mut self) -> Res<()> {
        if !self.saved_cmd.is_empty() {
            let cmd = self.saved_cmd.clone();
            self.put_keyboard_buffer(&cmd, true)?;
        }
        Ok(())
    }
}
