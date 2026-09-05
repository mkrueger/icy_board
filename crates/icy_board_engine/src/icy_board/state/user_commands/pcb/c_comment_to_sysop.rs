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

fn message_text(lines: &[String], allow_esc_codes: bool) -> String {
    let mut text = lines.join("\n");
    if !allow_esc_codes {
        text.retain(|ch| ch != '\u{1b}' && ch != '\u{1d}');
    }
    text
}

fn make_message(
    editor: &EditState,
    recipient: &str,
    text: &str,
    attributes: u32,
    password: &Option<String>,
    packout_date: Option<DateTime<Utc>>,
    sub_fields: &[MessageSubfield],
) -> JamMessage {
    let mut msg = JamMessage::default()
        .with_from(BString::from(editor.from.clone()))
        .with_to(BString::from(recipient))
        .with_subject(BString::from(editor.subj.clone()))
        .with_date_time(Utc::now())
        // The mark is what tells the scanner this message was written here and
        // has yet to go out, rather than having come in from the network.
        .with_attributes(attributes | jamjam::jam::attributes::MSG_LOCAL)
        .with_text(BString::from(text));
    if let Some(password) = password {
        msg = msg.with_password(&BString::from(password.clone()));
    }
    if let Some(packout_date) = packout_date {
        msg = msg.with_packout_date(packout_date);
    }
    for sub_field in sub_fields {
        msg = msg.with_sub_field(sub_field.clone());
    }
    msg
}

impl IcyBoardState {
    /// A negative conference number selects the recipient's mailbox instead of a message area.
    async fn comment_target(&mut self) -> (i32, i32) {
        if self.get_board().await.config.message.force_comments_to_main {
            (0, 0)
        } else {
            (-1, 0)
        }
    }

    pub async fn password_failure_comment(&mut self) -> Res<()> {
        let answer = self
            .input_field(
                IceText::WrongPasswordComment,
                1,
                "",
                "",
                Some(self.session.no_char.to_string()),
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::FIELDLEN | display_flags::YESNO,
            )
            .await?;
        if answer != self.session.yes_char.to_uppercase().to_string() {
            return Ok(());
        }

        let to = self.get_board().await.config.sysop.name.clone();
        let subject = self.get_display_text(IceText::WrongPasswordSubject)?;
        let (conf, area) = self.comment_target().await;
        self.write_message(
            conf,
            area,
            &to,
            subject.trim(),
            attributes::MSG_PRIVATE,
            None,
            None,
            Vec::new(),
            IceText::SavingComment,
        )
        .await
    }

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
        }

        self.enter_comment_to_sysop().await?;

        Ok(())
    }

    pub async fn enter_comment_to_sysop(&mut self) -> Res<()> {
        let to = self.get_board().await.config.sysop.name.clone();
        let (conf, area) = self.comment_target().await;
        let subj = format!("COMMENT {}", IcbTime::now());
        let receipt = self
            .input_field(
                IceText::RequireReturnReceipt,
                1,
                "",
                "",
                Some(self.session.no_char.to_string()),
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::YESNO | display_flags::FIELDLEN,
            )
            .await?;
        self.set_activity(NodeStatus::HandlingMail).await;
        let mut msg_attributes = attributes::MSG_PRIVATE;
        if receipt == self.session.yes_char.to_uppercase().to_string() {
            msg_attributes |= attributes::MSG_RECEIPTREQ;
        }
        self.write_message(conf, area, &to, &subj, msg_attributes, None, None, Vec::new(), IceText::SavingComment)
            .await?;

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
            max_lines: self.get_board().await.config.message.max_msg_lines.max(1) as usize,
        };

        match editor.edit_message(self).await? {
            EditResult::Abort => {}
            result @ (EditResult::SendMessage | EditResult::CarbonCopy) => {
                let msg = message_text(&editor.msg, self.get_board().await.config.message.allow_esc_codes);
                let original = make_message(&editor, &editor.to, &msg, attributes, &password, packout_date, &sub_fields);
                self.send_message(conf, area, original, text).await?;

                if matches!(result, EditResult::CarbonCopy)
                    && !matches!(text, IceText::SavingComment)
                    && self.get_board().await.config.message.allow_carbon_copy
                {
                    while let Some(recipient) = self.get_message_recipient(IceText::CarbonCopyTo, String::new(), true).await? {
                        let copy = make_message(&editor, &recipient, &msg, attributes, &password, packout_date, &sub_fields);
                        self.send_message(conf, area, copy, text).await?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Asks the user whether to use the full screen editor (mirrors `PCBoard`'s
    /// `msgeditor()` `TXT_USEFULLSCREEN` prompt). Only called when the user's editor
    /// preference is "ask".
    async fn prompt_use_fse(&mut self) -> Res<bool> {
        let ansi = self.session.disp_options.grapics_mode != GraphicsMode::Ctty;
        let default = if self.session.expert_mode() && ansi {
            self.session.yes_char
        } else {
            self.session.no_char
        };
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
            self.display_text(IceText::RequiresAnsi, display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER)
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

#[cfg(test)]
mod tests {
    use super::message_text;

    #[test]
    fn escape_codes_are_removed_when_disabled() {
        assert_eq!(message_text(&["A\u{1b}[31mB\u{1d}C".to_string()], false), "A[31mBC");
    }

    #[test]
    fn escape_codes_are_kept_when_enabled() {
        assert_eq!(message_text(&["A\u{1b}[31mB\u{1d}C".to_string()], true), "A\u{1b}[31mB\u{1d}C");
    }
}
