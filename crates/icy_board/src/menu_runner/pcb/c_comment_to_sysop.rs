use crate::mods::editor::{EditResult, EditState};

use crate::{menu_runner::PcbBoardCommand, Res};

use bstr::BString;
use chrono::Utc;
use icy_board_engine::{
    datetime::IcbTime,
    icy_board::{
        commands::Command,
        icb_text::IceText,
        state::{functions::display_flags, UserActivity},
    },
};
use icy_engine::Position;
use jamjam::jam::JamMessage;

impl PcbBoardCommand {
    pub fn comment_to_sysop(&mut self, action: &Command) -> Res<()> {
        let leave_comment = self.state.input_field(
            IceText::LeaveComment,
            1,
            "",
            &action.help,
            Some(self.state.session.no_char.to_string()),
            display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::FIELDLEN | display_flags::YESNO,
        )?;

        if leave_comment.is_empty() || leave_comment.chars().next().unwrap() == self.state.session.no_char {
            self.state.new_line()?;
            self.state.press_enter()?;
            self.display_menu = true;

            return Ok(());
        };

        let to = self.state.board.lock().unwrap().config.sysop.name.clone();
        let subj = format!("COMMENT {}", IcbTime::now().to_string());

        let receipt = self.state.input_field(
            IceText::RequireReturnReceipt,
            1,
            "",
            &"",
            Some(self.state.session.no_char.to_string()),
            display_flags::NEWLINE | display_flags::UPCASE | display_flags::YESNO | display_flags::FIELDLEN,
        )?;
        self.state.set_activity(UserActivity::CommentToSysop);
        self.write_message(
            -1,
            -1,
            &to,
            &subj,
            receipt == self.state.session.yes_char.to_uppercase().to_string(),
            IceText::SavingComment,
        )?;

        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }

    pub fn write_message(&mut self, conf: i32, area: i32, to: &str, subj: &str, _ret_receipt: bool, text: IceText) -> Res<()> {
        self.displaycmdfile("preedit")?;

        let mut editor = EditState {
            from: self.state.session.user_name.clone(),
            to: to.to_string(),
            subj: subj.to_string(),
            msg: Vec::new(),
            cursor: Position::new(0, 0),
            use_fse: self.state.session.use_fse,
            insert_mode: self.state.session.use_fse,
            top_line: 0,
        };

        match editor.edit_message(&mut self.state)? {
            EditResult::Abort => {}
            EditResult::SendMessage => {
                let msg = editor.msg.join("\n");
                let msg = JamMessage::default()
                    .with_from(BString::from(editor.from.clone()))
                    .with_to(BString::from(editor.to.clone()))
                    .with_subject(BString::from(editor.subj))
                    .with_date_time(Utc::now())
                    .with_text(BString::from(msg));

                self.send_message(conf, area, msg, text)?;
            }
        }
        Ok(())
    }
}
