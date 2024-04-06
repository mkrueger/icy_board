use icy_board_engine::{
    icy_board::{
        commands::{Command, CommandType},
        icb_text::IceText,
        state::{functions::display_flags, IcyBoardState},
        IcyBoardError,
    },
    vm::TerminalTarget,
};
use icy_ppe::Res;
mod delete_message;
mod message_reader;
mod recover_message;
mod show_bulletins;
mod take_survey;

pub struct PcbBoardCommand {
    pub state: IcyBoardState,
    pub display_menu: bool,
}
const MASK_COMMAND: &str  = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+-=[]{}|;':,.<>?/\\\" ";
const MASK_NUMBER: &str = "0123456789";

impl PcbBoardCommand {
    pub fn new(state: IcyBoardState) -> Self {
        Self {
            state,
            display_menu: true,
        }
    }

    pub fn do_command(&mut self) -> Res<()> {
        if self.display_menu && !self.state.session.expert_mode {
            self.display_menu()?;
            self.display_menu = false;
        }

        let command = self.state.input_field(
            IceText::CommandPrompt,
            40,
            MASK_COMMAND,
            "",
            display_flags::UPCASE,
        )?;

        let mut cmds = command.split(' ');

        let command = cmds.next().unwrap_or_default();
        for cmd in cmds {
            self.state.session.tokens.push_back(cmd.to_string());
        }

        if let Some(action) = self.state.try_find_command(command) {
            return self.dispatch_action(command, &action);
        }

        self.state.display_text(
            IceText::InvalidEntry,
            display_flags::NEWLINE | display_flags::LFAFTER | display_flags::LFBEFORE,
        )?;
        Ok(())
    }

    fn display_menu(&mut self) -> Res<()> {
        let current_conference = self.state.session.current_conference_number;

        let menu_file = if let Some(conference) = self
            .state
            .board
            .lock()
            .as_ref()
            .unwrap()
            .conferences
            .get(current_conference as usize)
        {
            if self.state.session.is_sysop {
                &conference.sysop_menu
            } else {
                &conference.users_menu
            }
            .clone()
        } else {
            return Ok(());
        };
        self.state.display_file(&menu_file)?;
        Ok(())
    }

    fn dispatch_action(&mut self, command: &str, action: &Command) -> Res<()> {
        if !self.check_sec(command, action.security)? {
            return Ok(());
        }

        match action.command_type {
            CommandType::AbandonConference => {
                self.abandon_conference()?;
            }
            CommandType::BulletinList => {
                self.show_bulletins(action)?;
            }
            CommandType::Goodbye => {
                self.state
                    .hangup(icy_board_engine::vm::HangupType::Goodbye)?;
            }
            CommandType::Help => {
                self.show_help()?;
            }
            CommandType::JoinConference => {
                self.join_conference(action)?;
            }
            CommandType::ShowMenu => {
                self.display_menu()?;
                self.display_menu = false;
            }
            CommandType::ToggleGraphics => {
                self.toggle_graphics()?;
            }
            CommandType::SetPageLength => {
                self.set_page_len(action)?;
            }
            CommandType::ExpertMode => {
                self.set_expert_mode()?;
            }
            CommandType::UserList => {
                self.show_user_list(action)?;
            }
            CommandType::ReadMessages => {
                self.read_messages(action)?;
            }

            CommandType::Survey => {
                self.take_survey(action)?;
            }

            CommandType::DeleteMessage => {
                self.delete_message(action)?;
            }
            CommandType::RestoreMessage => {
                self.restore_message(action)?;
            }
            _ => {
                return Err(Box::new(IcyBoardError::UnknownAction(format!(
                    "{:?}",
                    action.command_type
                ))));
            }
        }
        Ok(())
    }

    fn abandon_conference(&mut self) -> Res<()> {
        if self.state.board.lock().unwrap().conferences.is_empty() {
            self.state.display_text(
                IceText::NoConferenceAvailable,
                display_flags::NEWLINE | display_flags::LFBEFORE,
            )?;
            self.state.press_enter()?;
            return Ok(());
        }
        if self.state.session.current_conference_number > 0 {
            self.state.session.op_text = format!(
                "{} ({})",
                self.state.session.current_conference.name,
                self.state.session.current_conference_number
            );
            self.state.join_conference(0);
            self.state.display_text(
                IceText::ConferenceAbandoned,
                display_flags::NEWLINE | display_flags::NOTBLANK,
            )?;
            self.state.new_line()?;
            self.state.press_enter()?;
        }
        self.display_menu = true;
        Ok(())
    }

    fn join_conference(&mut self, action: &Command) -> Res<()> {
        if self.state.board.lock().unwrap().conferences.is_empty() {
            self.state.display_text(
                IceText::NoConferenceAvailable,
                display_flags::NEWLINE | display_flags::LFBEFORE,
            )?;
            self.state.press_enter()?;
            return Ok(());
        }
        let conf_number = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            let mnu = self
                .state
                .board
                .lock()
                .unwrap()
                .config
                .paths
                .conf_join_menu
                .clone();
            self.state.display_menu(&mnu)?;
            self.state.new_line()?;

            self.state.input_field(
                IceText::JoinConferenceNumber,
                40,
                MASK_COMMAND,
                &action.help,
                display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
            )?
        };
        let mut joined = false;
        if let Ok(number) = conf_number.parse::<i32>() {
            if 0 <= number
                && (number as usize) <= self.state.board.lock().unwrap().conferences.len()
            {
                self.state.join_conference(number);
                self.state.session.op_text = format!(
                    "{} ({})",
                    self.state.session.current_conference.name,
                    self.state.session.current_conference_number
                );
                self.state.display_text(
                    IceText::ConferenceJoined,
                    display_flags::NEWLINE | display_flags::NOTBLANK,
                )?;

                joined = true;
            }
        }

        if !joined {
            self.state.session.op_text = conf_number;
            self.state.display_text(
                IceText::InvalidConferenceNumber,
                display_flags::NEWLINE | display_flags::NOTBLANK,
            )?;
        }

        self.state.new_line()?;
        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }

    fn show_user_list(&mut self, action: &Command) -> Res<()> {
        self.state.new_line()?;
        let text = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            self.state.input_field(
                IceText::UserScan,
                40,
                MASK_COMMAND,
                &action.help,
                display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
            )?
        };

        self.state.display_text(
            IceText::UsersHeader,
            display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::NOTBLANK,
        )?;
        self.state
            .display_text(IceText::UserScanLine, display_flags::NOTBLANK)?;
        self.state.reset_color()?;
        let mut output = String::new();
        for u in self.state.board.lock().unwrap().users.iter() {
            if text.is_empty()
                || u.get_name()
                    .to_ascii_uppercase()
                    .contains(&text.to_ascii_uppercase())
            {
                output.push_str(&format!(
                    "{:<24} {:<30} {} {}\r\n",
                    u.get_name(),
                    u.city,
                    u.last_date_on.to_country_date(),
                    u.last_time_on
                ));
            }
        }
        self.state.print(TerminalTarget::Both, &output)?;

        self.state.new_line()?;
        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }

    fn set_expert_mode(&mut self) -> Res<()> {
        let mut expert_mode = !self.state.session.expert_mode;
        if let Some(token) = self.state.session.tokens.pop_front() {
            let token = token.to_ascii_uppercase();
            if token == "ON" {
                expert_mode = true;
            } else if token == "OFF" {
                expert_mode = false;
            }
        }
        self.state.session.expert_mode = expert_mode;
        if let Some(user) = &mut self.state.current_user {
            user.flags.expert_mode = expert_mode;
        }
        if expert_mode {
            self.state.display_text(
                IceText::ExpertmodeModeOn,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
            )?;
        } else {
            self.state.display_text(
                IceText::ExpertmodeModeOff,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
            )?;
            self.state.press_enter()?;
        }
        self.display_menu = true;
        Ok(())
    }

    fn toggle_graphics(&mut self) -> Res<()> {
        if false
        /*self.state.board.lock().unwrap().data..non_graphics*/
        {
            self.state.display_text(
                IceText::GraphicsUnavailable,
                display_flags::NEWLINE | display_flags::LFBEFORE,
            )?;
        } else {
            if !self.state.session.disp_options.disable_color {
                self.state.reset_color()?;
            }

            self.state.session.disp_options.disable_color =
                !self.state.session.disp_options.disable_color;

            let msg = if self.state.session.disp_options.disable_color {
                IceText::GraphicsOff
            } else {
                IceText::GraphicsOn
            };
            self.state
                .display_text(msg, display_flags::NEWLINE | display_flags::LFBEFORE)?;
        }
        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }

    fn set_page_len(&mut self, action: &Command) -> Res<()> {
        let page_len = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            self.state
                .display_text(IceText::CurrentPageLength, display_flags::LFBEFORE)?;
            self.state.print(
                TerminalTarget::Both,
                &format!(" {}\r\n", self.state.session.page_len),
            )?;
            self.state.input_field(
                IceText::EnterPageLength,
                2,
                MASK_NUMBER,
                &action.help,
                display_flags::FIELDLEN
                    | display_flags::NEWLINE
                    | display_flags::LFAFTER
                    | display_flags::HIGHASCII,
            )?
        };

        if !page_len.is_empty() {
            let page_len = page_len.parse::<u16>().unwrap_or_default();
            self.state.session.page_len = page_len;
            if let Some(user) = &mut self.state.current_user {
                user.page_len = page_len;
            }
        }
        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }

    fn show_help(&mut self) -> Res<()> {
        self.display_menu = true;
        let help_cmd = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            self.state.input_field(
                IceText::HelpPrompt,
                8,
                MASK_COMMAND,
                "",
                display_flags::UPCASE | display_flags::NEWLINE | display_flags::HIGHASCII,
            )?
        };
        if !help_cmd.is_empty() {
            let mut help_loc = self
                .state
                .board
                .lock()
                .unwrap()
                .config
                .paths
                .help_path
                .clone();
            let mut found = false;
            for action in &self.state.session.current_conference.commands {
                if action.input.contains(&help_cmd) && !action.help.is_empty() {
                    help_loc = help_loc.join(&action.help);
                    found = true;
                    break;
                }
            }
            if !found {
                help_loc = help_loc.join(format!("hlp{}", help_cmd).as_str());
            }
            let res = self.state.display_file(&help_loc)?;

            if res {
                self.state.press_enter()?;
            }
        }
        Ok(())
    }

    fn check_sec(&mut self, command: &str, required_sec: u8) -> Res<bool> {
        if self.state.session.cur_security >= required_sec {
            return Ok(true);
        }

        self.state.bell()?;
        self.state.session.op_text = command.to_string();
        self.state.display_text(
            IceText::MenuSelectionUnavailable,
            display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
        )?;

        self.state.session.security_violations += 1;
        if let Some(user) = &mut self.state.current_user {
            user.stats.num_sec_viol += 1;
        }
        if self.state.session.security_violations > 10 {
            self.state.display_text(
                IceText::SecurityViolation,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LOGIT,
            )?;
            self.state.display_text(
                IceText::AutoDisconnectNow,
                display_flags::NEWLINE | display_flags::LFBEFORE,
            )?;
            self.state
                .hangup(icy_board_engine::vm::HangupType::Hangup)?;
        }

        Ok(false)
    }
}
