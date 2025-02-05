use async_recursion::async_recursion;
use tokio::time::{sleep, Instant};

use crate::{
    icy_board::{
        commands::{ActionTrigger, AutoRun, Command, CommandAction, CommandType},
        icb_text::IceText,
        menu::{Menu, MenuType},
        security_expr::SecurityExpression,
        state::{control_codes, functions::display_flags},
        IcyBoardError, IcyBoardSerializer,
    },
    vm::TerminalTarget,
    Res,
};

use super::{functions::MASK_COMMAND, IcyBoardState};

impl IcyBoardState {
    pub async fn ask_run_command(&mut self) -> Res<()> {
        self.set_activity(super::NodeStatus::Available).await;
        if self.display_current_menu && !self.session.expert_mode {
            self.display_current_menu().await?;
            if self.session.request_logoff {
                return Ok(());
            }
            self.display_current_menu = false;
        }
        let command = self
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
        self.run_single_command(true, command).await
    }

    #[async_recursion(?Send)]
    pub async fn run_single_command(&mut self, _via_cmd_list: bool, command: String) -> Res<()> {
        self.session.push_tokens(&command);
        if let Some(command) = self.session.tokens.pop_front() {
            if let Some(action) = self.try_find_command(&command).await {
                return self.dispatch_command(&command, &action).await;
            }
            log::warn!("Command not found: '{}'", command);
            self.display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::LFAFTER | display_flags::LFBEFORE)
                .await?;
        }
        self.session.tokens.clear();
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
        self.gotoxy(TerminalTarget::Both, 1 + command.position.x as i32, 1 + command.position.y as i32)
            .await?;
        if current_item && !command.lighbar_display.is_empty() {
            self.print(TerminalTarget::Both, &command.lighbar_display).await?;
        } else {
            self.print(TerminalTarget::Both, &command.display).await?;
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
        while !self.session.request_logoff {
            self.autorun_commands(mnu, AutoRun::Every, 0).await?;
            self.display_file_with_error(&mnu.display_file, false).await?;
            let mut x = current_item + 1;
            self.move_lightbar(mnu, &mut x, current_item).await?;

            self.session.last_new_line_y = self.display_screen().caret.get_position().y;
            self.session.reset_num_lines();
            let pos = self.get_caret_position();
            for (i, command) in mnu.commands.iter().enumerate() {
                if command.display.is_empty() {
                    continue;
                }
                self.display_cmd_str(command, i == current_item).await?;
            }
            self.gotoxy(TerminalTarget::Both, pos.0, pos.1).await?;
            self.autorun_commands(mnu, AutoRun::After, 0).await?;

            let command = self.input_menu_prompt(mnu, &mut current_item, &menu_start_time).await?;
            if command.len() > 5 {
                self.saved_cmd = command.clone();
            }
            if command.is_empty() {
                if !mnu.commands[current_item].lighbar_display.is_empty() {
                    self.session.last_new_line_y = self.display_screen().caret.get_position().y;
                    self.session.reset_num_lines();
                    self.dispatch_command(&command, &mnu.commands[current_item]).await?;
                    continue;
                }
            }
            self.session.push_tokens(&command);
            if let Some(command_str) = self.session.tokens.pop_front() {
                let cmd = mnu.commands.iter().find(|cmd| cmd.keyword.eq_ignore_ascii_case(&command_str));
                if let Some(cmd) = cmd {
                    self.session.last_new_line_y = self.display_screen().caret.get_position().y;
                    self.session.reset_num_lines();
                    self.dispatch_command(&command_str, &cmd).await?;
                    self.session.tokens.clear();
                    continue;
                }

                if let Some(command) = self.try_find_command(&command_str).await {
                    self.session.last_new_line_y = self.display_screen().caret.get_position().y;
                    self.session.reset_num_lines();
                    self.dispatch_command(&command_str, &command).await?;
                    self.session.tokens.clear();
                    continue;
                }
                self.display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::LFAFTER | display_flags::LFBEFORE)
                    .await?;
            }
            self.session.tokens.clear();
        }
        Ok(())
    }

    pub async fn display_current_menu(&mut self) -> Res<()> {
        self.displaycmdfile("menu").await?;
        let menu_file = if self.session.is_sysop {
            self.session.current_conference.sysop_menu.clone()
        } else {
            self.session.current_conference.users_menu.clone()
        };
        let file = self.resolve_path(&menu_file);
        if file.with_extension("ppe").is_file() {
            self.run_ppe(&file.with_extension("ppe"), None).await?;
            return Ok(());
        }

        if file.with_extension("mnu").is_file() {
            let mnu = Menu::load(&file.with_extension("mnu"))?;
            self.run_menu(&mnu).await?;
            return Ok(());
        }

        self.display_file_with_error(&file, false).await?;
        Ok(())
    }

    async fn dispatch_command(&mut self, command_str: &str, command: &Command) -> Res<()> {
        if !self.check_sec(command_str, &command.security).await? {
            return Ok(());
        }
        for cmd_action in &command.actions {
            self.run_action(command, cmd_action).await?;
        }
        self.session.tokens.clear();

        Ok(())
    }

    async fn run_action(&mut self, command: &Command, cmd_action: &CommandAction) -> Res<()> {
        self.session.non_stop_off();
        match cmd_action.command_type {
            CommandType::GotoXY => {
                let pos = crate::icy_board::commands::Position::parse(&cmd_action.parameter);
                self.gotoxy(TerminalTarget::Both, 1 + pos.x as i32, 1 + pos.y as i32).await?;
            }
            CommandType::PrintText => {
                self.print(TerminalTarget::Both, &cmd_action.parameter).await?;
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
                self.show_bulletins().await?;
            }
            CommandType::CommentToSysop => {
                // C
                self.comment_to_sysop().await?;
            }

            CommandType::Download => {
                // D
                self.download().await?;
            }
            CommandType::FlagFiles => {
                // FLAG
                self.flag_files_cmd(false).await?;
            }
            CommandType::EnterMessage => {
                // E
                self.enter_message().await?;
            }

            CommandType::FileDirectory => {
                // F
                self.show_file_directories().await?;
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
                self.show_help_cmd().await?;
            }
            CommandType::InitialWelcome => {
                // I
                self.initial_welcome().await?;
            }
            CommandType::JoinConference => {
                // J
                self.join_conference_cmd().await?;
            }
            CommandType::DeleteMessage => {
                // K
                self.delete_message().await?;
            }
            CommandType::LocateFile => {
                // L
                self.find_files_cmd().await?;
            }
            CommandType::ToggleGraphics => {
                // M
                self.toggle_graphics().await?;
            }
            CommandType::NewFileScan => {
                // N
                if let Some(user) = &self.session.current_user {
                    self.find_new_files(user.stats.last_on.into()).await?;
                }
            }
            CommandType::PageSysop => {
                // O
                self.page_sysop_command().await?;
            }
            CommandType::SetPageLength => {
                // P
                self.set_page_len_command().await?;
            }
            CommandType::QuickMessageScan => {
                // Q
                self.quick_message_scan().await?;
            }
            CommandType::ReadMessages => {
                // R
                self.read_messages().await?;
            }
            CommandType::Survey => {
                // S
                self.take_survey().await?;
            }
            CommandType::SetTransferProtocol => {
                // T
                self.set_transfer_protocol().await?;
            }
            CommandType::UploadFile => {
                // U
                self.upload_file().await?;
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
            CommandType::YourMailScan => {
                // Y
                self.your_mail_scan().await?;
            }
            CommandType::ZippyDirectoryScan => {
                // Z
                self.zippy_directory_scan().await?;
            }

            CommandType::ShowMenu => {
                // MENU
                self.display_current_menu().await?;
                self.display_current_menu = false;
            }

            CommandType::DisplayNews => {
                // NEWS
                self.display_news().await?;
            }
            CommandType::UserList => {
                // USER
                self.show_user_list_cmd().await?;
            }
            CommandType::SetLanguage => {
                // LANG
                self.set_language_cmd().await?;
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
                self.open_door().await?;
            }

            CommandType::RestoreMessage => {
                // 4
                self.restore_message().await?;
            }

            CommandType::ReadEmail => {
                // @
                self.read_email().await?;
            }

            CommandType::RunPPE => {
                // PPE
                if !cmd_action.parameter.is_empty() {
                    self.session.push_tokens(&cmd_action.parameter);
                }
                self.ppe_run().await?;
            }

            CommandType::TextSearch => {
                // TS
                self.text_search().await?;
            }

            CommandType::Broadcast => {
                // BR
                self.broadcast_command().await?;
            }

            _ => {
                return Err(Box::new(IcyBoardError::UnknownAction(format!("{:?}", cmd_action.command_type))));
            }
        }
        Ok(())
    }

    async fn check_sec(&mut self, command: &str, required_sec: &SecurityExpression) -> Res<bool> {
        if required_sec.user_can_access(&self.session) {
            return Ok(true);
        }

        self.bell().await?;
        self.session.op_text = command.to_string();
        self.display_text(
            IceText::MenuSelectionUnavailable,
            display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
        )
        .await?;

        self.session.security_violations += 1;
        if let Some(user) = &mut self.session.current_user {
            user.stats.num_sec_viol += 1;
        }
        if self.session.security_violations > 10 {
            self.display_text(
                IceText::SecurityViolation,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LOGIT,
            )
            .await?;
            self.display_text(IceText::AutoDisconnectNow, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            self.goodbye().await?;
        }

        Ok(false)
    }

    pub async fn displaycmdfile(&mut self, command_file: &str) -> Res<bool> {
        let path = self.get_board().await.config.paths.command_display_path.clone();
        if !path.is_dir() {
            return Ok(false);
        }
        let file = path.join(command_file);
        if file.with_extension("ppe").is_file() {
            self.run_ppe(&path, None).await?;
            return Ok(true);
        }

        /* TODO: Menus
        if file.with_extension("mnu").is_file() {
            self.run_ppe(&path, None)?;
            return Ok(true);
        }
        */

        self.display_file_with_error(&file, false).await
    }

    async fn move_lightbar(&mut self, mnu: &Menu, current_item: &mut usize, new_item: usize) -> Res<()> {
        if new_item != *current_item {
            if new_item >= mnu.commands.len() {
                return Ok(());
            }

            self.print(TerminalTarget::Both, "\x1b[s").await?;
            if *current_item < mnu.commands.len() {
                let cmd = &mnu.commands[*current_item];
                self.gotoxy(TerminalTarget::Both, 1 + cmd.position.x as i32, 1 + cmd.position.y as i32).await?;
                self.print(TerminalTarget::Both, &cmd.display).await?;
            }

            let cmd = &mnu.commands[new_item];
            self.gotoxy(TerminalTarget::Both, 1 + cmd.position.x as i32, 1 + cmd.position.y as i32).await?;
            self.print(TerminalTarget::Both, &cmd.lighbar_display).await?;

            for a in &cmd.actions {
                if a.trigger == ActionTrigger::Selection {
                    self.run_action(cmd, a).await?;
                }
            }
            self.print(TerminalTarget::Both, "\x1b[u").await?;

            *current_item = new_item;
        }
        Ok(())
    }

    #[async_recursion(?Send)]
    pub async fn input_menu_prompt(&mut self, mnu: &Menu, current_item: &mut usize, start_time: &Instant) -> Res<String> {
        self.print(TerminalTarget::Both, &mnu.prompt).await?;

        let mut output = String::new();
        let len = 13;
        loop {
            tokio::select! {
                key_char = self.get_char_edit() => {
                    if let Ok(Some(key_char)) = key_char {
                        match key_char.ch {
                            '\n' | '\r' => {
                                break;
                            }
                            '\x08' => {
                                if !output.is_empty() {
                                    output.pop();
                                    self.print(TerminalTarget::Both, "\x08 \x08").await?;
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
                                    self.print(TerminalTarget::Both, &key_char.ch.to_string()).await?;
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
