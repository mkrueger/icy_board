use std::collections::HashMap;

use crate::Res;
use icy_board_engine::icy_board::state::IcyBoardState;

mod login;

pub struct PcbBoardCommand {
    pub state: IcyBoardState,
    pub display_menu: bool,
    pub autorun_times: HashMap<usize, u64>,
    pub saved_cmd: String,
}
pub const MASK_COMMAND: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+-=[]{}|;':,.<>?/\\\" ";
const MASK_NUMBER: &str = "0123456789";

impl PcbBoardCommand {
    pub fn new(state: IcyBoardState) -> Self {
        Self {
            state,
            display_menu: true,
            saved_cmd: String::new(),
            autorun_times: HashMap::new(),
        }
    }

    pub async fn do_command(&mut self) -> Res<()> {
        self.state.ask_run_command().await
    }
}
