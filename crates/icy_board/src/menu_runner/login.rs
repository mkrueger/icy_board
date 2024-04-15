use super::PcbBoardCommand;
use icy_ppe::Res;

impl PcbBoardCommand {
    pub fn login(&mut self) -> Res<bool> {
        Ok(false)
    }
}
