use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Alignment,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use crate::{executable::Executable, Res};
use chrono::{DateTime, Datelike, Local, Timelike, Utc};
use codepages::tables::UNICODE_TO_CP437;
use icy_engine::{ansi, Buffer, BufferParser, Caret};
use icy_engine::{ansi::constants::COLOR_OFFSETS, Position};
use icy_engine::{TextAttribute, TextPane};
use icy_net::{channel::ChannelConnection, Connection, ConnectionType};

use crate::{
    icy_board::IcyBoardError,
    vm::{run, DiskIO, TerminalTarget},
};
pub mod functions;

use self::functions::display_flags;

use super::{
    bulletins::BullettinList,
    conferences::Conference,
    icb_config::{IcbColor, DEFAULT_PCBOARD_DATE_FORMAT},
    icb_text::{IcbTextFile, IcbTextStyle, IceText},
    pcboard_data::Node,
    surveys::SurveyList,
    user_base::User,
    IcyBoard, IcyBoardSerializer,
};

#[derive(Clone, Copy, PartialEq)]
pub enum GraphicsMode {
    // No graphics or ansi codes
    Ctty,
    // Ansi codes - color codes on/off is extra
    Ansi,
    // Avatar codes - color codes on/off is extra
    Avatar,
    Rip,
}

#[derive(Clone)]
pub struct DisplayOptions {
    /// If true, the more prompt is automatically answered after 10 seconds.
    pub auto_more: bool,

    /// If true, the output is not paused by the more prompt.
    pub non_stop: bool,

    pub grapics_mode: GraphicsMode,

    pub disable_color: bool,

    ///  flag indicating whether or not the user aborted the display of data via ^K / ^X or answering no to a MORE? prompt
    pub abort_printout: bool,

    pub display_text: bool,
}
impl DisplayOptions {
    pub fn reset_printout(&mut self) {
        self.non_stop = false;
        self.abort_printout = false;
        self.auto_more = false;
    }
}

impl Default for DisplayOptions {
    fn default() -> Self {
        Self {
            auto_more: false,
            abort_printout: false,
            grapics_mode: GraphicsMode::Ansi,
            disable_color: false,
            non_stop: false,
            display_text: true,
        }
    }
}

#[derive(Clone, Default)]
pub struct TransferStatistics {
    pub downloaded_files: usize,
    pub downloaded_bytes: usize,
    pub uploaded_files: usize,
    pub uploaded_bytes: usize,

    pub dl_transfer_time: usize,
    pub ul_transfer_time: usize,
}

impl TransferStatistics {
    pub fn get_cps_download(&self) -> usize {
        if self.dl_transfer_time == 0 {
            return 0;
        }
        self.downloaded_bytes / self.dl_transfer_time
    }

    pub fn get_cps_upload(&self) -> usize {
        if self.ul_transfer_time == 0 {
            return 0;
        }
        self.uploaded_bytes / self.ul_transfer_time
    }

    pub fn get_cps_both(&self) -> usize {
        let total_time = self.dl_transfer_time + self.ul_transfer_time;
        if total_time == 0 {
            return 0;
        }
        // actually correct - it's not the average, but the accumlated csp
        (self.downloaded_bytes + self.uploaded_bytes) / total_time
    }
}

#[derive(Clone)]
pub struct Session {
    pub disp_options: DisplayOptions,
    pub current_conference_number: i32,
    pub current_message_area: usize,
    pub current_file_directory: usize,
    pub current_conference: Conference,
    pub caller_number: usize,
    pub is_local: bool,

    pub login_date: DateTime<Local>,

    pub cur_user: i32,
    pub cur_security: u8,
    pub cur_groups: Vec<String>,
    pub language: String,

    pub page_len: u16,

    pub is_sysop: bool,
    pub op_text: String,
    pub use_alias: bool,

    pub num_lines_printed: usize,
    pub last_new_line_y: i32,

    pub request_logoff: bool,

    pub expert_mode: bool,

    pub time_limit: i32,
    pub security_violations: i32,

    /// If true, the keyboard timer is checked.
    /// After it's elapsed logoff the user for inactivity.
    pub keyboard_timer_check: bool,

    pub tokens: VecDeque<String>,

    /// Store last password used so that the user doesn't need to re-enter it.
    pub last_password: String,

    // Used for dir listing
    pub disable_auto_more: bool,
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

    pub use_fse: bool,

    pub flagged_files: HashSet<PathBuf>,

    // Used in @X00 macros to save color, to restore it with @XFF
    pub saved_color: IcbColor,
}

impl Session {
    pub fn new() -> Self {
        Self {
            disp_options: DisplayOptions::default(),
            current_conference_number: 0,
            current_conference: Conference::default(),
            login_date: Local::now(),
            cur_user: -1,
            cur_security: 0,
            caller_number: 0,
            cur_groups: Vec::new(),
            num_lines_printed: 0,
            security_violations: 0,
            current_message_area: 0,
            current_file_directory: 0,
            last_new_line_y: 0,
            page_len: 24,
            is_sysop: false,
            is_local: false,
            op_text: String::new(),
            expert_mode: false,
            use_alias: false,
            time_limit: 1000,
            keyboard_timer_check: false,
            request_logoff: false,
            tokens: VecDeque::new(),
            last_password: String::new(),
            disable_auto_more: false,
            more_requested: false,
            cancel_batch: false,
            use_fse: true,
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
            flagged_files: HashSet::new(),
        }
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

    pub fn flag_for_download(&mut self, file: PathBuf) {
        self.flagged_files.insert(file);
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

pub enum UserActivity {
    LoggingIn,
    BrowseMenu,
    EnterMessage,
    BrowseFiles,
    ReadMessages,
    ReadBulletins,
    TakeSurvey,
    CommentToSysop,
    UploadFiles,
    DownloadFiles,

    Goodbye,

    RunningDoor,
    ChatWithSysop,
    GroupChat,
    PagingSysop,
    ReadBroadcast,
}

pub struct NodeState {
    pub connections: Arc<Mutex<Vec<Box<ChannelConnection>>>>,
    pub cur_user: i32,
    pub user_activity: UserActivity,
    pub enabled_chat: bool,
    pub node_number: usize,
    pub connection_type: ConnectionType,
    pub handle: Option<thread::JoinHandle<Result<(), String>>>,
}

impl NodeState {
    pub fn new(node_number: usize, connection_type: ConnectionType) -> Self {
        Self {
            connections: Arc::new(Mutex::new(Vec::new())),
            user_activity: UserActivity::LoggingIn,
            cur_user: -1,
            enabled_chat: true,
            node_number,
            connection_type,
            handle: None,
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

    pub board: Arc<Mutex<IcyBoard>>,

    pub node_state: Arc<Mutex<Vec<Option<NodeState>>>>,
    pub node: usize,

    pub nodes: Vec<Node>,

    pub transfer_statistics: TransferStatistics,

    pub current_user: Option<User>,

    pub session: Session,

    pub display_text: IcbTextFile,

    /// 0 = no debug, 1 - errors, 2 - errors and warnings, 3 - all
    pub debug_level: i32,
    pub env_vars: HashMap<String, String>,

    pub user_screen: VirtualScreen,
    sysop_screen: VirtualScreen,

    char_buffer: VecDeque<KeyChar>,
}

pub struct VirtualScreen {
    parser: ansi::Parser,
    pub caret: Caret,
    pub buffer: Buffer,
}

impl VirtualScreen {
    pub fn new() -> Self {
        let buffer = Buffer::new((80, 25));
        let caret = Caret::default();
        let mut parser = ansi::Parser::default();
        parser.bs_is_ctrl_char = true;
        Self { parser, caret, buffer }
    }
}

impl IcyBoardState {
    pub fn new(board: Arc<Mutex<IcyBoard>>, node_state: Arc<Mutex<Vec<Option<NodeState>>>>, node: usize, connection: Box<dyn Connection>) -> Self {
        let mut session = Session::new();
        session.caller_number = board.lock().unwrap().statistics.cur_caller_number() as usize;
        session.disp_options.disable_color = board.lock().unwrap().config.options.non_graphics;
        session.date_format = board.lock().unwrap().config.board.date_format.clone();
        let display_text = board.lock().unwrap().default_display_text.clone();
        let root_path = board.lock().unwrap().root_path.clone();
        Self {
            root_path,
            board,
            connection,
            node_state,
            node,
            nodes: Vec::new(),
            current_user: None,
            debug_level: 0,
            display_text,
            env_vars: HashMap::new(),
            session,
            transfer_statistics: TransferStatistics::default(),
            user_screen: VirtualScreen::new(),
            sysop_screen: VirtualScreen::new(),
            char_buffer: VecDeque::new(),
        }
    }
    fn update_language(&mut self) {
        if !self.session.language.is_empty() {
            let lang_file = &self.board.lock().unwrap().config.paths.icbtext;
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
        self.display_text = self.board.lock().unwrap().default_display_text.clone();
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
        !self.session.disp_options.disable_color && self.session.disp_options.grapics_mode != GraphicsMode::Ctty
    }

    fn check_time_left(&self) {
        // TODO: Check time left.
    }

    pub fn reset_color(&mut self) -> Res<()> {
        let color = self.board.lock().unwrap().config.color_configuration.default.clone();
        self.set_color(TerminalTarget::Both, color)
    }

    pub fn clear_screen(&mut self) -> Res<()> {
        match self.session.disp_options.grapics_mode {
            GraphicsMode::Ctty | GraphicsMode::Avatar => {
                // form feed character
                self.print(TerminalTarget::Both, "\x0C")?;
            }
            GraphicsMode::Ansi => {
                self.print(TerminalTarget::Both, "\x1B[2J\x1B[H")?;
            }
            _ => {
                // ignore
            }
        }
        Ok(())
    }

    pub fn clear_line(&mut self) -> Res<()> {
        if self.use_ansi() {
            self.print(TerminalTarget::Both, "\r\x1B[K")
        } else {
            // TODO
            Ok(())
        }
    }

    pub fn clear_eol(&mut self) -> Res<()> {
        match self.session.disp_options.grapics_mode {
            GraphicsMode::Ctty => {
                let x = self.user_screen.buffer.get_width() - self.user_screen.caret.get_position().x;
                for _ in 0..x {
                    self.print(TerminalTarget::Both, " ")?;
                }
                for _ in 0..x {
                    self.print(TerminalTarget::Both, "\x08")?;
                }
                Ok(())
            }
            GraphicsMode::Ansi | GraphicsMode::Avatar => self.print(TerminalTarget::Both, "\x1B[K"),
            GraphicsMode::Rip => self.print(TerminalTarget::Both, "\x1B[K"),
        }
    }

    pub fn join_conference(&mut self, conference: i32) {
        if conference >= 0 && conference < self.board.lock().unwrap().conferences.len() as i32 {
            self.session.current_conference_number = conference;
            if let Ok(board) = self.board.lock() {
                self.session.current_conference = board.conferences[conference as usize].clone();
            }
        }
    }

    fn next_line(&mut self) -> Res<bool> {
        if self.session.disp_options.non_stop {
            return Ok(true);
        }
        let cur_y = self.user_screen.caret.get_position().y;
        if cur_y > self.session.last_new_line_y {
            self.session.num_lines_printed += (cur_y - self.session.last_new_line_y) as usize;
        } else {
            self.session.num_lines_printed = cur_y as usize;
        }
        self.session.last_new_line_y = cur_y;

        if self.session.page_len > 0 && self.session.num_lines_printed >= self.session.page_len as usize {
            self.more_promt()
        } else {
            Ok(true)
        }
    }

    pub fn run_ppe<P: AsRef<Path>>(&mut self, file_name: &P, answer_file: Option<&Path>) -> Res<()> {
        match Executable::read_file(&file_name, false) {
            Ok(executable) => {
                let path = PathBuf::from(file_name.as_ref());
                let parent = path.parent().unwrap().to_str().unwrap().to_string();

                let mut io = DiskIO::new(&parent, answer_file);
                if let Err(err) = run(file_name, &executable, &mut io, self) {
                    log::error!("Error executing PPE {}: {}", file_name.as_ref().display(), err);
                    self.session.op_text = format!("{}", err);
                    self.display_text(IceText::ErrorExecPPE, display_flags::LFBEFORE | display_flags::LFAFTER)?;
                }
            }
            Err(err) => {
                log::error!("Error loading PPE {}: {}", file_name.as_ref().display(), err);
                self.session.op_text = format!("{}", err);
                self.display_text(IceText::ErrorLoadingPPE, display_flags::LFBEFORE | display_flags::LFAFTER)?;
            }
        }

        Ok(())
    }

    pub fn put_keyboard_buffer(&mut self, value: &str, is_visible: bool) -> Res<()> {
        let in_chars: Vec<char> = value.chars().collect();

        let src = if is_visible { KeySource::User } else { KeySource::StuffedHidden };
        for (i, c) in in_chars.iter().enumerate() {
            if *c == '^' && i + 1 < in_chars.len() && in_chars[i + 1] >= 'A' && in_chars[i + 1] <= '[' {
                let c = in_chars[i + 1] as u8 - b'@';
                self.char_buffer.push_back(KeyChar::new(src, c as char));
            } else {
                self.char_buffer.push_back(KeyChar::new(src, *c));
            }
        }
        // self.char_buffer.push_back(KeyChar::new(KeySource::StuffedHidden, '\n'));
        Ok(())
    }

    pub fn load_bullettins(&self) -> Res<BullettinList> {
        if let Ok(board) = self.board.lock() {
            let path = board.resolve_file(&self.session.current_conference.blt_file);
            BullettinList::load(&path)
        } else {
            Err("Board is locked".into())
        }
    }

    pub fn load_surveys(&self) -> Res<SurveyList> {
        if let Ok(board) = self.board.lock() {
            let path = board.resolve_file(&self.session.current_conference.survey_file);
            SurveyList::load(&path)
        } else {
            Err("Board is locked".into())
        }
    }

    pub fn get_pcbdat(&self) -> Res<String> {
        if let Ok(board) = self.board.lock() {
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
        } else {
            Err("Board is locked".into())
        }
    }

    pub fn try_find_command(&self, command: &str) -> Option<super::commands::Command> {
        let command = command.to_ascii_uppercase();
        for cmd in &self.session.current_conference.commands {
            if cmd.keyword == command {
                return Some(cmd.clone());
            }
        }

        for cmd in &self.board.lock().unwrap().commands.commands {
            if cmd.keyword == command {
                return Some(cmd.clone());
            }
        }

        for cmd in &self.session.current_conference.commands {
            if cmd.keyword.starts_with(&command) {
                return Some(cmd.clone());
            }
        }

        for cmd in &self.board.lock().unwrap().commands.commands {
            if cmd.keyword.starts_with(&command) {
                return Some(cmd.clone());
            }
        }

        None
    }

    pub fn resolve_path<P: AsRef<Path>>(&self, file: &P) -> PathBuf {
        if !file.as_ref().is_absolute() {
            return self.root_path.join(file);
        }
        file.as_ref().to_path_buf()
    }

    fn shutdown_connections(&mut self) {
        let _ = self.connection.shutdown();

        if let Ok(node_state) = self.node_state.lock() {
            if let Ok(connections) = &mut node_state[self.node].as_ref().unwrap().connections.lock() {
                for conn in connections.iter_mut() {
                    let _ = conn.shutdown();
                }
            }
        }
    }

    pub fn get_term_caps(&self) -> Res<()> {
        // TODO
        Ok(())
    }

    pub fn set_current_user(&mut self, user_number: usize) -> Res<()> {
        let old_language = self.session.language.clone();

        self.session.cur_user = user_number as i32;
        self.node_state.lock().unwrap()[self.node].as_mut().unwrap().cur_user = user_number as i32;
        let last_conference = if let Ok(mut board) = self.board.lock() {
            if user_number >= board.users.len() {
                log::error!("User number {} is out of range", user_number);
                return Err(IcyBoardError::UserNumberInvalid(user_number).into());
            }
            let mut user = board.users[user_number].clone();
            user.stats.last_on = Utc::now();
            user.stats.num_times_on += 1;
            let conf = user.last_conference as i32;
            board.statistics.add_caller(user.get_name().clone());
            if !user.date_format.is_empty() {
                self.session.date_format = user.date_format.clone();
            }
            self.session.language = user.language.clone();
            self.session.cur_security = user.security_level;
            self.session.page_len = user.page_len;
            self.session.user_name = user.get_name().clone();
            self.session.alias_name = user.alias.clone();
            self.session.use_fse = user.flags.use_fsedefault;

            self.current_user = Some(user);
            conf
        } else {
            return Err(IcyBoardError::Error("board locked".to_string()).into());
        };
        if self.session.language != old_language {
            self.update_language();
        }
        self.join_conference(last_conference);
        return Ok(());
    }

    pub fn save_current_user(&mut self) -> Res<()> {
        let old_language = self.session.language.clone();
        self.session.date_format = if let Some(user) = &self.current_user {
            self.session.language = user.language.clone();
            self.session.use_fse = user.flags.use_fsedefault;
            if !user.date_format.is_empty() {
                user.date_format.clone()
            } else {
                self.session.date_format.clone()
            }
        } else {
            self.session.date_format.clone()
        };
        if self.session.language != old_language {
            self.update_language();
        }

        if let Ok(mut board) = self.board.lock() {
            if let Some(user) = &self.current_user {
                for u in 0..board.users.len() {
                    if board.users[u].get_name() == user.get_name() {
                        board.set_user(user.clone(), u)?;
                        return Ok(());
                    }
                }
            }
        }
        log::error!("User not found in user list");
        Ok(())
    }

    fn find_more_specific_file(&self, base_name: String) -> PathBuf {
        if let Some(result) = self.find_more_specific_file_with_graphics(base_name.clone() + &self.session.cur_security.to_string()) {
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
            let lang_file = base_name.clone() + "." + &self.session.language;
            if let Some(result) = self.find_file_with_extension(&lang_file) {
                return Some(result);
            }
        }
        self.find_file_with_extension(&base_name)
    }

    fn find_file_with_extension(&self, lang_file: &String) -> Option<PathBuf> {
        let file = PathBuf::from(lang_file.clone() + ".pcb");
        if file.exists() {
            return Some(file);
        }
        let file = PathBuf::from(lang_file.clone() + ".ans");
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

    pub fn set_activity(&self, activity: UserActivity) {
        self.node_state.lock().unwrap()[self.node].as_mut().unwrap().user_activity = activity;
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
    pub fn gotoxy(&mut self, target: TerminalTarget, x: i32, y: i32) -> Res<()> {
        match self.session.disp_options.grapics_mode {
            GraphicsMode::Ctty => {
                // ignore
            }
            GraphicsMode::Ansi => {
                self.print(target, &format!("\x1B[{};{}H", y, x))?;
            }
            GraphicsMode::Avatar => {
                self.print(target, &format!("\x16\x08{}{}", (x as u8) as char, (y as u8) as char))?;
            }
            GraphicsMode::Rip => {
                self.print(target, &format!("\x1B[{};{}H", y, x))?;
            }
        }

        Ok(())
    }

    pub fn backward(&mut self, chars: i32) -> Res<()> {
        if self.use_ansi() {
            self.write_raw(TerminalTarget::Both, format!("\x1B[{}D", chars).chars().collect::<Vec<char>>().as_slice())
        } else {
            Ok(())
        }
    }

    pub fn forward(&mut self, chars: i32) -> Res<()> {
        if self.use_ansi() {
            self.write_raw(TerminalTarget::Both, format!("\x1B[{}C", chars).chars().collect::<Vec<char>>().as_slice())
        } else {
            Ok(())
        }
    }

    pub fn up(&mut self, chars: i32) -> Res<()> {
        if self.use_ansi() {
            self.write_raw(TerminalTarget::Both, format!("\x1B[{}A", chars).chars().collect::<Vec<char>>().as_slice())
        } else {
            Ok(())
        }
    }

    pub fn down(&mut self, chars: i32) -> Res<()> {
        if self.use_ansi() {
            self.write_raw(TerminalTarget::Both, format!("\x1B[{}B", chars).chars().collect::<Vec<char>>().as_slice())
        } else {
            Ok(())
        }
    }

    /// # Errors
    pub fn print(&mut self, target: TerminalTarget, str: &str) -> Res<()> {
        self.write_raw(target, str.chars().collect::<Vec<char>>().as_slice())
    }

    pub fn println(&mut self, target: TerminalTarget, str: &str) -> Res<()> {
        let mut line = str.chars().collect::<Vec<char>>();
        line.push('\r');
        line.push('\n');
        self.write_raw(target, line.as_slice())
    }

    fn write_chars(&mut self, target: TerminalTarget, data: &[char]) -> Res<()> {
        let mut user_bytes = Vec::new();
        let mut sysop_bytes = Vec::new();

        for c in data {
            if target != TerminalTarget::Sysop || self.session.is_sysop {
                let _ = self
                    .user_screen
                    .parser
                    .print_char(&mut self.user_screen.buffer, 0, &mut self.user_screen.caret, *c);
                if *c == '\n' {
                    self.next_line()?;
                }
                if let Some(&cp437) = UNICODE_TO_CP437.get(&c) {
                    user_bytes.push(cp437);
                } else {
                    user_bytes.push(b'.');
                }
            }
            if target != TerminalTarget::User {
                let _ = self
                    .sysop_screen
                    .parser
                    .print_char(&mut self.sysop_screen.buffer, 0, &mut self.sysop_screen.caret, *c);
                if let Some(&cp437) = UNICODE_TO_CP437.get(&c) {
                    sysop_bytes.push(cp437);
                } else {
                    sysop_bytes.push(b'.');
                }
            }
        }

        if target != TerminalTarget::Sysop || self.session.is_sysop {
            self.connection.write_all(&user_bytes)?;
        }

        if target != TerminalTarget::User {
            // Send user only not to other connections
            if let Ok(node_state) = self.node_state.lock() {
                if let Ok(connections) = &mut node_state[self.node].as_ref().unwrap().connections.lock() {
                    for conn in connections.iter_mut() {
                        let _ = conn.write_all(&sysop_bytes);
                    }
                }
            }
        }
        match target {
            TerminalTarget::Both | TerminalTarget::User => {
                self.session.cursor_pos = self.user_screen.caret.get_position();
            }
            TerminalTarget::Sysop => {
                self.session.cursor_pos = self.sysop_screen.caret.get_position();
            }
        }
        Ok(())
    }

    /// # Errors
    pub fn write_raw(&mut self, target: TerminalTarget, data: &[char]) -> Res<()> {
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
                            self.write_chars(target, &[*c])?;
                        }
                    }
                    PcbState::GotAt => {
                        if *c == 'X' || *c == 'x' {
                            state = PcbState::ReadColor1;
                        } else if *c == '@' {
                            self.write_chars(target, &[*c])?;
                            state = PcbState::GotAt;
                        } else {
                            state = PcbState::ReadAtSequence(c.to_string());
                        }
                    }
                    PcbState::ReadAtSequence(s) => {
                        if c.is_whitespace() {
                            self.write_chars(target, &['@'])?;
                            self.write_chars(target, s.chars().collect::<Vec<char>>().as_slice())?;
                            state = PcbState::Default;
                        } else if *c == '@' {
                            state = PcbState::Default;
                            if let Some(output) = self.translate_variable(target, &s) {
                                if output.len() == 0 {
                                    self.write_chars(target, &['@'])?;
                                    self.write_chars(target, s.chars().collect::<Vec<char>>().as_slice())?;
                                    state = PcbState::GotAt;
                                } else {
                                    self.write_chars(target, output.chars().collect::<Vec<char>>().as_slice())?;
                                }
                            }
                        } else {
                            state = PcbState::ReadAtSequence(s + &c.to_string());
                        }
                    }
                    PcbState::ReadColor1 => {
                        if c.is_ascii_hexdigit() {
                            state = PcbState::ReadColor2(*c);
                        } else {
                            self.write_chars(target, &['@', *c])?;
                            state = PcbState::Default;
                        }
                    }
                    PcbState::ReadColor2(ch1) => {
                        state = PcbState::Default;
                        if !c.is_ascii_hexdigit() {
                            self.write_chars(target, &['@', ch1, *c])?;
                        } else {
                            let color = (c.to_digit(16).unwrap() | (ch1.to_digit(16).unwrap() << 4)) as u8;

                            if color == 0 {
                                self.session.saved_color = self.cur_color();
                            } else if color == 0xFF {
                                self.set_color(target, self.cur_color())?;
                            } else {
                                self.set_color(target, color.into())?;
                            }
                        }
                    }
                }
            }

            if state != PcbState::Default {
                match state {
                    PcbState::Default => {}
                    PcbState::GotAt => self.write_chars(target, &['@'])?,
                    PcbState::ReadColor1 => self.write_chars(target, &['@', *data.last().unwrap()])?,
                    PcbState::ReadColor2(ch1) => self.write_chars(target, &['@', ch1, *data.last().unwrap()])?,
                    PcbState::ReadAtSequence(s) => {
                        self.write_chars(target, &['@'])?;
                        self.write_chars(target, s.chars().collect::<Vec<char>>().as_slice())?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn translate_variable(&mut self, target: TerminalTarget, input: &str) -> Option<String> {
        let mut split = input.split(':');
        let id = split.next().unwrap();
        let param = split.next();
        let mut result = String::new();
        match id {
            "ALIAS" => {
                if let Some(user) = &self.current_user {
                    result = user.alias.to_string();
                }
                if result.is_empty() {
                    result = self.session.get_first_name();
                }
            }
            "AUTOMORE" => {
                self.session.disp_options.auto_more = true;
                return None;
            }
            "BEEP" => {
                let _ = self.bell();
                return None;
            }
            "BICPS" => result = self.transfer_statistics.get_cps_both().to_string(),
            "BOARDNAME" => result = self.board.lock().unwrap().config.board.name.to_string(),
            "BPS" | "CARRIER" => result = self.get_bps().to_string(),

            // TODO
            "BYTECREDIT" | "BYTELIMIT" | "BYTERATIO" | "BYTESLEFT" => {
                // todo
            }

            "CITY" => {
                if let Some(user) = &self.current_user {
                    result = user.city_or_state.to_string();
                }
            }
            "CLREOL" => {
                let _ = self.clear_eol();
                return None;
            }
            "CLS" => {
                let _ = self.clear_screen();
                return None;
            }
            "CONFNAME" => result = self.session.current_conference.name.to_string(),
            "CONFNUM" => result = self.session.current_conference_number.to_string(),

            // TODO
            "CREDLEFT" | "CREDNOW" | "CREDSTART" | "CREDUSED" | "CURMSGNUM" => {}

            "DATAPHONE" => {
                if let Some(user) = &self.current_user {
                    result = user.bus_data_phone.to_string();
                }
            }
            "DAYBYTES" | "DELAY" | "DLBYTES" | "DLFILES" | "EVENT" => {}
            "EXPDATE" => {
                if let Some(user) = &self.current_user {
                    result = self.format_date(user.exp_date);
                } else {
                    result = "NEVER".to_string();
                }
            }
            "EXPDAYS" => {
                if self.board.lock().unwrap().config.subscription_info.is_enabled {
                    if let Some(user) = &self.current_user {
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
            "FBYTES" | "FFILES" | "FILECREDIT" | "FILERATIO" => {}
            "FIRST" => {
                result = self.session.get_first_name();
            }
            "FIRSTU" => {
                result = self.session.get_first_name().to_uppercase();
            }
            "FNUM" => {}
            "FREESPACE" => {}
            "GFXMODE" => {
                result = match self.session.disp_options.grapics_mode {
                    GraphicsMode::Ctty => self.display_text.get_display_text(IceText::GfxModeOff).unwrap().text,
                    GraphicsMode::Ansi => self.display_text.get_display_text(IceText::GfxModeAnsi).unwrap().text,
                    GraphicsMode::Avatar => self.display_text.get_display_text(IceText::GfxModeAvatar).unwrap().text,
                    GraphicsMode::Rip => self.display_text.get_display_text(IceText::GfxModeRip).unwrap().text,
                };
            }
            "HOMEPHONE" => {
                if let Some(user) = &self.current_user {
                    result = user.home_voice_phone.to_string();
                }
            }
            "HIGHMSGNUM" => {}
            "INAME" => {}
            "INCONF" => result = self.session.current_conference.name.to_string(),
            "KBLEFT" | "KBLIMIT" | "LASTCALLERNODE" | "LASTCALLERSYSTEM" | "LASTDATEON" | "LASTTIMEON" | "LMR" | "LOGDATE" | "LOGTIME" | "LOWMSGNUM"
            | "MAXBYTES" | "MAXFILES" => {}
            "MINLEFT" => result = "1000".to_string(),
            "MORE" => {
                let _ = self.more_promt();
                return None;
            }
            "MSGLEFT" => {
                if let Some(user) = &self.current_user {
                    result = user.stats.messages_left.to_string();
                }
            }
            "MSGREAD" => {
                if let Some(user) = &self.current_user {
                    result = user.stats.messages_read.to_string();
                }
            }
            "NOCHAR" => result = self.session.no_char.to_string(),
            "NODE" => result = self.node.to_string(),
            "NUMBLT" => {
                if let Ok(bullettins) = self.load_bullettins() {
                    result = bullettins.len().to_string();
                }
            }
            "NUMCALLS" => {
                result = self.board.lock().unwrap().statistics.total.calls.to_string();
            }
            "NUMCONF" => result = self.board.lock().unwrap().conferences.len().to_string(),
            "NUMDIR" => {
                result = self.session.current_conference.directories.len().to_string();
            }
            "DIRNAME" => {
                result = self.session.current_conference.directories[self.session.current_file_directory]
                    .name
                    .to_string();
            }
            "DIRNUM" => {
                result = self.session.current_conference.directories.len().to_string();
            }
            "AREA" => {
                result = self.session.current_conference.areas[self.session.current_message_area].name.to_string();
            }
            "NUMAREA" => {
                result = self.session.current_conference.areas.len().to_string();
            }
            "NUMTIMESON" => {
                if let Some(user) = &self.current_user {
                    result = user.stats.num_times_on.to_string();
                }
            }
            "OFFHOURS" => {}
            "OPTEXT" => result = self.session.op_text.to_string(),
            "PAUSE" => {
                self.session.disp_options.auto_more = true;
                let _ = self.press_enter();
                self.session.disp_options.auto_more = false;
                return None;
            }
            "POS" => {
                let x = self.user_screen.caret.get_position().x as usize;
                if let Some(value) = param {
                    if let Ok(i) = value.parse::<usize>() {
                        while result.len() + 1 < i - x {
                            result.push(' ');
                        }
                        return Some(result);
                    }
                }
            }

            "POFF" | "PON" | "PROLTR" | "PRODESC" | "PWXDATE" | "PWXDAYS" | "QOFF" | "QON" | "RATIOBYTES" | "RATIOFILES" => {}
            "RCPS" => result = self.transfer_statistics.get_cps_upload().to_string(),
            "RBYTES" => result = self.transfer_statistics.uploaded_bytes.to_string(),
            "RFILES" => result = self.transfer_statistics.uploaded_files.to_string(),
            "REAL" => {
                if let Some(user) = &self.current_user {
                    result = user.get_name().to_string();
                }
            }
            "SECURITY" => {
                if let Some(user) = &self.current_user {
                    result = user.security_level.to_string()
                }
            }
            "SCPS" => result = self.transfer_statistics.get_cps_download().to_string(),
            "SBYTES" => result = self.transfer_statistics.downloaded_bytes.to_string(),
            "SFILES" => result = self.transfer_statistics.downloaded_files.to_string(),
            "SYSDATE" => {
                result = self.format_date(Utc::now());
            }
            "SYSOPIN" => result = self.board.lock().unwrap().config.sysop.sysop_start.to_string(),
            "SYSOPOUT" => result = self.board.lock().unwrap().config.sysop.sysop_stop.to_string(),
            "SYSTIME" => {
                let now = Local::now();
                let t = now.time();
                result = format!("{:02}:{:02}", t.hour(), t.minute());
            }
            "TIMELIMIT" => result = self.session.time_limit.to_string(),
            "TIMELEFT" => {
                let now = Local::now();
                let time_on = now - self.session.login_date;
                if self.session.time_limit == 0 {
                    result = "UNLIMITED".to_string();
                } else {
                    result = (self.session.time_limit as i64 - time_on.num_minutes()).to_string();
                }
            }
            "TIMEUSED" => {}
            "TOTALTIME" => {}
            "UPBYTES" => {
                if let Some(user) = &self.current_user {
                    result = user.stats.ul_tot_upld_bytes.to_string();
                }
            }
            "UPFILES" => {
                if let Some(user) = &self.current_user {
                    result = user.stats.num_uploads.to_string();
                }
            }
            "USER" => {
                if let Some(user) = &self.current_user {
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
            "WAIT" => {}
            "WHO" => {}
            "XOFF" => {
                self.session.disp_options.disable_color = true;
                return None;
            }
            "XON" => {
                if !self.board.lock().unwrap().config.options.non_graphics {
                    self.session.disp_options.disable_color = false;
                }
                return None;
            }
            "YESCHAR" => result = self.session.yes_char.to_string(),
            _ => {
                if id.to_ascii_uppercase().starts_with("ENV=") {
                    let key = &id[4..];
                    if let Some(value) = self.get_env(key) {
                        result = value.to_string();
                    }
                }
            }
        }

        if let Some(mut param) = param {
            let mut alignment = Alignment::Left;
            if param.ends_with("C") || param.ends_with("c") {
                alignment = Alignment::Center;
                param = &param[..param.len() - 1];
            } else if param.ends_with("R") || param.ends_with("r") {
                alignment = Alignment::Right;
                param = &param[..param.len() - 1];
            }
            if let Ok(i) = param.parse::<usize>() {
                match alignment {
                    Alignment::Left => {
                        while result.chars().count() < i {
                            result.push(' ');
                        }
                    }
                    Alignment::Center => {
                        let len = result.chars().count();
                        let spaces = (i - len) / 2;
                        for _ in 0..spaces {
                            result.insert(0, ' ');
                        }
                        while result.chars().count() < i {
                            result.push(' ');
                        }
                    }
                    Alignment::Right => {
                        let len = result.chars().count();
                        while result.chars().count() < i - len {
                            result.insert(0, ' ');
                        }
                    }
                }
            }
        }

        Some(result)
    }

    /// # Errors
    pub fn get_char(&mut self) -> Res<Option<KeyChar>> {
        if let Some(ch) = self.char_buffer.pop_front() {
            return Ok(Some(ch));
        }
        let mut key_data = [0; 1];
        let size = self.connection.read(&mut key_data)?;
        if size == 1 {
            return Ok(Some(KeyChar::new(KeySource::User, key_data[0] as char)));
        }
        if let Ok(node_state) = &self.node_state.lock() {
            if let Ok(connections) = &mut node_state[self.node].as_ref().unwrap().connections.lock() {
                for conn in connections.iter_mut() {
                    let size = conn.read(&mut key_data)?;
                    if size == 1 {
                        return Ok(Some(KeyChar::new(KeySource::Sysop, key_data[0] as char)));
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(100));
        Ok(None)
    }

    pub fn inbytes(&mut self) -> i32 {
        self.char_buffer.len() as i32
    }

    pub fn cur_color(&self) -> IcbColor {
        let attr = self.user_screen.caret.get_attribute().as_u8(icy_engine::IceMode::Blink);
        IcbColor::Dos(attr)
    }

    pub fn set_color(&mut self, target: TerminalTarget, color: IcbColor) -> Res<()> {
        if !self.use_graphics() {
            return Ok(());
        }
        let new_color = match color {
            IcbColor::None => {
                return Ok(());
            }
            IcbColor::Dos(color) => {
                if self.user_screen.caret.get_attribute().as_u8(icy_engine::IceMode::Blink) == color {
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
                return self.write_chars(target, color_change.chars().collect::<Vec<char>>().as_slice());
            }
        }

        let mut color_change = "\x1B[".to_string();
        let was_bold = self.user_screen.caret.get_attribute().is_bold();
        let new_bold = new_color.is_bold() || new_color.get_foreground() > 7;
        let mut bg = self.user_screen.caret.get_attribute().get_background();
        let mut fg = self.user_screen.caret.get_attribute().get_foreground();
        if was_bold != new_bold {
            if new_bold {
                color_change += "1;";
            } else {
                color_change += "0;";
                fg = 7;
                bg = 0;
            }
        }

        if !self.user_screen.caret.get_attribute().is_blinking() && new_color.is_blinking() {
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
        self.write_chars(target, color_change.chars().collect::<Vec<char>>().as_slice())
    }

    pub fn get_caret_position(&mut self) -> (i32, i32) {
        (self.session.cursor_pos.x, self.session.cursor_pos.y)
    }

    /// # Errors
    pub fn goodbye(&mut self) -> Res<()> {
        /*   if HangupType::Hangup != hangup_type {

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


        } */
        self.display_text(IceText::ThanksForCalling, display_flags::LFBEFORE | display_flags::NEWLINE)?;
        self.reset_color()?;

        self.hangup()
    }

    pub fn hangup(&mut self) -> Res<()> {
        self.session.request_logoff = true;
        self.shutdown_connections();
        Ok(())
    }

    pub fn bell(&mut self) -> Res<()> {
        self.write_raw(TerminalTarget::Both, &['\x07'])
    }

    pub fn more_promt(&mut self) -> Res<bool> {
        if self.session.disable_auto_more {
            self.session.more_requested = true;
            return Ok(true);
        }
        let result = self.input_field(
            IceText::MorePrompt,
            12,
            "",
            "HLPMORE",
            None,
            display_flags::YESNO | display_flags::UPCASE | display_flags::STACKED | display_flags::ERASELINE,
        )?;
        Ok(result != "N")
    }

    pub fn press_enter(&mut self) -> Res<()> {
        self.session.disable_auto_more = true;
        self.session.more_requested = false;
        self.input_field(IceText::PressEnter, 0, "", "", None, display_flags::ERASELINE)?;
        Ok(())
    }

    pub fn new_line(&mut self) -> Res<()> {
        self.write_raw(TerminalTarget::Both, &['\r', '\n'])
    }

    pub fn format_date(&self, date_time: DateTime<Utc>) -> String {
        let local_time = date_time.with_timezone(&Local);
        local_time.format(&self.session.date_format).to_string()
    }
    pub fn format_time(&self, date_time: DateTime<Utc>) -> String {
        let local_time = date_time.with_timezone(&Local);
        local_time.format("%H:%M").to_string()
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
