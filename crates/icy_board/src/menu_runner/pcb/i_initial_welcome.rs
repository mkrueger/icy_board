use crate::{menu_runner::PcbBoardCommand, Res};
use icy_board_engine::vm::TerminalTarget;

impl PcbBoardCommand {
    pub fn initial_welcome(&mut self) -> Res<()> {
        let board_name = self.state.board.lock().unwrap().config.board.name.clone();
        self.state.print(TerminalTarget::Both, &board_name)?;
        self.state.new_line()?;
        let welcome_screen = self.state.board.lock().unwrap().config.paths.welcome.clone();
        let welcome_screen = self.state.resolve_path(&welcome_screen);
        self.state.display_file(&welcome_screen)?;
        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }
}
