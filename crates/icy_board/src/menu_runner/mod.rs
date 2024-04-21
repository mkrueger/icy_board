use std::fs;

use crate::Res;
use icy_board_engine::{
    icy_board::{
        commands::{Command, CommandType},
        icb_text::IceText,
        security::RequiredSecurity,
        state::{functions::display_flags, GraphicsMode, IcyBoardState, UserActivity},
        user_base::UserBase,
        IcyBoardError,
    },
    vm::TerminalTarget,
};
use jamjam::jam::{JamMessage, JamMessageBase};
mod comment_to_sysop;
mod delete_message;
mod download;
mod enter_message;
mod find_files;
mod login;
mod message_reader;
mod quick_message_scan;
mod recover_message;
mod set_language;
mod set_transfer_protocol;
mod show_bulletins;
mod show_file_directories;
mod take_survey;
mod toggle_alias;
mod view_settings;
mod write_settings;

pub struct PcbBoardCommand {
    pub state: IcyBoardState,
    pub display_menu: bool,
}
pub const MASK_COMMAND: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+-=[]{}|;':,.<>?/\\\" ";
const MASK_NUMBER: &str = "0123456789";

impl PcbBoardCommand {
    pub fn new(state: IcyBoardState) -> Self {
        Self { state, display_menu: true }
    }

    pub fn do_command(&mut self) -> Res<()> {
        self.state.node_state.lock().unwrap().user_activity = UserActivity::BrowseMenu;
        if self.display_menu && !self.state.session.expert_mode {
            self.display_menu()?;
            self.display_menu = false;
        }

        let command = self.state.input_field(
            IceText::CommandPrompt,
            40,
            MASK_COMMAND,
            "",
            None,
            display_flags::UPCASE | display_flags::NEWLINE,
        )?;
        if command.is_empty() {
            return Ok(());
        }
        let mut cmds = command.split(' ');

        let command = cmds.next().unwrap_or_default();
        for cmd in cmds {
            self.state.session.tokens.push_back(cmd.to_string());
        }

        if let Some(action) = self.state.try_find_command(command) {
            return self.dispatch_action(command, &action);
        }

        self.state
            .display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::LFAFTER | display_flags::LFBEFORE)?;
        Ok(())
    }

    fn display_menu(&mut self) -> Res<()> {
        self.displaycmdfile("menu")?;
        let menu_file = if self.state.session.is_sysop {
            self.state.session.current_conference.sysop_menu.clone()
        } else {
            self.state.session.current_conference.users_menu.clone()
        };
        self.state.display_file(&menu_file)?;
        Ok(())
    }

    fn display_news(&mut self) -> Res<()> {
        self.displaycmdfile("news")?;
        let news_file = self.state.session.current_conference.news_file.clone();
        let resolved_path = self.state.resolve_path(&news_file);

        if !resolved_path.exists() {
            self.state.display_text(IceText::NoNews, display_flags::NEWLINE)?;
        } else {
            self.state.display_file(&news_file)?;
        }
        self.state.new_line()?;
        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }

    fn dispatch_action(&mut self, command: &str, action: &Command) -> Res<()> {
        if !self.check_sec(command, &action.security)? {
            return Ok(());
        }

        match action.command_type {
            CommandType::AbandonConference => {
                // A
                self.abandon_conference()?;
            }
            CommandType::BulletinList => {
                // B
                self.show_bulletins(action)?;
            }
            CommandType::CommentToSysop => {
                // C
                self.comment_to_sysop(action)?;
            }

            CommandType::Download => {
                // D
                self.download(action)?;
            }
            CommandType::EnterMessage => {
                // E
                self.enter_message(action)?;
            }

            CommandType::FileDirectory => {
                // F
                self.show_file_directories(action)?;
            }

            CommandType::Goodbye => {
                // G
                self.goodbye_cmd()?;
            }
            CommandType::Help => {
                // H
                self.show_help()?;
            }
            CommandType::InitialWelcome => {
                // I
                self.initial_welcome()?;
            }
            CommandType::JoinConference => {
                // J
                self.join_conference(action)?;
            }
            CommandType::DeleteMessage => {
                // K
                self.delete_message(action)?;
            }
            CommandType::LocateFile => {
                // L
                self.find_files(action)?;
            }
            CommandType::ToggleGraphics => {
                // M
                self.toggle_graphics()?;
            }
            CommandType::NewFileScan => {
                // N
                self.find_new_files(action, 60000)?;
            }
            CommandType::SetPageLength => {
                // P
                self.set_page_len(action)?;
            }
            CommandType::QuickMessageScan => {
                // Q
                self.quick_message_scan(action)?;
            }
            CommandType::ReadMessages => {
                // R
                self.read_messages(action)?;
            }

            CommandType::Survey => {
                // S
                self.take_survey(action)?;
            }
            CommandType::SetTransferProtocol => {
                // T
                self.set_transfer_protocol(action)?;
            }

            CommandType::ViewSettings => {
                // V
                self.view_settings(action)?;
            }

            CommandType::WriteSettings => {
                // W
                self.write_settings(action)?;
            }
            CommandType::ExpertMode => {
                // X
                self.set_expert_mode()?;
            }
            CommandType::ZippyDirectoryScan => {
                // Z
                self.zippy_directory_scan(action)?;
            }

            CommandType::ShowMenu => {
                // MENU
                self.display_menu()?;
                self.display_menu = false;
            }

            CommandType::DisplayNews => {
                // MENU
                self.display_news()?;
            }
            CommandType::UserList => {
                // USER
                self.show_user_list(action)?;
            }
            CommandType::SetLanguage => {
                // LANG
                self.set_language(action)?;
            }
            CommandType::EnableAlias => {
                // ALIAS
                self.toggle_alias(action)?;
            }

            CommandType::RestoreMessage => {
                // 4
                self.restore_message(action)?;
            }

            CommandType::ReadEmail => {
                // R
                self.read_email(action)?;
            }

            _ => {
                return Err(Box::new(IcyBoardError::UnknownAction(format!("{:?}", action.command_type))));
            }
        }
        Ok(())
    }

    pub fn goodbye_cmd(&mut self) -> Res<()> {
        self.displaycmdfile("g")?;
        if let Some(token) = self.state.session.tokens.pop_front() {
            if token.to_ascii_uppercase() == self.state.session.yes_char.to_string().to_ascii_uppercase() {
                self.state.goodbye()?;
            }
        }
        self.state.goodbye()?;
        Ok(())
    }

    fn abandon_conference(&mut self) -> Res<()> {
        if self.state.board.lock().unwrap().conferences.is_empty() {
            self.state
                .display_text(IceText::NoConferenceAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)?;
            self.state.press_enter()?;
            return Ok(());
        }
        if self.state.session.current_conference_number > 0 {
            self.state.session.op_text = format!(
                "{} ({})",
                self.state.session.current_conference.name, self.state.session.current_conference_number
            );
            self.state.join_conference(0);
            self.state
                .display_text(IceText::ConferenceAbandoned, display_flags::NEWLINE | display_flags::NOTBLANK)?;
            self.state.new_line()?;
            self.state.press_enter()?;
        }
        self.display_menu = true;
        Ok(())
    }

    fn join_conference(&mut self, action: &Command) -> Res<()> {
        if self.state.board.lock().unwrap().conferences.is_empty() {
            self.state
                .display_text(IceText::NoConferenceAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)?;
            self.state.press_enter()?;
            return Ok(());
        }
        let conf_number = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            let mnu = self.state.board.lock().unwrap().config.paths.conf_join_menu.clone();
            let mnu = self.state.resolve_path(&mnu);

            self.state.display_menu(&mnu)?;
            self.state.new_line()?;

            self.state.input_field(
                IceText::JoinConferenceNumber,
                40,
                MASK_COMMAND,
                &action.help,
                None,
                display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
            )?
        };
        if !conf_number.is_empty() {
            let mut joined = false;
            if let Ok(number) = conf_number.parse::<i32>() {
                if 0 <= number && (number as usize) <= self.state.board.lock().unwrap().conferences.len() {
                    self.state.join_conference(number);
                    self.state.session.op_text = format!(
                        "{} ({})",
                        self.state.session.current_conference.name, self.state.session.current_conference_number
                    );
                    self.state
                        .display_text(IceText::ConferenceJoined, display_flags::NEWLINE | display_flags::NOTBLANK)?;

                    joined = true;
                }
            }

            if !joined {
                self.state.session.op_text = conf_number;
                self.state
                    .display_text(IceText::InvalidConferenceNumber, display_flags::NEWLINE | display_flags::NOTBLANK)?;
            }
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
                None,
                display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
            )?
        };

        self.state
            .display_text(IceText::UsersHeader, display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::NOTBLANK)?;
        self.state.display_text(IceText::UserScanLine, display_flags::NOTBLANK)?;
        self.state.reset_color()?;
        let mut output = String::new();
        for u in self.state.board.lock().unwrap().users.iter() {
            if text.is_empty() || u.get_name().to_ascii_uppercase().contains(&text.to_ascii_uppercase()) {
                output.push_str(&format!(
                    "{:<24} {:<30} {} {}\r\n",
                    u.get_name(),
                    u.city_or_state,
                    self.state.format_date(u.stats.last_on),
                    self.state.format_time(u.stats.last_on)
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
        self.displaycmdfile("x")?;

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
                IceText::ViewSettingsExpertModeOn,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
            )?;
        } else {
            self.state.display_text(
                IceText::ViewSettingsExpertModeOff,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
            )?;
            self.state.press_enter()?;
        }
        self.display_menu = true;
        Ok(())
    }

    fn toggle_graphics(&mut self) -> Res<()> {
        self.displaycmdfile("m")?;

        /* no_graphics disabled atm
        if self.state.board.lock().unwrap().config.no_graphis {
            self.state
                .display_text(IceText::GraphicsUnavailable, display_flags::NEWLINE | display_flags::LFBEFORE)?;

            return;
        } */

        if !self.state.session.disp_options.disable_color {
            self.state.reset_color()?;
        }

        if let Some(token) = self.state.session.tokens.pop_front() {
            let token = token.to_ascii_uppercase();

            match token.as_str() {
                "CT" => {
                    self.state.session.disp_options.disable_color = true;
                    self.state.session.disp_options.grapics_mode = GraphicsMode::Off;
                    self.state.display_text(IceText::CTTYOn, display_flags::NEWLINE | display_flags::LFBEFORE)?;
                }
                "AN" => {
                    self.state.session.disp_options.disable_color = false;
                    self.state.session.disp_options.grapics_mode = GraphicsMode::Ansi;
                    self.state.display_text(IceText::AnsiOn, display_flags::NEWLINE | display_flags::LFBEFORE)?;
                }
                "AV" => {
                    self.state.session.disp_options.disable_color = false;
                    self.state.session.disp_options.grapics_mode = GraphicsMode::Avatar;
                    self.state.display_text(IceText::AvatarOn, display_flags::NEWLINE | display_flags::LFBEFORE)?;
                }
                "GR" => {
                    self.state.session.disp_options.disable_color = false;
                    self.state.session.disp_options.grapics_mode = GraphicsMode::Ansi;
                    self.state.display_text(IceText::GraphicsOn, display_flags::NEWLINE | display_flags::LFBEFORE)?;
                }
                "RI" => {
                    self.state.session.disp_options.disable_color = false;
                    self.state.session.disp_options.grapics_mode = GraphicsMode::Rip;
                    self.state.display_text(IceText::RIPModeOn, display_flags::NEWLINE | display_flags::LFBEFORE)?;
                }
                _ => {
                    self.state
                        .display_text(IceText::GraphicsUnavailable, display_flags::NEWLINE | display_flags::LFBEFORE)?;
                    return Ok(());
                }
            }
        } else {
            self.state.session.disp_options.disable_color = !self.state.session.disp_options.disable_color;
            let msg = if self.state.session.disp_options.disable_color {
                IceText::GraphicsOff
            } else {
                IceText::GraphicsOn
            };
            self.state.display_text(msg, display_flags::NEWLINE | display_flags::LFAFTER)?;
        }
        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }

    fn set_page_len(&mut self, action: &Command) -> Res<()> {
        let page_len = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            self.state.display_text(IceText::CurrentPageLength, display_flags::LFBEFORE)?;
            self.state.print(TerminalTarget::Both, &format!(" {}\r\n", self.state.session.page_len))?;
            self.state.input_field(
                IceText::EnterPageLength,
                2,
                MASK_NUMBER,
                &action.help,
                Some(self.state.session.page_len.to_string()),
                display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
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
                None,
                display_flags::UPCASE | display_flags::NEWLINE | display_flags::HIGHASCII,
            )?
        };
        if !help_cmd.is_empty() {
            let mut help_loc = self.state.board.lock().unwrap().config.paths.help_path.clone();
            let mut found = false;
            for action in &self.state.session.current_conference.commands {
                if action.keyword.contains(&help_cmd) && !action.help.is_empty() {
                    help_loc = help_loc.join(&action.help);
                    found = true;
                    break;
                }
            }
            if !found {
                help_loc = help_loc.join(format!("hlp{}", help_cmd).as_str());
            }
            let am = self.state.session.disable_auto_more;
            self.state.session.disable_auto_more = false;
            self.state.session.disp_options.non_stop = false;
            let res = self.state.display_file(&help_loc)?;
            self.state.session.disable_auto_more = am;

            if res {
                self.state.press_enter()?;
            }
        }
        Ok(())
    }

    fn check_sec(&mut self, command: &str, required_sec: &RequiredSecurity) -> Res<bool> {
        if required_sec.user_can_access(&self.state.session) {
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
            self.state
                .display_text(IceText::AutoDisconnectNow, display_flags::NEWLINE | display_flags::LFBEFORE)?;
            self.state.goodbye()?;
        }

        Ok(false)
    }

    fn initial_welcome(&mut self) -> Res<()> {
        let board_name = self.state.board.lock().unwrap().config.board.name.clone();
        self.state.print(TerminalTarget::Both, &board_name)?;
        self.state.new_line()?;
        let welcome_screen = self.state.board.lock().unwrap().config.paths.welcome.clone();
        let welcome_screen = self.state.resolve_path(&welcome_screen);
        self.state.display_file(&welcome_screen)?;
        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }

    fn displaycmdfile(&mut self, command_file: &str) -> Res<bool> {
        let path = self.state.board.lock().unwrap().config.paths.command_display_path.clone();
        if !path.is_dir() {
            return Ok(false);
        }
        let file = path.join(command_file);

        if file.with_extension("ppe").is_file() {
            self.state.run_ppe(&path, None)?;
            return Ok(true);
        }

        /* TODO: Menus
        if file.with_extension("mnu").is_file() {
            self.state.run_ppe(&path, None)?;
            return Ok(true);
        }
        */
        if file.exists() {
            self.state.display_file(&file)?;
            return Ok(true);
        }

        Ok(false)
    }

    fn get_email_msgbase(&mut self, user_name: &str) -> Res<JamMessageBase> {
        let home_dir = if let Ok(board) = self.state.board.lock() {
            let name = if user_name == self.state.session.sysop_name {
                &board.users[0].get_name()
            } else {
                user_name
            };
            let home_dir = UserBase::get_user_home_dir(&self.state.resolve_path(&board.config.paths.home_dir), name);
            home_dir
        } else {
            return Err(IcyBoardError::ErrorLockingBoard.into());
        };

        if !home_dir.exists() {
            log::error!("Homedir for user {} does not exist", user_name);
            self.state.display_text(IceText::MessageBaseError, display_flags::NEWLINE)?;
            return Err(IcyBoardError::HomeDirMissing(user_name.to_string()).into());
        }

        let msg_dir = home_dir.join("msg");
        fs::create_dir_all(&msg_dir)?;
        let msg_base = msg_dir.join("email");
        Ok(if msg_base.with_extension("jhr").exists() {
            JamMessageBase::open(msg_base)?
        } else {
            log::info!("Creating new email message base for user {}", user_name);
            JamMessageBase::create(msg_base)?
        })
    }

    fn send_message(&mut self, conf: i32, area: i32, msg: JamMessage, text: IceText) -> Res<()> {
        let msg_base = if conf < 0 {
            let user_name = msg.get_to().unwrap().to_string();
            self.get_email_msgbase(&user_name)
        } else {
            let msg_base = self.state.board.lock().unwrap().conferences[conf as usize].areas[area as usize]
                .filename
                .clone();
            let msg_base = self.state.resolve_path(&msg_base);
            if msg_base.with_extension("jhr").exists() {
                JamMessageBase::open(msg_base)
            } else {
                JamMessageBase::create(msg_base)
            }
        };

        match msg_base {
            Ok(mut msg_base) => {
                let number = msg_base.write_message(&msg)?;
                msg_base.write_jhr_header()?;

                self.state.display_text(text, display_flags::DEFAULT)?;
                self.state.println(TerminalTarget::Both, &number.to_string())?;
                self.state.new_line()?;
            }
            Err(err) => {
                log::error!("while opening message base: {}", err.to_string());
                self.state.display_text(IceText::MessageBaseError, display_flags::NEWLINE)?;
            }
        }

        Ok(())
    }
}
