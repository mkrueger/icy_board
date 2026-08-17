use crate::{
    Res,
    icy_board::{
        conferences::Conference,
        icb_config::IcbColor,
        icb_text::IceText,
        state::{IcyBoardState, functions::display_flags},
        user_base::ConferenceFlags,
    },
    vm::TerminalTarget,
};

#[derive(Clone, Copy, PartialEq)]
pub enum SelectMode {
    Register,
    SelectCmd,
}
const MASK_CONFNUMBERS: &str = "0123456789-DQSH?";
/// PCBoard's mask_crsxn: locked out, registered, scan, conference sysop, net status.
const MASK_CONFFLAGS: &str = "CLRSXN";

fn with_flag(value: ConferenceFlags, flag: ConferenceFlags, on: bool) -> ConferenceFlags {
    if on { value | flag } else { value & !flag }
}

impl IcyBoardState {
    pub async fn select_conferences(&mut self, select_mode: SelectMode) -> Res<()> {
        let divider = "-".repeat(79);
        let num_lines = if self.session.page_len < 4 || self.session.page_len > 50 {
            19
        } else {
            self.session.page_len - 4
        };
        let begin = 0;
        let num_conf = self.board.lock().await.conferences.len();
        let conferences = &self.board.lock().await.conferences.clone();
        let mut done = false;
        while !done {
            if self.session.tokens.is_empty() {
                self.print_header(&divider).await?;
                let mut line_number = 0;
                for x in begin..num_conf {
                    let conference = &conferences[x];
                    if select_mode == SelectMode::SelectCmd && !self.is_registered(conference, x as u16) {
                        continue;
                    }
                    self.print_conference_line(&conference, x, select_mode).await?;
                    line_number += 1;
                }
                for _ in line_number..num_lines {
                    self.new_line().await?;
                }
                self.set_color(TerminalTarget::Both, IcbColor::dos_white()).await?;
                self.println(TerminalTarget::Both, &divider).await?;

                let (txt, help) = match select_mode {
                    SelectMode::SelectCmd => (IceText::ConferenceNumbers, "hlpsel"),
                    SelectMode::Register => (IceText::ConferenceNumbers2, "hlpreg"),
                };

                let text = self
                    .input_field(
                        txt,
                        58,
                        MASK_CONFNUMBERS,
                        help,
                        None,
                        display_flags::ERASELINE | display_flags::STACKED | display_flags::UPCASE,
                    )
                    .await?;
                self.session.push_tokens(&text);
            } else {
                done = true;
            }

            // A pending token answers the next question instead of it being asked, so the
            // numbers are held aside while the flags question is put.
            let pending: Vec<String> = self.session.tokens.drain(..).collect();

            // Registering asks which flags the numbers that follow should get, once per answer.
            let flags = if select_mode == SelectMode::Register && pending.first().is_some_and(|token| token != "Q") {
                self.input_field(
                    IceText::SelectConferenceFlags,
                    5,
                    MASK_CONFFLAGS,
                    "",
                    None,
                    display_flags::NEWLINE | display_flags::FIELDLEN | display_flags::GUIDE | display_flags::UPCASE,
                )
                .await?
            } else {
                String::new()
            };

            for token in pending {
                match token.as_str() {
                    "Q" => {
                        done = true;
                        break;
                    }
                    "S" => {
                        self.apply_selection(0, num_conf, Some(true), select_mode, &flags).await?;
                    }
                    "D" => {
                        self.apply_selection(0, num_conf, Some(false), select_mode, &flags).await?;
                    }
                    _ => {
                        let mut str = token;
                        let value;
                        if str.ends_with('D') {
                            value = Some(false);
                            str.pop();
                        } else if str.ends_with('S') {
                            value = Some(true);
                            str.pop();
                        } else {
                            value = None;
                        }

                        if str.contains('-') {
                            let mut parts = str.split('-');
                            if let (Some(from), Some(to)) = (parts.next(), parts.next()) {
                                if let (Ok(from), Ok(to)) = (from.parse::<usize>(), to.parse::<usize>()) {
                                    self.apply_selection(from, to, value, select_mode, &flags).await?;
                                }
                            }
                        } else {
                            if let Ok(num) = str.parse::<usize>() {
                                self.apply_selection(num, num, value, select_mode, &flags).await?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn print_header(&mut self, divider: &str) -> Res<()> {
        self.clear_screen(TerminalTarget::Both).await?;
        self.display_text(IceText::ConferenceHeader1, display_flags::NEWLINE).await?;
        self.display_text(IceText::ConferenceHeader2, display_flags::NEWLINE).await?;
        self.set_color(TerminalTarget::Both, IcbColor::dos_white()).await?;
        self.println(TerminalTarget::Both, divider).await?;
        self.reset_color(TerminalTarget::Both).await?;
        Ok(())
    }

    async fn print_conference_line(&mut self, conf: &Conference, num: usize, select_mode: SelectMode) -> Res<()> {
        self.reset_color(TerminalTarget::Both).await?;

        let str = format!("{:5}{}", num, ' ');
        self.print(TerminalTarget::Both, &str).await?;

        self.print(TerminalTarget::Both, &conf.name).await?;
        self.set_color(TerminalTarget::Both, IcbColor::dos_dark_gray()).await?;

        for i in conf.name.len()..52 {
            self.print(TerminalTarget::Both, if i % 2 == 0 { " " } else { "." }).await?;
        }
        self.set_color(TerminalTarget::Both, IcbColor::dos_gray()).await?;
        let mut flag_str = String::new();
        if let Some(user) = &self.session.current_user {
            if let Some(flags) = user.conference_flags.get(&num) {
                match select_mode {
                    SelectMode::SelectCmd => {
                        if flags.contains(ConferenceFlags::Selected) {
                            flag_str.push('X');
                        }
                    }
                    SelectMode::Register => {
                        if flags.contains(ConferenceFlags::Registered) {
                            flag_str.push('R');
                            if flags.contains(ConferenceFlags::Expired) {
                                flag_str.push('X');
                            }
                        } else if flags.contains(ConferenceFlags::Expired) {
                            flag_str.push('L');
                        }
                        if flags.contains(ConferenceFlags::Selected) {
                            flag_str.push('S');
                        }

                        if flags.contains(ConferenceFlags::Sysop) {
                            flag_str.push('S');
                        }
                        if flags.contains(ConferenceFlags::NetStatus) {
                            flag_str.push('N');
                        }
                    }
                }
            }
        }
        self.set_color(TerminalTarget::Both, IcbColor::dos_light_cyan()).await?;

        let str = format!(" {:<5}", flag_str);
        self.println(TerminalTarget::Both, &str).await?;
        Ok(())
    }

    async fn apply_selection(&mut self, from: usize, to: usize, set_selection_to: Option<bool>, select_mode: SelectMode, flags: &str) -> Res<()> {
        if select_mode == SelectMode::Register {
            self.apply_conference_flags(from, to, flags).await
        } else {
            self.change_selection(from, to, set_selection_to).await
        }
    }

    /// The answer names the flags a conference should end up with, so a letter that is
    /// absent clears its own flag.
    async fn apply_conference_flags(&mut self, from: usize, to: usize, flags: &str) -> Res<()> {
        let sysop_level = self.get_board().await.config.sysop_command_level.sysop;
        let locked_out = flags.contains('L');
        let registered = flags.contains('R');
        let expired = flags.contains('X');
        let selected = flags.contains('S');
        let conference_sysop = flags.contains('C');
        let net_status = flags.contains('N');

        if let Some(user) = &mut self.session.current_user {
            // The caller's own level decides this, not one raised by conference sysop access.
            let may_set_sysop = user.security_level >= sysop_level;
            for i in from..=to {
                let mut value = *user.conference_flags.get(&i).unwrap_or(&ConferenceFlags::empty());
                if locked_out {
                    value &= !ConferenceFlags::Registered;
                    value |= ConferenceFlags::Expired;
                } else {
                    value = with_flag(value, ConferenceFlags::Registered, registered);
                    value = with_flag(value, ConferenceFlags::Expired, expired);
                    value = with_flag(value, ConferenceFlags::Selected, selected);
                    if may_set_sysop {
                        value = with_flag(value, ConferenceFlags::Sysop, conference_sysop);
                    }
                    value = with_flag(value, ConferenceFlags::NetStatus, net_status);
                }
                if value.is_empty() {
                    user.conference_flags.remove(&i);
                } else {
                    user.conference_flags.insert(i, value);
                }
            }
        }
        Ok(())
    }

    async fn change_selection(&mut self, from: usize, to: usize, set_selection_to: Option<bool>) -> Res<()> {
        if let Some(user) = &mut self.session.current_user {
            for i in from..=to {
                let value = *user.conference_flags.get(&i).unwrap_or(&ConferenceFlags::empty());
                let value = match set_selection_to {
                    Some(true) => value | ConferenceFlags::Selected,
                    Some(false) => value & !ConferenceFlags::Selected,
                    None => value ^ ConferenceFlags::Selected,
                };
                if value.is_empty() {
                    user.conference_flags.remove(&i);
                } else {
                    user.conference_flags.insert(i, value);
                }
            }
        }
        Ok(())
    }
}
