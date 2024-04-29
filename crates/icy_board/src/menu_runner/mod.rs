use crate::Res;
use icy_board_engine::{
    icy_board::{
        commands::{Command, CommandType},
        icb_text::IceText,
        security::RequiredSecurity,
        state::{functions::display_flags, IcyBoardState, UserActivity},
        IcyBoardError,
    },
    vm::TerminalTarget,
};
use jamjam::jam::{JamMessage, JamMessageBase};

mod login;
mod message_reader;
mod new;
mod pcb;

pub struct PcbBoardCommand {
    pub state: IcyBoardState,
    pub display_menu: bool,

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
        }
    }

    pub fn do_command(&mut self) -> Res<()> {
        self.state.set_activity(UserActivity::BrowseMenu);
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
        if command.len() > 5 {
            self.saved_cmd = command.clone();
        }
        if command.is_empty() {
            return Ok(());
        }

        self.state.session.push_tokens(&command);
        let command = self.state.session.tokens.pop_front().unwrap();

        if let Some(action) = self.state.try_find_command(&command) {
            return self.dispatch_action(&command, &action);
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

    fn dispatch_action(&mut self, command: &str, action: &Command) -> Res<()> {
        if !self.check_sec(command, &action.security)? {
            return Ok(());
        }

        match action.command_type {
            CommandType::RedisplayCommand => {
                // !
                self.redisplay_cmd()?;
            }
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
            CommandType::Bye => {
                // BYE
                self.bye_cmd()?;
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
            CommandType::UploadFile => {
                // U
                self.upload_file(action)?;
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
            CommandType::PersonalMail => {
                // Y
                self.personal_mail(action)?;
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
                // NEWS
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
            CommandType::WhoIsOnline => {
                // WHO
                self.who_display_nodes(action)?;
            }

            CommandType::OpenDoor => {
                // DOOR/OPEN
                self.open_door(action)?;
            }

            CommandType::RestoreMessage => {
                // 4
                self.restore_message(action)?;
            }

            CommandType::ReadEmail => {
                // @
                self.read_email(action)?;
            }
            CommandType::RunPPE => {
                // PPE
                self.ppe_run()?;
            }

            CommandType::TextSearch => {
                // TS
                self.text_search(action)?;
            }

            _ => {
                return Err(Box::new(IcyBoardError::UnknownAction(format!("{:?}", action.command_type))));
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

        self.state.display_file_with_error(&file, false)
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
