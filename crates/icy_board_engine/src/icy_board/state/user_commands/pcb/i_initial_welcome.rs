use crate::vm::TerminalTarget;
use crate::{icy_board::state::IcyBoardState, Res};

impl IcyBoardState {
    pub async fn initial_welcome(&mut self) -> Res<()> {
        let board_name = self.get_board().await.config.board.name.clone();
        self.print(TerminalTarget::Both, &board_name).await?;
        self.new_line().await?;
        let welcome_screen = self.get_board().await.config.paths.welcome.clone();
        let welcome_screen = self.resolve_path(&welcome_screen);
        self.display_file(&welcome_screen).await?;
        Ok(())
    }
}
