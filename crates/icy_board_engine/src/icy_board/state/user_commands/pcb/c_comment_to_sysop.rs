use crate::icy_board::state::user_commands::mods::editor::{EditResult, EditState};
use crate::{icy_board::state::IcyBoardState, Res};

use crate::icy_board::user_base::FSEMode;
use crate::{
    datetime::IcbTime,
    icy_board::{
        icb_text::IceText,
        state::{functions::display_flags, NodeStatus},
    },
};
use bstr::BString;
use chrono::Utc;
use icy_engine::Position;
use jamjam::jam::JamMessage;

impl IcyBoardState {
    pub async fn comment_to_sysop(&mut self, help: &str) -> Res<()> {
        let leave_comment = self
            .input_field(
                IceText::LeaveComment,
                1,
                "",
                help,
                Some(self.session.no_char.to_string()),
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::FIELDLEN | display_flags::YESNO,
            )
            .await?;

        if leave_comment.is_empty() || leave_comment.chars().next().unwrap() == self.session.no_char {
            self.new_line().await?;
            self.press_enter().await?;
            self.display_current_menu = true;

            return Ok(());
        };

        self.enter_comment_to_sysop().await?;

        Ok(())
    }

    pub async fn enter_comment_to_sysop(&mut self) -> Res<()> {
        let to = self.get_board().await.config.sysop.name.clone();
        let subj = format!("COMMENT {}", IcbTime::now().to_string());
        let receipt = self
            .input_field(
                IceText::RequireReturnReceipt,
                1,
                "",
                &"",
                Some(self.session.no_char.to_string()),
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::YESNO | display_flags::FIELDLEN,
            )
            .await?;
        self.set_activity(NodeStatus::HandlingMail).await;
        self.write_message(
            -1,
            -1,
            &to,
            &subj,
            receipt == self.session.yes_char.to_uppercase().to_string(),
            IceText::SavingComment,
        )
        .await?;

        self.press_enter().await?;
        self.display_current_menu = true;
        Ok(())
    }

    pub async fn write_message(&mut self, conf: i32, area: i32, to: &str, subj: &str, _ret_receipt: bool, text: IceText) -> Res<()> {
        self.displaycmdfile("preedit").await?;

        let mut editor = EditState {
            from: self.session.user_name.clone(),
            to: to.to_string(),
            subj: subj.to_string(),
            msg: Vec::new(),
            cursor: Position::new(0, 0),
            use_fse: self.session.fse_mode == FSEMode::Yes,
            insert_mode: self.session.fse_mode == FSEMode::Yes,
            top_line: 0,
            max_line_length: 79,
        };

        match editor.edit_message(self).await? {
            EditResult::Abort => {}
            EditResult::SendMessage => {
                let msg = editor.msg.join("\n");
                let msg = JamMessage::default()
                    .with_from(BString::from(editor.from.clone()))
                    .with_to(BString::from(editor.to.clone()))
                    .with_subject(BString::from(editor.subj))
                    .with_date_time(Utc::now())
                    .with_text(BString::from(msg));

                self.send_message(conf, area, msg, text).await?;
            }
        }
        Ok(())
    }
}
