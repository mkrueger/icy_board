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
