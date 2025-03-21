use crate::{Res, icy_board::state::IcyBoardState};

use crate::icy_board::{
    icb_text::IceText,
    state::{
        NodeStatus,
        functions::{MASK_ASCII, display_flags},
    },
};

impl IcyBoardState {
    pub async fn write_email(&mut self) -> Res<()> {
        self.set_activity(NodeStatus::EnterMessage).await;
        let to = self
            .input_field(
                IceText::To,
                54,
                &MASK_ASCII,
                "",
                None,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::FIELDLEN,
            )
            .await?;
        let lowercase_to = to.to_lowercase();
        let user_exists = self
            .board
            .lock()
            .await
            .users
            .iter()
            .any(|user| user.name.to_ascii_lowercase() == to || user.alias.to_ascii_lowercase() == to)
            || self.board.lock().await.config.sysop.name.to_ascii_lowercase() == lowercase_to;
        if !user_exists {
            self.session.op_text = to;
            self.display_text(IceText::NotInUsersFile, display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::BELL)
                .await?;
            return Ok(());
        }

        let subject = self
            .input_field(
                IceText::MessageSubject,
                54,
                &MASK_ASCII,
                "",
                None,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::FIELDLEN,
            )
            .await?;

        if subject.is_empty() {
            self.new_line().await?;
            return Ok(());
        };

        self.write_message(-1, -1, &to, &subject, false, IceText::SavingMessage).await?;

        Ok(())
    }
}
