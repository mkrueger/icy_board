use crate::{menu_runner::PcbBoardCommand, Res};

impl PcbBoardCommand {
    pub fn ppe_run(&mut self) -> Res<()> {
        if let Some(token) = self.state.session.tokens.pop_front() {
            let mut ppe_path = self.state.resolve_path(&token);
            if !ppe_path.exists() {
                ppe_path = ppe_path.with_extension("ppe");
            }

            self.state.run_ppe(&ppe_path, None)?;
        }
        Ok(())
    }
}
