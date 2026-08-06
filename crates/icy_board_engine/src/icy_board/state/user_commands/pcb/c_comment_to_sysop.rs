use crate::icy_board::state::user_commands::mods::editor::{EditResult, EditState};
use crate::{Res, icy_board::state::IcyBoardState};

use crate::icy_board::user_base::FSEMode;
use crate::{
    datetime::IcbTime,
    icy_board::{
        icb_text::IceText,
        state::{GraphicsMode, NodeStatus, functions::display_flags},
    },
};
use bstr::BString;
use chrono::{DateTime, Utc};
use icy_engine::Position;
use jamjam::jam::{JamMessage, attributes, msg_header::MessageSubfield};

impl IcyBoardState {
    pub async fn comment_to_sysop(&mut self) -> Res<()> {
        let leave_comment = self
            .input_field(
                IceText::LeaveComment,
                1,
                "",
                "",
                Some(self.session.no_char.to_string()),
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::FIELDLEN | display_flags::YESNO,
            )
            .await?;

        if leave_comment.is_empty() || leave_comment.chars().next().unwrap() == self.session.no_char {
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
        let mut msg_attributes = attributes::MSG_PRIVATE;
        if receipt == self.session.yes_char.to_uppercase().to_string() {
            msg_attributes |= attributes::MSG_RECEIPTREQ;
        }
        self.write_message(-1, -1, &to, &subj, msg_attributes, None, None, Vec::new(), IceText::SavingComment).await?;

        Ok(())
    }

    pub async fn write_message(
        &mut self,
        conf: i32,
        area: i32,
        to: &str,
        subj: &str,
        attributes: u32,
        password: Option<String>,
        packout_date: Option<DateTime<Utc>>,
        sub_fields: Vec<MessageSubfield>,
        text: IceText,
    ) -> Res<()> {
        self.displaycmdfile("preedit").await?;

        // PCBoard asks whether to use the full screen editor unless the user has a
        // fixed preference (msgeditor()/TXT_USEFULLSCREEN). Skipping this prompt made
        // stuffed answers (e.g. from a PPE) leak into the message body.
        let use_fse = match self.session.fse_mode {
            FSEMode::Yes => true,
            FSEMode::No => false,
            FSEMode::Ask => self.prompt_use_fse().await?,
        };

        let mut editor = EditState {
            from: self.session.user_name.clone(),
            to: to.to_string(),
            subj: subj.to_string(),
            msg: Vec::new(),
            cursor: Position::new(0, 0),
            use_fse,
            insert_mode: use_fse,
            top_line: 0,
            max_line_length: 79,
        };

        match editor.edit_message(self).await? {
            EditResult::Abort => {}
            EditResult::SendMessage => {
                let msg = editor.msg.join("\n");
                let mut msg = JamMessage::default()
                    .with_from(BString::from(editor.from.clone()))
                    .with_to(BString::from(editor.to.clone()))
                    .with_subject(BString::from(editor.subj))
                    .with_date_time(Utc::now())
                    .with_attributes(attributes)
                    .with_text(BString::from(msg));

                if let Some(password) = &password {
                    msg = msg.with_password(&BString::from(password.clone()));
                }
                if let Some(packout_date) = packout_date {
                    msg = msg.with_packout_date(packout_date);
                }
                for sub_field in sub_fields {
                    msg = msg.with_sub_field(sub_field);
                }

                self.send_message(conf, area, msg, text).await?;
            }
        }
        Ok(())
    }

    /// Asks the user whether to use the full screen editor (mirrors PCBoard's
    /// msgeditor() TXT_USEFULLSCREEN prompt). Only called when the user's editor
    /// preference is "ask".
    async fn prompt_use_fse(&mut self) -> Res<bool> {
        let ansi = self.session.disp_options.grapics_mode != GraphicsMode::Ctty;
        let default = if self.session.expert_mode() && ansi { self.session.yes_char } else { self.session.no_char };
        let mut answer = self
            .input_field(
                IceText::UseFullScreen,
                1,
                "",
                "",
                Some(default.to_string()),
                display_flags::YESNO | display_flags::NEWLINE | display_flags::UPCASE | display_flags::FIELDLEN,
            )
            .await?;

        // The full screen editor requires ANSI - re-ask if selected without it.
        let yes = self.session.yes_char.to_uppercase().to_string();
        if answer == yes && !ansi {
            self.display_text(
                IceText::RequiresAnsi,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
            )
            .await?;
            answer = self
                .input_field(
                    IceText::UseFullScreen,
                    1,
                    "",
                    "",
                    Some(self.session.no_char.to_string()),
                    display_flags::YESNO | display_flags::NEWLINE | display_flags::UPCASE | display_flags::FIELDLEN,
                )
                .await?;
        }
        Ok(answer == yes)
    }
}
