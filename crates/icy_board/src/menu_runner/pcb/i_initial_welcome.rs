use crate::{menu_runner::PcbBoardCommand, Res};
use icy_board_engine::vm::TerminalTarget;

impl PcbBoardCommand {
    pub async fn initial_welcome(&mut self) -> Res<()> {
        let board_name = self.state.get_board().await.config.board.name.clone();
        self.state.print(TerminalTarget::Both, &board_name).await?;
        self.state.new_line().await?;
        let welcome_screen = self.state.get_board().await.config.paths.welcome.clone();
        let welcome_screen = self.state.resolve_path(&welcome_screen);
        self.state.display_file(&welcome_screen).await?;
        self.state.new_line().await?;
        self.state.press_enter().await?;
        self.display_menu = true;
        Ok(())
    }
}
