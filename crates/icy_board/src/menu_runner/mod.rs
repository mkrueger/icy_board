use icy_board_engine::{
    icy_board::{
        commands::{Command, CommandType},
        icb_text::IceText,
        security::RequiredSecurity,
        state::{functions::display_flags, GraphicsMode, IcyBoardState},
        IcyBoardError,
    },
    vm::TerminalTarget,
};
use icy_ppe::Res;
mod delete_message;
mod find_files;
mod message_reader;
mod recover_message;
mod set_transfer_protocol;
mod show_bulletins;
mod show_file_directories;
mod take_survey;

pub struct PcbBoardCommand {
    pub state: IcyBoardState,
    pub display_menu: bool,
}

unsafe impl Send for PcbBoardCommand {}
unsafe impl Sync for PcbBoardCommand {}

const MASK_COMMAND: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+-=[]{}|;':,.<>?/\\\" ";
const MASK_NUMBER: &str = "0123456789";

impl PcbBoardCommand {
    pub fn new(state: IcyBoardState) -> Self {
        Self { state, display_menu: true }
    }

    pub async fn do_command(&mut self) -> Res<()> {
        if self.display_menu && !self.state.session.expert_mode {
            self.display_menu().await?;
            self.display_menu = false;
        }

        let command = self
            .state
            .input_field(IceText::CommandPrompt, 40, MASK_COMMAND, "", display_flags::UPCASE)
            .await?;

        let mut cmds = command.split(' ');

        let command = cmds.next().unwrap_or_default();
        for cmd in cmds {
            self.state.session.tokens.push_back(cmd.to_string());
        }

        if let Some(action) = self.state.try_find_command(command).await {
            return self.dispatch_action(command, &action).await;
        }

        self.state
            .display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::LFAFTER | display_flags::LFBEFORE)
            .await?;
        Ok(())
    }

    async fn display_menu(&mut self) -> Res<()> {
        self.displaycmdfile("menu").await?;
        let menu_file = if self.state.session.is_sysop {
            self.state.session.current_conference.sysop_menu.clone()
        } else {
            self.state.session.current_conference.users_menu.clone()
        };
        self.state.display_file(&menu_file).await?;
        Ok(())
    }

    async fn display_news(&mut self) -> Res<()> {
        self.displaycmdfile("news").await?;
        let news_file = self.state.session.current_conference.news_file.clone();
        let resolved_path = self.state.resolve_path(&news_file).await;

        if !resolved_path.exists() {
            self.state.display_text(IceText::NoNews, display_flags::NEWLINE).await?;
        } else {
            self.state.display_file(&news_file).await?;
        }
        self.state.new_line().await?;
        self.state.press_enter().await;
        self.display_menu = true;
        Ok(())
    }

    async fn dispatch_action(&mut self, command: &str, action: &Command) -> Res<()> {
        if !self.check_sec(command, &action.security).await? {
            return Ok(());
        }

        match action.command_type {
            CommandType::AbandonConference => {
                // A
                self.abandon_conference().await?;
            }
            CommandType::BulletinList => {
                // B
                self.show_bulletins(action).await?;
            }
            CommandType::FileDirectory => {
                // F
                self.show_file_directories(action).await?;
            }

            CommandType::Goodbye => {
                // G
                self.displaycmdfile("g").await?;

                // force logoff - no flagged files scan
                if let Some(token) = self.state.session.tokens.pop_front() {
                    if token.to_ascii_uppercase() == self.state.yes_char.to_string().to_ascii_uppercase() {
                        self.state.hangup().await?;
                    }
                }

                // todo : check flagged files & parse input
                self.state.hangup().await?;
            }
            CommandType::Help => {
                // H
                self.show_help().await?;
            }
            CommandType::InitialWelcome => {
                // I
                self.initial_welcome().await?;
            }
            CommandType::JoinConference => {
                // J
                self.join_conference(action).await?;
            }
            CommandType::DeleteMessage => {
                // K
                self.delete_message(action).await?;
            }
            CommandType::LocateFile => {
                // L
                self.find_files(action).await?;
            }
            CommandType::ToggleGraphics => {
                // M
                self.toggle_graphics().await?;
            }
            CommandType::NewFileScan => {
                // N
                self.find_new_files(action, 60000).await?;
            }
            CommandType::SetPageLength => {
                // P
                self.set_page_len(action).await?;
            }
            CommandType::ReadMessages => {
                // R
                self.read_messages(action).await?;
            }

            CommandType::Survey => {
                // S
                self.take_survey(action).await?;
            }
            CommandType::SetTransferProtocol => {
                // T
                self.set_transfer_protocol(action).await?;
            }
            CommandType::ExpertMode => {
                // X
                self.set_expert_mode().await?;
            }
            CommandType::ZippyDirectoryScan => {
                // Z
                self.zippy_directory_scan(action).await?;
            }

            CommandType::ShowMenu => {
                // MENU
                self.display_menu().await?;
                self.display_menu = false;
            }

            CommandType::DisplayNews => {
                // MENU
                self.display_news().await?;
            }
            CommandType::UserList => {
                // USER
                self.show_user_list(action).await?;
            }

            CommandType::RestoreMessage => {
                // 4
                self.restore_message(action).await?;
            }

            _ => {
                return Err(Box::new(IcyBoardError::UnknownAction(format!("{:?}", action.command_type))));
            }
        }
        Ok(())
    }

    async fn abandon_conference(&mut self) -> Res<()> {
        if self.state.board.lock().unwrap().conferences.is_empty() {
            self.state
                .display_text(IceText::NoConferenceAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            self.state.press_enter().await;
            return Ok(());
        }
        if self.state.session.current_conference_number > 0 {
            self.state.session.op_text = format!(
                "{} ({})",
                self.state.session.current_conference.name, self.state.session.current_conference_number
            );
            self.state.join_conference(0);
            self.state
                .display_text(IceText::ConferenceAbandoned, display_flags::NEWLINE | display_flags::NOTBLANK)
                .await?;
            self.state.new_line().await?;
            self.state.press_enter().await;
        }
        self.display_menu = true;
        Ok(())
    }

    async fn join_conference(&mut self, action: &Command) -> Res<()> {
        if self.state.board.lock().unwrap().conferences.is_empty() {
            self.state
                .display_text(IceText::NoConferenceAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            self.state.press_enter().await;
            return Ok(());
        }
        let conf_number = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            let mnu = self.state.board.lock().unwrap().config.paths.conf_join_menu.clone();
            let mnu = self.state.resolve_path(&mnu).await;

            self.state.display_menu(&mnu).await?;
            self.state.new_line().await?;

            self.state
                .input_field(
                    IceText::JoinConferenceNumber,
                    40,
                    MASK_COMMAND,
                    &action.help,
                    display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
                )
                .await?
        };
        let mut joined = false;
        if let Ok(number) = conf_number.parse::<i32>() {
            if 0 <= number && (number as usize) <= self.state.board.lock().unwrap().conferences.len() {
                self.state.join_conference(number).await;
                self.state.session.op_text = format!(
                    "{} ({})",
                    self.state.session.current_conference.name, self.state.session.current_conference_number
                );
                self.state
                    .display_text(IceText::ConferenceJoined, display_flags::NEWLINE | display_flags::NOTBLANK)
                    .await?;

                joined = true;
            }
        }

        if !joined {
            self.state.session.op_text = conf_number;
            self.state
                .display_text(IceText::InvalidConferenceNumber, display_flags::NEWLINE | display_flags::NOTBLANK)
                .await?;
        }

        self.state.new_line().await?;
        self.state.press_enter().await;
        self.display_menu = true;
        Ok(())
    }

    async fn show_user_list(&mut self, action: &Command) -> Res<()> {
        self.state.new_line().await?;
        let text = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            self.state
                .input_field(
                    IceText::UserScan,
                    40,
                    MASK_COMMAND,
                    &action.help,
                    display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
                )
                .await?
        };

        self.state
            .display_text(IceText::UsersHeader, display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::NOTBLANK)
            .await?;
        self.state.display_text(IceText::UserScanLine, display_flags::NOTBLANK).await?;
        self.state.reset_color().await?;
        let mut output = String::new();
        for u in self.state.board.lock().unwrap().users.iter() {
            if text.is_empty() || u.get_name().to_ascii_uppercase().contains(&text.to_ascii_uppercase()) {
                output.push_str(&format!(
                    "{:<24} {:<30} {} {}\r\n",
                    u.get_name(),
                    u.city,
                    u.last_date_on.to_country_date(),
                    u.last_time_on
                ));
            }
        }
        self.state.print(TerminalTarget::Both, &output).await?;

        self.state.new_line().await?;
        self.state.press_enter().await;
        self.display_menu = true;
        Ok(())
    }

    async fn set_expert_mode(&mut self) -> Res<()> {
        self.displaycmdfile("x").await?;

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
            self.state
                .display_text(
                    IceText::ExpertmodeModeOn,
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
                )
                .await?;
        } else {
            self.state
                .display_text(
                    IceText::ExpertmodeModeOff,
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
                )
                .await?;
            self.state.press_enter().await;
        }
        self.display_menu = true;
        Ok(())
    }

    async fn toggle_graphics(&mut self) -> Res<()> {
        self.displaycmdfile("m").await?;

        /* no_graphics disabled atm
        if self.state.board.lock().unwrap().config.no_graphis {
            self.state
                .display_text(IceText::GraphicsUnavailable, display_flags::NEWLINE | display_flags::LFBEFORE)?;

            return;
        } */

        if let Some(token) = self.state.session.tokens.pop_front() {
            let token = token.to_ascii_uppercase();

            match token.as_str() {
                "CT" => {
                    self.state.session.disp_options.disable_color = true;
                    self.state.session.disp_options.grapics_mode = GraphicsMode::Off;
                    self.state
                        .display_text(IceText::CTTYOn, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                }
                "AN" => {
                    self.state.session.disp_options.disable_color = false;
                    self.state.session.disp_options.grapics_mode = GraphicsMode::Ansi;
                    self.state
                        .display_text(IceText::AnsiOn, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                }
                "AV" => {
                    self.state.session.disp_options.disable_color = false;
                    self.state.session.disp_options.grapics_mode = GraphicsMode::Avatar;
                    self.state
                        .display_text(IceText::AnsiOn, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                }
                "GR" => {
                    self.state.session.disp_options.disable_color = false;
                    self.state.session.disp_options.grapics_mode = GraphicsMode::Ansi;
                    self.state
                        .display_text(IceText::GraphicsOn, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                }
                "RI" => {
                    self.state.session.disp_options.disable_color = false;
                    self.state.session.disp_options.grapics_mode = GraphicsMode::Rip;
                    self.state
                        .display_text(IceText::RIPModeOn, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                }
                _ => {}
            }
        }

        {
            if !self.state.session.disp_options.disable_color {
                self.state.reset_color().await?;
            }

            self.state.session.disp_options.disable_color = !self.state.session.disp_options.disable_color;

            let msg = if self.state.session.disp_options.disable_color {
                IceText::GraphicsOff
            } else {
                IceText::GraphicsOn
            };
            self.state.display_text(msg, display_flags::NEWLINE | display_flags::LFBEFORE).await?;
        }
        self.state.press_enter().await;
        self.display_menu = true;
        Ok(())
    }

    async fn set_page_len(&mut self, action: &Command) -> Res<()> {
        let page_len = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            self.state.display_text(IceText::CurrentPageLength, display_flags::LFBEFORE).await?;
            self.state.print(TerminalTarget::Both, &format!(" {}\r\n", self.state.session.page_len)).await?;
            self.state
                .input_field(
                    IceText::EnterPageLength,
                    2,
                    MASK_NUMBER,
                    &action.help,
                    display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
                )
                .await?
        };

        if !page_len.is_empty() {
            let page_len = page_len.parse::<u16>().unwrap_or_default();
            self.state.session.page_len = page_len;
            if let Some(user) = &mut self.state.current_user {
                user.page_len = page_len;
            }
        }
        self.state.press_enter().await;
        self.display_menu = true;
        Ok(())
    }

    async fn show_help(&mut self) -> Res<()> {
        self.display_menu = true;
        let help_cmd = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            self.state
                .input_field(
                    IceText::HelpPrompt,
                    8,
                    MASK_COMMAND,
                    "",
                    display_flags::UPCASE | display_flags::NEWLINE | display_flags::HIGHASCII,
                )
                .await?
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
            let res = self.state.display_file(&help_loc).await?;

            if res {
                self.state.press_enter().await;
            }
        }
        Ok(())
    }

    async fn check_sec(&mut self, command: &str, required_sec: &RequiredSecurity) -> Res<bool> {
        if required_sec.user_can_access(&self.state.session) {
            return Ok(true);
        }

        self.state.bell().await?;
        self.state.session.op_text = command.to_string();
        self.state
            .display_text(
                IceText::MenuSelectionUnavailable,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
            )
            .await?;

        self.state.session.security_violations += 1;
        if let Some(user) = &mut self.state.current_user {
            user.stats.num_sec_viol += 1;
        }
        if self.state.session.security_violations > 10 {
            self.state
                .display_text(
                    IceText::SecurityViolation,
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LOGIT,
                )
                .await?;
            self.state
                .display_text(IceText::AutoDisconnectNow, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            self.state.hangup().await?;
        }

        Ok(false)
    }

    async fn initial_welcome(&mut self) -> Res<()> {
        let board_name = self.state.board.lock().unwrap().config.board.name.clone();
        self.state.print(TerminalTarget::Both, &board_name).await?;
        self.state.new_line().await?;

        let welcome_screen = self.state.board.lock().unwrap().config.paths.welcome.clone();

        let welcome_screen = self.state.resolve_path(&welcome_screen).await;
        self.state.display_file(&welcome_screen).await?;
        Ok(())
    }

    async fn displaycmdfile(&mut self, command_file: &str) -> Res<bool> {
        let path = self.state.board.lock().unwrap().config.paths.command_display_path.clone();
        if !path.is_dir() {
            return Ok(false);
        }
        let file = path.join(command_file);

        if file.with_extension("ppe").is_file() {
            self.state.run_ppe(&path, None).await?;
            return Ok(true);
        }

        /* TODO: Menus
        if file.with_extension("mnu").is_file() {
            self.state.run_ppe(&path, None)?;
            return Ok(true);
        }
        */
        if file.exists() {
            self.state.display_file(&file).await?;
            return Ok(true);
        }

        Ok(false)
    }
}
