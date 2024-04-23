use crate::{menu_runner::PcbBoardCommand, Res};

impl PcbBoardCommand {
    pub fn redisplay_cmd(&mut self) -> Res<()> {
        if !self.saved_cmd.is_empty() {
            self.state.put_keyboard_buffer(&self.saved_cmd, true)?;
        }
        Ok(())
    }
}
