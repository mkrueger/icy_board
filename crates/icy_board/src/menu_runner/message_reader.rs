use crate::Res;
use icy_board_engine::{
    icy_board::{
        icb_text::{IceText, TextEntry},
        state::{functions::display_flags, IcyBoardState},
    },
    vm::TerminalTarget,
};
use jamjam::jam::{msg_header::JamMessageHeader, JamMessageBase};

use super::{PcbBoardCommand, MASK_COMMAND};

pub struct MessageViewer {
    date_num: TextEntry,
    to_line: TextEntry,
    _reply_line: TextEntry,
    from_line: TextEntry,
    _not_avail: TextEntry,
    _not_read: TextEntry,
    _rcv_only: TextEntry,
    _grp_pwd: TextEntry,
    _snd_pwd: TextEntry,
    _public: TextEntry,
    refer_num: TextEntry,
    _read: TextEntry,
    subj_line: TextEntry,
    _status: TextEntry,
    _file: TextEntry,
    _list: TextEntry,
    none: TextEntry,
    confarea: TextEntry,
    separator: TextEntry,
    _all_name: TextEntry,
    _ret_rec_name: TextEntry,
    _comment: TextEntry,
    _echo: TextEntry,
    _all_conf_indicator: TextEntry,
    _read_only_indicator: TextEntry,
    left: usize,
    right: usize,
}

impl MessageViewer {
    pub fn load(dt: &icy_board_engine::icy_board::icb_text::IcbTextFile) -> Res<Self> {
        let date_num = dt.get_display_text(IceText::MessageDateNumber)?;
        let to_line = dt.get_display_text(IceText::MessageToLine)?;
        let reply_line = dt.get_display_text(IceText::MessageReplies)?;
        let from_line = dt.get_display_text(IceText::MessageFrom)?;
        let not_avail = dt.get_display_text(IceText::MessageNA)?;
        let not_read = dt.get_display_text(IceText::MessageNotRead)?;
        let rcv_only = dt.get_display_text(IceText::MessageReceiverOnly)?;
        let grp_pwd = dt.get_display_text(IceText::MessageGroupPassword)?;
        let snd_pwd = dt.get_display_text(IceText::MessageSenderPassword)?;
        let public = dt.get_display_text(IceText::MessagePublic)?;
        let refer_num = dt.get_display_text(IceText::MessageReferNumber)?;
        let read = dt.get_display_text(IceText::MessageRead)?;
        let subj_line = dt.get_display_text(IceText::MessageSubjectLine)?;
        let status = dt.get_display_text(IceText::MessageStatus)?;
        let file = dt.get_display_text(IceText::MessageFile)?;
        let list = dt.get_display_text(IceText::MessageList)?;
        let none = dt.get_display_text(IceText::None)?;
        let confarea = dt.get_display_text(IceText::MessagesConfArea)?;
        let separator = dt.get_display_text(IceText::Separator)?;
        let all_name = dt.get_display_text(IceText::AllName)?;
        let ret_rec_name = dt.get_display_text(IceText::FromReturnReceipt)?;
        let comment = dt.get_display_text(IceText::Comment)?;
        let echo = dt.get_display_text(IceText::Echo)?;
        let all_conf_indicator = dt.get_display_text(IceText::AllConfIndicator)?;
        let read_only_indicator = dt.get_display_text(IceText::ReadonlyIndicator)?;

        let mut left = date_num.text.find(':').unwrap_or(0);
        left = left.max(to_line.text.find(':').unwrap_or(0));
        left = left.max(from_line.text.find(':').unwrap_or(0));
        left = left.max(subj_line.text.find(':').unwrap_or(0));
        left = left.max(confarea.text.find(':').unwrap_or(0));

        let right = confarea.text.rfind(':').unwrap_or(79);

        Ok(Self {
            date_num,
            to_line,
            _reply_line: reply_line,
            from_line,
            _not_avail: not_avail,
            _not_read: not_read,
            _rcv_only: rcv_only,
            _grp_pwd: grp_pwd,
            _snd_pwd: snd_pwd,
            _public: public,
            refer_num,
            _read: read,
            subj_line,
            _status: status,
            _file: file,
            _list: list,
            none,
            confarea,
            separator,
            _all_name: all_name,
            _ret_rec_name: ret_rec_name,
            _comment: comment,
            _echo: echo,
            _all_conf_indicator: all_conf_indicator,
            _read_only_indicator: read_only_indicator,
            left,
            right,
        })
    }

    pub fn format_hdr_text(&self, txt: &str, left: &str, right: &str) -> String {
        let mut result = txt.to_string();

        while result.len() < 80 {
            result.push(' ');
        }

        let xleft = self.left + 2;
        let xright = self.right + 2;

        result.replace_range(xleft..xleft + left.len(), left);
        result.replace_range(xright..xright + right.len(), right);

        result
    }

    pub async fn display_header(&self, state: &mut IcyBoardState, msg_base: &JamMessageBase, header: &JamMessageHeader) -> Res<()> {
        state.clear_screen(TerminalTarget::Both).await?;

        let c1 = state.get_board().await.config.color_configuration.msg_hdr_date.clone();
        state.set_color(TerminalTarget::Both, c1).await?;
        let time = if let Some(dt) = chrono::DateTime::from_timestamp(header.date_written as i64, 0) {
            dt.to_string()
        } else {
            String::new()
        };
        let msg_counter = format!(
            "{} {} {} ({} {})",
            header.message_number,
            self.separator.text,
            msg_base.active_messages(),
            self.refer_num.text,
            if header.reply_to == 0 {
                self.none.text.clone()
            } else {
                header.reply_to.to_string()
            }
        );
        let txt = self.format_hdr_text(&self.date_num.text, &time, &msg_counter);
        state.print(TerminalTarget::Both, &txt).await?;

        let c1 = state.get_board().await.config.color_configuration.msg_hdr_to.clone();
        state.set_color(TerminalTarget::Both, c1).await?;
        let txt = self.format_hdr_text(&self.to_line.text, &header.get_to().unwrap().to_string(), "");
        state.print(TerminalTarget::Both, &txt).await?;

        let c1 = state.get_board().await.config.color_configuration.msg_hdr_from.clone();
        state.set_color(TerminalTarget::Both, c1).await?;
        let txt = self.format_hdr_text(&self.from_line.text, &header.get_from().unwrap().to_string(), "");
        state.print(TerminalTarget::Both, &txt).await?;

        let c1 = state.get_board().await.config.color_configuration.msg_hdr_subj.clone();
        state.set_color(TerminalTarget::Both, c1).await?;
        let txt = self.format_hdr_text(&self.subj_line.text, &header.get_subject().unwrap().to_string(), "");
        state.print(TerminalTarget::Both, &txt).await?;

        let c1 = state.get_board().await.config.color_configuration.msg_hdr_read.clone();
        state.set_color(TerminalTarget::Both, c1).await?;
        /*        let txt = self.format_hdr_text(&self.read.text, "", "");
                state.print(TerminalTarget::Both, &txt)?;
        */

        let area = state.session.current_message_area;
        let txt = self.format_hdr_text(
            &self.confarea.text,
            &state.session.current_conference.name,
            &state.session.current_conference.areas[area].name,
        );
        state.print(TerminalTarget::Both, &txt).await?;
        state.reset_color(TerminalTarget::Both).await?;

        Ok(())
    }

    async fn display_body(&self, state: &mut IcyBoardState, text: &str) -> Res<()> {
        state.print(TerminalTarget::Both, text).await
    }
}

impl PcbBoardCommand {
    pub async fn read_msgs_from_base(&mut self, message_base: JamMessageBase, help: &str) -> Res<()> {
        let viewer = MessageViewer::load(&self.state.display_text)?;

        while !self.state.session.disp_options.abort_printout {
            let prompt = if self.state.session.expert_mode {
                IceText::MessageReadCommandExpert
            } else {
                IceText::MessageReadCommand
            };
            self.state.session.op_text = format!("{}-{}", message_base.base_messagenumber(), message_base.active_messages());

            let text = self
                .state
                .input_field(
                    prompt,
                    40,
                    MASK_COMMAND,
                    help,
                    None,
                    display_flags::UPCASE | display_flags::NEWLINE | display_flags::NEWLINE,
                )
                .await?;
            if text.is_empty() {
                break;
            }

            if let Ok(number) = text.parse::<u32>() {
                self.read_message_number(&message_base, &viewer, number, None).await?;
            }
        }
        self.state.press_enter().await?;
        self.display_menu = true;
        Ok(())
    }

    pub async fn read_message_number(
        &mut self,
        message_base: &JamMessageBase,
        viewer: &MessageViewer,
        mut number: u32,
        matches: Option<Vec<(usize, usize)>>,
    ) -> Res<()> {
        if number == 0 {
            return Ok(());
        }
        loop {
            match message_base.read_header(number) {
                Ok(header) => {
                    let mut text = message_base.read_msg_text(&header)?.to_string();
                    viewer.display_header(&mut self.state, message_base, &header).await?;
                    if let Some(matches) = &matches {
                        let mut new_text = String::new();
                        let mut last = 0;
                        for (start, end) in matches {
                            new_text.push_str(&text[last..*start]);
                            new_text.push_str("@X70");
                            new_text.push_str(&text[*start..*end]);
                            new_text.push_str("@X07");
                            last = *end;
                        }
                        new_text.push_str(&text[last..]);
                        text = new_text;
                    }
                    if header.needs_password() {
                        if self
                            .state
                            .check_password(IceText::PasswordToReadMessage, 0, |pwd| header.is_password_valid(pwd))
                            .await?
                        {
                            viewer.display_body(&mut self.state, &text).await?;
                        }
                    } else {
                        viewer.display_body(&mut self.state, &text).await?;
                    }
                    self.state.new_line().await?;
                }
                Err(err) => {
                    log::error!("Error reading message header: {}", err);
                    self.state
                        .display_text(IceText::NoMailFound, display_flags::NEWLINE | display_flags::LFAFTER)
                        .await?;
                }
            }
            let prompt = if self.state.session.expert_mode {
                IceText::EndOfMessageExpertmode
            } else {
                IceText::EndOfMessage
            };
            let text = self
                .state
                .input_field(
                    prompt,
                    40,
                    MASK_COMMAND,
                    "hlpendr", // TODO: Hard coded help flag!
                    None,
                    display_flags::UPCASE | display_flags::LFBEFORE | display_flags::NEWLINE,
                )
                .await?;
            if text.is_empty() {
                break;
            }
            if let Ok(new_number) = text.parse::<u32>() {
                number = new_number;
                continue;
            }
        }

        Ok(())
    }
}
