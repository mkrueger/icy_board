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
    #[async_recursion(?Send)]
    pub async fn run_single_command(&mut self, _via_cmd_list: bool) -> Res<bool> {
        if let Some(command) = self.session.tokens.pop_front() {
            if let Some(action) = self.try_find_command(&command).await {
                return self.dispatch_command(&command, &action).await;
            }
            log::warn!("Command not found: '{}'", command);
            self.display_text(
                IceText::InvalidEntry,
                display_flags::NEWLINE | display_flags::LFAFTER | display_flags::LFBEFORE | display_flags::BELL,
            )
            .await?;
        }
        self.session.tokens.clear();
        Ok(false)
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

    async fn dispatch_command(&mut self, command_str: &str, command: &Command) -> Res<bool> {
        if !self.check_sec(command_str, &command.security).await? {
            return Ok(true);
        }
        for cmd_action in &command.actions {
            self.run_action(command, cmd_action).await?;
        }
        self.session.tokens.clear();

        Ok(true)
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
            CommandType::AbandonConference => {
                let sec = self.session.user_command_level.cmd_a.clone();
                if !self.check_sec("A", &sec).await? {
                    return Ok(());
                }
                // A
                self.abandon_conference().await?;
            }
            CommandType::BulletinList => {
                let sec = self.session.user_command_level.cmd_b.clone();
                if !self.check_sec("B", &sec).await? {
                    return Ok(());
                }
                // B
                self.show_bulletins().await?;
            }
            CommandType::CommentToSysop => {
                let sec = self.session.user_command_level.cmd_c.clone();
                if !self.check_sec("C", &sec).await? {
                    return Ok(());
                }
                // C
                self.comment_to_sysop().await?;
            }

            CommandType::Download => {
                let sec = self.session.user_command_level.cmd_d.clone();
                if !self.check_sec("D", &sec).await? {
                    return Ok(());
                }
                // D
                self.download().await?;
            }
            CommandType::FlagFiles => {
                let sec = self.session.user_command_level.cmd_d.clone();
                if !self.check_sec("FLAG", &sec).await? {
                    return Ok(());
                }
                // FLAG
                self.flag_files_cmd(false).await?;
            }
            CommandType::EnterMessage => {
                let sec = self.session.user_command_level.cmd_e.clone();
                if !self.check_sec("E", &sec).await? {
                    return Ok(());
                }
                // E
                self.enter_message().await?;
            }

            CommandType::FileDirectory => {
                let sec = self.session.user_command_level.cmd_f.clone();
                if !self.check_sec("F", &sec).await? {
                    return Ok(());
                }
                // F
                self.show_file_directories().await?;
            }

            CommandType::Goodbye => {
                // G
                self.goodbye_cmd().await?;
            }
            CommandType::Bye => {
                // BYE
                self.bye_cmd(false).await?;
            }
            CommandType::Help => {
                let sec = self.session.user_command_level.cmd_h.clone();
                if !self.check_sec("H", &sec).await? {
                    return Ok(());
                }
                // H
                self.show_help_cmd().await?;
            }
            CommandType::InitialWelcome => {
                let sec = self.session.user_command_level.cmd_i.clone();
                if !self.check_sec("I", &sec).await? {
                    return Ok(());
                }
                // I
                self.initial_welcome().await?;
            }
            CommandType::JoinConference => {
                let sec = self.session.user_command_level.cmd_j.clone();
                if !self.check_sec("J", &sec).await? {
                    return Ok(());
                }
                // J
                self.join_conference_cmd().await?;
            }
            CommandType::DeleteMessage => {
                let sec = self.session.user_command_level.cmd_k.clone();
                if !self.check_sec("K", &sec).await? {
                    return Ok(());
                }
                // K
                self.delete_message().await?;
            }
            CommandType::LocateFile => {
                let sec = self.session.user_command_level.cmd_l.clone();
                if !self.check_sec("L", &sec).await? {
                    return Ok(());
                }
                // L
                self.find_files_cmd().await?;
            }
            CommandType::ToggleGraphics => {
                let sec = self.session.user_command_level.cmd_m.clone();
                if !self.check_sec("M", &sec).await? {
                    return Ok(());
                }
                // M
                self.toggle_graphics().await?;
            }
            CommandType::NewFileScan => {
                let sec = self.session.user_command_level.cmd_n.clone();
                if !self.check_sec("N", &sec).await? {
                    return Ok(());
                }
                // N
                if let Some(user) = &self.session.current_user {
                    self.find_new_files(user.stats.last_on.into()).await?;
                }
            }
            CommandType::PageSysop => {
                let sec = self.session.user_command_level.cmd_o.clone();
                if !self.check_sec("O", &sec).await? {
                    return Ok(());
                }
                // O
                self.page_sysop_command().await?;
            }
            CommandType::SetPageLength => {
                let sec = self.session.user_command_level.cmd_p.clone();
                if !self.check_sec("P", &sec).await? {
                    return Ok(());
                }
                // P
                self.set_page_len_command().await?;
            }
            CommandType::QuickMessageScan => {
                let sec = self.session.user_command_level.cmd_q.clone();
                if !self.check_sec("Q", &sec).await? {
                    return Ok(());
                }
                // Q
                self.quick_message_scan().await?;
            }
            CommandType::ReadMessages => {
                let sec = self.session.user_command_level.cmd_r.clone();
                if !self.check_sec("R", &sec).await? {
                    return Ok(());
                }
                // R
                self.read_messages().await?;
            }
            CommandType::Survey => {
                let sec = self.session.user_command_level.cmd_s.clone();
                if !self.check_sec("S", &sec).await? {
                    return Ok(());
                }
                // S
                self.take_survey().await?;
            }
            CommandType::SetTransferProtocol => {
                let sec = self.session.user_command_level.cmd_t.clone();
                if !self.check_sec("T", &sec).await? {
                    return Ok(());
                }
                // T
                self.set_transfer_protocol().await?;
            }
            CommandType::UploadFile => {
                let sec = self.session.user_command_level.cmd_u.clone();
                if !self.check_sec("R", &sec).await? {
                    return Ok(());
                }
                // U
                self.upload_file().await?;
            }
            CommandType::ViewSettings => {
                let sec = self.session.user_command_level.cmd_v.clone();
                if !self.check_sec("V", &sec).await? {
                    return Ok(());
                }
                // V
                self.view_settings().await?;
            }

            CommandType::WriteSettings => {
                let sec = self.session.user_command_level.cmd_w.clone();
                if !self.check_sec("W", &sec).await? {
                    return Ok(());
                }
                // W
                self.write_settings().await?;
            }
            CommandType::ExpertMode => {
                let sec = self.session.user_command_level.cmd_x.clone();
                if !self.check_sec("X", &sec).await? {
                    return Ok(());
                }
                // X
                self.set_expert_mode().await?;
            }
            CommandType::YourMailScan => {
                let sec = self.session.user_command_level.cmd_y.clone();
                if !self.check_sec("Y", &sec).await? {
                    return Ok(());
                }
                // Y
                self.your_mail_scan().await?;
            }
            CommandType::ZippyDirectoryScan => {
                let sec = self.session.user_command_level.cmd_z.clone();
                if !self.check_sec("Z", &sec).await? {
                    return Ok(());
                }
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
                let sec = self.session.user_command_level.cmd_show_user_list.clone();
                if !self.check_sec("USER", &sec).await? {
                    return Ok(());
                }
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
                let sec = self.session.user_command_level.cmd_who.clone();
                if !self.check_sec("WHO", &sec).await? {
                    return Ok(());
                }
                // WHO
                self.who_display_nodes().await?;
            }

            CommandType::OpenDoor => {
                let sec = self.session.user_command_level.cmd_open_door.clone();
                if !self.check_sec("DOOR", &sec).await? {
                    return Ok(());
                }
                // DOOR/OPEN
                self.open_door().await?;
            }

            CommandType::TestFile => {
                let sec = self.session.user_command_level.cmd_test_file.clone();
                if !self.check_sec("TEST", &sec).await? {
                    return Ok(());
                }
                self.println(TerminalTarget::Both, "TODO TEST_FILE").await?;
                // TODO
                // self.test_file().await?;
            }

            CommandType::GroupChat => {
                let sec = self.session.user_command_level.cmd_chat.clone();
                if !self.check_sec("CHAT", &sec).await? {
                    return Ok(());
                }
                self.println(TerminalTarget::Both, "TODO CHAIT").await?;
                // TODO
                // self.group_chat().await?;
            }

            CommandType::RestoreMessage => {
                let sec = self.session.sysop_command_level.sec_4_recover_deleted_msg.clone();
                if !self.check_sec("CHAT", &sec).await? {
                    return Ok(());
                }
                // 4
                self.restore_message().await?;
            }

            CommandType::ReadEmail => {
                let sec = self.session.user_command_level.cmd_r.clone();
                if !self.check_sec("@", &sec).await? {
                    return Ok(());
                }
                // @
                self.read_email().await?;
            }

            CommandType::WriteEmail => {
                let sec = self.session.user_command_level.cmd_e.clone();
                if !self.check_sec("@W", &sec).await? {
                    return Ok(());
                }
                // @W
                self.write_email().await?;
            }

            CommandType::RunPPE => {
                let sec = self.session.sysop_command_level.sec_10_shelled_dos_func.clone();
                if !self.check_sec("PPE", &sec).await? {
                    return Ok(());
                }
                // PPE
                if !cmd_action.parameter.is_empty() {
                    self.session.push_tokens(&cmd_action.parameter);
                }
                self.ppe_run().await?;
            }

            CommandType::TextSearch => {
                let sec = self.session.user_command_level.cmd_r.clone();
                if !self.check_sec("TS", &sec).await? {
                    return Ok(());
                }
                // TS
                self.text_search().await?;
            }

            CommandType::Broadcast => {
                let sec = self.session.sysop_command_level.use_broadcast_command.clone();
                if !self.check_sec("PPE", &sec).await? {
                    return Ok(());
                }
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
