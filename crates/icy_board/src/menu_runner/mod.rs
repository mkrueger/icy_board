use std::{collections::HashMap, time::Instant};

use crate::Res;
use async_recursion::async_recursion;
use icy_board_engine::{
    icy_board::{
        commands::{ActionTrigger, AutoRun, Command, CommandAction, CommandType}, icb_text::IceText, menu::{Menu, MenuType}, security_expr::SecurityExpression, state::{control_codes, functions::display_flags, IcyBoardState, UserActivity}, IcyBoardError, IcyBoardSerializer
    },
    vm::TerminalTarget,
};
use tokio::time::sleep;

mod login;
mod message_reader;
mod new;
mod pcb;

pub struct PcbBoardCommand {
    pub state: IcyBoardState,
    pub display_menu: bool,
    pub autorun_times: HashMap<usize, u64>,
    pub saved_cmd: String,
}
pub const MASK_COMMAND: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+-=[]{}|;':,.<>?/\\\" ";
const MASK_NUMBER: &str = "0123456789";

impl PcbBoardCommand {
    pub fn new(state: IcyBoardState) -> Self {
        Self {
            state,
            display_menu: true,
            saved_cmd: String::new(),
            autorun_times: HashMap::new(),
        }
    }

    #[async_recursion(?Send)]
    pub async fn do_command(&mut self) -> Res<()> {
        self.state.set_activity(UserActivity::BrowseMenu).await;
        if self.display_menu && !self.state.session.expert_mode {
            self.display_menu().await?;
            if self.state.session.request_logoff {
                return Ok(());
            }
            self.display_menu = false;
        }
        let command = self
            .state
            .input_field(
                IceText::CommandPrompt,
                40,
                MASK_COMMAND,
                "",
                None,
                display_flags::UPCASE | display_flags::NEWLINE,
            )
            .await?;
        if command.len() > 5 {
            self.saved_cmd = command.clone();
        }
        if command.is_empty() {
            return Ok(());
        }

        self.state.session.push_tokens(&command);
        if let Some(command) = self.state.session.tokens.pop_front() {
            if let Some(action) = self.state.try_find_command(&command).await {
                return self.dispatch_command(&command, &action).await;
            }
            self.state
                .display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::LFAFTER | display_flags::LFBEFORE)
                .await?;
        }
        self.state.session.tokens.clear();
        Ok(())
    }

    async fn autorun_commands(&mut self, mnu: &Menu, auto_run: AutoRun, cur_sec: u64) -> Res<()> {
        for (i, cmd) in mnu.commands.iter().enumerate() {
            if cmd.auto_run == auto_run {
                if AutoRun::Loop == cmd.auto_run {
                    if let Some(last_run) = self.autorun_times.get(&i) {
                        log::info!("Last run: {}, cur_sec: {}, autorun_time: {}", last_run, cur_sec, cmd.autorun_time);
                        if cur_sec - last_run < cmd.autorun_time {
                            continue;
                        }
                    }
                    self.dispatch_command("", cmd).await?;
                    self.autorun_times.insert(i, cur_sec);
                } else {
                    self.dispatch_command("", cmd).await?;
                }
            }
        }
        Ok(())
    }

    async fn display_cmd_str(&mut self, command: &Command, current_item: bool) -> Res<()> {
        self.state
            .gotoxy(TerminalTarget::Both, 1 + command.position.x as i32, 1 + command.position.y as i32)
            .await?;
        if current_item && !command.lighbar_display.is_empty() {
            self.state.print(TerminalTarget::Both, &command.lighbar_display).await?;
        } else {
            self.state.print(TerminalTarget::Both, &command.display).await?;
        }
        Ok(())
    }

    #[async_recursion(?Send)]
    async fn run_menu(&mut self, mnu: &Menu) -> Res<()> {
        log::warn!("Run menu: {}", mnu.title);
        let mut current_item = 0;
        self.autorun_times.clear();
        self.autorun_commands(mnu, AutoRun::FirstCmd, 0).await?;
        let menu_start_time = Instant::now();
        while !self.state.session.request_logoff {
            self.autorun_commands(mnu, AutoRun::Every, 0).await?;
            self.state.display_file_with_error(&mnu.display_file, false).await?;
            let mut x = current_item + 1;
            self.move_lightbar(mnu, &mut x, current_item).await?;

            self.state.session.last_new_line_y = self.state.user_screen.caret.get_position().y;
            self.state.session.num_lines_printed = 0;
            let pos = self.state.get_caret_position();
            for (i, command) in mnu.commands.iter().enumerate() {
                if command.display.is_empty() {
                    continue;
                }
                self.display_cmd_str(command, i == current_item).await?;
            }
            self.state.gotoxy(TerminalTarget::Both, pos.0, pos.1).await?;
            self.autorun_commands(mnu, AutoRun::After, 0).await?;

            let command = self.input_menu_prompt(mnu, &mut current_item, &menu_start_time).await?;
            if command.len() > 5 {
                self.saved_cmd = command.clone();
            }
            if command.is_empty() {
                if !mnu.commands[current_item].lighbar_display.is_empty() {
                    self.state.session.last_new_line_y = self.state.user_screen.caret.get_position().y;
                    self.state.session.num_lines_printed = 0;
                    self.dispatch_command(&command, &mnu.commands[current_item]).await?;
                    continue;
                }
            }
            self.state.session.push_tokens(&command);
            if let Some(command_str) = self.state.session.tokens.pop_front() {
                let cmd = mnu.commands.iter().find(|cmd| cmd.keyword.eq_ignore_ascii_case(&command_str));
                if let Some(cmd) = cmd {
                    self.state.session.last_new_line_y = self.state.user_screen.caret.get_position().y;
                    self.state.session.num_lines_printed = 0;
                    self.dispatch_command(&command_str, &cmd).await?;
                    self.state.session.tokens.clear();
                    continue;
                }

                if let Some(command) = self.state.try_find_command(&command_str).await {
                    self.state.session.last_new_line_y = self.state.user_screen.caret.get_position().y;
                    self.state.session.num_lines_printed = 0;
                    self.dispatch_command(&command_str, &command).await?;
                    self.state.session.tokens.clear();
                    continue;
                }
                self.state
                    .display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::LFAFTER | display_flags::LFBEFORE)
                    .await?;
            }
            self.state.session.tokens.clear();
        }
        Ok(())
    }

    async fn display_menu(&mut self) -> Res<()> {
        self.displaycmdfile("menu").await?;
        let menu_file = if self.state.session.is_sysop {
            self.state.session.current_conference.sysop_menu.clone()
        } else {
            self.state.session.current_conference.users_menu.clone()
        };
        let file = self.state.resolve_path(&menu_file);
        if file.with_extension("ppe").is_file() {
            self.state.run_ppe(&file.with_extension("ppe"), None).await?;
            return Ok(());
        }

        if file.with_extension("mnu").is_file() {
            let mnu = Menu::load(&file.with_extension("mnu"))?;
            self.run_menu(&mnu).await?;
            return Ok(());
        }

        self.state.display_file_with_error(&file, false).await?;
        Ok(())
    }

    async fn dispatch_command(&mut self, command_str: &str, command: &Command) -> Res<()> {
        if !self.check_sec(command_str, &command.security).await? {
            return Ok(());
        }
        let help = &command.help;
        for cmd_action in &command.actions {
            self.run_action(command, cmd_action, help).await?;
        }
        self.state.session.tokens.clear();

        Ok(())
    }

    async fn run_action(&mut self, command: &Command, cmd_action: &CommandAction, help: &String) -> Res<()> {
        match cmd_action.command_type {
            CommandType::GotoXY => {
                let pos = icy_board_engine::icy_board::commands::Position::parse(&cmd_action.parameter);
                self.state.gotoxy(TerminalTarget::Both, 1 + pos.x as i32, 1 + pos.y as i32).await?;
            }
            CommandType::PrintText => {
                self.state.print(TerminalTarget::Both, &cmd_action.parameter).await?;
            }
            CommandType::RefreshDisplayString => {
                self.display_cmd_str(command, false).await?;
            }
            CommandType::RedisplayCommand => {
                // !
                self.redisplay_cmd()?;
            }
            CommandType::AbandonConference => {
                // A
                self.abandon_conference().await?;
            }
            CommandType::BulletinList => {
                // B
                self.show_bulletins(help).await?;
            }
            CommandType::CommentToSysop => {
                // C
                self.comment_to_sysop(help).await?;
            }

            CommandType::Download => {
                // D
                self.download().await?;
            }
            CommandType::EnterMessage => {
                // E
                self.enter_message(help).await?;
            }

            CommandType::FileDirectory => {
                // F
                self.show_file_directories(help).await?;
            }

            CommandType::Goodbye => {
                // G
                self.goodbye_cmd().await?;
            }
            CommandType::Bye => {
                // BYE
                self.bye_cmd().await?;
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
                self.join_conference(help).await?;
            }
            CommandType::DeleteMessage => {
                // K
                self.delete_message(help).await?;
            }
            CommandType::LocateFile => {
                // L
                self.find_files(help).await?;
            }
            CommandType::ToggleGraphics => {
                // M
                self.toggle_graphics().await?;
            }
            CommandType::NewFileScan => {
                // N
                self.find_new_files(help, 60000).await?;
            }
            CommandType::PageSysop => {
                // O
                self.page_sysop(help).await?;
            }
            CommandType::SetPageLength => {
                // P
                self.set_page_len(help).await?;
            }
            CommandType::QuickMessageScan => {
                // Q
                self.quick_message_scan(help).await?;
            }
            CommandType::ReadMessages => {
                // R
                self.read_messages(help).await?;
            }
            CommandType::Survey => {
                // S
                self.take_survey(help).await?;
            }
            CommandType::SetTransferProtocol => {
                // T
                self.set_transfer_protocol().await?;
            }
            CommandType::UploadFile => {
                // U
                self.upload_file(help).await?;
            }
            CommandType::ViewSettings => {
                // V
                self.view_settings().await?;
            }

            CommandType::WriteSettings => {
                // W
                self.write_settings().await?;
            }
            CommandType::ExpertMode => {
                // X
                self.set_expert_mode().await?;
            }
            CommandType::PersonalMail => {
                // Y
                self.personal_mail(help).await?;
            }
            CommandType::ZippyDirectoryScan => {
                // Z
                self.zippy_directory_scan(help).await?;
            }

            CommandType::ShowMenu => {
                // MENU
                self.display_menu().await?;
                self.display_menu = false;
            }

            CommandType::DisplayNews => {
                // NEWS
                self.display_news().await?;
            }
            CommandType::UserList => {
                // USER
                self.show_user_list(help).await?;
            }
            CommandType::SetLanguage => {
                // LANG
                self.set_language().await?;
            }
            CommandType::EnableAlias => {
                // ALIAS
                self.toggle_alias().await?;
            }
            CommandType::WhoIsOnline => {
                // WHO
                self.who_display_nodes().await?;
            }

            CommandType::OpenDoor => {
                // DOOR/OPEN
                self.open_door(help).await?;
            }

            CommandType::RestoreMessage => {
                // 4
                self.restore_message(help).await?;
            }

            CommandType::ReadEmail => {
                // @
                self.read_email(help).await?;
            }
            CommandType::RunPPE => {
                // PPE
                self.ppe_run().await?;
            }

            CommandType::TextSearch => {
                // TS
                self.text_search(help).await?;
            }

            CommandType::Broadcast => {
                // BR
                self.broadcast().await?;
            }

            _ => {
                return Err(Box::new(IcyBoardError::UnknownAction(format!("{:?}", cmd_action.command_type))));
            }
        }
        Ok(())
    }

    async fn check_sec(&mut self, command: &str, required_sec: &SecurityExpression) -> Res<bool> {
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
        if let Some(user) = &mut self.state.session.current_user {
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
            self.state.goodbye().await?;
        }

        Ok(false)
    }

    async fn displaycmdfile(&mut self, command_file: &str) -> Res<bool> {
        let path = self.state.get_board().await.config.paths.command_display_path.clone();
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

        self.state.display_file_with_error(&file, false).await
    }

    async fn move_lightbar(&mut self, mnu: &Menu, current_item: &mut usize, new_item: usize) -> Res<()> {
        if new_item != *current_item {
            if new_item >= mnu.commands.len() {
                return Ok(());
            }

            self.state.print(TerminalTarget::Both, "\x1b[s").await?;
            if *current_item < mnu.commands.len() {
                let cmd = &mnu.commands[*current_item];
                self.state
                    .gotoxy(TerminalTarget::Both, 1 + cmd.position.x as i32, 1 + cmd.position.y as i32)
                    .await?;
                self.state.print(TerminalTarget::Both, &cmd.display).await?;
            }

            let cmd = &mnu.commands[new_item];
            self.state
                .gotoxy(TerminalTarget::Both, 1 + cmd.position.x as i32, 1 + cmd.position.y as i32)
                .await?;
            self.state.print(TerminalTarget::Both, &cmd.lighbar_display).await?;

            for a in &cmd.actions {
                if a.trigger == ActionTrigger::Selection {
                    self.run_action(cmd, a, &cmd.help).await?;
                }
            }
            self.state.print(TerminalTarget::Both, "\x1b[u").await?;

            *current_item = new_item;
        }
        Ok(())
    }

    #[async_recursion(?Send)]
    pub async fn input_menu_prompt(&mut self, mnu: &Menu, current_item: &mut usize, start_time: &Instant) -> Res<String> {
        self.state.print(TerminalTarget::Both, &mnu.prompt).await?;

        let mut output = String::new();
        let len = 13;
        loop {
            tokio::select! {
                key_char = self.state.get_char_edit() => {
                    if let Ok(Some(key_char)) = key_char {
                        match key_char.ch {
                            '\n' | '\r' => {
                                break;
                            }
                            '\x08' => {
                                if !output.is_empty() {
                                    output.pop();
                                    self.state.print(TerminalTarget::Both, "\x08 \x08").await?;
                                }
                            }
                            control_codes::UP => {
                                self.move_lightbar(mnu, current_item, mnu.up(*current_item)).await?;
                            }
                            control_codes::DOWN => {
                                self.move_lightbar(mnu, current_item, mnu.down(*current_item)).await?;
                            }
                            control_codes::RIGHT => {
                                self.move_lightbar(mnu, current_item, mnu.right(*current_item)).await?;
                            }
                            control_codes::LEFT => {
                                self.move_lightbar(mnu, current_item, mnu.left(*current_item)).await?;
                            }
                            _ => {
                                if (output.len() as i32) < len && MASK_COMMAND.contains(key_char.ch) {
                                    output.push(key_char.ch);
                                    self.state.print(TerminalTarget::Both, &key_char.ch.to_string()).await?;
                                    if mnu.menu_type == MenuType::Hotkey {
                                        return Ok(output);
                                    }
                                }
                            }
                        }
                    } else {
                        self.autorun_commands(mnu, AutoRun::Loop, Instant::now().duration_since(*start_time).as_secs()).await?;
                    }
                }
                _ = sleep(std::time::Duration::from_millis(500)) => {
                    self.autorun_commands(mnu, AutoRun::Loop, Instant::now().duration_since(*start_time).as_secs()).await?;
                }

            }
        }

        Ok(output)
    }
}
