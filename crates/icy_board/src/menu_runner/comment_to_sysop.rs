use super::PcbBoardCommand;
use icy_board_engine::{
    icy_board::{commands::Command, icb_text::IceText, state::functions::display_flags},
    vm::opencap,
};
use icy_ppe::Res;

impl PcbBoardCommand {
    pub fn comment_to_sysop(&mut self, action: &Command) -> Res<()> {
        let leave_comment = self.state.input_field(
            IceText::LeaveComment,
            1,
            "",
            &action.help,
            Some(self.state.session.no_char.to_string()),
            display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::LFAFTER | display_flags::FIELDLEN | display_flags::YESNO,
        )?;

        if leave_comment.is_empty() || leave_comment.chars().next().unwrap() == self.state.session.no_char {
            return Ok(());
        };

        let to = self.state.board.lock().unwrap().config.sysop.name.clone();
        let subj = "comment";

        let receipt = self.state.input_field(
            IceText::RequireReturnReceipt,
            1,
            "",
            &action.help,
            Some(self.state.session.no_char.to_string()),
            display_flags::NEWLINE | display_flags::UPCASE | display_flags::YESNO | display_flags::FIELDLEN,
        )?;
        write_message(-1, -1, &to, &subj, receipt == self.state.session.yes_char.to_uppercase().to_string())?;
        Ok(())
    }
}

fn write_message(conf: i32, area: i32, to: &str, subj: &str, ret_receipt: bool) -> Res<()> {
    Ok(())
}
