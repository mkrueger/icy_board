use crate::{menu_runner::PcbBoardCommand, Res};

impl PcbBoardCommand {
    pub fn goodbye_cmd(&mut self) -> Res<()> {
        self.displaycmdfile("g")?;
        if let Some(token) = self.state.session.tokens.pop_front() {
            if token.to_ascii_uppercase() == self.state.session.yes_char.to_string().to_ascii_uppercase() {
                self.state.goodbye()?;
            }
        }
        self.state.goodbye()?;
        Ok(())
    }
}
