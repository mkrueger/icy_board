use crate::{Res, icy_board::state::IcyBoardState};

impl IcyBoardState {
    pub async fn read_email(&mut self) -> Res<()> {
        let name = self.session.user_name.to_string();
        let msg_base = self.get_email_msgbase(&name).await?;
        self.read_msgs_from_base(msg_base, true).await?;
        Ok(())
    }
}
