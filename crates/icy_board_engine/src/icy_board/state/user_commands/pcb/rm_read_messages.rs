use crate::{
    Res,
    icy_board::{
        icb_text::IceText,
        state::{IcyBoardState, functions::display_flags},
    },
};

impl IcyBoardState {
    pub async fn read_memorized_messages_command(&mut self, t: u8) -> Res<()> {
        let Some((area, _num)) = self.session.memorized_msg else {
            self.display_text(IceText::NotMemorized, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(());
        };

        match t {
            1 => {
                self.session.push_tokens("RM+");
            }
            2 => {
                self.session.push_tokens("RM-");
            }
            _ => {
                self.session.push_tokens("RM+");
            }
        }
        self.read_messages_in_area(area).await?;

        Ok(())
    }
}
