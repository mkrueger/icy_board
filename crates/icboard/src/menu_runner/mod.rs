use icy_board_engine::icy_board::state::IcyBoardState;
use std::collections::HashMap;

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
}
