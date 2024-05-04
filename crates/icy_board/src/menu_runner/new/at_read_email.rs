use icy_board_engine::icy_board::commands::Command;

use crate::{menu_runner::PcbBoardCommand, Res};

impl PcbBoardCommand {
    pub async fn read_email(&mut self, action: &Command) -> Res<()> {
        let name = self.state.session.user_name.to_string();
        let msg_base = self.state.get_email_msgbase(&name).await?;
        self.read_msgs_from_base(msg_base, action).await?;
        Ok(())
    }
}
