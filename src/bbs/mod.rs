use icy_board_engine::{
    icy_board::{
        state::{functions::display_flags, IcyBoardState},
        text_messages::{
            AUTODISCONNECTNOW, COMMANDPROMPT, CONFABANDONED, CONFJOINED, CURPAGELEN,
            ENTERPAGELENGTH, EXPERTOFF, EXPERTON, GRAPHICSOFF, GRAPHICSON, GRAPHICSUNAVAIL,
            HELPPROMPT, INVALIDCONFNUM, INVALIDENTRY, JOINCONFNUM, MENUSELUNAVAIL, NOCONFAVAILABLE,
            SECURITYVIOL, USERSCAN, USERSCANLINE, USERSHEADER,
        },
        IcyBoardError,
    },
    vm::TerminalTarget,
};
use icy_ppe::Res;

use self::actions::Menu;

pub mod actions;

pub struct IcyBoardCommand {
    pub state: IcyBoardState,
    pub menu: Menu,
    pub display_menu: bool,
}
const MASK_COMMAND: &str  = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+-=[]{}|;':,.<>?/\\\" ";
const MASK_NUMBER: &str = "0123456789";

impl IcyBoardCommand {
    pub fn new(state: IcyBoardState) -> Self {
        let cmd_txt = include_str!("../../data/menu.cmd");
        let menu = Menu::read(cmd_txt);
        Self {
            state,
            menu,
            display_menu: true,
        }
    }

    pub fn do_command(&mut self) -> Res<()> {
        if self.display_menu && !self.state.session.expert_mode {
            self.display_menu()?;
            self.display_menu = false;
        }

        let command =
            self.state
                .input_field(COMMANDPROMPT, 40, MASK_COMMAND, display_flags::UPCASE)?;

        let mut cmds = command.split(' ');

        let command = cmds.next().unwrap_or_default();
        for cmd in cmds {
            self.state.session.tokens.push_back(cmd.to_string());
        }

        for action in &self.menu.actions {
            if action.commands.contains(&command.to_string()) {
                return self.dispatch_action(command, action.action.clone());
            }
        }
        self.state.display_text(
            INVALIDENTRY,
            display_flags::NEWLINE | display_flags::LFAFTER | display_flags::LFBEFORE,
        )?;
        Ok(())
    }

    fn display_menu(&mut self) -> Res<()> {
        let current_conference = self.state.session.current_conference.number;

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

    fn dispatch_action(&mut self, command: &str, clone: Option<String>) -> Res<()> {
        if let Some(act) = clone {
            match act.as_str() {
                "ABANDON_CONFERENCE" => {
                    self.abandon_conference(command)?;
                }
                "GOODBYE" => {
                    self.state
                        .hangup(icy_board_engine::vm::HangupType::Goodbye)?;
                }
                "WHO" => {}
                "HELP" => {
                    self.show_help()?;
                }
                "JOIN_CONFERENCE" => {
                    self.join_conference(command)?;
                }
                "MENU" => {
                    self.display_menu()?;
                    self.display_menu = false;
                }
                "TOGGLE_GRAPHICS" => {
                    self.toggle_graphics(command)?;
                }
                "SET_PAGE_LEN" => {
                    self.set_page_len(command)?;
                }
                "SET_EXPERT_MODE" => {
                    self.set_expert_mode(command)?;
                }
                "USER_LIST" => {
                    self.show_user_list(command)?;
                }
                _ => {
                    return Err(Box::new(IcyBoardError::UnknownAction(act.to_string())));
                }
            }
        }
        Ok(())
    }

    fn abandon_conference(&mut self, command: &str) -> Res<()> {
        let required_sec = self
            .state
            .board
            .lock()
            .unwrap()
            .data
            .user_levels
            .cmd_abandon_conf;
        if !self.check_sec(command, required_sec)? {
            return Ok(());
        }
        if self.state.board.lock().unwrap().conferences.is_empty() {
            self.state.display_text(
                NOCONFAVAILABLE,
                display_flags::NEWLINE | display_flags::LFBEFORE,
            )?;
            self.state.press_enter()?;
            return Ok(());
        }
        if self.state.session.current_conference.number > 0 {
            self.state.session.op_text = format!(
                "{} ({})",
                self.state.session.current_conference.name,
                self.state.session.current_conference.number
            );
            self.state.join_conference(0);
            self.state.display_text(
                CONFABANDONED,
                display_flags::NEWLINE | display_flags::NOTBLANK,
            )?;
            self.state.new_line()?;
            self.state.press_enter()?;
        }

        self.display_menu = true;
        Ok(())
    }

    fn join_conference(&mut self, command: &str) -> Res<()> {
        let required_sec = self
            .state
            .board
            .lock()
            .unwrap()
            .data
            .user_levels
            .cmd_join_conf;
        if !self.check_sec(command, required_sec)? {
            return Ok(());
        }
        if self.state.board.lock().unwrap().conferences.is_empty() {
            self.state.display_text(
                NOCONFAVAILABLE,
                display_flags::NEWLINE | display_flags::LFBEFORE,
            )?;
            self.state.press_enter()?;
            return Ok(());
        }
        let conf_number = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            let conf_menu = self.state.board.lock().unwrap().data.path.conf_menu.clone();
            self.state.display_menu(&conf_menu)?;
            self.state.new_line()?;

            self.state.input_field(
                JOINCONFNUM,
                40,
                MASK_COMMAND,
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
                    self.state.session.current_conference.number
                );
                self.state
                    .display_text(CONFJOINED, display_flags::NEWLINE | display_flags::NOTBLANK)?;

                joined = true;
            }
        }

        if !joined {
            self.state.session.op_text = conf_number.clone();
            self.state.display_text(
                INVALIDCONFNUM,
                display_flags::NEWLINE | display_flags::NOTBLANK,
            )?;
        }

        self.state.new_line()?;
        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }

    fn show_user_list(&mut self, command: &str) -> Res<()> {
        let required_sec = self
            .state
            .board
            .lock()
            .unwrap()
            .data
            .user_levels
            .cmd_show_user_list;
        if !self.check_sec(command, required_sec)? {
            return Ok(());
        }
        self.state.new_line()?;
        let text = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            self.state.input_field(
                USERSCAN,
                40,
                MASK_COMMAND,
                display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
            )?
        };

        self.state.display_text(
            USERSHEADER,
            display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::NOTBLANK,
        )?;
        self.state
            .display_text(USERSCANLINE, display_flags::NOTBLANK)?;
        self.state.reset_color()?;
        let mut output = String::new();
        for u in self.state.board.lock().unwrap().users.iter() {
            if text.is_empty()
                || u.user
                    .name
                    .to_ascii_uppercase()
                    .contains(&text.to_ascii_uppercase())
            {
                output.push_str(&format!(
                    "{:<24} {:<30} {} {}\r\n",
                    u.user.name,
                    u.user.city,
                    u.user.last_date_on.to_country_date(),
                    u.user.last_time_on
                ));
            }
        }
        self.state.print(TerminalTarget::Both, &output)?;

        self.state.new_line()?;
        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }

    fn set_expert_mode(&mut self, command: &str) -> Res<()> {
        let required_sec = self
            .state
            .board
            .lock()
            .unwrap()
            .data
            .user_levels
            .cmd_xpert_mode;
        if !self.check_sec(command, required_sec)? {
            return Ok(());
        }
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
            user.user.expert_mode = expert_mode;
        }
        if expert_mode {
            self.state.display_text(
                EXPERTON,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
            )?;
        } else {
            self.state.display_text(
                EXPERTOFF,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
            )?;
            self.state.press_enter()?;
        }
        self.display_menu = true;
        Ok(())
    }

    fn toggle_graphics(&mut self, command: &str) -> Res<()> {
        let required_sec = self
            .state
            .board
            .lock()
            .unwrap()
            .data
            .user_levels
            .cmd_toggle_graphics;
        if !self.check_sec(command, required_sec)? {
            return Ok(());
        }
        if self.state.board.lock().unwrap().data.non_graphics {
            self.state.display_text(
                GRAPHICSUNAVAIL,
                display_flags::NEWLINE | display_flags::LFBEFORE,
            )?;
        } else {
            if !self.state.session.disp_options.disable_color {
                self.state.reset_color()?;
            }

            self.state.session.disp_options.disable_color =
                !self.state.session.disp_options.disable_color;

            let msg = if self.state.session.disp_options.disable_color {
                GRAPHICSOFF
            } else {
                GRAPHICSON
            };
            self.state
                .display_text(msg, display_flags::NEWLINE | display_flags::LFBEFORE)?;
        }
        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }

    fn set_page_len(&mut self, command: &str) -> Res<()> {
        let required_sec = self
            .state
            .board
            .lock()
            .unwrap()
            .data
            .user_levels
            .cmd_set_page_length;
        if !self.check_sec(command, required_sec)? {
            return Ok(());
        }
        let page_len = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            self.state
                .display_text(CURPAGELEN, display_flags::LFBEFORE)?;
            self.state.print(
                TerminalTarget::Both,
                &format!(" {}\r\n", self.state.session.page_len),
            )?;
            self.state.input_field(
                ENTERPAGELENGTH,
                2,
                MASK_NUMBER,
                display_flags::FIELDLEN
                    | display_flags::NEWLINE
                    | display_flags::LFAFTER
                    | display_flags::HIGHASCII,
            )?
        };

        if !page_len.is_empty() {
            let page_len = page_len.parse::<i32>().unwrap_or_default();
            self.state.session.page_len = page_len as usize;
            if let Some(user) = &mut self.state.current_user {
                user.user.page_len = page_len;
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
                HELPPROMPT,
                8,
                MASK_COMMAND,
                display_flags::UPCASE | display_flags::NEWLINE | display_flags::HIGHASCII,
            )?
        };
        if !help_cmd.is_empty() {
            let mut help_loc = self.state.board.lock().unwrap().data.path.help_loc.clone();
            let mut found = false;
            for action in &self.menu.actions {
                if action.commands.contains(&help_cmd) && !action.help.is_empty() {
                    help_loc.push_str(&action.help);
                    found = true;
                    break;
                }
            }
            if !found {
                help_loc.push_str(format!("HLP{}", help_cmd).as_str());
            }
            let res = self.state.display_file(&help_loc)?;

            if res {
                self.state.press_enter()?;
            }
        }
        Ok(())
    }

    fn check_sec(&mut self, command: &str, required_sec: i32) -> Res<bool> {
        if (self.state.session.cur_security as i32) >= required_sec {
            return Ok(true);
        }

        self.state.bell()?;
        self.state.session.op_text = command.to_string();
        self.state.display_text(
            MENUSELUNAVAIL,
            display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
        )?;

        self.state.session.security_violations += 1;
        if let Some(user) = &mut self.state.current_user {
            if let Some(stats) = &mut user.inf.call_stats {
                stats.num_sec_viol += 1;
            }
        }
        if self.state.session.security_violations > 10 {
            self.state.display_text(
                SECURITYVIOL,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LOGIT,
            )?;
            self.state.display_text(
                AUTODISCONNECTNOW,
                display_flags::NEWLINE | display_flags::LFBEFORE,
            )?;
            self.state
                .hangup(icy_board_engine::vm::HangupType::Hangup)?;
        }

        Ok(false)
    }
}
