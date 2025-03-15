use std::{
    collections::{HashMap, VecDeque},
    fs,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use crate::{Res, executable::Executable, search_patterns::PatternExpr, vm::expressions::fix_casing};
use async_recursion::async_recursion;
use chrono::{DateTime, Local, Utc};
use codepages::tables::UNICODE_TO_CP437;
use dizbase::file_base::FileBase;
use icy_engine::{OutputFormat, SaveOptions, ScreenPreperation, ansi};
use icy_engine::{Position, ansi::constants::COLOR_OFFSETS};
use icy_engine::{TextAttribute, TextPane};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection, iemsi::EmsiICI, termcap_detect::TerminalCaps, terminal::virtual_screen::VirtualScreen};
use regex::Regex;
use tokio::{sync::Mutex, time::sleep};

use crate::{
    icy_board::IcyBoardError,
    vm::{DiskIO, TerminalTarget, run},
};
pub mod functions;
pub mod menu_runner;
pub mod user_commands;
use self::functions::display_flags;

use super::{
    IcyBoard,
    bbs::{BBS, BBSMessage},
    commands::{AutoRun, Command, CommandAction, CommandType},
    conferences::Conference,
    icb_config::{DEFAULT_PCBOARD_DATE_FORMAT, IcbColor, SysopCommandLevels, UserCommandLevels},
    icb_text::{IcbTextFile, IcbTextStyle, IceText},
    macro_parser::{Macro, MacroCommand},
    security_expr::SecurityExpression,
    user_base::{ConferenceFlags, FSEMode, Password, User},
};

#[derive(Clone, Copy, PartialEq, Default)]
pub enum GraphicsMode {
    // No graphics or ansi codes
    Ctty,
    // Ansi codes - without colors
    Ansi,
    // Ansi codes + color codes
    #[default]
    Graphics,
    // Avatar codes + color codes
    Avatar,
    Rip,
}

#[derive(Clone)]
pub struct DisplayOptions {
    /// If true, the more prompt is automatically answered after 10 seconds.
    pub auto_more: bool,

    pub count_lines: bool,

    pub grapics_mode: GraphicsMode,

    ///  flag indicating whether or not the user aborted the display of data via ^K / ^X or answering no to a MORE? prompt
    pub abort_printout: bool,
    ///  flag if last printout was aborted
    pub was_aborted: bool,

    pub display_text: bool,
    pub show_on_screen: bool,

    pub in_file_list: Option<PathBuf>,

    // Enable CTRL-X / CTRL-K checking for display_files
    pub allow_break: bool,

    /// If current command should be in non-stop mode
    pub non_stop_during_cmd: bool,

    /// If last printout was in non-stop mode
    pub was_non_stop: bool,

    pub num_lines_printed: usize,
}

impl DisplayOptions {
    pub fn force_count_lines(&mut self) {
        self.count_lines = true;
        self.num_lines_printed = 0;
    }
    pub fn force_non_stop(&mut self) {
        self.count_lines = false;
        self.num_lines_printed = 0;
    }

    pub fn no_change(&mut self) {
        if self.non_stop_during_cmd {
            self.count_lines = false;
        } else {
            self.count_lines = true;
            self.num_lines_printed = 0;
        }
    }

    pub fn check_display_status(&mut self) {
        if self.non_stop_during_cmd {
            self.non_stop_during_cmd = false;
            self.was_non_stop = true;
        }
        if self.abort_printout {
            self.abort_printout = false;
            self.was_aborted = true;
        }
        self.auto_more = false;
    }
}

impl Default for DisplayOptions {
    fn default() -> Self {
        Self {
            auto_more: false,
            abort_printout: false,
            grapics_mode: GraphicsMode::Graphics,
            display_text: true,
            show_on_screen: true,
            in_file_list: None,
            allow_break: true,
            non_stop_during_cmd: false,
            was_non_stop: false,
            was_aborted: false,
            count_lines: true,
            num_lines_printed: 0,
        }
    }
}

#[derive(Clone, Default)]
pub struct TransferStatistics {
    pub downloaded_files: usize,
    pub downloaded_bytes: usize,
    pub downloaded_cps: usize,

    pub uploaded_files: usize,
    pub uploaded_bytes: usize,
    pub uploaded_cps: usize,
}

impl TransferStatistics {
    pub fn get_cps_both(&self) -> usize {
        (self.downloaded_cps + self.uploaded_cps) / 2
    }
}

#[derive(Clone)]
pub struct Session {
    pub disp_options: DisplayOptions,
    pub current_conference_number: u16,
    pub current_message_area: usize,
    pub current_file_directory: usize,
    pub current_conference: Conference,
    pub caller_number: usize,
    pub is_local: bool,
    pub paged_sysop: bool,

    pub user_command_level: UserCommandLevels,
    pub sysop_command_level: SysopCommandLevels,

    pub login_date: DateTime<Utc>,

    pub current_user: Option<User>,
    pub cur_user_id: i32,
    pub cur_security: u8,
    pub cur_groups: Vec<String>,
    pub language: String,

    pub page_len: u16,

    pub is_sysop: bool,
    pub op_text: String,
    pub use_alias: bool,

    pub last_new_line_y: i32,

    pub request_logoff: bool,

    pub time_limit: i32,
    pub security_violations: i32,

    /// If true, the keyboard timer is checked.
    /// After it's elapsed logoff the user for inactivity.
    pub keyboard_timer_check: bool,

    pub tokens: VecDeque<String>,

    /// Store last password used so that the user doesn't need to re-enter it.
    pub last_password: String,

    pub more_requested: bool,
    pub cancel_batch: bool,

    // needed to copy that for new users.
    pub user_name: String,
    pub alias_name: String,
    pub sysop_name: String,

    pub date_format: String,

    pub cursor_pos: Position,

    pub yes_char: char,
    pub no_char: char,
    pub yes_no_mask: String,

    pub fse_mode: FSEMode,

    // Used in @X00 macros to save color, to restore it with @XFF
    pub saved_color: IcbColor,

    pub emsi: Option<EmsiICI>,

    pub bytes_remaining: i64,

    // The maximum number of files in flagged_files
    pub batch_limit: usize,
    pub flagged_files: Vec<PathBuf>,

    /// The current message number read (used for @CURMSGNUM@ macro)
    pub current_messagenumber: u32,
    pub high_msg_num: u32,
    pub low_msg_num: u32,
    pub last_msg_read: u32,
    pub highest_msg_read: u32,

    pub term_caps: TerminalCaps,

    pub search_pattern: Option<Regex>,

    /// The current default answer on last input_string
    pub default_answer: Option<String>,
    pub last_answer: Option<String>,

    pub memorized_msg: Option<(usize, u32)>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            user_command_level: UserCommandLevels::default(),
            sysop_command_level: SysopCommandLevels::default(),
            disp_options: DisplayOptions::default(),
            current_conference_number: 0,
            current_conference: Conference::default(),
            login_date: Utc::now(),
            current_user: None,
            cur_user_id: -1,
            cur_security: 0,
            caller_number: 0,
            cur_groups: Vec::new(),
            security_violations: 0,
            current_message_area: 0,
            current_file_directory: 0,
            last_new_line_y: 0,
            page_len: 24,
            is_sysop: false,
            is_local: false,
            op_text: String::new(),
            use_alias: false,
            time_limit: 1000,
            keyboard_timer_check: false,
            request_logoff: false,
            tokens: VecDeque::new(),
            last_password: String::new(),
            more_requested: false,
            cancel_batch: false,
            fse_mode: FSEMode::Yes,
            user_name: String::new(),
            alias_name: String::new(),
            date_format: DEFAULT_PCBOARD_DATE_FORMAT.to_string(),
            cursor_pos: Position::default(),
            language: String::new(),
            yes_char: 'Y',
            no_char: 'N',
            yes_no_mask: "YyNn".to_string(),
            saved_color: IcbColor::Dos(7),

            sysop_name: "SYSOP".to_string(),
            flagged_files: Vec::new(),
            emsi: None,
            paged_sysop: false,
            bytes_remaining: 0,

            // Seems to be hardcoded in PCBoard
            batch_limit: 30,
            current_messagenumber: 0,
            high_msg_num: 0,
            low_msg_num: 0,
            last_msg_read: 0,
            highest_msg_read: 0,
            term_caps: TerminalCaps::LOCAL,
            search_pattern: None,
            default_answer: None,
            last_answer: None,
            memorized_msg: None,
        }
    }

    pub fn expert_mode(&self) -> bool {
        if let Some(user) = &self.current_user { user.flags.expert_mode } else { false }
    }

    pub fn push_tokens(&mut self, command: &str) -> usize {
        let mut res = 0;
        for cmd in crate::tokens::tokenize(command) {
            self.tokens.push_back(cmd.to_string());
            res += 1;
        }
        self.disp_options.non_stop_during_cmd = false;
        self.disp_options.no_change();
        res
    }

    pub fn get_username_or_alias(&self) -> String {
        if self.use_alias && self.current_conference.allow_aliases {
            self.alias_name.clone()
        } else {
            self.user_name.clone()
        }
    }

    pub fn get_first_name(&self) -> String {
        if let Some(idx) = self.user_name.find(' ') {
            self.user_name[..idx].to_string()
        } else {
            self.user_name.clone()
        }
    }

    pub fn get_last_name(&self) -> String {
        if let Some(idx) = self.user_name.find(' ') {
            self.user_name[idx + 1..].to_string()
        } else {
            String::new()
        }
    }

    pub fn minutes_left(&self) -> i32 {
        self.time_limit
    }
    pub fn seconds_left(&self) -> i32 {
        self.time_limit * 60
    }

    pub(crate) fn calculate_balance(&self) -> f64 {
        // TODO implement balance calculation
        0.0
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy)]
pub enum NodeStatus {
    NoCaller,
    Available,
    RunningDoor,
    EnterMessage,
    GroupChat,
    HandlingMail,
    LogoffPending,
    NodeMessage,
    RunningEvent,
    LogIntoSystem,
    PagingSysop,
    ChatWithSysop,
    RecycleBBS,
    TakeSurvey,
    Transfer,
    Unavailable,
    DropDOSDelayed,
    DropDOSNow,
    ReadBulletins,
}

impl NodeStatus {
    pub fn to_char(&self) -> char {
        match self {
            NodeStatus::NoCaller => ' ',
            NodeStatus::Available => 'A',
            NodeStatus::RunningDoor => 'D',
            NodeStatus::EnterMessage => 'E',
            NodeStatus::GroupChat => 'G',
            NodeStatus::HandlingMail => 'H',
            NodeStatus::LogoffPending => 'L',
            NodeStatus::NodeMessage => 'M',
            NodeStatus::RunningEvent => 'N',
            NodeStatus::LogIntoSystem => 'O',
            NodeStatus::PagingSysop => 'P',
            NodeStatus::ChatWithSysop => 'C',
            NodeStatus::RecycleBBS => 'R',
            NodeStatus::TakeSurvey => 'S',
            NodeStatus::Transfer => 'T',
            NodeStatus::Unavailable => 'U',
            NodeStatus::DropDOSDelayed => 'W',
            NodeStatus::DropDOSNow => 'X',
            NodeStatus::ReadBulletins => 'B',
        }
    }
    pub fn from_char(ch: char) -> Option<Self> {
        match ch.to_ascii_uppercase() {
            ' ' => Some(NodeStatus::NoCaller),
            'A' => Some(NodeStatus::Available),
            'D' => Some(NodeStatus::RunningDoor),
            'E' => Some(NodeStatus::EnterMessage),
            'G' => Some(NodeStatus::GroupChat),
            'H' => Some(NodeStatus::HandlingMail),
            'L' => Some(NodeStatus::LogoffPending),
            'M' => Some(NodeStatus::NodeMessage),
            'N' => Some(NodeStatus::RunningEvent),
            'O' => Some(NodeStatus::LogIntoSystem),
            'P' => Some(NodeStatus::PagingSysop),
            'C' => Some(NodeStatus::ChatWithSysop),
            'R' => Some(NodeStatus::RecycleBBS),
            'S' => Some(NodeStatus::TakeSurvey),
            'T' => Some(NodeStatus::Transfer),
            'U' => Some(NodeStatus::Unavailable),
            'W' => Some(NodeStatus::DropDOSDelayed),
            'X' => Some(NodeStatus::DropDOSNow),
            'B' => Some(NodeStatus::ReadBulletins),
            _ => None,
        }
    }
}

pub struct NodeState {
    pub sysop_connection: Option<ChannelConnection>,
    pub bbs_channel: Option<tokio::sync::mpsc::Receiver<BBSMessage>>,
    pub cur_user: i32,
    pub cur_conference: u16,
    pub graphics_mode: GraphicsMode,
    pub status: NodeStatus,
    pub operation: String,
    pub enabled_chat: bool,
    pub node_number: usize,
    pub connection_type: ConnectionType,
    pub logon_time: DateTime<Utc>,
    pub handle: Option<thread::JoinHandle<()>>,
}

unsafe impl Send for NodeState {}
unsafe impl Sync for NodeState {}

impl NodeState {
    pub fn new(node_number: usize, connection_type: ConnectionType, rx: tokio::sync::mpsc::Receiver<BBSMessage>) -> Self {
        Self {
            sysop_connection: None,
            bbs_channel: Some(rx),
            status: NodeStatus::NoCaller,
            operation: String::new(),
            graphics_mode: GraphicsMode::Ansi,
            cur_user: -1,
            cur_conference: 0,
            enabled_chat: true,
            node_number,
            connection_type,
            handle: None,
            logon_time: Utc::now(),
        }
    }
}

#[derive(Clone, Debug, Copy, PartialEq)]
pub enum KeySource {
    User,
    StuffedHidden,
    Sysop,
}

pub struct KeyChar {
    pub ch: char,
    pub source: KeySource,
}

impl KeyChar {
    pub fn new(src: KeySource, c: char) -> Self {
        Self { ch: c, source: src }
    }
}

pub struct IcyBoardState {
    root_path: PathBuf,
    pub connection: Box<dyn Connection>,
    pub bbs: Arc<Mutex<BBS>>,
    pub board: Arc<tokio::sync::Mutex<IcyBoard>>,

    pub node_state: Arc<Mutex<Vec<Option<NodeState>>>>,
    pub node: usize,

    pub transfer_statistics: TransferStatistics,

    pub session: Session,

    pub display_text: IcbTextFile,

    /// 0 = no debug, 1 - errors, 2 - errors and warnings, 3 - all
    pub debug_level: i32,
    pub env_vars: HashMap<String, String>,

    user_screen: VirtualScreen,
    sysop_screen: VirtualScreen,

    char_buffer: VecDeque<KeyChar>,

    pub display_current_menu: bool,
    pub autorun_times: HashMap<usize, u64>,
    pub saved_cmd: String,
    pub file_bases: HashMap<PathBuf, Arc<Mutex<FileBase>>>,
}

impl IcyBoardState {
    pub fn display_screen(&self) -> &VirtualScreen {
        if self.session.is_sysop || self.session.cur_user_id < 0 {
            &self.sysop_screen
        } else {
            &self.user_screen
        }
    }
    pub async fn new(
        bbs: Arc<Mutex<BBS>>,
        board: Arc<tokio::sync::Mutex<IcyBoard>>,
        node_state: Arc<Mutex<Vec<Option<NodeState>>>>,
        node: usize,
        connection: Box<dyn Connection>,
    ) -> Self {
        if node > node_state.lock().await.len() {
            panic!("Node number {node} out of range");
        }
        let mut session = Session::new();
        session.user_command_level = board.lock().await.config.user_command_level.clone();
        session.sysop_command_level = board.lock().await.config.sysop_command_level.clone();
        session.caller_number = board.lock().await.statistics.cur_caller_number() as usize;
        session.date_format = board.lock().await.config.board.date_format.clone();
        let display_text: IcbTextFile = board.lock().await.default_display_text.clone();
        let root_path = board.lock().await.root_path.clone();
        let mut p1 = ansi::Parser::default();
        p1.bs_is_ctrl_char = true;
        let mut p2 = ansi::Parser::default();
        p2.bs_is_ctrl_char = true;

        Self {
            root_path,
            bbs,
            board,
            connection,
            node_state,
            node,
            debug_level: 0,
            display_text,
            env_vars: HashMap::new(),
            session,
            transfer_statistics: TransferStatistics::default(),
            user_screen: VirtualScreen::new(p1),
            sysop_screen: VirtualScreen::new(p2),
            char_buffer: VecDeque::new(),

            display_current_menu: true,
            saved_cmd: String::new(),
            autorun_times: HashMap::new(),
            file_bases: HashMap::new(),
        }
    }
    async fn update_language(&mut self) {
        if !self.session.language.is_empty() {
            let lang_file = self.get_board().await.config.paths.icbtext.clone();
            let lang_file = lang_file.with_extension(format!("{}.toml", self.session.language));
            let lang_file = self.resolve_path(&lang_file);

            log::info!("Loading language file: {}", lang_file.display());
            if lang_file.exists() {
                if let Ok(display_text) = IcbTextFile::load(&lang_file) {
                    self.display_text = display_text;
                    return;
                }
            }
        }
        let dt = self.get_board().await.default_display_text.clone();
        self.display_text = dt;
    }
    /// Turns on keyboard check & resets the keyboard check timer.
    pub fn reset_keyboard_check_timer(&mut self) {
        self.session.keyboard_timer_check = true;
    }

    pub fn get_env(&self, key: &str) -> Option<&String> {
        self.env_vars.get(key)
    }

    pub fn set_env(&mut self, key: &str, value: &str) {
        self.env_vars.insert(key.to_string(), value.to_string());
    }

    pub fn remove_env(&mut self, env: &str) {
        self.env_vars.remove(env);
    }

    fn use_graphics(&self) -> bool {
        self.session.disp_options.grapics_mode != GraphicsMode::Ansi && self.session.disp_options.grapics_mode != GraphicsMode::Ctty
    }

    fn check_time_left(&self) {
        // TODO: Check time left.
    }

    pub async fn reset_color(&mut self, target: TerminalTarget) -> Res<()> {
        let color = self.get_board().await.config.color_configuration.default.clone();
        self.set_color(target, color).await
    }

    pub async fn clear_screen(&mut self, target: TerminalTarget) -> Res<()> {
        self.session.disp_options.no_change();
        match self.session.disp_options.grapics_mode {
            GraphicsMode::Ctty | GraphicsMode::Avatar => {
                // form feed character
                self.print(target, "\x0C").await?;
            }
            GraphicsMode::Ansi | GraphicsMode::Graphics => {
                self.print(target, "\x1B[2J\x1B[H").await?;
            }
            GraphicsMode::Rip => {
                // ignore
            }
        }
        Ok(())
    }

    pub async fn clear_line(&mut self, target: TerminalTarget) -> Res<()> {
        if self.use_ansi() {
            self.print(target, "\r\x1B[K").await
        } else {
            // TODO
            Ok(())
        }
    }

    pub async fn clear_eol(&mut self, target: TerminalTarget) -> Res<()> {
        match self.session.disp_options.grapics_mode {
            GraphicsMode::Ctty => {
                let x = self.user_screen.buffer.get_width() - self.user_screen.caret.get_position().x;
                for _ in 0..x {
                    self.print(target, " ").await?;
                }
                for _ in 0..x {
                    self.print(target, "\x08").await?;
                }
                Ok(())
            }
            GraphicsMode::Ansi | GraphicsMode::Graphics | GraphicsMode::Avatar => self.print(target, "\x1B[K").await,
            GraphicsMode::Rip => self.print(target, "\x1B[K").await,
        }
    }

    pub async fn join_conference(&mut self, conference: u16, _quick_join: bool, show_intro: bool) -> Res<()> {
        // todo: display news on join.
        if (conference as usize) < self.get_board().await.conferences.len() {
            self.session.current_conference_number = conference;
            let c = self.get_board().await.conferences[conference as usize].clone();
            self.session.current_conference = c;
            self.session.current_message_area = 0;
            if let Some(state) = self.node_state.lock().await[self.node].as_mut() {
                state.cur_conference = self.session.current_conference_number;
            }
            if let Some(user) = &mut self.session.current_user {
                user.last_conference = conference;
            }

            if self.get_board().await.config.switches.force_intro_on_join || show_intro {
                if self.session.current_conference.intro_file.is_file() {
                    let f = self.session.current_conference.intro_file.clone();
                    self.display_file(&f).await?;
                }
            }
        }
        Ok(())
    }

    #[async_recursion(?Send)]
    async fn next_line(&mut self) -> Res<()> {
        if self.session.disp_options.count_lines {
            self.session.disp_options.num_lines_printed += 1;
        }
        if self.session.page_len > 0 && self.session.disp_options.num_lines_printed > self.session.page_len as usize {
            if self.session.disp_options.abort_printout {
                return Ok(());
            }
            if !self.session.disp_options.count_lines {
                self.session.more_requested = true;
                return Ok(());
            }
            if let Err(err) = self.more_promt().await {
                log::error!("Error in more prompt: {}", err);
            }
        }
        Ok(())
    }

    pub async fn run_ppe<P: AsRef<Path>>(&mut self, file_name: &P, answer_file: Option<&Path>) -> Res<()> {
        match Executable::read_file(&file_name, false) {
            Ok(executable) => {
                self.run_executable(file_name, answer_file, executable).await?;
            }
            Err(err) => {
                log::error!("Error loading PPE {}: {}", file_name.as_ref().display(), err);
                self.session.op_text = format!("{}", err);
                self.display_text(IceText::ErrorLoadingPPE, display_flags::LFBEFORE | display_flags::LFAFTER)
                    .await?;
            }
        }
        // clear all ppe parameters
        self.session.tokens.clear();
        Ok(())
    }

    pub async fn run_executable<P: AsRef<Path>>(&mut self, file_name: &P, answer_file: Option<&Path>, executable: Executable) -> Res<()> {
        self.session.disp_options.no_change();
        let canonicalized_path: PathBuf = file_name.as_ref().canonicalize()?;

        let canonicalized_path = PathBuf::from(adjust_canonicalization(canonicalized_path));
        let parent = canonicalized_path.parent().unwrap().to_str().unwrap().to_string();
        let mut io = DiskIO::new(&parent, answer_file);
        Ok(if let Err(err) = run(&canonicalized_path, &executable, &mut io, self).await {
            log::error!("Error executing PPE {}: {}", canonicalized_path.display(), err);
            self.session.op_text = format!("{}", err);
            self.display_text(IceText::ErrorExecPPE, display_flags::LFBEFORE | display_flags::LFAFTER)
                .await?;
        })
    }

    pub fn stuff_keyboard_buffer(&mut self, value: &str, is_visible: bool) -> Res<()> {
        let in_chars: Vec<char> = value.chars().collect();

        let src = if is_visible { KeySource::User } else { KeySource::StuffedHidden };
        let mut i = 0;
        while i < in_chars.len() {
            let c = in_chars[i];
            i += 1;
            if c == '^' && i < in_chars.len() {
                let next = in_chars[i].to_ascii_uppercase();
                if next >= 'A' && next <= '[' {
                    let ctrl_c = next as u8 - b'@';
                    self.char_buffer.push_back(KeyChar::new(src, ctrl_c as char));
                    i += 1;
                }
            } else {
                self.char_buffer.push_back(KeyChar::new(src, c));
            }
        }
        // self.char_buffer.push_back(KeyChar::new(KeySource::StuffedHidden, '\n'));
        Ok(())
    }

    pub async fn get_pcbdat(&self) -> Res<String> {
        let board = self.get_board().await;
        let path = board.resolve_file(&board.config.paths.tmp_work_path);

        let path = PathBuf::from(path);
        if !path.is_dir() {
            fs::create_dir_all(&path)?;
        }
        let output = path.join("pcboard.dat");

        if let Err(err) = board.export_pcboard(&output) {
            log::error!("Error writing pcbdat.dat file: {}", err);
            return Err(err);
        }
        Ok(output.to_str().unwrap().to_string())
    }

    pub async fn try_find_command(&self, command: &str, via_cmd_list: bool) -> Option<super::commands::Command> {
        let command = command.to_ascii_uppercase();
        if via_cmd_list {
            for cmd in &self.session.current_conference.commands {
                if cmd.keyword == command {
                    return Some(cmd.clone());
                }
            }

            for cmd in &self.get_board().await.commands.commands {
                if cmd.keyword == command {
                    return Some(cmd.clone());
                }
            }

            for cmd in &self.session.current_conference.commands {
                if cmd.keyword.starts_with(&command) {
                    return Some(cmd.clone());
                }
            }

            for cmd in &self.get_board().await.commands.commands {
                if cmd.keyword.starts_with(&command) {
                    return Some(cmd.clone());
                }
            }
        }

        return match command.as_str() {
            "A" => convert_cmd(CommandType::AbandonConference),
            "B" => convert_cmd(CommandType::BulletinList),
            "C" => convert_cmd(CommandType::CommentToSysop),
            "E" => convert_cmd(CommandType::EnterMessage),
            "RM" => convert_cmd(CommandType::ReadMemorizedMessage(0)),
            "RM+" => convert_cmd(CommandType::ReadMemorizedMessage(1)),
            "RM-" => convert_cmd(CommandType::ReadMemorizedMessage(2)),
            "F" => convert_cmd(CommandType::FileDirectory),
            "BD" => convert_cmd(CommandType::BatchDownload),
            "BU" => convert_cmd(CommandType::BatchUpload),
            "G" => convert_cmd(CommandType::Goodbye),
            "?" => convert_cmd(CommandType::Help),
            "I" => convert_cmd(CommandType::InitialWelcome),
            "K" => convert_cmd(CommandType::DeleteMessage),
            "L" => convert_cmd(CommandType::LocateFile),
            "M" => convert_cmd(CommandType::ToggleGraphics),
            "N" => convert_cmd(CommandType::NewFileScan),
            "O" => convert_cmd(CommandType::PageSysop),
            "P" => convert_cmd(CommandType::SetPageLength),
            "Q" => convert_cmd(CommandType::QuickMessageScan),
            "R" => convert_cmd(CommandType::ReadMessages),
            "S" => convert_cmd(CommandType::Survey),
            "T" => convert_cmd(CommandType::SetTransferProtocol),
            "V" => convert_cmd(CommandType::ViewSettings),
            "W" => convert_cmd(CommandType::WriteSettings),
            "X" => convert_cmd(CommandType::ExpertMode),
            "Y" => convert_cmd(CommandType::YourMailScan),
            "Z" => convert_cmd(CommandType::ZippyDirectoryScan),
            "TS" => convert_cmd(CommandType::TextSearch),
            "4" => convert_cmd(CommandType::RestoreMessage),
            "@" => convert_cmd(CommandType::ReadEmail),
            "@W" => convert_cmd(CommandType::WriteEmail),
            _ => {
                if "ALIAS".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::EnableAlias);
                }
                if "BYE".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::Bye);
                }
                if "CHAT".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::GroupChat);
                }
                if "WHO".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::WhoIsOnline);
                }
                if "BROADCAST".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::Broadcast);
                }
                if "HELP".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::Help);
                }
                if "DOOR".starts_with(command.as_str()) || "Open".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::OpenDoor);
                }
                if "DOWN".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::Download);
                }
                if "FLAG".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::FlagFiles);
                }
                if "REPLY".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::ReplyMessage);
                }
                if "JOIN".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::JoinConference);
                }
                if "LANG".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::SetLanguage);
                }
                if "MENU".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::ShowMenu);
                }
                if "NEWS".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::DisplayNews);
                }
                if "PPE".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::RunPPE);
                }

                if "QWK".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::QWK);
                }
                if "MENU".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::ShowMenu);
                }
                if "SELECT".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::SelectConferences);
                }
                if "TEST".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::TestFile);
                }
                if "UPLOAD".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::UploadFile);
                }
                if "USERS".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::UserList);
                }
                if "AREA".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::ChangeMessageArea);
                }
                None
            }
        };
    }

    pub fn resolve_path<P: AsRef<Path>>(&self, file: &P) -> PathBuf {
        if !file.as_ref().is_absolute() {
            return self.root_path.join(file);
        }
        file.as_ref().to_path_buf()
    }

    async fn shutdown_connections(&mut self) {
        let _ = self.connection.shutdown();
        if let Some(state) = self.node_state.lock().await[self.node].as_mut() {
            if let Some(sysop_connection) = &mut state.sysop_connection {
                let _ = sysop_connection.shutdown();
            }
        }
    }

    pub async fn set_current_user(&mut self, user_number: usize) -> Res<()> {
        let old_language = self.session.language.clone();

        self.session.cur_user_id = user_number as i32;
        if let Some(state) = self.node_state.lock().await[self.node].as_mut() {
            state.cur_user = user_number as i32;
            state.graphics_mode = self.session.disp_options.grapics_mode;
        }
        if user_number >= self.get_board().await.users.len() {
            log::error!("User number {} is out of range", user_number);
            return Err(IcyBoardError::UserNumberInvalid(user_number).into());
        }
        let mut user = self.get_board().await.users[user_number].clone();
        user.stats.num_times_on += 1;
        let last_conference: u16 = user.last_conference;
        self.get_board().await.statistics.add_caller(user.get_name().clone());
        self.get_board().await.save_statistics()?;
        if !user.date_format.is_empty() {
            self.session.date_format = user.date_format.clone();
        }
        self.session.language = user.language.clone();
        self.session.cur_security = user.security_level;
        self.session.page_len = user.page_len;
        self.session.user_name = user.get_name().clone();
        self.session.alias_name = user.alias.clone();
        self.session.fse_mode = user.flags.fse_mode.clone();

        self.session.current_user = Some(user);
        if self.session.language != old_language {
            self.update_language().await;
        }
        self.join_conference(last_conference, false, false).await?;
        return Ok(());
    }

    pub async fn save_current_user(&mut self) -> Res<()> {
        let old_language = self.session.language.clone();
        self.session.date_format = if let Some(user) = &self.session.current_user {
            self.session.language = user.language.clone();
            self.session.fse_mode = user.flags.fse_mode.clone();
            if !user.date_format.is_empty() {
                user.date_format.clone()
            } else {
                self.session.date_format.clone()
            }
        } else {
            self.session.date_format.clone()
        };
        if self.session.language != old_language {
            self.update_language().await;
        }

        if let Some(user) = &mut self.session.current_user {
            let login_date = self.session.login_date.to_utc();
            if user.stats.last_on.date_naive() != login_date.date_naive() {
                user.stats.minutes_today = 0;
            }
            user.stats.minutes_today += (Utc::now() - login_date).num_minutes() as u16;

            user.stats.last_on = login_date;
        }

        if let Some(user) = &self.session.current_user {
            let mut board = self.get_board().await;
            for u in 0..board.users.len() {
                if board.users[u].get_name() == user.get_name() {
                    board.users[u] = user.clone();
                    board.save_userbase()?;
                    return Ok(());
                }
            }
        }
        log::error!("User not found in user list");
        Ok(())
    }

    fn find_more_specific_file(&self, base_name: String) -> PathBuf {
        if let Some(result) = self.find_more_specific_file_with_graphics(base_name.clone() + self.session.cur_security.to_string().as_str()) {
            return result;
        }
        if let Some(result) = self.find_more_specific_file_with_graphics(base_name.clone()) {
            return result;
        }

        PathBuf::from(base_name)
    }

    fn find_more_specific_file_with_graphics(&self, base_name: String) -> Option<PathBuf> {
        if self.session.disp_options.grapics_mode == GraphicsMode::Rip {
            if let Some(result) = self.find_more_specific_file_with_language(base_name.clone() + "r") {
                return Some(result);
            }
        }
        if self.session.disp_options.grapics_mode == GraphicsMode::Avatar {
            if let Some(result) = self.find_more_specific_file_with_language(base_name.clone() + "v") {
                return Some(result);
            }
        }
        if self.session.disp_options.grapics_mode != GraphicsMode::Ctty {
            if let Some(result) = self.find_more_specific_file_with_language(base_name.clone() + "g") {
                return Some(result);
            }
        }

        self.find_more_specific_file_with_language(base_name)
    }

    fn find_more_specific_file_with_language(&self, base_name: String) -> Option<PathBuf> {
        if !self.session.language.is_empty() {
            let lang_file = base_name.clone() + "." + self.session.language.as_str();
            if let Some(result) = self.find_file_with_extension(&lang_file) {
                return Some(result);
            }
        }
        self.find_file_with_extension(&base_name)
    }

    fn find_file_with_extension(&self, lang_file: &String) -> Option<PathBuf> {
        if self.session.disp_options.grapics_mode == GraphicsMode::Rip {
            let file = PathBuf::from(lang_file.clone() + ".rip");
            if file.exists() {
                return Some(file);
            }
        }

        if self.session.disp_options.grapics_mode == GraphicsMode::Graphics {
            let file = PathBuf::from(lang_file.clone() + ".ans");
            if file.exists() {
                return Some(file);
            }

            let file = PathBuf::from(lang_file.clone() + ".avt");
            if file.exists() {
                return Some(file);
            }
        }

        let file = PathBuf::from(lang_file.clone() + ".pcb");
        if file.exists() {
            return Some(file);
        }

        let file = PathBuf::from(lang_file.clone() + ".asc");
        if file.exists() {
            return Some(file);
        }

        let file = PathBuf::from(lang_file);
        if file.exists() {
            return Some(file);
        }

        None
    }

    pub async fn set_activity(&self, node_status: NodeStatus) {
        let text = match node_status {
            NodeStatus::LogIntoSystem => IceText::LogIntoSystem,
            NodeStatus::Available => IceText::Available,
            NodeStatus::Unavailable => IceText::Unavailable,
            NodeStatus::EnterMessage => IceText::EnterMessage,

            NodeStatus::Transfer => IceText::Transfer,

            NodeStatus::HandlingMail => IceText::HandlingMail,
            NodeStatus::TakeSurvey => IceText::AnswerSurvey,

            NodeStatus::ReadBulletins => IceText::HandlingMail,

            NodeStatus::RunningDoor => IceText::InADOOR,
            NodeStatus::ChatWithSysop => IceText::ChatWithSysop,
            NodeStatus::GroupChat => IceText::GroupChat,
            NodeStatus::PagingSysop => IceText::PagingSysop,
            NodeStatus::LogoffPending => IceText::LogoffPending,
            NodeStatus::NoCaller => IceText::NoCaller,
            NodeStatus::NodeMessage => IceText::ReceivedMessage,
            NodeStatus::RunningEvent => IceText::RunningEvent,
            NodeStatus::RecycleBBS => IceText::RecycleBBS,
            NodeStatus::DropDOSDelayed => IceText::DropDOSDelayed,
            NodeStatus::DropDOSNow => IceText::DropDOSNow,
        };
        let txt = self.display_text.get_display_text(text).unwrap();
        if let Some(state) = self.node_state.lock().await[self.node].as_mut() {
            state.operation = txt.text;
            state.status = node_status;
        } else {
            log::error!("Node {} not found", self.node);
        }
    }

    pub async fn set_grapics_mode(&mut self, mode: GraphicsMode) {
        if let Some(state) = self.node_state.lock().await[self.node].as_mut() {
            state.graphics_mode = mode;
        } else {
            log::error!("Node {} not found", self.node);
        }
        self.session.disp_options.grapics_mode = mode;
    }

    /// Gives back the user password, or 'SECRET' if the user password should not be given to doors.
    pub async fn door_user_password(&self) -> String {
        if self.get_board().await.config.options.give_user_password_to_doors {
            if let Some(user) = &self.session.current_user {
                return user.password.password.to_string();
            }
        }

        "SECRET".to_string()
    }

    pub async fn get_board(&self) -> tokio::sync::MutexGuard<IcyBoard> {
        self.board.lock().await
    }

    pub async fn broadcast(&self, lonode: u16, hinode: u16, message: &str) -> Res<()> {
        for i in lonode..=hinode {
            if i == self.node as u16 {
                continue;
            }
            if let Some(Some(channel)) = self.bbs.lock().await.bbs_channels.get(i as usize) {
                let _ = channel.send(BBSMessage::Broadcast(message.to_string())).await;
            }
        }
        Ok(())
    }

    pub async fn page_sysop(&mut self) -> Res<()> {
        self.session.paged_sysop = true;
        self.display_text(IceText::Paging, display_flags::LFBEFORE).await?;

        for _i in 0..15 {
            self.print(TerminalTarget::Both, ".").await?;
            self.bell().await?;
            let i = Instant::now();
            loop {
                if i.elapsed().as_secs() >= 1 {
                    break;
                }
                let Some(ch) = self.get_char(TerminalTarget::Both).await? else {
                    continue;
                };
                if ch.ch == '\x1b' || ch.ch as u32 == 11 {
                    self.new_line().await?;
                    return Ok(());
                }
                if ch.source == KeySource::Sysop {
                    self.chat().await?;
                    self.display_text(IceText::SysopChatEnded, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    async fn chat(&mut self) -> Res<()> {
        self.display_text(IceText::SysopChatActive, display_flags::NEWLINE | display_flags::LFBEFORE)
            .await?;
        self.session.paged_sysop = false;

        loop {
            let Some(ch) = self.get_char(TerminalTarget::Both).await? else {
                sleep(Duration::from_millis(50)).await;
                continue;
            };
            if ch.ch == '\n' || ch.ch == '\r' {
                self.new_line().await?;
                continue;
            }
            if ch.ch as u8 == 8 {
                self.print(TerminalTarget::Both, "\x08 \x08").await?;
                continue;
            }
            if ch.ch == '\x1b' || ch.ch as u32 == 11 {
                return Ok(());
            }
            if ch.source == KeySource::Sysop {
                self.set_color(TerminalTarget::Both, IcbColor::dos_light_green()).await?;
            } else {
                self.reset_color(TerminalTarget::Both).await?;
            }
            self.print(TerminalTarget::Both, &ch.ch.to_string()).await?;
        }
    }

    pub fn search_init(&mut self, pattern: String, case_sensitive: bool) -> bool {
        match PatternExpr::parse(&pattern) {
            Ok(pattern) => {
                let mut pattern = pattern.to_regex();
                if !case_sensitive {
                    pattern = format!("(?i){}", pattern);
                }
                if let Ok(r) = Regex::new(&pattern) {
                    self.session.search_pattern = Some(r);
                    return true;
                } else {
                    log::error!("Error parsing search pattern: {}", pattern);
                }
            }
            Err(err) => log::error!("Error parsing search pattern: {}", err),
        }
        return false;
    }

    pub fn stop_search(&mut self) {
        self.session.search_pattern = None;
    }

    pub fn is_lockedout(&self, conf_number: u16) -> bool {
        if let Some(user) = &self.session.current_user {
            if let Some(flags) = user.conference_flags.get(&(conf_number as usize)) {
                if flags.contains(ConferenceFlags::Expired | ConferenceFlags::Registered) {
                    return true;
                }
            }
        }
        false
    }

    pub fn is_registered(&self, conference: &Conference, conf_number: u16) -> bool {
        if self.session.current_conference_number == conf_number || self.session.is_sysop {
            return true;
        }

        if conf_number == 0 && !self.session.user_command_level.cmd_a.session_can_access(&self.session) {
            return false;
        }

        if self.is_lockedout(conf_number) {
            return false;
        }

        if let Some(user) = &self.session.current_user {
            if let Some(flags) = user.conference_flags.get(&(conf_number as usize)) {
                if flags.contains(ConferenceFlags::UserSelected) {
                    return true;
                }
            }
        }

        if conference.is_public {
            return true;
        }

        false
    }

    async fn show_dir_menu(&mut self) -> Res<()> {
        let mnu: PathBuf = self.session.current_conference.dir_menu.clone();
        self.display_menu(&mnu).await?;
        self.session.disp_options.num_lines_printed = 0;
        Ok(())
    }

    async fn show_area_menu(&mut self) -> Res<()> {
        let mnu = self.session.current_conference.area_menu.clone();
        self.display_menu(&mnu).await?;
        self.session.disp_options.num_lines_printed = 0;
        Ok(())
    }
}

#[derive(PartialEq)]
enum PcbState {
    Default,
    GotAt,
    ReadColor1,
    ReadColor2(char),
    ReadAtSequence(String),
}

impl IcyBoardState {
    pub fn use_ansi(&self) -> bool {
        true
    }

    pub fn is_sysop(&self) -> bool {
        self.session.is_sysop
    }

    pub fn get_bps(&self) -> i32 {
        115_200
    }

    /// # Errors
    pub async fn gotoxy(&mut self, target: TerminalTarget, x: i32, y: i32) -> Res<()> {
        match self.session.disp_options.grapics_mode {
            GraphicsMode::Ctty => {
                // ignore
            }
            GraphicsMode::Ansi | GraphicsMode::Graphics => {
                self.print(target, &format!("\x1B[{};{}H", y, x)).await?;
            }
            GraphicsMode::Avatar => {
                self.print(target, &format!("\x16\x08{}{}", (x as u8) as char, (y as u8) as char)).await?;
            }
            GraphicsMode::Rip => {
                self.print(target, &format!("\x1B[{};{}H", y, x)).await?;
            }
        }

        Ok(())
    }

    pub async fn backward(&mut self, chars: i32) -> Res<()> {
        if self.use_ansi() {
            self.write_raw(TerminalTarget::Both, format!("\x1B[{}D", chars).chars().collect::<Vec<char>>().as_slice())
                .await
        } else {
            Ok(())
        }
    }

    pub async fn forward(&mut self, chars: i32) -> Res<()> {
        if self.use_ansi() {
            self.write_raw(TerminalTarget::Both, format!("\x1B[{}C", chars).chars().collect::<Vec<char>>().as_slice())
                .await
        } else {
            Ok(())
        }
    }

    pub async fn up(&mut self, chars: i32) -> Res<()> {
        if self.use_ansi() {
            self.write_raw(TerminalTarget::Both, format!("\x1B[{}A", chars).chars().collect::<Vec<char>>().as_slice())
                .await
        } else {
            Ok(())
        }
    }

    pub async fn down(&mut self, chars: i32) -> Res<()> {
        if self.use_ansi() {
            self.write_raw(TerminalTarget::Both, format!("\x1B[{}B", chars).chars().collect::<Vec<char>>().as_slice())
                .await
        } else {
            Ok(())
        }
    }

    /// # Errors
    pub async fn print(&mut self, target: TerminalTarget, str: &str) -> Res<()> {
        self.write_raw(target, str.chars().collect::<Vec<char>>().as_slice()).await
    }

    pub async fn print_found_text(&mut self, target: TerminalTarget, str: &str) -> Res<()> {
        let chars = str.chars().collect::<Vec<char>>();
        if let Some(regex) = &self.session.search_pattern.clone() {
            if let Some(find) = regex.find(str) {
                self.write_raw(target, &chars[..find.start()]).await?;
                let old_color = self.user_screen.caret.get_attribute();
                if old_color.get_background() == 0 {
                    self.set_color(target, IcbColor::Dos(0x70)).await?;
                } else {
                    self.set_color(target, IcbColor::Dos(0x07)).await?;
                }
                self.write_raw(target, &chars[find.start()..find.end()]).await?;
                self.set_color(target, IcbColor::Dos(old_color.as_u8(icy_engine::IceMode::Blink))).await?;
                self.write_raw(target, &chars[find.end()..]).await?;
                return Ok(());
            }
        }

        self.write_raw(target, chars.as_slice()).await
    }

    pub async fn println(&mut self, target: TerminalTarget, str: &str) -> Res<()> {
        let line = str.chars().collect::<Vec<char>>();
        self.write_raw(target, line.as_slice()).await?;
        self.new_line().await
    }

    async fn write_chars(&mut self, target: TerminalTarget, data: &[char]) -> Res<()> {
        let mut user_bytes = Vec::new();
        let mut sysop_bytes = Vec::new();
        for c in data {
            if target != TerminalTarget::Sysop || self.session.is_sysop || self.session.current_user.is_none() {
                let _ = self.user_screen.print_char(*c);
                if let Some(&cp437) = UNICODE_TO_CP437.get(&c) {
                    user_bytes.push(cp437);
                } else {
                    user_bytes.push(b'.');
                }
            }
            if target != TerminalTarget::User {
                let _ = self.sysop_screen.print_char(*c);
                if let Some(&cp437) = UNICODE_TO_CP437.get(&c) {
                    sysop_bytes.push(cp437);
                } else {
                    sysop_bytes.push(b'.');
                }
            }
            if *c == '\n' {
                self.write_chars_internal(target, &user_bytes, &sysop_bytes).await?;
                user_bytes.clear();
                sysop_bytes.clear();
                self.next_line().await?;
            }
        }
        self.write_chars_internal(target, &user_bytes, &sysop_bytes).await?;
        Ok(())
    }

    async fn write_chars_internal(&mut self, target: TerminalTarget, user_bytes: &Vec<u8>, sysop_bytes: &Vec<u8>) -> Res<()> {
        if target != TerminalTarget::Sysop || self.session.is_sysop {
            if !user_bytes.is_empty() {
                self.connection.send(&user_bytes).await?;
            }
        }

        if target != TerminalTarget::User {
            if !sysop_bytes.is_empty() {
                // Send user only not to other connections
                let mut node_state = self.node_state.lock().await;
                match node_state[self.node].as_mut() {
                    Some(ns) => {
                        if let Some(sysop_connection) = &mut ns.sysop_connection {
                            let _ = sysop_connection.send(&sysop_bytes).await;
                        }
                    }
                    None => {
                        log::error!("Node {} was empty", self.node);
                    }
                }
            }
        }
        match target {
            TerminalTarget::Both | TerminalTarget::User => {
                if !user_bytes.is_empty() {
                    self.session.cursor_pos = self.user_screen.caret.get_position();
                }
            }
            TerminalTarget::Sysop => {
                if !sysop_bytes.is_empty() {
                    self.session.cursor_pos = self.sysop_screen.caret.get_position();
                }
            }
        }
        Ok(())
    }

    /// # Errors
    #[async_recursion(?Send)]
    pub async fn write_raw(&mut self, target: TerminalTarget, data: &[char]) -> Res<()> {
        if self.session.disp_options.display_text {
            let mut state = PcbState::Default;

            for c in data {
                if *c == '\x1A' {
                    break;
                }
                match state {
                    PcbState::Default => {
                        if *c == '@' {
                            state = PcbState::GotAt;
                        } else {
                            self.write_chars(target, &[*c]).await?;
                        }
                    }
                    PcbState::GotAt => {
                        if *c == 'X' || *c == 'x' {
                            state = PcbState::ReadColor1;
                        } else if *c == '@' {
                            self.write_chars(target, &[*c]).await?;
                            state = PcbState::GotAt;
                        } else {
                            state = PcbState::ReadAtSequence(c.to_string());
                        }
                    }
                    PcbState::ReadAtSequence(s) => {
                        if c.is_whitespace() {
                            self.write_chars(target, &['@']).await?;
                            self.write_chars(target, s.chars().collect::<Vec<char>>().as_slice()).await?;
                            state = PcbState::Default;
                        } else if *c == '@' {
                            state = PcbState::Default;
                            if let Ok(pm) = Macro::from_str(&s) {
                                if let Some(output) = self.run_macro(target, pm).await {
                                    self.write_chars(target, output.chars().collect::<Vec<char>>().as_slice()).await?;
                                }
                            } else {
                                self.write_chars(target, &['@']).await?;
                                self.write_chars(target, s.chars().collect::<Vec<char>>().as_slice()).await?;
                                state = PcbState::GotAt;
                            }
                        } else {
                            state = PcbState::ReadAtSequence(s + c.to_string().as_str());
                        }
                    }
                    PcbState::ReadColor1 => {
                        if c.is_ascii_hexdigit() {
                            state = PcbState::ReadColor2(*c);
                        } else {
                            self.write_chars(target, &['@', *c]).await?;
                            state = PcbState::Default;
                        }
                    }
                    PcbState::ReadColor2(ch1) => {
                        state = PcbState::Default;
                        if !c.is_ascii_hexdigit() {
                            self.write_chars(target, &['@', ch1, *c]).await?;
                        } else {
                            let color = (c.to_digit(16).unwrap() | (ch1.to_digit(16).unwrap() << 4)) as u8;

                            if color == 0 {
                                self.session.saved_color = self.cur_color();
                            } else if color == 0xFF {
                                self.set_color(target, self.cur_color()).await?;
                            } else {
                                self.set_color(target, color.into()).await?;
                            }
                        }
                    }
                }
            }

            if state != PcbState::Default {
                match state {
                    PcbState::Default => {}
                    PcbState::GotAt => self.write_chars(target, &['@']).await?,
                    PcbState::ReadColor1 => self.write_chars(target, &['@', *data.last().unwrap()]).await?,
                    PcbState::ReadColor2(ch1) => self.write_chars(target, &['@', ch1, *data.last().unwrap()]).await?,
                    PcbState::ReadAtSequence(s) => {
                        self.write_chars(target, &['@']).await?;
                        self.write_chars(target, s.chars().collect::<Vec<char>>().as_slice()).await?;
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn run_macro(&mut self, target: TerminalTarget, id: Macro) -> Option<String> {
        let mut result = String::new();
        match &id.command {
            MacroCommand::Alias => {
                if let Some(user) = &self.session.current_user {
                    result = user.alias.to_string();
                }
                if result.is_empty() {
                    result = self.session.get_first_name();
                }
            }
            MacroCommand::AutoMore => {
                self.session.disp_options.auto_more = true;
                return None;
            }
            MacroCommand::Beep => {
                let _ = self.bell().await;
                return None;
            }
            MacroCommand::BICPS => result = self.transfer_statistics.get_cps_both().to_string(),
            MacroCommand::BoardName => result = self.get_board().await.config.board.name.to_string(),
            MacroCommand::BPS | MacroCommand::Carrier => result = self.get_bps().to_string(),

            // TODO
            MacroCommand::ByteCredit | MacroCommand::ByteLimit | MacroCommand::ByteRatio | MacroCommand::BytesLeft => {
                // todo
            }

            MacroCommand::City => {
                if let Some(user) = &self.session.current_user {
                    result = user.city_or_state.to_string();
                }
            }
            MacroCommand::ClrEol => {
                let _ = self.clear_eol(target).await;
                return None;
            }
            MacroCommand::Cls => {
                let _ = self.clear_screen(target).await;
                return None;
            }
            MacroCommand::ConfName => result = self.session.current_conference.name.to_string(),
            MacroCommand::ConfNum => result = self.session.current_conference_number.to_string(),

            MacroCommand::CredLeft | MacroCommand::CredNow | MacroCommand::CredStart | MacroCommand::CredUsed => {
                // todo
            }

            MacroCommand::CurMsgNum => {
                result = self.session.current_messagenumber.to_string();
            }

            MacroCommand::DataPhone => {
                if let Some(user) = &self.session.current_user {
                    result = user.bus_data_phone.to_string();
                }
            }
            MacroCommand::DayBytes | MacroCommand::DlBytes | MacroCommand::DlFiles | MacroCommand::Event => {}

            MacroCommand::Delay(delay) => {
                sleep(Duration::from_millis(*delay as u64 * 10)).await;
                return None;
            }
            MacroCommand::ExpDate => {
                if let Some(user) = &self.session.current_user {
                    result = self.format_date(user.exp_date.to_utc_date_time());
                } else {
                    result = "NEVER".to_string();
                }
            }
            MacroCommand::ExpDays => {
                if self.get_board().await.config.subscription_info.is_enabled {
                    if let Some(user) = &self.session.current_user {
                        if user.exp_date.year() != 0 {
                            result  =
                                0.to_string() // TODO
                                               /*
                                               (self.session.login_date.to_julian_date()
                                                   - user.user.reg_exp_date.to_julian_date())
                                               .to_string(),*/
                            ;
                        }
                    }
                }
                if result.is_empty() {
                    let entry = self.display_text.get_display_text(IceText::Unlimited).unwrap();
                    if entry.style != IcbTextStyle::Plain {
                        let _ = self.set_color(target, entry.style.to_color());
                    }
                    result = entry.text;
                }
            }
            MacroCommand::FBytes | MacroCommand::FFiles | MacroCommand::FileCredit | MacroCommand::FileRatio => {}
            MacroCommand::First => {
                result = fix_casing(self.session.get_first_name());
            }

            MacroCommand::FirstU => {
                result = self.session.get_first_name().to_uppercase();
            }
            MacroCommand::FNum => {
                result = (self.session.flagged_files.len() + 1).to_string();
            }
            MacroCommand::FreeSpace => {}
            MacroCommand::GfxMode => {
                result = match self.session.disp_options.grapics_mode {
                    GraphicsMode::Ctty => self.display_text.get_display_text(IceText::GfxModeOff).unwrap().text,
                    GraphicsMode::Ansi => self.display_text.get_display_text(IceText::GfxModeAnsi).unwrap().text,
                    GraphicsMode::Graphics => self.display_text.get_display_text(IceText::GfxModeGraphics).unwrap().text,
                    GraphicsMode::Avatar => self.display_text.get_display_text(IceText::GfxModeAvatar).unwrap().text,
                    GraphicsMode::Rip => self.display_text.get_display_text(IceText::GfxModeRip).unwrap().text,
                };
            }
            MacroCommand::HomePhone => {
                if let Some(user) = &self.session.current_user {
                    result = user.home_voice_phone.to_string();
                }
            }
            MacroCommand::HighMSGNum => result = self.session.high_msg_num.to_string(),
            MacroCommand::IName => {}
            MacroCommand::InConf => {
                if self.session.current_conference_number == 0 {
                    if let Ok(main_board_txt) = self.display_text.get_display_text(IceText::Mainboard) {
                        result = format!("{} ", main_board_txt.text);
                    } else {
                        log::error!("Mainboard text not found");
                    }
                } else {
                    if let Ok(main_board_txt) = self.display_text.get_display_text(IceText::Conference) {
                        result = format!(
                            "{} ({}){} ",
                            self.session.current_conference.name.to_string(),
                            self.session.current_conference_number,
                            main_board_txt.text
                        );
                    } else {
                        log::error!("Conference text not found");
                    }
                }
            }
            MacroCommand::LogDate => result = self.format_date(self.session.login_date),
            MacroCommand::LogTime => result = self.format_time(self.session.login_date),
            MacroCommand::LastDateOn => {
                if let Some(user) = &self.session.current_user {
                    result = self.format_date(user.stats.last_on);
                }
            }
            MacroCommand::LastTimeOn => {
                if let Some(user) = &self.session.current_user {
                    result = self.format_time(user.stats.last_on);
                }
            }
            MacroCommand::LowMsgNum => {
                result = self.session.low_msg_num.to_string();
            }
            MacroCommand::LMR => {
                result = self.session.last_msg_read.to_string();
            }

            MacroCommand::KBLeft
            | MacroCommand::KBLimit
            | MacroCommand::LastCallerNode
            | MacroCommand::LastCallerSystem
            | MacroCommand::MaxBytes
            | MacroCommand::MaxFiles => {}
            MacroCommand::MinLeft => result = "1000".to_string(),
            MacroCommand::More => {
                if let Err(err) = self.more_promt().await {
                    log::error!("Error in more prompt: {}", err);
                }
                return None;
            }
            MacroCommand::MsgLeft => {
                if let Some(user) = &self.session.current_user {
                    result = user.stats.messages_left.to_string();
                }
            }
            MacroCommand::MsgRead => {
                if let Some(user) = &self.session.current_user {
                    result = user.stats.messages_read.to_string();
                }
            }
            MacroCommand::NoChar => result = self.session.no_char.to_string(),
            MacroCommand::Node => result = self.node.to_string(),
            MacroCommand::NumBLT => {
                if let Some(bullettins) = &self.session.current_conference.bulletins {
                    result = bullettins.len().to_string();
                } else {
                    result = "0".to_string();
                }
            }
            MacroCommand::NumCalls => {
                result = self.get_board().await.statistics.total.calls.to_string();
            }
            MacroCommand::NumConf => result = self.get_board().await.conferences.len().to_string(),
            MacroCommand::NumDir => {
                if let Some(dirs) = &self.session.current_conference.directories {
                    result = dirs.len().to_string();
                } else {
                    result = "0".to_string();
                }
            }
            MacroCommand::NumArea => {
                if let Some(areas) = &self.session.current_conference.areas {
                    result = areas.len().to_string();
                } else {
                    result = "0".to_string();
                }
            }
            MacroCommand::DirName => {
                if let Some(dirs) = &self.session.current_conference.directories {
                    result = dirs[self.session.current_file_directory].name.to_string();
                } else {
                    result = String::new();
                }
            }
            MacroCommand::DirNum => {
                result = self.session.current_file_directory.to_string();
            }
            MacroCommand::AreaName => {
                if let Some(areas) = &self.session.current_conference.areas {
                    result = areas[self.session.current_message_area].name.to_string();
                } else {
                    result = String::new();
                }
            }
            MacroCommand::AreaNum => {
                result = (self.session.current_message_area + 1).to_string();
            }
            MacroCommand::NumTimesOn => {
                if let Some(user) = &self.session.current_user {
                    result = user.stats.num_times_on.to_string();
                }
            }
            MacroCommand::OffHours => {}
            MacroCommand::OpText => result = self.session.op_text.to_string(),
            MacroCommand::Pause => {
                self.session.disp_options.auto_more = true;
                let _ = self.press_enter().await;
                self.session.disp_options.auto_more = false;
                return None;
            }
            MacroCommand::POS(value) => {
                let x = self.user_screen.caret.get_position().x as usize;
                while result.len() + x + 1 < *value as usize {
                    result.push(' ');
                }
                return Some(result);
            }
            MacroCommand::POFF => {
                self.session.disp_options.force_non_stop();
                return None;
            }
            MacroCommand::PON => {
                self.session.disp_options.force_count_lines();
                return None;
            }
            MacroCommand::ProLTR => {
                if let Some(user) = &self.session.current_user {
                    result = user.protocol.to_string();
                }
            }
            MacroCommand::ProDesc => {
                if let Some(user) = &self.session.current_user {
                    if let Some(prot) = self.board.lock().await.protocols.find_protocol(&user.protocol) {
                        result = prot.description.to_string();
                    }
                }
            }
            MacroCommand::QOFF => {
                self.session.disp_options.allow_break = false;
                return None;
            }
            MacroCommand::QON => {
                self.session.disp_options.allow_break = true;
                return None;
            }
            MacroCommand::PwxDate | MacroCommand::PwxDays | MacroCommand::RatioBytes | MacroCommand::RatioFiles => {}
            MacroCommand::RCPS => result = self.transfer_statistics.uploaded_cps.to_string(),
            MacroCommand::RBytes => result = self.transfer_statistics.uploaded_bytes.to_string(),
            MacroCommand::RFiles => result = self.transfer_statistics.uploaded_files.to_string(),
            MacroCommand::Real => {
                if let Some(user) = &self.session.current_user {
                    result = user.get_name().to_string();
                }
            }
            MacroCommand::Security => {
                if let Some(user) = &self.session.current_user {
                    result = user.security_level.to_string()
                }
            }
            MacroCommand::SCPS => result = self.transfer_statistics.downloaded_cps.to_string(),
            MacroCommand::SBytes => result = self.transfer_statistics.downloaded_bytes.to_string(),
            MacroCommand::SFiles => result = self.transfer_statistics.downloaded_files.to_string(),
            MacroCommand::SysDate => {
                result = self.format_date(Utc::now());
            }
            MacroCommand::SysopIn => result = self.get_board().await.config.limits.sysop_start.to_string(),
            MacroCommand::SysopOut => result = self.get_board().await.config.limits.sysop_stop.to_string(),
            MacroCommand::SysTime => {
                result = self.format_time(Utc::now());
            }
            MacroCommand::TimeLimit => result = self.session.time_limit.to_string(),
            MacroCommand::TimeLeft => {
                let now = Utc::now();
                let time_on = now - self.session.login_date;
                if self.session.time_limit == 0 {
                    result = "UNLIMITED".to_string();
                } else {
                    result = (self.session.time_limit as i64 - time_on.num_minutes()).to_string();
                }
            }
            MacroCommand::TimeUsed => result = (Utc::now() - self.session.login_date).num_minutes().to_string(),
            MacroCommand::TotalTime => {
                let mut current = (Utc::now() - self.session.login_date).num_minutes();
                if let Some(user) = &self.session.current_user {
                    current += user.stats.minutes_today as i64;
                }
                result = current.to_string();
            }
            MacroCommand::UpBytes => {
                if let Some(user) = &self.session.current_user {
                    result = user.stats.total_upld_bytes.to_string();
                }
            }
            MacroCommand::UpFiles => {
                if let Some(user) = &self.session.current_user {
                    result = user.stats.num_uploads.to_string();
                }
            }
            MacroCommand::User => {
                if let Some(user) = &self.session.current_user {
                    if self.session.use_alias {
                        if user.alias.is_empty() {
                            result = user.get_name().to_ascii_uppercase().to_string();
                        } else {
                            result = user.alias.to_ascii_uppercase().to_string();
                        }
                    } else {
                        result = user.get_name().to_ascii_uppercase().to_string();
                    }
                } else {
                    result = "0".to_string();
                }
            }
            MacroCommand::Wait => {
                let _ = self.press_enter().await;
                return None;
            }
            MacroCommand::Who => {
                let _ = self.who_display_nodes().await;
                return None;
            }
            MacroCommand::XOff => {
                self.session.disp_options.grapics_mode = GraphicsMode::Ansi;
                return None;
            }
            MacroCommand::XON => {
                if !self.get_board().await.config.switches.non_graphics {
                    self.session.disp_options.grapics_mode = GraphicsMode::Graphics;
                }
                return None;
            }
            MacroCommand::YesChar => result = self.session.yes_char.to_string(),
            MacroCommand::Env(id) => {
                if let Some(value) = self.get_env(&id) {
                    result = value.to_string();
                }
            }
            MacroCommand::Hangup => {
                let _ = self.logoff_user(false).await;
                return None;
            }
            MacroCommand::SwitchColor(color) => {
                let _ = self.set_color(target, IcbColor::Dos(*color)).await;
                return None;
            }
        }
        Some(id.format_value(&result))
    }

    /// # Errors
    pub async fn get_char(&mut self, target: TerminalTarget) -> Res<Option<KeyChar>> {
        if let Some(ch) = self.char_buffer.pop_front() {
            match target {
                TerminalTarget::Both => {
                    return Ok(Some(ch));
                }
                TerminalTarget::User => {
                    if ch.source == KeySource::User {
                        return Ok(Some(ch));
                    } else {
                        self.char_buffer.push_back(ch);
                    }
                }
                TerminalTarget::Sysop => {
                    if ch.source == KeySource::Sysop {
                        return Ok(Some(ch));
                    } else {
                        self.char_buffer.push_back(ch);
                    }
                }
            }
        }

        let mut sysop_connection;
        let bbs_channel;
        {
            if let Some(state) = self.node_state.lock().await[self.node].as_mut() {
                sysop_connection = state.sysop_connection.take();
                bbs_channel = state.bbs_channel.take();
            } else {
                log::error!("Node {} not found", self.node);
                return Err(Box::new(IcyBoardError::NodeNotFound(self.node)));
            }
        }

        let mut user_key_data = [0; 1];
        let Some(mut bbs_channel) = bbs_channel else {
            return Ok(None);
        };

        if let Some(mut sysop_connection) = sysop_connection.take() {
            let mut sysop_key_data = [0; 1];
            tokio::select! {
                msg = bbs_channel.recv() => {
                    if let Some(state) = self.node_state.lock().await[self.node].as_mut() {
                        state.bbs_channel = Some(bbs_channel);
                    }
                    match msg {
                        Some(BBSMessage::SysopLogout) => {
                            if let Some(state) = self.node_state.lock().await[self.node].as_mut() {
                                state.sysop_connection = None;
                            }

                        }
                        Some(BBSMessage::SysopLogin) => {
                            self.print_sysop_screen().await?;
                        }
                        Some(BBSMessage::Broadcast(msg)) => {
                            self.show_broadcast(msg).await?;
                        }
                        _ => {}
                    }
                    return Ok(None);
                }
                size = sysop_connection.read(&mut sysop_key_data) => {
                    match size {
                        Ok(1) => {
                            if let Some(state) = self.node_state.lock().await[self.node].as_mut() {
                                state.sysop_connection = Some(sysop_connection);
                                state.bbs_channel = Some(bbs_channel);
                            }
                            if target == TerminalTarget::User {
                                self.char_buffer.push_back(KeyChar::new(KeySource::Sysop, sysop_key_data[0] as char));
                                return Ok(None);
                            }

                            return Ok(Some(KeyChar::new(KeySource::Sysop, sysop_key_data[0] as char)));
                        }
                        Err(_) => {
                        }
                        _ => {}
                    }
                }
                size2 = self.connection.read(&mut user_key_data) => {
                    if let Ok(1) = size2 {
                        if let Some(state) = self.node_state.lock().await[self.node].as_mut() {
                            state.sysop_connection = Some(sysop_connection);
                            state.bbs_channel = Some(bbs_channel);
                        }
                        if target == TerminalTarget::Sysop {
                            self.char_buffer.push_back(KeyChar::new(KeySource::User, user_key_data[0] as char));
                            return Ok(None);
                        }
                        return Ok(Some(KeyChar::new(KeySource::User, user_key_data[0] as char)));
                    }
                }
                _ = sleep(Duration::from_millis(100)) => {
                    if let Some(state) = self.node_state.lock().await[self.node].as_mut() {
                        state.sysop_connection = Some(sysop_connection);
                        state.bbs_channel = Some(bbs_channel);
                    }
                    return Ok(None);
                }
            }
        } else {
            tokio::select! {
                msg = bbs_channel.recv() => {
                    if let Some(state) = self.node_state.lock().await[self.node].as_mut() {
                        state.bbs_channel = Some(bbs_channel);
                    }
                    match msg {
                        Some(BBSMessage::SysopLogout) => {
                            // Ignore
                        }
                        Some(BBSMessage::SysopLogin) => {
                            self.print_sysop_screen().await?;
                        }
                        Some(BBSMessage::Broadcast(msg)) => {
                            self.show_broadcast(msg).await?;
                        }
                        _ => {}
                    }
                    return Ok(None);

                }
                size2 = self.connection.read(&mut user_key_data) => {
                    if let Ok(1) = size2 {
                        if let Some(state) = self.node_state.lock().await[self.node].as_mut() {
                            state.bbs_channel = Some(bbs_channel);
                        }
                        if target == TerminalTarget::Sysop {
                            // No sysop, only user
                            return Ok(None);
                        }
                        return Ok(Some(KeyChar::new(KeySource::User, user_key_data[0] as char)));
                    }
                }
                _ = sleep(Duration::from_millis(100)) => {
                    if let Some(state) = self.node_state.lock().await[self.node].as_mut() {
                        state.bbs_channel = Some(bbs_channel);
                    }
                    return Ok(None);
                }

            }
        }

        thread::sleep(Duration::from_millis(100));
        Ok(None)
    }

    pub async fn get_char_edit(&mut self) -> Res<Option<KeyChar>> {
        let ch = self.get_char(TerminalTarget::Both).await?;
        if ch.is_none() {
            return Ok(None);
        }
        let mut ch: KeyChar = ch.unwrap();
        match ch.ch {
            control_codes::DEL_HIGH => {
                ch.ch = control_codes::DEL;
            }
            '\x1B' => {
                if let Some(key_char) = self.get_char(TerminalTarget::Both).await? {
                    if key_char.ch == '[' {
                        if let Some(key_char) = self.get_char(TerminalTarget::Both).await? {
                            match key_char.ch {
                                'A' => ch.ch = control_codes::UP,
                                'B' => ch.ch = control_codes::DOWN,
                                'C' => ch.ch = control_codes::RIGHT,
                                'D' => ch.ch = control_codes::LEFT,

                                'H' => ch.ch = control_codes::HOME,
                                'K' => ch.ch = control_codes::END,

                                'V' => ch.ch = control_codes::PG_UP,
                                'U' => ch.ch = control_codes::PG_DN,
                                '@' => {
                                    self.get_char(TerminalTarget::Both).await?;
                                    ch.ch = control_codes::INS;
                                }

                                '6' => {
                                    self.get_char(TerminalTarget::Both).await?;
                                    ch.ch = control_codes::PG_UP;
                                }
                                '5' => {
                                    self.get_char(TerminalTarget::Both).await?;
                                    ch.ch = control_codes::PG_DN;
                                }
                                '2' => {
                                    self.get_char(TerminalTarget::Both).await?;
                                    ch.ch = control_codes::INS;
                                }

                                'F' => ch.ch = control_codes::END,

                                _ => {
                                    // don't pass ctrl codes
                                    return Ok(None);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(Some(ch))
    }

    async fn show_broadcast(&mut self, msg: String) -> Res<()> {
        let mut buf = self.user_screen.buffer.flat_clone(false);
        let pos = self.user_screen.caret.get_position();
        self.set_activity(NodeStatus::NodeMessage).await;
        self.new_line().await?;
        self.set_color(TerminalTarget::Both, IcbColor::dos_white()).await?;
        self.println(TerminalTarget::Both, &"Broadcast:").await?;
        self.println(TerminalTarget::Both, &msg).await?;
        self.bell().await?;

        self.press_enter().await?;

        let mut options = SaveOptions::default();
        options.screen_preparation = ScreenPreperation::ClearScreen;
        options.save_sauce = false;
        options.modern_terminal_output = true;
        let res = icy_engine::formats::PCBoard::default().to_bytes(&mut buf, &options)?;
        let res = unsafe { String::from_utf8_unchecked(res) };
        self.print(TerminalTarget::Both, &res).await?;
        self.gotoxy(TerminalTarget::Both, pos.x, pos.y).await?;
        Ok(())
    }

    async fn print_sysop_screen(&mut self) -> Res<()> {
        let mut options = icy_engine::SaveOptions::default();
        options.screen_preparation = icy_engine::ScreenPreperation::ClearScreen;
        options.save_sauce = false;
        options.modern_terminal_output = true;
        let res = icy_engine::formats::PCBoard::default().to_bytes(&mut self.user_screen.buffer, &options)?;
        let res = unsafe { String::from_utf8_unchecked(res) };
        self.print(TerminalTarget::Sysop, &res).await?;
        let p = self.user_screen.caret.get_position();
        self.gotoxy(TerminalTarget::Sysop, p.x + 1, p.y + 1).await?;
        Ok(())
    }

    pub fn inbytes(&mut self) -> i32 {
        self.char_buffer.len() as i32
    }

    pub fn cur_color(&self) -> IcbColor {
        let attr = self.user_screen.caret.get_attribute().as_u8(icy_engine::IceMode::Blink);
        IcbColor::Dos(attr)
    }

    pub async fn set_color(&mut self, target: TerminalTarget, color: IcbColor) -> Res<()> {
        if !self.use_graphics() {
            return Ok(());
        }
        let screen = if target == TerminalTarget::Sysop {
            &mut self.sysop_screen
        } else {
            &mut self.user_screen
        };

        let new_color = match color {
            IcbColor::None => {
                return Ok(());
            }
            IcbColor::Dos(color) => {
                if screen.caret.get_attribute().as_u8(icy_engine::IceMode::Blink) == color {
                    return Ok(());
                }

                TextAttribute::from_u8(color, icy_engine::IceMode::Blink)
            }
            IcbColor::IcyEngine(_fg) => {
                todo!();
            }
        };

        if self.session.disp_options.grapics_mode == GraphicsMode::Avatar {
            if let IcbColor::Dos(color) = color {
                let color_change = format!("\x16\x01{}", color as char);
                return self.write_chars(target, color_change.chars().collect::<Vec<char>>().as_slice()).await;
            }
        }

        let mut color_change = "\x1B[".to_string();
        let was_bold = screen.caret.get_attribute().is_bold();
        let new_bold = new_color.is_bold() || new_color.get_foreground() > 7;
        let mut bg = screen.caret.get_attribute().get_background();
        let mut fg = screen.caret.get_attribute().get_foreground();
        if was_bold != new_bold {
            if new_bold {
                color_change += "1;";
            } else {
                color_change += "0;";
                fg = 7;
                bg = 0;
            }
        }

        if !screen.caret.get_attribute().is_blinking() && new_color.is_blinking() {
            color_change += "5;";
        }

        if fg != new_color.get_foreground() {
            color_change += format!("{};", COLOR_OFFSETS[new_color.get_foreground() as usize % 8] + 30).as_str();
        }

        if bg != new_color.get_background() {
            color_change += format!("{};", COLOR_OFFSETS[new_color.get_background() as usize % 8] + 40).as_str();
        }

        color_change.pop();
        color_change += "m";
        self.write_chars(target, color_change.chars().collect::<Vec<char>>().as_slice()).await
    }

    pub fn get_caret_position(&mut self) -> (i32, i32) {
        (self.session.cursor_pos.x, self.session.cursor_pos.y)
    }

    /// # Errors
    pub async fn goodbye(&mut self) -> Res<()> {
        /*     if HangupType::Hangup != hangup_type {

                    if HangupType::Goodbye == hangup_type {
                        let logoff_script = self
                            .board
                            .lock()
                            .as_ref()
                            .unwrap()
                            .data
                            .paths
                            .logoff_script
                            .clone();
                        self.display_file(&logoff_script)?;
                    }


                }
                self.display_text(IceText::ThanksForCalling, display_flags::LFBEFORE | display_flags::NEWLINE)
                    .await?;
                self.reset_color(TerminalTarget::Both).await?;
        */
        self.hangup().await
    }

    pub async fn hangup(&mut self) -> Res<()> {
        self.session.request_logoff = true;
        self.shutdown_connections().await;
        Ok(())
    }

    pub async fn bell(&mut self) -> Res<()> {
        self.write_raw(TerminalTarget::Both, &['\x07']).await
    }

    pub async fn more_promt(&mut self) -> Res<()> {
        if self.session.request_logoff {
            return Ok(());
        }
        if self.session.disp_options.in_file_list.is_some() {
            self.filebase_more().await?;
            return Ok(());
        }

        loop {
            let result = self
                .input_field(
                    IceText::MorePrompt,
                    12,
                    "YyNnHhSs",
                    "HLPMORE",
                    None,
                    display_flags::UPCASE | display_flags::STACKED | display_flags::ERASELINE,
                )
                .await?;
            self.session.disp_options.no_change();
            match result.as_str() {
                "Y" | "" => {
                    return Ok(());
                }
                "NS" => {
                    self.session.disp_options.force_non_stop();
                    return Ok(());
                }
                "N" => {
                    self.session.disp_options.abort_printout = true;
                    return Ok(());
                }
                _ => {}
            }
        }
    }

    pub async fn press_enter(&mut self) -> Res<()> {
        self.session.more_requested = false;
        self.input_field(IceText::PressEnter, 0, "", "", None, display_flags::ERASELINE).await?;
        Ok(())
    }

    pub async fn new_line(&mut self) -> Res<()> {
        self.write_chars(TerminalTarget::Both, &['\r', '\n']).await
    }

    pub async fn fresh_line(&mut self) -> Res<()> {
        if self.user_screen.caret.get_position().x > 0 {
            self.new_line().await?;
        }
        Ok(())
    }

    pub fn format_date(&self, date_time: DateTime<Utc>) -> String {
        let local_time: DateTime<Local> = date_time.with_timezone(&Local);
        local_time.format(&self.session.date_format).to_string()
    }
    pub fn format_time(&self, date_time: DateTime<Utc>) -> String {
        let local_time = date_time.with_timezone(&Local);
        local_time.format("%H:%M").to_string()
    }

    pub async fn is_valid_password(&self, new_pwd: &str) -> Res<bool> {
        Ok(new_pwd.len() >= self.board.lock().await.config.limits.min_pwd_length as usize)
    }

    pub async fn change_password(&mut self, new_pwd: &str) -> Res<bool> {
        if !self.is_valid_password(new_pwd).await? {
            return Ok(false);
        }
        let exp_days = self.get_board().await.config.limits.password_expire_days;
        if let Some(user) = &mut self.session.current_user {
            let old = user.password.password.clone();
            user.password.password = Password::PlainText(new_pwd.to_string());
            user.password.times_changed = user.password.times_changed.wrapping_add(1);
            user.password.last_change = Utc::now();
            user.password.prev_pwd.push(old);
            while user.password.prev_pwd.len() > 3 {
                user.password.prev_pwd.remove(0);
            }
            if exp_days > 0 {
                user.password.expire_date = Utc::now() + chrono::Duration::days(exp_days as i64);
            }
            self.get_board().await.save_userbase()?;
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn get_filebase(&mut self, dir: &PathBuf, metadata_path: &PathBuf) -> Res<Arc<Mutex<FileBase>>> {
        if let Some(some) = self.file_bases.get(dir) {
            return Ok(some.clone());
        }
        match FileBase::open(&dir, metadata_path) {
            Ok(new_base) => {
                let arc: Arc<Mutex<FileBase>> = Arc::new(Mutex::new(new_base));
                self.file_bases.insert(dir.clone(), arc.clone());
                return Ok(arc);
            }
            Err(err) => {
                log::error!("Could not open file base ({}) : {} ", dir.display(), err);
                self.session.op_text = dir.display().to_string();
                self.display_text(IceText::NotFoundOnDisk, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                Err(err)
            }
        }
    }
}

pub mod control_codes {
    pub const NUL: char = '\x00';
    pub const CTRL_A: char = '\x01';
    pub const CTRL_B: char = '\x02';
    pub const CTRL_C: char = '\x03';
    pub const CTRL_D: char = '\x04';
    pub const CTRL_E: char = '\x05';
    pub const CTRL_F: char = '\x06';
    pub const CTRL_G: char = '\x07';
    pub const CTRL_H: char = '\x08';
    pub const CTRL_I: char = '\x09';
    pub const CTRL_J: char = '\x0A';
    pub const CTRL_K: char = '\x0B';
    pub const CTRL_L: char = '\x0C';
    pub const CTRL_M: char = '\x0D';
    pub const CTRL_N: char = '\x0E';
    pub const CTRL_O: char = '\x0F';
    pub const CTRL_P: char = '\x10';
    pub const CTRL_Q: char = '\x11';
    pub const CTRL_R: char = '\x12';
    pub const CTRL_S: char = '\x13';
    pub const CTRL_T: char = '\x14';
    pub const CTRL_U: char = '\x15';
    pub const CTRL_V: char = '\x16';
    pub const CTRL_W: char = '\x17';
    pub const CTRL_X: char = '\x18';
    pub const CTRL_Y: char = '\x19';
    pub const CTRL_Z: char = '\x1A';
    pub const ESC: char = '\x1B';
    pub const DEL_HIGH: char = '\x7F';

    pub const LEFT: char = CTRL_S;
    pub const RIGHT: char = CTRL_D;
    pub const UP: char = CTRL_E;
    pub const DOWN: char = CTRL_X;

    pub const PG_UP: char = CTRL_R;
    pub const PG_DN: char = CTRL_C;

    pub const DEL: char = CTRL_G;
    pub const BS: char = CTRL_H;
    pub const TAB: char = CTRL_I;

    pub const HOME: char = CTRL_W;
    pub const END: char = CTRL_P;

    pub const INS: char = CTRL_V;

    pub const CTRL_LEFT: char = CTRL_A;
    pub const CTRL_RIGHT: char = CTRL_F;
    pub const CTRL_END: char = CTRL_K;

    pub const RETURN: char = CTRL_M;
}

fn convert_cmd(cmd_type: CommandType) -> Option<Command> {
    Some(Command {
        keyword: "".to_string(),
        display: "".to_string(),
        lighbar_display: "".to_string(),
        help: "".to_string(),
        auto_run: AutoRun::Disabled,
        autorun_time: 0,
        position: Default::default(),
        actions: vec![CommandAction {
            command_type: cmd_type,
            parameter: "".to_string(),
            trigger: Default::default(),
        }],
        security: SecurityExpression::from_req_security(0),
    })
}

#[cfg(not(target_os = "windows"))]
fn adjust_canonicalization<P: AsRef<Path>>(p: P) -> String {
    p.as_ref().display().to_string()
}

#[cfg(target_os = "windows")]
fn adjust_canonicalization<P: AsRef<Path>>(p: P) -> String {
    const VERBATIM_PREFIX: &str = r#"\\?\"#;
    let p = p.as_ref().display().to_string();
    if p.starts_with(VERBATIM_PREFIX) {
        p[VERBATIM_PREFIX.len()..].to_string()
    } else {
        p
    }
}
