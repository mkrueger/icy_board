use crate::{menu_runner::PcbBoardCommand, Res};

impl PcbBoardCommand {
    pub async fn read_email(&mut self, help: &str) -> Res<()> {
        let name = self.state.session.user_name.to_string();
        let msg_base = self.state.get_email_msgbase(&name).await?;
        self.read_msgs_from_base(msg_base, help).await?;
        Ok(())
    }
}
