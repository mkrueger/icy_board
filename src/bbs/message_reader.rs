use icy_board_engine::{
    icy_board::{
        commands::Command,
        icb_text::{IceText, TextEntry},
        state::{functions::display_flags, IcyBoardState},
    },
    vm::TerminalTarget,
};
use icy_ppe::Res;
use jamjam::jam::{msg_header::JamMessageHeader, JamMessageBase};

use super::{PcbBoardCommand, MASK_COMMAND};

struct MessageViewer {
    date_num: TextEntry,
    to_line: TextEntry,
    reply_line: TextEntry,
    from_line: TextEntry,
    not_avail: TextEntry,
    not_read: TextEntry,
    rcv_only: TextEntry,
    grp_pwd: TextEntry,
    snd_pwd: TextEntry,
    public: TextEntry,
    refer_num: TextEntry,
    read: TextEntry,
    subj_line: TextEntry,
    status: TextEntry,
    file: TextEntry,
    list: TextEntry,
    none: TextEntry,
    confarea: TextEntry,
    separator: TextEntry,
    all_name: TextEntry,
    ret_rec_name: TextEntry,
    comment: TextEntry,
    echo: TextEntry,
    all_conf_indicator: TextEntry,
    read_only_indicator: TextEntry,
    left: usize,
    right: usize,
}

impl MessageViewer {
    fn load(dt: &icy_board_engine::icy_board::icb_text::IcbTextFile) -> Res<Self> {
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
            reply_line,
            from_line,
            not_avail,
            not_read,
            rcv_only,
            grp_pwd,
            snd_pwd,
            public,
            refer_num,
            read,
            subj_line,
            status,
            file,
            list,
            none,
            confarea,
            separator,
            all_name,
            ret_rec_name,
            comment,
            echo,
            all_conf_indicator,
            read_only_indicator,
            left,
            right,
        })
    }

    pub fn format_hdr_text(&self, txt: &str, left: &str, right: &str) -> String {
        let mut result = txt.to_string();

        while result.len() < 80 {
            result.push(' ');
        }

        let xleft = self.left + 3;
        let xright = self.right + 3;

        result.replace_range(xleft..xleft + left.len(), left);
        result.replace_range(xright..xright + right.len(), right);

        result
    }

    pub fn display_header(
        &self,
        state: &mut IcyBoardState,
        msg_base: &JamMessageBase,
        header: &JamMessageHeader,
    ) -> Res<()> {
        state.clear_screen()?;

        let c1 = state
            .board
            .lock()
            .unwrap()
            .config
            .color_configuration
            .msg_hdr_date
            .clone();
        state.set_color(c1)?;
        let time = if let Some(dt) = chrono::DateTime::from_timestamp(header.date_written as i64, 0)
        {
            dt.to_string()
        } else {
            String::new()
        };
        let msg_counter = format!(
            "{} {} {}",
            header.message_number,
            self.separator.text,
            msg_base.active_messages()
        );
        let txt = self.format_hdr_text(&self.date_num.text, &time, &msg_counter);
        state.print(TerminalTarget::Both, &txt)?;

        let c1 = state
            .board
            .lock()
            .unwrap()
            .config
            .color_configuration
            .msg_hdr_to
            .clone();
        state.set_color(c1)?;
        let txt = self.format_hdr_text(&self.to_line.text, &header.get_to().unwrap(), "");
        state.print(TerminalTarget::Both, &txt)?;

        let c1 = state
            .board
            .lock()
            .unwrap()
            .config
            .color_configuration
            .msg_hdr_from
            .clone();
        state.set_color(c1)?;
        let txt = self.format_hdr_text(&self.from_line.text, &header.get_from().unwrap(), "");
        state.print(TerminalTarget::Both, &txt)?;

        let c1 = state
            .board
            .lock()
            .unwrap()
            .config
            .color_configuration
            .msg_hdr_subj
            .clone();
        state.set_color(c1)?;
        let txt = self.format_hdr_text(&self.subj_line.text, &header.get_subject().unwrap(), "");
        state.print(TerminalTarget::Both, &txt)?;

        let c1 = state
            .board
            .lock()
            .unwrap()
            .config
            .color_configuration
            .msg_hdr_read
            .clone();
        state.set_color(c1)?;
        /*        let txt = self.format_hdr_text(&self.read.text, "", "");
                state.print(TerminalTarget::Both, &txt)?;
        */
        let area = state.session.current_message_area;
        let txt = self.format_hdr_text(
            &self.confarea.text,
            &state.session.current_conference.name,
            &state.session.current_conference.message_areas[area].name,
        );
        state.print(TerminalTarget::Both, &txt)?;
        state.reset_color()?;

        Ok(())
    }

    fn display_body(&self, state: &mut IcyBoardState, text: &str) -> Res<()> {
        state.print(TerminalTarget::Both, text)
    }
}

impl PcbBoardCommand {
    pub fn read_messages(&mut self, action: &Command) -> Res<()> {
        let viewer = MessageViewer::load(&self.state.board.lock().unwrap().display_text)?;

        let message_base_file = &self.state.session.current_conference.message_areas[0].filename;
        let msgbase_file_resolved = self
            .state
            .board
            .lock()
            .unwrap()
            .resolve_file(message_base_file);

        match JamMessageBase::open(&msgbase_file_resolved) {
            Ok(message_base) => {
                while !self.state.session.disp_options.abort_printout {
                    let prompt = if self.state.session.expert_mode {
                        IceText::MessageReadCommandExpert
                    } else {
                        IceText::MessageReadCommand
                    };
                    self.state.session.op_text = format!(
                        "{}-{}",
                        message_base.base_messagenumber(),
                        message_base.active_messages()
                    );

                    let text = self.state.input_field(
                        prompt,
                        40,
                        MASK_COMMAND,
                        &action.help,
                        display_flags::UPCASE | display_flags::NEWLINE | display_flags::NEWLINE,
                    )?;
                    if text.is_empty() {
                        break;
                    }

                    if let Ok(number) = text.parse::<u32>() {
                        if number == 0 {
                            continue;
                        }
                        match message_base.read_header(number) {
                            Ok(header) => {
                                let text = message_base.read_msg_text(&header)?;
                                viewer.display_header(&mut self.state, &message_base, &header)?;

                                if header.needs_password() {
                                    if self
                                        .state
                                        .check_password(IceText::PasswordToReadMessage, |pwd| {
                                            header.is_password_valid(pwd)
                                        })?
                                    {
                                        viewer.display_body(&mut self.state, &text)?;
                                    }
                                } else {
                                    viewer.display_body(&mut self.state, &text)?;
                                }
                            }
                            Err(err) => {
                                log::error!("Error reading message header: {}", err);
                                self.state.display_text(
                                    IceText::NoMailFound,
                                    display_flags::NEWLINE | display_flags::LFAFTER,
                                )?;
                            }
                        }
                    }
                }
                self.state.press_enter()?;
                self.display_menu = true;
                Ok(())
            }
            Err(err) => {
                log::error!("Message index load error {}", err);
                log::error!("Creating new message index at {}", &msgbase_file_resolved);
                self.state.display_text(
                    IceText::CreatingNewMessageIndex,
                    display_flags::NEWLINE | display_flags::LFAFTER,
                )?;
                if JamMessageBase::create(msgbase_file_resolved).is_ok() {
                    log::error!("successfully created new message index.");
                    return self.read_messages(action);
                }
                log::error!("failed to create message index.");

                self.state.display_text(
                    IceText::PathErrorInSystemConfiguration,
                    display_flags::NEWLINE | display_flags::LFAFTER,
                )?;

                self.state.press_enter()?;
                self.display_menu = true;
                Ok(())
            }
        }
    }
}
