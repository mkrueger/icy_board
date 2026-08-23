use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use crate::{
    Res,
    executable::Executable,
    icy_board::state::{user_commands::groupchat::GroupChatPreferences, virtual_screen::VirtualScreen},
    search_patterns::PatternExpr,
    vm::expressions::fix_casing,
};
use async_recursion::async_recursion;
use chrono::{DateTime, Local, Utc};
use codepages::tables::UNICODE_TO_CP437;
use dizbase::file_base::FileBase;
use icy_engine::Position;
use icy_engine::SaveOptions;
use icy_engine::formats::{CharacterFormatOptions, FileFormat, FormatOptions, ScreenPreperation};
use icy_engine::{TextAttribute, TextPane};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection, iemsi::EmsiICI, termcap_detect, termcap_detect::TerminalCaps};
use icy_parser_core::ANSI_COLOR_OFFSETS;
use regex::Regex;
use tokio::{sync::Mutex, time::sleep};

use crate::{
    icy_board::IcyBoardError,
    vm::{DiskIO, TerminalTarget, run},
};
pub mod functions;
pub mod menu_runner;
pub mod ppl_audio;
pub mod ppl_error;
pub mod ppl_events;
pub mod ppl_graphics;
pub mod ppl_keys;
pub mod ppl_mouse;
pub mod ppl_surface;
pub mod ppl_terminal_control;
pub mod ppl_terminal_info;
pub mod ppl_terminal_input;
pub mod ppl_terminal_state;
pub mod user_commands;
pub mod virtual_screen;
use self::functions::display_flags;

/// How long a graphics capability query waits before the terminal counts as silent.
const GFX_PROBE_TIMEOUT: Duration = Duration::from_millis(500);

/// The libsndfile formats PPL can name, as `(PPL format, major, subtype)`. They are
/// probed together so no answer has to arrive from behind an upload.
pub(crate) const SOUND_FORMATS: &[(i32, u32, u32)] = &[(1, 1, 0), (2, 2, 0), (3, 23, 0), (4, 32, 96), (5, 32, 100)];

fn keyboard_timeout_elapsed(is_local: bool, enabled: bool, minutes: u16, elapsed: Duration) -> bool {
    !is_local && enabled && minutes > 0 && elapsed >= Duration::from_secs(u64::from(minutes) * 60)
}

#[cfg(test)]
mod option_tests {
    use super::{Duration, keyboard_timeout_elapsed};

    #[test]
    fn keyboard_timeout_uses_minutes_and_zero_disables_it() {
        assert!(!keyboard_timeout_elapsed(false, true, 5, Duration::from_secs(299)));
        assert!(keyboard_timeout_elapsed(false, true, 5, Duration::from_mins(5)));
        assert!(!keyboard_timeout_elapsed(false, true, 0, Duration::from_hours(1)));
        assert!(!keyboard_timeout_elapsed(true, true, 1, Duration::from_mins(1)));
        assert!(!keyboard_timeout_elapsed(false, false, 1, Duration::from_mins(1)));
    }
}

use super::{
    IcyBoard,
    bbs::{BBS, BBSMessage},
    commands::{AutoRun, Command, CommandAction, CommandType},
    conferences::Conference,
    events::{self, EventWindow},
    icb_config::{DEFAULT_PCBOARD_DATE_FORMAT, IcbColor, SysopCommandLevels, UserCommandLevels},
    icb_text::{IcbTextFile, IceText},
    limits::{self, BatchSoFar, TransferHistory, TransferLimits},
    macro_parser::{Macro, MacroCommand},
    security_expr::SecurityExpression,
    user_base::{ConferenceFlags, FSEMode, Password, PasswordVerdict, User},
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

pub struct PPEExecute {
    pub ppe: PathBuf,
    pub user_name: Option<String>,
    pub password: Option<String>,
    pub args: Vec<String>,
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
        usize::midpoint(self.downloaded_cps, self.uploaded_cps)
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

    /// Where an (A)ll conference scan stopped, zero when none was left unfinished.
    pub start_conf: u16,

    pub user_command_level: UserCommandLevels,
    pub sysop_command_level: SysopCommandLevels,

    pub login_date: DateTime<Utc>,

    pub current_user: Option<User>,
    pub cur_user_id: i32,
    pub cur_security: u8,
    pub subscription_expired: bool,
    pub cur_groups: Vec<String>,
    pub language: String,

    pub page_len: u16,

    pub is_sysop: bool,
    pub op_text: String,
    pub use_alias: bool,

    pub last_new_line_y: i32,

    pub request_logoff: bool,

    pub time_limit: i32,
    /// Set when a pending event has cut the session short. ADJTIME may then only
    /// take time away, never give it back.
    pub time_adjusted_for_event: bool,
    pub security_violations: i32,

    /// If true, the keyboard timer is checked.
    /// After it's elapsed logoff the user for inactivity.
    pub keyboard_timer_check: bool,
    pub keyboard_timer_started: Instant,

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

    /// Bytes the caller may still download today; -1 is `PCBoard`'s unlimited.
    pub bytes_remaining: i64,

    /// What the caller's security level allows them to download.
    pub transfer_limits: TransferLimits,

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

    /// The current default answer on last `input_string`
    pub default_answer: Option<String>,
    pub last_answer: Option<String>,

    pub memorized_msg: Option<(usize, u32)>,
    pub group_chat: GroupChatPreferences,
    pub joined_conferences: HashSet<u16>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            user_command_level: UserCommandLevels::default(),
            sysop_command_level: SysopCommandLevels::default(),
            disp_options: DisplayOptions::default(),
            current_conference_number: 0,
            current_conference: Conference::default(),
            start_conf: 0,
            login_date: Utc::now(),
            current_user: None,
            cur_user_id: -1,
            cur_security: 0,
            subscription_expired: false,
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
            time_adjusted_for_event: false,
            keyboard_timer_check: true,
            keyboard_timer_started: Instant::now(),
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
            transfer_limits: TransferLimits::default(),

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
            group_chat: GroupChatPreferences::default(),
            joined_conferences: HashSet::new(),
        }
    }

    pub fn expert_mode(&self) -> bool {
        if let Some(user) = &self.current_user { user.flags.expert_mode } else { false }
    }

    pub fn push_tokens(&mut self, command: &str) -> usize {
        let mut res = 0;
        for cmd in crate::tokens::tokenize(command) {
            self.tokens.push_back(cmd.clone());
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
    /// The line the node list shows in its status column.
    pub fn text(&self) -> IceText {
        match self {
            NodeStatus::NoCaller => IceText::NoCaller,
            NodeStatus::Available => IceText::Available,
            NodeStatus::RunningDoor => IceText::InADOOR,
            NodeStatus::EnterMessage => IceText::EnterMessage,
            NodeStatus::GroupChat => IceText::GroupChat,
            NodeStatus::HandlingMail => IceText::HandlingMail,
            NodeStatus::LogoffPending => IceText::LogoffPending,
            NodeStatus::NodeMessage => IceText::ReceivedMessage,
            NodeStatus::RunningEvent => IceText::RunningEvent,
            NodeStatus::LogIntoSystem => IceText::LogIntoSystem,
            NodeStatus::PagingSysop => IceText::PagingSysop,
            NodeStatus::ChatWithSysop => IceText::ChatWithSysop,
            NodeStatus::RecycleBBS => IceText::RecycleBBS,
            NodeStatus::TakeSurvey => IceText::AnswerSurvey,
            NodeStatus::Transfer => IceText::Transfer,
            NodeStatus::Unavailable => IceText::Unavailable,
            NodeStatus::DropDOSDelayed => IceText::DropDOSDelayed,
            NodeStatus::DropDOSNow => IceText::DropDOSNow,
            // PCBoard had no such state, so it needs a line of its own.
            NodeStatus::ReadBulletins => IceText::ReadingBulletins,
        }
    }

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
    /// What a PPE wrote into the node's USERNET record, empty until one does.
    pub user_name: String,
    pub city: String,
    pub enabled_chat: bool,
    pub node_number: usize,
    pub connection_type: ConnectionType,
    pub logon_time: DateTime<Utc>,
    pub handle: Option<thread::JoinHandle<Res<()>>>,
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
            user_name: String::new(),
            city: String::new(),
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
    /// KBDSTUFF: originates from a PPE but is echoed like typed input.
    StuffedVisible,
    StuffedHidden,
    /// KBDFILE: stuffed from a script file, otherwise identical to `StuffedHidden`.
    StuffedFile,
    Sysop,
}

impl KeySource {
    pub fn is_stuffed(self) -> bool {
        matches!(self, KeySource::StuffedVisible | KeySource::StuffedHidden | KeySource::StuffedFile)
    }

    pub fn is_hidden(self) -> bool {
        matches!(self, KeySource::StuffedHidden | KeySource::StuffedFile)
    }
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

    /// The files being displayed right now, innermost last.
    displayed_files: Vec<PathBuf>,
    /// How many PPEs are running on top of each other.
    ppe_nesting: usize,

    /// Where `OPENCAP` is teeing everything the caller sees, until `CLOSECAP`.
    capture_file: Option<std::fs::File>,

    /// Content hashes of sound files already pushed to the client's disk cache
    /// this connection, so repeat plays only need a cheap `Load` instead of
    /// resending the whole file through `LoadBlob`.
    pub sound_cache: HashSet<String>,

    /// Raw media bytes uploaded to the terminal cache during this connection.
    media_upload_bytes: usize,

    /// Last `SNDVOLUME` percent requested per logical channel (0-13), defaulting to
    /// 100 so music/fx start at full loudness instead of the client's quiet
    /// default headroom.
    pub sound_volume: [i32; 14],

    pub sound_active: [bool; 14],

    /// The file each `AUDIO` channel was loaded from, indexed by logical channel.
    ppl_audio: [Option<String>; 14],

    pub sound_formats: HashMap<i32, bool>,

    /// Whether the media queries have gone out yet. They are answered once per call,
    /// because a terminal does not change mid call.
    media_probed: bool,

    /// What the last graphics operation reported, or `-1` when it has not run yet.
    /// The VM empties this into its own last error once the statement is over.
    pub gfx_error: i32,

    /// Names below `gfx/` that this caller's cache is known to hold already,
    /// seeded from the terminal's own listing and extended as uploads happen.
    pub gfx_cache: HashSet<String>,

    gfx_probe: termcap_detect::TerminalProbe,

    /// Bytes read while waiting for a terminal reply that turned out not to be one.
    /// They stay undecoded until something asks for input, because only then is it
    /// settled whether they are keystrokes or mouse reports.
    raw_input: VecDeque<u8>,

    pub ppl_graphics: Option<ppl_graphics::PplGraphicsState>,
    ppl_event_keys: ppl_events::LogicalKeyState,
    ppl_audio_notify: ppl_events::AudioNotifyState,
    pub ppl_keys: ppl_keys::PplKeyState,
    pub ppl_mouse: ppl_mouse::PplMouseState,
    pub ppl_terminal: ppl_terminal_control::PplTerminalControl,
    term_input_handle: Option<u64>,
    next_term_input_handle: u64,
}

impl IcyBoardState {
    pub fn reserve_media_upload(&mut self, bytes: usize) -> bool {
        const MAX_MEDIA_UPLOAD_BYTES: usize = 256 * 1024 * 1024;
        let Some(total) = self.media_upload_bytes.checked_add(bytes) else {
            return false;
        };
        if total > MAX_MEDIA_UPLOAD_BYTES {
            return false;
        }
        self.media_upload_bytes = total;
        true
    }

    pub fn display_screen(&self) -> &VirtualScreen {
        if self.session.is_sysop || self.session.cur_user_id < 0 {
            &self.sysop_screen
        } else {
            &self.user_screen
        }
    }

    pub fn display_screen_mut(&mut self) -> &mut VirtualScreen {
        if self.session.is_sysop || self.session.cur_user_id < 0 {
            &mut self.sysop_screen
        } else {
            &mut self.user_screen
        }
    }
    pub async fn new(
        bbs: Arc<Mutex<BBS>>,
        board: Arc<tokio::sync::Mutex<IcyBoard>>,
        node_state: Arc<Mutex<Vec<Option<NodeState>>>>,
        node: usize,
        connection: Box<dyn Connection>,
    ) -> Self {
        assert!(node <= node_state.lock().await.len(), "Node number {node} out of range");
        let mut session = Session::new();
        session.user_command_level = board.lock().await.config.user_command_level.clone();
        session.sysop_command_level = board.lock().await.config.sysop_command_level.clone();
        session.caller_number = board.lock().await.statistics.cur_caller_number() as usize;
        session.date_format = board.lock().await.config.board.date_format.clone();
        let display_text: IcbTextFile = board.lock().await.default_display_text.clone();
        let root_path = board.lock().await.root_path.clone();
        let p1 = icy_parser_core::AnsiParser::default();
        //p1.bs_is_ctrl_char = true;
        let p2 = icy_parser_core::AnsiParser::default();
        //p2.bs_is_ctrl_char = true;
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
            displayed_files: Vec::new(),
            ppe_nesting: 0,
            capture_file: None,
            sound_cache: HashSet::new(),
            media_upload_bytes: 0,
            sound_volume: [100; 14],
            sound_active: [false; 14],
            ppl_audio: std::array::from_fn(|_| None),
            sound_formats: HashMap::new(),
            media_probed: false,
            gfx_error: -1,
            gfx_cache: HashSet::new(),
            gfx_probe: termcap_detect::TerminalProbe::default(),
            raw_input: VecDeque::new(),
            ppl_graphics: None,
            ppl_event_keys: ppl_events::LogicalKeyState::default(),
            ppl_audio_notify: ppl_events::AudioNotifyState::default(),
            ppl_keys: ppl_keys::PplKeyState::default(),
            ppl_mouse: ppl_mouse::PplMouseState::default(),
            ppl_terminal: ppl_terminal_control::PplTerminalControl::default(),
            term_input_handle: None,
            next_term_input_handle: 1,
        }
    }
    async fn update_language(&mut self) {
        if !self.session.language.is_empty() {
            let lang_file = self.get_board().await.config.paths.icbtext.clone();
            let lang_file = lang_file.with_extension(format!("{}.toml", self.session.language));
            let lang_file = self.resolve_path(&lang_file);

            log::info!("Loading language file: {}", lang_file.display());
            if lang_file.exists()
                && let Ok(display_text) = IcbTextFile::load(&lang_file)
            {
                self.display_text = display_text;
                return;
            }
        }
        let dt = self.get_board().await.default_display_text.clone();
        self.display_text = dt;
    }
    /// Turns on keyboard check & resets the keyboard check timer.
    pub fn reset_keyboard_check_timer(&mut self) {
        self.session.keyboard_timer_check = true;
        self.session.keyboard_timer_started = Instant::now();
    }

    async fn keyboard_timed_out(&mut self) -> Res<bool> {
        let timeout = self.get_board().await.config.limits.keyboard_timeout;
        if !keyboard_timeout_elapsed(
            self.session.is_local,
            self.session.keyboard_timer_check,
            timeout,
            self.session.keyboard_timer_started.elapsed(),
        ) {
            return Ok(false);
        }

        // The logoff cannot unwind a running PPE, so the check has to be disarmed
        // here or every later poll would report the timeout again.
        self.session.keyboard_timer_check = false;
        self.display_text(IceText::KeyboardTimeExpired, display_flags::NEWLINE | display_flags::LFBEFORE)
            .await?;
        self.hangup().await?;
        Ok(true)
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

    /// Hangs up once the caller's time is gone. `PCBoard` watches the session clock from
    /// its keyboard loop, so the check sits in front of every prompt. It watches it for
    /// everyone, sysop included - an unlimited sysop holds a level that says so.
    async fn check_time_left(&mut self) {
        if self.session.request_logoff {
            return;
        }
        let online = (Utc::now() - self.session.login_date).num_minutes();
        if !limits::session_expired(self.session.time_limit, online) {
            return;
        }
        let _ = self
            .display_text(
                IceText::TimelimitExceeded,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LOGIT | display_flags::BELL,
            )
            .await;
        let _ = self.hangup().await;
    }

    pub async fn reset_color(&mut self, target: TerminalTarget) -> Res<()> {
        let color = self.get_board().await.config.color_configuration.default.clone();
        self.set_color(target, color).await
    }

    pub async fn clear_screen(&mut self, target: TerminalTarget) -> Res<()> {
        // Clearing the screen starts the page over, it does not turn a pause back on that
        // an @POFF@ before it turned off. See printcls() in DISPLAY.C.
        self.session.disp_options.num_lines_printed = 0;
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
                let x = self.user_screen.buffer.width() - self.user_screen.buffer.caret.x;
                for _ in 0..x {
                    self.print(target, " ").await?;
                }
                for _ in 0..x {
                    self.print(target, "\x08").await?;
                }
                Ok(())
            }
            GraphicsMode::Ansi | GraphicsMode::Graphics | GraphicsMode::Avatar | GraphicsMode::Rip => self.print(target, "\x1B[K").await,
        }
    }

    /// Takes the time limit and the transfer allowance of the caller from the PWRD
    /// security level definitions.
    async fn apply_security_level_limits(&mut self) {
        self.apply_pwrd_limits().await;
        // The level hands out a fresh limit every time it is read, so the event has to be
        // taken off again afterwards or a conference join would undo it.
        self.limit_time_for_event().await;
    }

    async fn apply_pwrd_limits(&mut self) {
        let board = self.get_board().await;
        let Some(level) = board.sec_levels.find_match(self.session.cur_security, &self.session.last_password) else {
            if board.sec_levels.is_empty() {
                return;
            }
            drop(board);
            self.session.time_limit = 10;
            self.session.batch_limit = 30;
            self.session.transfer_limits = TransferLimits {
                daily_allowance: Some(0),
                bytes_remaining: Some(0),
                ..Default::default()
            };
            self.session.bytes_remaining = 0;
            return;
        };
        let time_per_day = level.time_per_day;
        let batch_limit = level.batch_limit;
        // Earlier calls only count against the limit when the board enforces a daily one
        // and the level asks for it. A demo account shares its id, so it never carries
        // time over from whoever was on it before.
        let enforce_daily_time = board.config.system_control.enforce_daily_time_limit && level.enforce_time_limit && !level.is_demo_account;
        let mut limits = TransferLimits::from_security_level(level, self.get_bps().max(0) as u32);
        drop(board);
        let used_today = self.session.current_user.as_ref().map_or(0, |user| user.stats.minutes_today);
        self.session.time_limit = limits::session_time_limit(time_per_day, used_today, enforce_daily_time);
        self.session.batch_limit = if batch_limit == 0 { 30 } else { batch_limit as usize };
        // What the caller already spent today comes off the allowance, so re-reading the
        // level on a conference join cannot hand them a fresh one.
        if let Some(user) = &self.session.current_user {
            limits.charge_todays_usage(user.stats.today_dnld_bytes);
        }
        self.session.bytes_remaining = limits.bytes_remaining.unwrap_or(-1);
        self.session.transfer_limits = limits;
    }

    /// Minutes the caller has left before the board hangs up on them. `None` when the
    /// session is not limited at all, which is what a time limit of zero means.
    pub fn minutes_left(&self) -> Option<i64> {
        if self.session.time_limit == 0 {
            return None;
        }
        let online = (Utc::now() - self.session.login_date).num_minutes();
        Some(self.session.time_limit as i64 - online)
    }

    /// Bytes the caller may still download, counting what the flagged batch will take.
    /// `None` when nothing constrains them.
    fn bytes_available(&self) -> Option<i64> {
        let Some(user) = &self.session.current_user else {
            return None;
        };
        let history = TransferHistory {
            num_uploads: user.stats.num_uploads,
            num_downloads: user.stats.num_downloads,
            total_upld_bytes: user.stats.total_upld_bytes,
            total_dnld_bytes: user.stats.total_dnld_bytes,
        };
        let mut limits = self.session.transfer_limits.clone();
        limits.bytes_remaining = (self.session.bytes_remaining >= 0).then_some(self.session.bytes_remaining);
        let free_areas: Vec<&Path> = self
            .session
            .current_conference
            .directories
            .as_ref()
            .map(|directories| directories.iter())
            .into_iter()
            .flatten()
            .filter(|area| area.is_free)
            .map(|area| area.path.as_path())
            .collect();
        let flagged = self
            .session
            .flagged_files
            .iter()
            .filter(|file| !file.parent().is_some_and(|dir| free_areas.contains(&dir)))
            .filter_map(|f| std::fs::metadata(f).ok())
            .map(|m| m.len())
            .sum();
        limits.bytes_available(&history, BatchSoFar { files: 0, bytes: flagged })
    }

    fn unlimited_text(&mut self) -> String {
        self.get_display_text(IceText::Unlimited).unwrap_or_default()
    }

    /// A conference may raise or lower the security level of the caller while they
    /// are in it. `PCBoard`'s `ConfPwrdAdjust` re-reads PWRD when that happens.
    async fn apply_conference_security(&mut self) {
        let Some(user) = &self.session.current_user else {
            return;
        };
        let base = if self.session.subscription_expired {
            user.exp_security_level
        } else {
            user.security_level
        } as i32;
        let adjusted = (base + self.session.current_conference.add_conference_security).clamp(0, u8::MAX as i32) as u8;
        if adjusted == self.session.cur_security {
            return;
        }
        self.session.cur_security = adjusted;
        if self.get_board().await.config.system_control.reread_sec_level_on_join {
            self.apply_security_level_limits().await;
        }
    }

    /// Selects a conference and applies its session state without displaying any
    /// of the files or prompts a caller sees while joining it.
    pub async fn set_current_conference(&mut self, conference: u16) -> Res<bool> {
        if (conference as usize) >= self.get_board().await.conferences.len() {
            return Ok(false);
        }
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
        self.apply_conference_security().await;
        Ok(true)
    }

    pub async fn join_conference(&mut self, conference: u16, quick_join: bool, show_intro: bool) -> Res<()> {
        let news_behavior = self.board.lock().await.config.switches.display_news_behavior;
        let scan_new_blt = self.board.lock().await.config.switches.scan_new_blt;
        let display_userinfo_at_login = self.board.lock().await.config.switches.display_userinfo_at_login;

        let (show_news, only_new) = match news_behavior {
            super::icb_config::DisplayNewsBehavior::OnlyNewer | super::icb_config::DisplayNewsBehavior::OncePerDay => (true, true),

            super::icb_config::DisplayNewsBehavior::Always => (true, false),
            super::icb_config::DisplayNewsBehavior::Never => (false, false),
        };

        // Everything below the intro only happens the first time round.
        let first_join = !self.session.joined_conferences.contains(&conference);

        if !self.set_current_conference(conference).await? {
            return Ok(());
        }

        if show_news {
            self.display_news(only_new).await?;
        }

        if (self.get_board().await.config.switches.force_intro_on_join || show_intro) && self.session.current_conference.intro_file.is_file() {
            let f = self.session.current_conference.intro_file.clone();
            self.display_file(&f).await?;
        }

        if first_join {
            self.ask_to_view_conference_members(quick_join).await?;
            if scan_new_blt {
                self.scan_new_bulletins().await?;
            }
            self.ask_to_scan_message_base().await?;
        }
        self.session.joined_conferences.insert(conference);

        if display_userinfo_at_login {
            let sec: SecurityExpression = self.session.user_command_level.cmd_v.clone();
            if !sec.session_can_access(&self.session) {
                self.view_settings().await?;
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
                log::error!("Error in more prompt: {err}");
            }
        }
        Ok(())
    }

    /// Answers `false` when the PPE gave up or failed, which is what keeps a script
    /// questionnaire from saving the answers it collected.
    pub async fn run_ppe<P: AsRef<Path>>(&mut self, file_name: &P, answer_file: Option<&Path>) -> Res<bool> {
        let mut keep_answers = false;
        match Executable::read_file(&file_name, false) {
            Ok(executable) => {
                keep_answers = self.run_executable(file_name, answer_file, executable).await?;
            }
            Err(err) => {
                log::error!("Error loading PPE {}: {}", file_name.as_ref().display(), err);
                self.session.op_text = format!("{err}");
                self.display_text(IceText::ErrorLoadingPPE, display_flags::LFBEFORE | display_flags::LFAFTER)
                    .await?;
            }
        }
        // clear all ppe parameters
        self.session.tokens.clear();
        Ok(keep_answers)
    }

    pub async fn run_executable<P: AsRef<Path>>(&mut self, file_name: &P, answer_file: Option<&Path>, executable: Executable) -> Res<bool> {
        // PCBoard stacked no more than 16 PPEs, doScript() refused the next one.
        // See MAX_SCR_STK in SCRMISC.CPP.
        const MAX_PPE_NESTING: usize = 16;
        if self.ppe_nesting >= MAX_PPE_NESTING {
            log::warn!("PPE nesting limit reached, not running {}", file_name.as_ref().display());
            return Ok(false);
        }
        self.session.disp_options.no_change();
        let canonicalized_path: PathBuf = file_name.as_ref().canonicalize()?;

        let canonicalized_path = PathBuf::from(adjust_canonicalization(canonicalized_path));
        let parent = canonicalized_path.parent().unwrap().to_str().unwrap().to_string();
        let mut io: DiskIO = DiskIO::new(&parent, answer_file);
        self.ppe_nesting += 1;
        let result = run(&canonicalized_path, &executable, &mut io, self).await;
        self.ppe_nesting -= 1;
        if self.ppe_nesting == 0 {
            self.cleanup_ppl_media().await;
        }
        match result {
            Ok(keep_answers) => Ok(keep_answers),
            Err(err) => {
                log::error!("Error executing PPE {}: {}", canonicalized_path.display(), err);
                self.session.op_text = format!("{err}");
                self.display_text(IceText::ErrorExecPPE, display_flags::LFBEFORE | display_flags::LFAFTER)
                    .await?;
                Ok(false)
            }
        }
    }

    pub(crate) async fn cleanup_ppl_media(&mut self) {
        if let Some(handle) = self.term_input_handle {
            let _ = self.release_term_input(handle).await;
        }
        if let Some(recording) = self.ppl_terminal.finish_recording()
            && !recording.overflowed
        {
            let slot = recording.slot;
            let _ = self.define_ppl_macro(recording).await;
            let _ = self.play_ppl_macro(slot).await;
        }
        for slot in self.ppl_terminal.take_defined_slots() {
            let _ = self.print(TerminalTarget::Both, &dec_macro_definition(slot, &[], false)).await;
        }
        if self.ppl_terminal.take_update_depth() > 0 {
            let _ = self.print(TerminalTarget::Both, "\x1b[?2026l").await;
        }
        let margins_active = {
            let terminal = &self.display_screen().buffer.buffer.terminal_state;
            terminal.margins_top_bottom().is_some() || terminal.margins_left_right().is_some()
        };
        if margins_active {
            let _ = self.print(TerminalTarget::Both, "\x1b[r\x1b[?69l").await;
        }
        if self.ppl_mouse.is_enabled() {
            self.ppl_mouse.disable();
            let _ = self.connection.send(ppl_mouse::MOUSE_OFF_SEQUENCE).await;
        }
        if self.ppl_keys.is_enabled() {
            self.ppl_keys.disable();
            let _ = self.connection.send(b"\x1b[=2l\x1b[=1l").await;
        }
        if self.ppl_graphics.take().is_some_and(|graphics| graphics.fullscreen) {
            let _ = self.connection.send(b"\x1b[?1070h\x1b[?80h\x1b[?7h\x1b[?25h").await;
        }
        for logical_channel in 0..self.sound_active.len() {
            if self.sound_active[logical_channel] {
                let channel = logical_channel + 2;
                let command = format!("\x1b_SyncTERM:A;Flush;C={channel};O=0\x1b\\");
                let _ = self.connection.send(command.as_bytes()).await;
                self.sound_active[logical_channel] = false;
            }
        }
        self.sound_active.fill(false);
        self.ppl_audio_notify.set_watching(false);
        self.reset_ppl_input_parsers();
        self.ppl_audio.fill(None);
        self.sound_volume.fill(100);
        self.gfx_error = -1;
    }

    pub fn create_term_input_handle(&mut self) -> Option<u64> {
        if self.term_input_handle.is_some() {
            return None;
        }
        let handle = self.next_term_input_handle;
        self.next_term_input_handle = self.next_term_input_handle.saturating_add(1).max(1);
        self.term_input_handle = Some(handle);
        Some(handle)
    }

    pub fn term_input_is_valid(&self, handle: u64) -> bool {
        handle != 0 && self.term_input_handle == Some(handle)
    }

    async fn release_term_input(&mut self, handle: u64) -> Res<bool> {
        if !self.term_input_is_valid(handle) {
            return Ok(false);
        }
        if self.ppl_mouse.is_enabled() {
            self.ppl_mouse.disable();
            self.connection.send(ppl_mouse::MOUSE_OFF_SEQUENCE).await?;
        }
        if self.ppl_keys.is_enabled() {
            self.ppl_keys.disable();
            self.connection.send(b"\x1b[=2l\x1b[=1l").await?;
        }
        self.ppl_event_keys.clear();
        self.term_input_handle = None;
        Ok(true)
    }

    pub async fn term_input_member(
        &mut self,
        handle: u64,
        name: &unicase::Ascii<String>,
        arguments: &[crate::executable::VariableValue],
    ) -> Res<crate::executable::VariableValue> {
        use crate::icy_board::state::ppl_terminal_input::{FREE, KEYBOARD_OFF, KEYBOARD_ON, MOUSE_OFF, MOUSE_ON, POLL, WAIT};

        if !self.term_input_is_valid(handle) {
            return Ok(if *name == *POLL || *name == *WAIT {
                self.empty_ppl_event().value()
            } else {
                crate::executable::VariableValue::new_bool(false)
            });
        }
        if *name == *POLL {
            return Ok(self.poll_ppl_event().await?.value());
        }
        if *name == *WAIT {
            let milliseconds = arguments[0].as_int();
            let timeout = (milliseconds >= 0).then(|| Duration::from_millis(milliseconds as u64));
            return Ok(self.wait_ppl_event(timeout).await?.value());
        }
        if *name == *MOUSE_ON {
            let mode = arguments[0].as_int();
            let tracking = arguments.get(1).map_or(2, crate::executable::VariableValue::as_int);
            if !self.ppl_mouse.enable(mode, tracking) {
                return Ok(crate::executable::VariableValue::new_bool(false));
            }
            let sequence = self.ppl_mouse.enable_sequence(tracking);
            self.connection.send(&sequence).await?;
            return Ok(crate::executable::VariableValue::new_bool(true));
        }
        if *name == *MOUSE_OFF {
            self.ppl_mouse.disable();
            self.connection.send(ppl_mouse::MOUSE_OFF_SEQUENCE).await?;
            return Ok(crate::executable::VariableValue::new_bool(true));
        }
        if *name == *KEYBOARD_ON {
            let suppress = arguments.first().is_some_and(crate::executable::VariableValue::as_bool);
            self.ppl_keys.enable();
            let sequence = if suppress {
                b"\x1b[=1h\x1b[=2h".as_slice()
            } else {
                b"\x1b[=2l\x1b[=1h".as_slice()
            };
            self.connection.send(sequence).await?;
            return Ok(crate::executable::VariableValue::new_bool(true));
        }
        if *name == *KEYBOARD_OFF {
            self.ppl_keys.disable();
            self.connection.send(b"\x1b[=2l\x1b[=1l").await?;
            return Ok(crate::executable::VariableValue::new_bool(true));
        }
        if *name == *FREE {
            return Ok(crate::executable::VariableValue::new_bool(self.release_term_input(handle).await?));
        }
        Err("Invalid TERMINPUT function".into())
    }

    /// A PPE leaves its own half finished sequences behind; the board reads the keyboard
    /// next and must not inherit them. Real keystrokes among them are handed back.
    fn reset_ppl_input_parsers(&mut self) {
        self.ppl_event_keys.clear();
        let stale: Vec<u8> = self
            .ppl_keys
            .take_pending_bytes()
            .into_iter()
            .chain(self.ppl_audio_notify.take_pending_bytes())
            .collect();
        for byte in stale {
            self.char_buffer.push_back(KeyChar::new(KeySource::User, byte as char));
        }
    }

    pub async fn flush_keyboard_input(&mut self) -> Res<()> {
        self.raw_input.clear();
        self.char_buffer.clear();
        self.ppl_event_keys.clear();
        self.ppl_keys.clear();
        let _ = self.ppl_audio_notify.take_pending_bytes();

        let mut input = [0u8; 64];
        while self.connection.try_read(&mut input).await? > 0 {}
        Ok(())
    }

    pub fn stuff_keyboard_buffer(&mut self, value: &str, is_visible: bool) -> Res<()> {
        let src = if is_visible { KeySource::StuffedVisible } else { KeySource::StuffedHidden };
        self.stuff_keyboard_buffer_from(value, src)
    }

    pub fn stuff_keyboard_buffer_from(&mut self, value: &str, src: KeySource) -> Res<()> {
        let in_chars: Vec<char> = value.chars().collect();

        let mut i = 0;
        while i < in_chars.len() {
            let c = in_chars[i];
            i += 1;
            if c == '^' && i < in_chars.len() {
                let next = in_chars[i].to_ascii_uppercase();
                if ('A'..='[').contains(&next) {
                    let ctrl_c = next as u8 - b'@';
                    self.char_buffer.push_back(KeyChar::new(src, ctrl_c as char));
                    i += 1;
                }
            } else {
                self.char_buffer.push_back(KeyChar::new(src, c));
            }
        }
        Ok(())
    }

    /// True while there are keystrokes left that KBDFILE put into the buffer.
    pub fn kbdfilused(&self) -> bool {
        self.char_buffer.iter().any(|c| c.source == KeySource::StuffedFile)
    }

    /// True when the next command will come out of a PPE's stuffed keyboard buffer.
    /// `PCBoard` skips CMD.LST in that case so a PPE started from CMD.LST cannot
    /// stuff its own keyword and re-trigger itself.
    pub fn ppl_typeahead(&self) -> bool {
        self.char_buffer.front().is_some_and(|c| c.source.is_stuffed())
    }

    pub async fn get_pcbdat(&self) -> Res<String> {
        let board = self.get_board().await;
        let path = board.resolve_file(&board.config.paths.tmp_work_path);

        if !path.is_dir() {
            fs::create_dir_all(&path)?;
        }
        let output = path.join("pcboard.dat");

        if let Err(err) = board.export_pcboard(&output) {
            log::error!("Error writing pcbdat.dat file: {err}");
            return Err(err);
        }
        Ok(output.to_str().unwrap().to_string())
    }

    pub async fn try_find_command(&self, command: &str, via_cmd_list: bool) -> Option<super::commands::Command> {
        let command = command.to_ascii_uppercase();
        if via_cmd_list {
            let conference = &self.session.current_conference.commands;
            let board = self.get_board().await;
            let found = super::commands::find_exact(conference, &command)
                .or_else(|| super::commands::find_exact(&board.commands, &command))
                .or_else(|| super::commands::find_prefix(conference, &command))
                .or_else(|| super::commands::find_prefix(&board.commands, &command));
            if let Some(cmd) = found {
                return Some(cmd.clone());
            }
        }

        // A built-in carries no security of its own, so it answers to the level the
        // board configured for that command.
        let mut cmd = Self::builtin_command(command)?;
        if let Some(action) = cmd.actions.first() {
            cmd.security = self
                .session
                .sysop_command_level
                .security_for(&action.command_type)
                .unwrap_or_else(|| self.session.user_command_level.security_for(&action.command_type));
        }
        Some(cmd)
    }

    /// The commands the board answers to when no command list claims the keyword.
    fn builtin_command(command: String) -> Option<super::commands::Command> {
        match command.as_str() {
            "A" => convert_cmd(CommandType::AbandonConference),
            "B" => convert_cmd(CommandType::BulletinList),
            "C" => convert_cmd(CommandType::CommentToSysop),
            "D" => convert_cmd(CommandType::Download),
            "E" => convert_cmd(CommandType::EnterMessage),
            "RM" => convert_cmd(CommandType::ReadMemorizedMessage(0)),
            "RM+" => convert_cmd(CommandType::ReadMemorizedMessage(1)),
            "RM-" => convert_cmd(CommandType::ReadMemorizedMessage(2)),
            "F" => convert_cmd(CommandType::FileDirectory),
            "BD" | "DB" => convert_cmd(CommandType::BatchDownload),
            "BU" | "UB" => convert_cmd(CommandType::BatchUpload),
            "G" => convert_cmd(CommandType::Goodbye),
            "?" | "H" => convert_cmd(CommandType::Help),
            "I" => convert_cmd(CommandType::InitialWelcome),
            "J" => convert_cmd(CommandType::JoinConference),
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
            "U" => convert_cmd(CommandType::UploadFile),
            "V" => convert_cmd(CommandType::ViewSettings),
            "W" => convert_cmd(CommandType::WriteSettings),
            "X" => convert_cmd(CommandType::ExpertMode),
            "Y" => convert_cmd(CommandType::YourMailScan),
            "Z" => convert_cmd(CommandType::ZippyDirectoryScan),
            "TS" => convert_cmd(CommandType::TextSearch),
            "1" => convert_cmd(CommandType::ViewCallerLog),
            "2" => convert_cmd(CommandType::ViewUserFile),
            "3" => convert_cmd(CommandType::PackMessageBase),
            "4" => convert_cmd(CommandType::RestoreMessage),
            "5" => convert_cmd(CommandType::HeaderScan),
            "6" => convert_cmd(CommandType::ViewTextFile),
            "7" => convert_cmd(CommandType::UserMaintenance),
            "8" => convert_cmd(CommandType::PackUserFile),
            "11" => convert_cmd(CommandType::NodeList),
            "12" => convert_cmd(CommandType::LogoffNode),
            "13" => convert_cmd(CommandType::NodeCallerLog),
            "16" => convert_cmd(CommandType::DirCommand),
            "@" => convert_cmd(CommandType::ReadEmail),
            "@W" => convert_cmd(CommandType::WriteEmail),
            _ => {
                // Like PCBoard, only words are abbreviated - a single letter is either one of
                // the commands above or nothing, so "D" can't be read as "DOOR".
                if command.len() < 2 {
                    return None;
                }
                if "ALIAS".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::EnableAlias);
                }
                if "BYE".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::Bye);
                }
                if "CHAT".starts_with(command.as_str()) || "NODE".starts_with(command.as_str()) {
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
                if "DOOR".starts_with(command.as_str()) || "OPEN".starts_with(command.as_str()) {
                    return convert_cmd(CommandType::OpenDoor);
                }
                if "DOWNLOAD".starts_with(command.as_str()) {
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
        }
    }

    pub fn resolve_path<P: AsRef<Path>>(&self, file: &P) -> PathBuf {
        if !file.as_ref().is_absolute() {
            return self.root_path.join(file);
        }
        file.as_ref().to_path_buf()
    }

    async fn shutdown_connections(&mut self) {
        self.session.request_logoff = true;
        let _ = self.connection.shutdown().await;

        if let Some(state) = self.node_state.lock().await[self.node].as_mut()
            && let Some(sysop_connection) = &mut state.sysop_connection
        {
            let _ = sysop_connection.shutdown().await;
        }
    }

    /// Appends one time stamped line to the caller log, `PCBoard`'s CALLER file.
    /// `PCBoard` kept one log per node; `icy_board` shares a single file, so the node
    /// is stamped on every line and sysop command 13 filters on it.
    pub async fn write_caller_log(&self, text: &str) {
        let path = {
            let board = self.get_board().await;
            if !board.config.options.call_log {
                return;
            }
            board.resolve_file(&board.config.paths.caller_log)
        };
        if path.as_os_str().is_empty() {
            return;
        }
        let line = format!("{} [{}] {}\r\n", chrono::Local::now().format("%H:%M"), self.node + 1, text);
        let result = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut file| std::io::Write::write_all(&mut file, line.as_bytes()));
        if let Err(err) = result {
            log::error!("Can't write to caller log {}: {}", path.display(), err);
        }
    }

    /// Writes the logon block of the caller log, the extra lines are switchable
    /// exactly like `PCBoard`'s `LogCallerNumber`, `LogConnectStr` and `LogSecLevel`.
    pub async fn log_logon_to_caller_log(&mut self) {
        let (log_number, log_connect, log_security, caller_number) = {
            let board = self.get_board().await;
            (
                board.config.options.log_caller_number,
                board.config.options.log_connect_string,
                board.config.options.log_security_level,
                board.statistics.cur_caller_number(),
            )
        };
        let name = self.session.user_name.clone();
        self.write_caller_log(&format!("{name} logged on node {}", self.node + 1)).await;
        if log_connect {
            let connection = self.node_state.lock().await[self.node]
                .as_ref()
                .map_or(String::new(), |state| format!("{:?}", state.connection_type));
            self.write_caller_log(&format!("Connect: {connection}")).await;
        }
        if log_number {
            self.write_caller_log(&format!("Caller Number: {caller_number}")).await;
        }
        if log_security {
            self.write_caller_log(&format!("Security Level: {}", self.session.cur_security)).await;
        }
    }

    pub async fn set_current_user(&mut self, user_number: usize, join_conference: bool) -> Res<()> {
        self.session.cur_user_id = user_number as i32;
        if let Some(state) = self.node_state.lock().await[self.node].as_mut() {
            state.cur_user = user_number as i32;
            state.graphics_mode = self.session.disp_options.grapics_mode;
        }
        if user_number >= self.get_board().await.users.len() {
            log::error!("User number {user_number} is out of range");
            return Err(IcyBoardError::UserNumberInvalid(user_number).into());
        }
        let mut user = self.get_board().await.users[user_number].clone();

        // The daily figures belong to the day they were made on, and `last_on` still holds
        // the previous call until the user is saved again.
        if user.stats.last_on.date_naive() != Utc::now().date_naive() {
            user.stats.minutes_today = 0;
            user.stats.today_num_downloads = 0;
            user.stats.today_num_uploads = 0;
            user.stats.today_dnld_bytes = 0;
            user.stats.today_upld_bytes = 0;
        }

        let old_language = self.session.language.clone();
        user.stats.num_times_on += 1;
        let last_conference: u16 = user.last_conference;
        self.get_board().await.statistics.add_caller(user.get_name().clone());
        self.get_board().await.save_statistics()?;
        if !user.date_format.is_empty() {
            self.session.date_format.clone_from(&user.date_format);
        }
        self.session.language.clone_from(&user.language);
        let subscription = self.get_board().await.config.subscription_info.clone();
        self.session.subscription_expired = super::subscription::status(
            subscription.is_enabled,
            user.expiration_date,
            subscription.warning_days,
            self.session.login_date.date_naive(),
        )
        .is_expired();
        self.session.cur_security = if self.session.subscription_expired {
            user.exp_security_level
        } else {
            user.security_level
        };
        self.session.page_len = user.page_len;
        self.session.user_name.clone_from(user.get_name());
        self.session.alias_name.clone_from(&user.alias);
        self.session.fse_mode = user.flags.fse_mode.clone();

        self.session.current_user = Some(user);
        self.apply_security_level_limits().await;
        if self.session.language != old_language {
            self.update_language().await;
        }
        if join_conference {
            let conference = if self.subscription_can_access_conference(last_conference) {
                last_conference
            } else {
                0
            };
            self.join_conference(conference, false, false).await?;
        }
        Ok(())
    }

    pub async fn save_current_user(&mut self) -> Res<()> {
        let old_language = self.session.language.clone();
        self.session.date_format = if let Some(user) = &self.session.current_user {
            self.session.language.clone_from(&user.language);
            self.session.fse_mode = user.flags.fse_mode.clone();
            if user.date_format.is_empty() {
                self.session.date_format.clone()
            } else {
                user.date_format.clone()
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
        if self.session.disp_options.grapics_mode == GraphicsMode::Rip
            && let Some(result) = self.find_more_specific_file_with_language(base_name.clone() + "r")
        {
            return Some(result);
        }
        if self.session.disp_options.grapics_mode == GraphicsMode::Avatar
            && let Some(result) = self.find_more_specific_file_with_language(base_name.clone() + "v")
        {
            return Some(result);
        }
        if self.session.disp_options.grapics_mode != GraphicsMode::Ctty
            && let Some(result) = self.find_more_specific_file_with_language(base_name.clone() + "g")
        {
            return Some(result);
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
        let txt = self.display_text.get_display_text(node_status.text()).unwrap();
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
        if self.get_board().await.config.options.give_user_password_to_doors
            && let Some(user) = &self.session.current_user
        {
            return user.password.password.to_string();
        }

        "SECRET".to_string()
    }

    pub async fn get_board(&'_ self) -> tokio::sync::MutexGuard<'_, IcyBoard> {
        self.board.lock().await
    }

    /// The event the board is heading towards, if any is scheduled.
    pub async fn event_window(&self) -> Option<EventWindow> {
        let board = self.board.lock().await;
        events::next_window(&board.config.event, &board.events, &chrono::Local::now())
    }

    /// Cuts the session short so that the caller is gone before the event starts.
    pub async fn limit_time_for_event(&mut self) {
        let now = chrono::Local::now();
        let Some(window) = self.event_window().await else {
            return;
        };
        // Never zero, which reads as an unlimited session; a caller this close to an
        // event gets a minute and is then hung up on.
        let minutes = (window.minutes_until_suspend(&now) as i32).max(1);
        if self.session.time_limit == 0 || minutes < self.session.time_limit {
            self.session.time_limit = minutes;
            self.session.time_adjusted_for_event = true;
        }
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

    /// Rings the sysop. Returns true when the page was answered or given up on,
    /// false when it rang out.
    pub async fn page_sysop(&mut self) -> Res<bool> {
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
                    return Ok(true);
                }
                if ch.source == KeySource::Sysop {
                    self.chat().await?;
                    self.display_text(IceText::SysopChatEnded, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                    return Ok(true);
                }
            }
        }

        Ok(false)
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
                    pattern = format!("(?i){pattern}");
                }
                if let Ok(r) = Regex::new(&pattern) {
                    self.session.search_pattern = Some(r);
                    return true;
                }
                log::error!("Error parsing search pattern: {pattern}");
            }
            Err(err) => log::error!("Error parsing search pattern: {err}"),
        }
        false
    }

    pub fn stop_search(&mut self) {
        self.session.search_pattern = None;
    }

    pub fn is_lockedout(&self, conf_number: u16) -> bool {
        if let Some(user) = &self.session.current_user
            && let Some(flags) = user.conference_flags.get(&(conf_number as usize))
            && flags.contains(ConferenceFlags::Expired)
            && !flags.contains(ConferenceFlags::Registered)
        {
            return true;
        }
        false
    }

    pub fn subscription_can_access_conference(&self, conf_number: u16) -> bool {
        if !self.session.subscription_expired || conf_number == 0 || self.session.is_sysop {
            return true;
        }
        self.session
            .current_user
            .as_ref()
            .and_then(|user| user.conference_flags.get(&(conf_number as usize)))
            .is_some_and(|flags| super::subscription::conference_access(true, *flags))
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

        if let Some(user) = &self.session.current_user
            && let Some(flags) = user.conference_flags.get(&(conf_number as usize))
            && flags.contains(ConferenceFlags::Selected)
        {
            return true;
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

    pub async fn create_password(&self, pw1: impl Into<String>) -> Password {
        let pw1 = pw1.into().to_lowercase();
        match self.get_board().await.config.system_control.password_storage_method {
            super::icb_config::PasswordStorageMethod::Argon2 => Password::new_argon2(pw1),
            super::icb_config::PasswordStorageMethod::BCrypt => Password::new_bcrypt(pw1),
            super::icb_config::PasswordStorageMethod::PlainText => Password::PlainText(pw1),
        }
    }

    async fn scan_new_bulletins(&mut self) -> Res<()> {
        let sec = self.session.user_command_level.cmd_b.clone();
        if !sec.session_can_access(&self.session) {
            return Ok(());
        }

        self.reset_color(TerminalTarget::Both).await?;
        self.display_text(IceText::ScanningBulletins, display_flags::LFBEFORE).await?;

        let prev_login = if let Some(user) = &self.session.current_user {
            user.stats.last_on
        } else {
            // If no user context, use a very old date so nothing is considered new
            Utc::now() - chrono::Duration::days(365 * 50)
        };
        let Some(bulletins) = self.session.current_conference.bulletins.clone() else {
            return Ok(());
        };

        for (i, b) in bulletins.iter().enumerate() {
            // Skip if bulletin requires higher security
            if !b.required_security.session_can_access(&self.session) {
                continue;
            }

            let path = self.resolve_path(&b.path);

            // If the file doesn’t exist, silently skip
            let Ok(meta) = std::fs::metadata(&path) else {
                continue;
            };

            let Ok(modified_sys) = meta.modified() else {
                continue;
            };

            // Convert SystemTime → DateTime<Utc>
            let modified: chrono::DateTime<Utc> = modified_sys.into();

            // Only list bulletins modified after the previous login
            if modified > prev_login {
                /*
                                let name = path
                                    .file_name()
                                    .map(|s| s.to_string_lossy().to_string())
                                    .unwrap_or_else(|| path.to_string_lossy().to_string());
                */

                self.backupcleareol(TerminalTarget::Both, self.user_screen.buffer.caret.x as usize).await?;
                self.new_line().await?;
                self.set_color(TerminalTarget::Both, IcbColor::dos_light_red()).await?;
                self.display_text(IceText::BulletinsUpdated, display_flags::NEWLINE).await?;
                self.display_text(IceText::NewBulletins, display_flags::DEFAULT).await?;
                self.println(TerminalTarget::Both, &format!("{}", i + 1)).await?;
            }
            if self.session.disp_options.abort_printout {
                break;
            }
            self.new_line().await?;
        }
        self.session.disp_options.check_display_status();
        Ok(())
    }

    async fn backupcleareol(&mut self, target: TerminalTarget, num_cols: usize) -> Res<()> {
        // If we don't need to move, just clear EOL.
        if num_cols == 0 {
            return self.clear_eol(target).await;
        }

        // ANSI / graphics-capable terminals
        if self.use_ansi() {
            // Move left num_cols then clear to end of line.
            // Example: ESC[{n}D moves cursor left n cols, ESC[K clears to end of line.
            let seq = format!("\x1B[{num_cols}D\x1B[K");
            self.write_raw(target, seq.chars().collect::<Vec<char>>().as_slice()).await?;
            return Ok(());
        }

        for _ in 0..num_cols {
            self.print(target, "\x08 \x08").await?;
        }
        Ok(())
    }

    async fn list_channels(&mut self) -> Res<()> {
        // Get the group chat state
        let chat_state = self.bbs.lock().await.group_chat.clone();
        let chat_guard = chat_state.lock().await;

        // List all channels (1-255)
        let mut channels_shown = 0;
        for channel_num in 1..=255u8 {
            // Get channel info from the state
            let Ok(participants) = chat_guard.list_participants(channel_num) else {
                continue;
            };

            let room_idx = channel_num as usize;
            if room_idx >= chat_guard.rooms.len() {
                continue;
            }

            let room = &chat_guard.rooms[room_idx];
            let user_count = participants.len();

            // Skip empty channels unless they have a topic
            if user_count == 0 && room.topic.is_none() {
                continue;
            }

            // Channel number

            self.session.op_text = format!("{channel_num}");
            self.display_text(IceText::ChannelText, display_flags::LFBEFORE).await?;
            if let Some(topic) = &room.topic {
                let max_topic_len = 40;
                let display_topic = if topic.len() > max_topic_len {
                    format!("{}...", &topic[..max_topic_len - 3])
                } else {
                    topic.clone()
                };
                self.print(TerminalTarget::Both, &display_topic).await?;
            }

            self.new_line().await?;
            channels_shown += 1;

            // Check for abort or page break
            if self.session.disp_options.abort_printout {
                break;
            }
        }

        // Footer
        if channels_shown == 0 {
            self.display_text(IceText::NoChannelsInUse, display_flags::LFBEFORE).await?;
            return Ok(());
        }

        let len = self.user_screen.buffer.caret.x;
        self.new_line().await?;

        self.print(TerminalTarget::Both, &"=".repeat(len as usize)).await?;
        self.new_line().await?;

        Ok(())
    }

    pub async fn get_raw_byte(&mut self) -> Res<Option<u8>> {
        // Try to read a single byte from the connection
        let mut buf = [0u8; 1];
        match self.connection.read(&mut buf).await {
            Ok(1) => Ok(Some(buf[0])),
            // No data available
            _ => Ok(None), // Shouldn't happen with 1-byte buffer
        }
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

fn dec_macro_definition(slot: usize, bytes: &[u8], delete_all: bool) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut sequence = format!("\x1bP{slot};{};1!z", usize::from(delete_all));
    sequence.reserve(bytes.len() * 2 + 2);
    for byte in bytes {
        sequence.push(HEX[usize::from(byte >> 4)] as char);
        sequence.push(HEX[usize::from(byte & 0x0F)] as char);
    }
    sequence.push('\x1b');
    sequence.push('\\');
    sequence
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
            GraphicsMode::Ansi | GraphicsMode::Graphics | GraphicsMode::Rip => {
                self.print(target, &format!("\x1B[{y};{x}H")).await?;
            }
            GraphicsMode::Avatar => {
                self.print(target, &format!("\x16\x08{}{}", (x as u8) as char, (y as u8) as char)).await?;
            }
        }

        Ok(())
    }

    pub async fn backward(&mut self, chars: i32) -> Res<()> {
        if self.use_ansi() {
            self.write_raw(TerminalTarget::Both, format!("\x1B[{chars}D").chars().collect::<Vec<char>>().as_slice())
                .await
        } else {
            Ok(())
        }
    }

    pub async fn forward(&mut self, chars: i32) -> Res<()> {
        if self.use_ansi() {
            self.write_raw(TerminalTarget::Both, format!("\x1B[{chars}C").chars().collect::<Vec<char>>().as_slice())
                .await
        } else {
            Ok(())
        }
    }

    pub async fn up(&mut self, chars: i32) -> Res<()> {
        if self.use_ansi() {
            self.write_raw(TerminalTarget::Both, format!("\x1B[{chars}A").chars().collect::<Vec<char>>().as_slice())
                .await
        } else {
            Ok(())
        }
    }

    pub async fn down(&mut self, chars: i32) -> Res<()> {
        if self.use_ansi() {
            self.write_raw(TerminalTarget::Both, format!("\x1B[{chars}B").chars().collect::<Vec<char>>().as_slice())
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
        if let Some(regex) = &self.session.search_pattern.clone()
            && let Some(find) = regex.find(str)
        {
            // regex offsets are byte indices; convert them to char indices for `chars`.
            let start = str[..find.start()].chars().count();
            let end = str[..find.end()].chars().count();
            self.write_raw(target, &chars[..start]).await?;
            let old_color = self.user_screen.buffer.caret.attribute;
            if old_color.background() == 0 {
                self.set_color(target, IcbColor::Dos(0x70)).await?;
            } else {
                self.set_color(target, IcbColor::Dos(0x07)).await?;
            }
            self.write_raw(target, &chars[start..end]).await?;
            self.set_color(target, IcbColor::Dos(old_color.as_u8(icy_engine::IceMode::Blink))).await?;
            self.write_raw(target, &chars[end..]).await?;
            return Ok(());
        }

        self.write_raw(target, chars.as_slice()).await
    }

    pub async fn println(&mut self, target: TerminalTarget, str: &str) -> Res<()> {
        let line = str.chars().collect::<Vec<char>>();
        self.write_raw(target, line.as_slice()).await?;
        self.new_line().await
    }

    async fn write_chars(&mut self, target: TerminalTarget, data: &[char]) -> Res<()> {
        if self.ppl_terminal.record(
            target,
            data,
            self.session.term_caps.is_utf8,
            self.session.is_sysop,
            self.session.current_user.is_none(),
        ) {
            return Ok(());
        }
        let mut user_bytes = Vec::new();
        let mut sysop_bytes = Vec::new();
        let user_is_utf8 = self.session.term_caps.is_utf8;
        let mut buf = [0; 4];

        for c in data {
            if target != TerminalTarget::Sysop || self.session.is_sysop || self.session.current_user.is_none() {
                let _ = self.user_screen.print_char(*c);
                if user_is_utf8 {
                    let encoded = c.encode_utf8(&mut buf);
                    user_bytes.extend_from_slice(encoded.as_bytes());
                } else if let Some(&cp437) = UNICODE_TO_CP437.get(c) {
                    user_bytes.push(cp437);
                } else {
                    user_bytes.push(b'.');
                }
            }
            if target != TerminalTarget::User {
                let _ = self.sysop_screen.print_char(*c);
                if let Some(&cp437) = UNICODE_TO_CP437.get(c) {
                    sysop_bytes.push(cp437);
                } else {
                    sysop_bytes.push(b'.');
                }
            }
            if *c == '\n' {
                self.write_chars_internal(target, &user_bytes, &sysop_bytes).await?;
                user_bytes.clear();
                sysop_bytes.clear();
            }
        }
        self.write_chars_internal(target, &user_bytes, &sysop_bytes).await?;
        Ok(())
    }

    pub async fn play_ppl_macro(&mut self, slot: usize) -> Res<bool> {
        if !self.ppl_terminal.is_defined(slot) {
            return Ok(false);
        }
        self.print(TerminalTarget::Both, &format!("\x1b[{slot}*z")).await?;
        Ok(true)
    }

    pub async fn finish_ppl_macro(&mut self) -> Res<Option<bool>> {
        let Some(recording) = self.ppl_terminal.finish_recording() else {
            return Ok(None);
        };
        if recording.overflowed {
            return Ok(Some(false));
        }
        self.define_ppl_macro(recording).await?;
        Ok(Some(true))
    }

    async fn define_ppl_macro(&mut self, recording: ppl_terminal_control::FinishedMacro) -> Res<()> {
        let slot = recording.slot;
        if self.session.is_sysop || self.session.current_user.is_none() {
            self.print(TerminalTarget::Both, &dec_macro_definition(slot, &recording.user_bytes, false))
                .await?;
            self.ppl_terminal.mark_defined(slot);
            return Ok(());
        }
        self.print(TerminalTarget::User, &dec_macro_definition(slot, &recording.user_bytes, false))
            .await?;
        self.print(TerminalTarget::Sysop, &dec_macro_definition(slot, &recording.sysop_bytes, false))
            .await?;
        self.ppl_terminal.mark_defined(slot);
        Ok(())
    }

    pub async fn delete_ppl_macro(&mut self, slot: usize) -> Res<()> {
        self.print(TerminalTarget::Both, &dec_macro_definition(slot, &[], false)).await?;
        self.ppl_terminal.mark_deleted(slot);
        Ok(())
    }

    pub async fn clear_ppl_macros(&mut self) -> Res<()> {
        self.print(TerminalTarget::Both, &dec_macro_definition(0, &[], true)).await?;
        self.ppl_terminal.clear_defined();
        Ok(())
    }

    /// Starts capturing everything the caller sees into `path`, replacing any
    /// capture already running. Reports whether the file could be opened.
    pub fn open_capture(&mut self, path: &Path) -> bool {
        match std::fs::File::create(path) {
            Ok(file) => {
                self.capture_file = Some(file);
                true
            }
            Err(err) => {
                log::error!("Can't open capture file {}: {}", path.display(), err);
                self.capture_file = None;
                false
            }
        }
    }

    /// Stops the capture. Closing one that was never opened is not an error.
    pub fn close_capture(&mut self) {
        self.capture_file = None;
    }

    async fn write_chars_internal(&mut self, target: TerminalTarget, user_bytes: &[u8], sysop_bytes: &[u8]) -> Res<()> {
        if (target != TerminalTarget::Sysop || self.session.is_sysop) && !user_bytes.is_empty() {
            // A capture sees what the caller sees, whether or not the display is on.
            if let Some(file) = &mut self.capture_file {
                let _ = std::io::Write::write_all(file, user_bytes);
            }
            self.connection.send(user_bytes).await?;
        }

        if target != TerminalTarget::User && !sysop_bytes.is_empty() {
            // Send user only not to other connections
            let mut node_state = self.node_state.lock().await;
            match node_state[self.node].as_mut() {
                Some(ns) => {
                    if let Some(sysop_connection) = &mut ns.sysop_connection {
                        let _ = sysop_connection.send(sysop_bytes).await;
                    }
                }
                None => {
                    log::error!("Node {} was empty", self.node);
                }
            }
        }
        match target {
            TerminalTarget::Both | TerminalTarget::User => {
                if !user_bytes.is_empty() {
                    self.session.cursor_pos = self.user_screen.buffer.caret.position();
                }
            }
            TerminalTarget::Sysop => {
                if !sysop_bytes.is_empty() {
                    self.session.cursor_pos = self.sysop_screen.buffer.caret.position();
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
                        // Only a URL macro carries link text, so only there does a space stay inside.
                        let keeps_space = *c == ' ' && s.get(..4).is_some_and(|prefix| prefix.eq_ignore_ascii_case("URL:"));
                        if c.is_whitespace() && !keeps_space {
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
                        if c.is_ascii_hexdigit() {
                            let color = (c.to_digit(16).unwrap() | (ch1.to_digit(16).unwrap() << 4)) as u8;

                            if color == 0 {
                                self.session.saved_color = self.cur_color();
                            } else if color == 0xFF {
                                self.set_color(target, self.cur_color()).await?;
                            } else {
                                self.set_color(target, color.into()).await?;
                            }
                        } else {
                            self.write_chars(target, &['@', ch1, *c]).await?;
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
                    result = user.alias.clone();
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
            MacroCommand::BoardName => result = self.get_board().await.config.board.name.clone(),
            MacroCommand::BPS | MacroCommand::Carrier => result = self.get_bps().to_string(),

            MacroCommand::ByteCredit => result = self.session.transfer_limits.byte_credit.to_string(),
            MacroCommand::ByteLimit => {
                result = match self.session.transfer_limits.daily_allowance {
                    None => self.unlimited_text(),
                    Some(bytes) => bytes.to_string(),
                }
            }
            MacroCommand::ByteRatio => {
                if let Some(user) = &self.session.current_user {
                    result = limits::format_ratio(user.stats.total_dnld_bytes, user.stats.total_upld_bytes);
                }
            }
            MacroCommand::BytesLeft => {
                result = match self.bytes_available() {
                    None => self.unlimited_text(),
                    Some(bytes) => bytes.to_string(),
                }
            }
            MacroCommand::KBLeft => {
                result = match self.bytes_available() {
                    None => self.unlimited_text(),
                    Some(bytes) => (bytes / 1024).to_string(),
                }
            }
            MacroCommand::City => {
                if let Some(user) = &self.session.current_user {
                    result = user.city_or_state.clone();
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
            MacroCommand::ConfName => result = self.session.current_conference.name.clone(),
            MacroCommand::ConfNum => result = self.session.current_conference_number.to_string(),

            MacroCommand::CredLeft
            | MacroCommand::CredNow
            | MacroCommand::CredStart
            | MacroCommand::CredUsed
            | MacroCommand::Event
            | MacroCommand::FreeSpace
            | MacroCommand::IName
            | MacroCommand::LastCallerNode
            | MacroCommand::LastCallerSystem
            | MacroCommand::OffHours
            | MacroCommand::PwxDate
            | MacroCommand::PwxDays => {
                // todo
            }

            MacroCommand::CurMsgNum => {
                result = self.session.current_messagenumber.to_string();
            }

            MacroCommand::DataPhone => {
                if let Some(user) = &self.session.current_user {
                    result = user.bus_data_phone.clone();
                }
            }
            MacroCommand::DayBytes => {
                if let Some(user) = &self.session.current_user {
                    result = user.stats.today_dnld_bytes.to_string();
                }
            }
            MacroCommand::DlBytes => {
                if let Some(user) = &self.session.current_user {
                    result = user.stats.total_dnld_bytes.to_string();
                }
            }
            MacroCommand::DlFiles => {
                if let Some(user) = &self.session.current_user {
                    result = user.stats.num_downloads.to_string();
                }
            }
            MacroCommand::Delay(delay) => {
                sleep(Duration::from_millis(*delay as u64 * 10)).await;
                return None;
            }
            MacroCommand::ExpDate => {
                let enabled = self.get_board().await.config.subscription_info.is_enabled;
                if enabled
                    && let Some(user) = &self.session.current_user
                    && user.expiration_date != DateTime::<Utc>::default()
                {
                    result = self.format_date(user.expiration_date);
                }
                if result.is_empty() {
                    result = "00-00-00".to_string();
                }
            }
            MacroCommand::ExpDays => {
                let enabled = self.get_board().await.config.subscription_info.is_enabled;
                if let Some(user) = &self.session.current_user {
                    result = super::subscription::days_until_expiration(enabled, user.expiration_date, self.session.login_date.date_naive())
                        .map_or_else(|| self.unlimited_text(), |days| days.to_string());
                }
            }
            MacroCommand::FileCredit => result = self.session.transfer_limits.file_credit.to_string(),
            MacroCommand::FileRatio => {
                if let Some(user) = &self.session.current_user {
                    result = limits::format_ratio(user.stats.num_downloads, user.stats.num_uploads);
                }
            }
            MacroCommand::First => {
                result = fix_casing(self.session.get_first_name());
            }

            MacroCommand::FirstU => {
                result = self.session.get_first_name().to_uppercase();
            }
            MacroCommand::FNum => {
                result = (self.session.flagged_files.len() + 1).to_string();
            }
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
                    result = user.home_voice_phone.clone();
                }
            }
            MacroCommand::HighMSGNum => result = self.session.high_msg_num.to_string(),
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
                            self.session.current_conference.name.clone(),
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

            MacroCommand::KBLimit => {
                result = match limits::kilobyte_limit(self.session.transfer_limits.daily_allowance, self.session.transfer_limits.total_byte_limit) {
                    None => self.unlimited_text(),
                    Some(kb) => kb.to_string(),
                }
            }
            MacroCommand::MaxBytes => {
                let limit = self.session.transfer_limits.total_byte_limit;
                result = if limit == 0 { self.unlimited_text() } else { limit.to_string() };
            }
            MacroCommand::MaxFiles => {
                let limit = self.session.transfer_limits.total_file_limit;
                result = if limit == 0 { self.unlimited_text() } else { limit.to_string() };
            }
            MacroCommand::RatioBytes => {
                let ratio = self.session.transfer_limits.byte_ratio_tenths;
                result = if ratio == 0 {
                    self.unlimited_text()
                } else {
                    format!("{}.{}:1", ratio / 10, ratio % 10)
                };
            }
            MacroCommand::RatioFiles => {
                let ratio = self.session.transfer_limits.file_ratio_tenths;
                result = if ratio == 0 {
                    self.unlimited_text()
                } else {
                    format!("{}.{}:1", ratio / 10, ratio % 10)
                };
            }
            MacroCommand::FBytes => {
                result = self
                    .session
                    .flagged_files
                    .iter()
                    .filter_map(|f| std::fs::metadata(f).ok())
                    .map(|m| m.len())
                    .sum::<u64>()
                    .to_string();
            }
            MacroCommand::FFiles => result = self.session.flagged_files.len().to_string(),

            MacroCommand::MinLeft => {
                result = match self.minutes_left() {
                    None => self.unlimited_text(),
                    Some(minutes) => minutes.max(0).to_string(),
                }
            }
            MacroCommand::More => {
                if let Err(err) = self.more_promt().await {
                    log::error!("Error in more prompt: {err}");
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
                    result = dirs[self.session.current_file_directory].name.clone();
                } else {
                    result = String::new();
                }
            }
            MacroCommand::DirNum => {
                result = self.session.current_file_directory.to_string();
            }
            MacroCommand::AreaName => {
                if let Some(areas) = &self.session.current_conference.areas {
                    result = areas[self.session.current_message_area].name.clone();
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
            MacroCommand::OpText => result = self.session.op_text.clone(),
            MacroCommand::Pause => {
                self.session.disp_options.auto_more = true;
                let _ = self.press_enter().await;
                self.session.disp_options.auto_more = false;
                return None;
            }
            MacroCommand::POS(value) => {
                let x = self.user_screen.buffer.caret.x as usize;
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
                    result = user.protocol.clone();
                }
            }
            MacroCommand::ProDesc => {
                if let Some(user) = &self.session.current_user
                    && let Some(prot) = self.board.lock().await.protocols.find_protocol(&user.protocol)
                {
                    result = prot.description.clone();
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
            MacroCommand::RCPS => result = self.transfer_statistics.uploaded_cps.to_string(),
            MacroCommand::RBytes => result = self.transfer_statistics.uploaded_bytes.to_string(),
            MacroCommand::RFiles => result = self.transfer_statistics.uploaded_files.to_string(),
            MacroCommand::Real => {
                if let Some(user) = &self.session.current_user {
                    result = user.get_name().clone();
                }
            }
            MacroCommand::Security => {
                if let Some(user) = &self.session.current_user {
                    result = user.security_level.to_string();
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
            MacroCommand::SysopName => {
                if self.get_board().await.config.sysop.use_real_name {
                    result = self.get_board().await.users[0].name.clone();
                } else {
                    result = self.get_board().await.config.sysop.name.clone();
                }
            }
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
            MacroCommand::Url { label, uri } => {
                result = format!("\x1b]8;;{uri}\x1b\\{label}\x1b]8;;\x1b\\");
            }
            MacroCommand::User => {
                if let Some(user) = &self.session.current_user {
                    if self.session.use_alias {
                        if user.alias.is_empty() {
                            result.clone_from(&user.get_name().to_ascii_uppercase());
                        } else {
                            result.clone_from(&user.alias.to_ascii_uppercase());
                        }
                    } else {
                        result.clone_from(&user.get_name().to_ascii_uppercase());
                    }
                } else {
                    result = "0".to_string();
                }
            }
            MacroCommand::Version => {
                result = env!("CARGO_PKG_VERSION").to_string();
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
                if let Some(value) = self.get_env(id) {
                    result = value.clone();
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
        self.drain_raw_input();
        self.drain_stale_protocol_input();
        let stale = self.ppl_mouse.take_stale_keyboard();
        for byte in stale.into_iter().rev() {
            self.char_buffer.push_front(KeyChar::new(KeySource::User, byte as char));
        }
        if let Some(ch) = self.char_buffer.pop_front() {
            match target {
                TerminalTarget::Both => {
                    if ch.source == KeySource::User {
                        self.session.keyboard_timer_started = Instant::now();
                    }
                    return Ok(Some(ch));
                }
                TerminalTarget::User => {
                    if ch.source == KeySource::User || ch.source.is_stuffed() {
                        if ch.source == KeySource::User {
                            self.session.keyboard_timer_started = Instant::now();
                        }
                        return Ok(Some(ch));
                    }
                    self.char_buffer.push_back(ch);
                }
                TerminalTarget::Sysop => {
                    if ch.source == KeySource::Sysop {
                        return Ok(Some(ch));
                    }
                    self.char_buffer.push_back(ch);
                }
            }
        }

        if self.keyboard_timed_out().await? {
            return Ok(None);
        }
        self.check_time_left().await;
        if self.session.request_logoff {
            return Ok(None);
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
                        state.sysop_connection = Some(sysop_connection);
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
                        Some(BBSMessage::Shutdown(msg)) => {
                            self.shutdown_for_event(msg).await?;
                        }
                        Some(BBSMessage::GroupChat(event)) => {
                            self.handle_group_chat_event(event)?;
                        }
                        _ => {}
                    }
                    return Ok(None);
                }
                size = sysop_connection.read(&mut sysop_key_data) => {
                    // Both were taken out of the node, so a read that brought nothing has to
                    // hand them back too - otherwise this node never reads input again.
                    if let Some(state) = self.node_state.lock().await[self.node].as_mut() {
                        state.sysop_connection = Some(sysop_connection);
                        state.bbs_channel = Some(bbs_channel);
                    }
                    if let Ok(1) = size {
                        if target == TerminalTarget::User {
                            self.char_buffer.push_back(KeyChar::new(KeySource::Sysop, sysop_key_data[0] as char));
                            return Ok(None);
                        }

                        return Ok(Some(KeyChar::new(KeySource::Sysop, sysop_key_data[0] as char)));
                    }
                }
                size2 = self.connection.read(&mut user_key_data) => {
                    if let Some(state) = self.node_state.lock().await[self.node].as_mut() {
                        state.sysop_connection = Some(sysop_connection);
                        state.bbs_channel = Some(bbs_channel);
                    }
                    if let Ok(1) = size2 {
                        self.session.keyboard_timer_started = Instant::now();
                        let mut keys = self.process_user_input_byte(user_key_data[0]).into_iter();
                        let key = keys.next();
                        if target == TerminalTarget::Sysop {
                            if let Some(key) = key {
                                self.char_buffer.push_back(key);
                            }
                            self.char_buffer.extend(keys);
                            return Ok(None);
                        }
                        self.char_buffer.extend(keys);
                        return Ok(key);
                    }
                }
                () = sleep(Duration::from_millis(100)) => {
                    if let Some(state) = self.node_state.lock().await[self.node].as_mut() {
                        state.sysop_connection = Some(sysop_connection);
                        state.bbs_channel = Some(bbs_channel);
                    }
                    self.drain_stale_protocol_input();
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
                        Some(BBSMessage::SysopLogin) => {
                            self.print_sysop_screen().await?;
                        }
                        Some(BBSMessage::Broadcast(msg)) => {
                            self.show_broadcast(msg).await?;
                        }
                        Some(BBSMessage::Shutdown(msg)) => {
                            self.shutdown_for_event(msg).await?;
                        }
                        Some(BBSMessage::GroupChat(event)) => {
                            self.handle_group_chat_event(event)?;
                        }
                        _ => {}
                    }
                    return Ok(None);

                }
                size2 = self.connection.read(&mut user_key_data) => {
                    if let Some(state) = self.node_state.lock().await[self.node].as_mut() {
                        state.bbs_channel = Some(bbs_channel);
                    }
                    if let Ok(1) = size2 {
                        self.session.keyboard_timer_started = Instant::now();
                        let mut keys = self.process_user_input_byte(user_key_data[0]).into_iter();
                        let key = keys.next();
                        if target == TerminalTarget::Sysop {
                            // No sysop, only user
                            if let Some(key) = key {
                                self.char_buffer.push_back(key);
                            }
                            self.char_buffer.extend(keys);
                            return Ok(None);
                        }
                        self.char_buffer.extend(keys);
                        return Ok(key);
                    }
                }
                () = sleep(Duration::from_millis(100)) => {
                    if let Some(state) = self.node_state.lock().await[self.node].as_mut() {
                        state.bbs_channel = Some(bbs_channel);
                    }
                    self.drain_stale_protocol_input();
                    return Ok(None);
                }

            }
        }

        sleep(Duration::from_millis(100)).await;
        Ok(None)
    }

    fn process_user_input_byte(&mut self, byte: u8) -> Vec<KeyChar> {
        let mut keys = Vec::new();
        self.ppl_audio_notify.set_watching(self.sound_active.iter().any(|active| *active));
        for byte in self.ppl_mouse.feed(byte) {
            for byte in self.ppl_keys.feed(byte) {
                for byte in self.gfx_probe.feed(byte) {
                    for byte in self.ppl_audio_notify.feed(byte) {
                        keys.push(KeyChar::new(KeySource::User, byte as char));
                    }
                }
            }
        }
        keys
    }

    fn drain_stale_protocol_input(&mut self) {
        for byte in self.ppl_keys.take_stale_keyboard() {
            self.char_buffer.push_back(KeyChar::new(KeySource::User, byte as char));
        }
        for byte in self.ppl_audio_notify.take_stale_keyboard() {
            self.char_buffer.push_back(KeyChar::new(KeySource::User, byte as char));
        }
    }

    /// `TERMINPUT.Poll` never reaches `get_char`, so the sequences it parks have to be
    /// released here or they are held until something else reads the keyboard.
    fn drain_stale_event_input(&mut self) {
        self.drain_stale_protocol_input();
        for byte in self.ppl_mouse.take_stale_keyboard() {
            self.char_buffer.push_back(KeyChar::new(KeySource::User, byte as char));
        }
    }

    /// Asks the caller's terminal what it is and what it can do, once per call.
    pub async fn detect_terminal(&mut self) -> Res<()> {
        let (capabilities, probed) = TerminalCaps::detect(&mut *self.connection).await?;
        self.session.term_caps = capabilities;
        self.media_probed = true;
        self.take_probe_result(probed).await
    }

    /// Makes sure the media queries have gone out, for a caller that never went through
    /// the login probe - the built in terminal is one.
    pub async fn probe_terminal_media(&mut self) -> Res<()> {
        if self.media_probed {
            return Ok(());
        }
        self.media_probed = true;
        let probed = termcap_detect::probe_media(&mut *self.connection, &mut self.gfx_probe).await?;
        self.session.term_caps.gfx = probed.gfx;
        self.session.term_caps.sound = probed.sound;
        self.session.term_caps.answered |= probed.answered;
        self.take_probe_result(probed).await
    }

    /// Files the probe's leftovers where they belong: unread bytes back into the input,
    /// cache names into the caller's caches, and the sound formats worth asking about.
    async fn take_probe_result(&mut self, probed: termcap_detect::MediaProbeResult) -> Res<()> {
        self.raw_input.extend(probed.leftover);
        for name in probed.cache_listing {
            if name.starts_with(ppl_graphics::SOUND_CACHE_PREFIX) {
                self.sound_cache.insert(name);
            } else if name.starts_with(ppl_graphics::CACHE_PREFIX) {
                self.gfx_cache.insert(name);
            }
        }
        if self.session.term_caps.sound {
            for (format, major, subtype) in SOUND_FORMATS {
                self.probe_sound_format(*format, *major, *subtype).await?;
            }
        }
        Ok(())
    }

    pub async fn query_gfx_capabilities(&mut self) -> Res<termcap_detect::GfxCapabilities> {
        self.probe_terminal_media().await?;
        Ok(self.session.term_caps.gfx)
    }

    /// The file an `AUDIO` channel was loaded from, if it still holds one.
    pub fn ppl_audio_file(&self, channel: i32) -> Option<&String> {
        self.ppl_audio.get(usize::try_from(channel).ok()?)?.as_ref()
    }

    /// Takes the next free `AUDIO` channel for `file`, or nothing when all are in use.
    pub fn take_ppl_audio(&mut self, file: String) -> Option<i32> {
        let channel = self.ppl_audio.iter().position(Option::is_none)?;
        self.ppl_audio[channel] = Some(file);
        Some(channel as i32)
    }

    /// Gives an `AUDIO` channel back, so a long call can load more than it can hold.
    pub fn release_ppl_audio(&mut self, channel: i32) {
        if let Ok(channel) = usize::try_from(channel)
            && let Some(slot) = self.ppl_audio.get_mut(channel)
        {
            *slot = None;
        }
    }

    /// Waits for the terminal to report its cursor position, which it can only do
    /// once it has worked through everything that was sent before the request.
    pub async fn await_terminal_ack(&mut self) -> Res<()> {
        self.query_terminal_csi(b"\x1b[6n", |reply| {
            let body = reply.strip_prefix("\x1b[")?.strip_suffix('R')?;
            let (row, column) = body.split_once(';')?;
            row.parse::<u16>().ok()?;
            column.parse::<u16>().ok()?;
            Some(true)
        })
        .await?;
        Ok(())
    }

    /// Lets the terminal catch up after a media upload big enough that it would still
    /// be reading those bytes when the next query starts its own clock - which is how
    /// a probe ends up timing out behind a picture. Terminals that have never answered
    /// anything are not waited for.
    pub async fn acknowledge_upload(&mut self, bytes: usize) -> Res<()> {
        const LARGE_UPLOAD_BYTES: usize = 256 * 1024;

        if bytes < LARGE_UPLOAD_BYTES || !self.session.term_caps.answered {
            return Ok(());
        }
        self.await_terminal_ack().await
    }

    pub async fn query_terminal_csi(&mut self, query: &[u8], matches: impl Fn(&str) -> Option<bool>) -> Res<Option<bool>> {
        self.connection.send(query).await?;
        let deadline = Instant::now() + GFX_PROBE_TIMEOUT;
        let mut pending = Vec::new();
        while Instant::now() < deadline {
            let mut input = [0u8; 256];
            let read = if self.raw_input.is_empty() {
                self.connection.try_read(&mut input).await?
            } else {
                let read = self.raw_input.len().min(input.len());
                for byte in &mut input[..read] {
                    *byte = self.raw_input.pop_front().unwrap();
                }
                read
            };
            if read == 0 {
                sleep(Duration::from_millis(5)).await;
                continue;
            }
            for (index, byte) in input[..read].iter().enumerate() {
                if pending.is_empty() && *byte != 0x1b {
                    self.raw_input.push_back(*byte);
                    continue;
                }
                pending.push(*byte);
                if pending.len() == 2 && pending[1] != b'[' {
                    self.raw_input.extend(pending.drain(..));
                    continue;
                }
                if pending.len() > 2 && (0x40..=0x7e).contains(byte) {
                    let sequence = std::mem::take(&mut pending);
                    if let Ok(text) = std::str::from_utf8(&sequence)
                        && let Some(result) = matches(text)
                    {
                        self.raw_input.extend(&input[index + 1..read]);
                        self.session.term_caps.answered = true;
                        return Ok(Some(result));
                    }
                    self.raw_input.extend(sequence);
                }
            }
        }
        self.raw_input.extend(pending);
        Ok(None)
    }

    pub async fn query_sound_available(&mut self) -> Res<bool> {
        self.probe_terminal_media().await?;
        Ok(self.session.term_caps.sound)
    }

    pub async fn query_sound_format(&mut self, format: i32, major: u32, subtype: u32) -> Res<bool> {
        if let Some(supported) = self.sound_formats.get(&format) {
            return Ok(*supported);
        }
        if !self.query_sound_available().await? {
            return Ok(false);
        }
        self.probe_sound_format(format, major, subtype).await
    }

    async fn probe_sound_format(&mut self, format: i32, major: u32, subtype: u32) -> Res<bool> {
        if let Some(supported) = self.sound_formats.get(&format) {
            return Ok(*supported);
        }
        let query = format!("\x1b_SyncTERM:Q;libsndfileFormat;{major};{subtype}\x1b\\");
        let prefix = format!("\x1b[=7;101;{major};{subtype};");
        // A terminal that reported libsndfile but let this probe run out is taken at its
        // word: an APC it cannot use is ignored anyway, while a remembered "no" would
        // leave the format silent for the rest of the call. A large upload ahead of the
        // probe is enough to push the answer past the deadline.
        let supported = self
            .query_terminal_csi(query.as_bytes(), |reply| {
                let value = reply.strip_prefix(&prefix)?.strip_suffix('n')?;
                Some(value == "1")
            })
            .await?
            .unwrap_or(true);
        self.sound_formats.insert(format, supported);
        Ok(supported)
    }

    /// Turns bytes held back by a capability probe into whatever the caller is reading now.
    fn drain_raw_input(&mut self) {
        while let Some(byte) = self.raw_input.pop_front() {
            let keys = self.process_user_input_byte(byte);
            self.char_buffer.extend(keys);
        }
    }

    fn take_pending_ppl_event(&mut self) -> Option<ppl_events::PplEvent> {
        let dropped = self.ppl_keys.take_dropped().saturating_add(self.ppl_mouse.take_dropped());
        if dropped > 0 {
            return Some(self.ppl_event_with_mode(ppl_events::PplEvent::overflow(dropped)));
        }
        if let Some(event) = self.ppl_event_keys.poll() {
            return Some(self.ppl_event_with_mode(event));
        }
        if let Some(channel) = self.ppl_audio_notify.poll() {
            if let Some(active) = self.sound_active.get_mut(channel as usize) {
                *active = false;
            }
            return Some(self.ppl_event_with_mode(ppl_events::PplEvent::sound(channel)));
        }
        while let Some(key) = self.char_buffer.pop_front() {
            self.ppl_event_keys.feed(key);
            if let Some(event) = self.ppl_event_keys.poll() {
                return Some(self.ppl_event_with_mode(event));
            }
        }
        if self.ppl_keys.poll() {
            return Some(self.ppl_event_with_mode(ppl_events::PplEvent::key_edge(self.ppl_keys.current())));
        }
        let event_type = self.ppl_mouse.poll();
        (event_type != ppl_mouse::MOUSE_EVENT_NONE).then(|| self.ppl_event_with_mode(ppl_events::PplEvent::mouse(self.ppl_mouse.current())))
    }

    fn process_ppl_event_byte(&mut self, byte: u8) -> Option<ppl_events::PplEvent> {
        let keys = self.process_user_input_byte(byte);
        self.char_buffer.extend(keys);
        self.take_pending_ppl_event()
    }

    /// Nothing happened, timed like any other event so a program can read the clock
    /// from a poll that came back empty.
    fn empty_ppl_event(&self) -> ppl_events::PplEvent {
        self.ppl_event_with_mode(ppl_events::PplEvent::default())
    }

    fn ppl_event_with_mode(&self, mut event: ppl_events::PplEvent) -> ppl_events::PplEvent {
        event.pixels = self.ppl_mouse.pixels();
        event.time = self.ppl_event_keys.elapsed_ms();
        event
    }

    pub async fn poll_ppl_event(&mut self) -> Res<ppl_events::PplEvent> {
        self.drain_stale_event_input();
        if let Some(event) = self.take_pending_ppl_event() {
            return Ok(event);
        }
        while let Some(byte) = self.raw_input.pop_front() {
            if let Some(event) = self.process_ppl_event_byte(byte) {
                return Ok(event);
            }
        }

        loop {
            let mut input = [0u8; 64];
            let read = self.connection.try_read(&mut input).await?;
            if read == 0 {
                return Ok(self.empty_ppl_event());
            }
            // Input read here never passes the keyboard timer in `get_char`, and a game
            // answering only events would look idle until the board hangs it up.
            self.session.keyboard_timer_started = Instant::now();
            for (index, byte) in input[..read].iter().enumerate() {
                if let Some(event) = self.process_ppl_event_byte(*byte) {
                    self.raw_input.extend(&input[index + 1..read]);
                    return Ok(event);
                }
            }
        }
    }

    pub async fn wait_ppl_event(&mut self, timeout: Option<Duration>) -> Res<ppl_events::PplEvent> {
        let event = self.poll_ppl_event().await?;
        if event.event_type != ppl_events::EVENT_NONE || timeout.is_some_and(|timeout| timeout.is_zero()) {
            return Ok(event);
        }

        let deadline = timeout.map(|timeout| Instant::now() + timeout);
        loop {
            let result = if let Some(deadline) = deadline {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Ok(self.empty_ppl_event());
                }
                match tokio::time::timeout(remaining, self.get_char(TerminalTarget::Both)).await {
                    Ok(result) => result,
                    Err(_) => return Ok(self.empty_ppl_event()),
                }
            } else {
                self.get_char(TerminalTarget::Both).await
            };
            if let Some(key) = result? {
                self.ppl_event_keys.feed(key);
            }
            if let Some(event) = self.take_pending_ppl_event() {
                return Ok(event);
            }
        }
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
                if let Some(key_char) = self.get_edit_sequence_char(ch.source).await? {
                    if key_char.ch != '[' {
                        self.char_buffer.push_front(key_char);
                        return Ok(Some(ch));
                    }
                    let Some(key_char) = self.get_edit_sequence_char(ch.source).await? else {
                        return Ok(Some(ch));
                    };
                    match key_char.ch {
                        'A' => ch.ch = control_codes::UP,
                        'B' => ch.ch = control_codes::DOWN,
                        'C' => ch.ch = control_codes::RIGHT,
                        'D' => ch.ch = control_codes::LEFT,

                        'H' => ch.ch = control_codes::HOME,
                        'K' | 'F' => ch.ch = control_codes::END,

                        'V' => ch.ch = control_codes::PG_UP,
                        'U' => ch.ch = control_codes::PG_DN,
                        '@' | '2' => {
                            self.get_edit_sequence_char(ch.source).await?;
                            ch.ch = control_codes::INS;
                        }

                        '6' => {
                            self.get_edit_sequence_char(ch.source).await?;
                            ch.ch = control_codes::PG_UP;
                        }
                        '5' => {
                            self.get_edit_sequence_char(ch.source).await?;
                            ch.ch = control_codes::PG_DN;
                        }
                        _ => {
                            // don't pass ctrl codes
                            return Ok(None);
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(Some(ch))
    }

    async fn get_edit_sequence_char(&mut self, source: KeySource) -> Res<Option<KeyChar>> {
        if source == KeySource::Sysop {
            self.get_char(TerminalTarget::Both).await
        } else {
            Ok(self.char_buffer.pop_front())
        }
    }

    /// Says goodbye and drops the line - the caller gets no say in it.
    async fn shutdown_for_event(&mut self, msg: String) -> Res<()> {
        self.new_line().await?;
        self.set_color(TerminalTarget::Both, IcbColor::dos_white()).await?;
        self.println(TerminalTarget::Both, &msg).await?;
        self.bell().await?;
        self.reset_color(TerminalTarget::Both).await?;
        self.hangup().await
    }

    async fn show_broadcast(&mut self, msg: String) -> Res<()> {
        let buf = self.user_screen.buffer.buffer.clone();
        let pos = self.user_screen.buffer.caret.position();
        self.set_activity(NodeStatus::NodeMessage).await;
        self.new_line().await?;
        self.set_color(TerminalTarget::Both, IcbColor::dos_white()).await?;
        self.println(TerminalTarget::Both, "Broadcast:").await?;
        self.println(TerminalTarget::Both, &msg).await?;
        self.bell().await?;

        self.press_enter().await?;

        let options = SaveOptions {
            format: FormatOptions::Character(CharacterFormatOptions {
                screen_prep: ScreenPreperation::ClearScreen,
                ..Default::default()
            }),
            ..Default::default()
        };
        let res = FileFormat::PCBoard.to_bytes(&buf, &options)?;
        let res = unsafe { String::from_utf8_unchecked(res) };
        self.print(TerminalTarget::Both, &res).await?;
        self.gotoxy(TerminalTarget::Both, pos.x, pos.y).await?;
        Ok(())
    }

    async fn print_sysop_screen(&mut self) -> Res<()> {
        let options = SaveOptions {
            format: FormatOptions::Character(CharacterFormatOptions {
                screen_prep: ScreenPreperation::ClearScreen,
                ..Default::default()
            }),
            ..Default::default()
        };
        let res = FileFormat::PCBoard.to_bytes(&self.user_screen.buffer.buffer, &options)?;
        let res = unsafe { String::from_utf8_unchecked(res) };
        self.print(TerminalTarget::Sysop, &res).await?;
        let p = self.user_screen.buffer.caret.position();
        self.gotoxy(TerminalTarget::Sysop, p.x + 1, p.y + 1).await?;
        Ok(())
    }

    pub fn inbytes(&mut self) -> i32 {
        self.char_buffer.len() as i32
    }

    /// Like 'inbytes' but does not count stuffed hidden characters
    pub fn kbdbufsize(&mut self) -> i32 {
        self.char_buffer.iter().filter(|c| !c.source.is_hidden()).count() as i32
    }

    pub fn cur_color(&self) -> IcbColor {
        let attr = self.user_screen.buffer.caret.attribute.as_u8(icy_engine::IceMode::Blink);
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
                if screen.buffer.caret.attribute.as_u8(icy_engine::IceMode::Blink) == color {
                    return Ok(());
                }

                TextAttribute::from_u8(color, icy_engine::IceMode::Blink)
            }
            IcbColor::IcyEngine(_fg) => {
                todo!();
            }
        };

        if self.session.disp_options.grapics_mode == GraphicsMode::Avatar
            && let IcbColor::Dos(color) = color
        {
            let color_change = format!("\x16\x01{}", color as char);
            return self.write_chars(target, color_change.chars().collect::<Vec<char>>().as_slice()).await;
        }

        let mut color_change = "\x1B[".to_string();
        let was_bold = screen.buffer.caret.attribute.is_bold();
        let new_bold = new_color.is_bold() || new_color.foreground() > 7;
        let mut bg = screen.buffer.caret.attribute.background();
        let mut fg = screen.buffer.caret.attribute.foreground();
        if was_bold != new_bold {
            if new_bold {
                color_change += "1;";
            } else {
                color_change += "0;";
                fg = 7;
                bg = 0;
            }
        }

        if !screen.buffer.caret.attribute.is_blinking() && new_color.is_blinking() {
            color_change += "5;";
        }

        if fg != new_color.foreground() {
            color_change += format!("{};", ANSI_COLOR_OFFSETS[new_color.foreground() as usize % 8] + 30).as_str();
        }

        if bg != new_color.background() {
            color_change += format!("{};", ANSI_COLOR_OFFSETS[new_color.background() as usize % 8] + 40).as_str();
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

    /// `PCBoard` counted a line here and nowhere else, so a PPE drawing its own screen with
    /// PRINT and cursor positioning never ran into a MORE prompt. See `newline()` in DISPLAY.C.
    pub async fn new_line(&mut self) -> Res<()> {
        self.write_chars(TerminalTarget::Both, &['\r', '\n']).await?;
        self.next_line().await
    }

    pub async fn fresh_line(&mut self) -> Res<()> {
        if self.user_screen.buffer.caret.x > 0 {
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

    /// Takes a password on behalf of PPL's `NEWPWD`, which `PCBoard` put through
    /// the same rules as the `W` command.
    pub async fn change_password(&mut self, new_pwd: &str) -> Res<bool> {
        let min_len = self.get_board().await.config.limits.min_pwd_length;
        let exp_days = self.get_board().await.config.limits.password_expire_days;
        let Some(user) = &self.session.current_user else {
            return Ok(false);
        };
        if user.password.check_new_password(user.get_name(), new_pwd, min_len) != PasswordVerdict::Ok {
            return Ok(false);
        }
        let password = self.create_password(new_pwd.to_string()).await;
        if let Some(user) = &mut self.session.current_user {
            user.password.accept_new_password(password, Utc::now(), exp_days);
            self.get_board().await.save_userbase()?;
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn get_filebase(&mut self, dir: &PathBuf, metadata_path: &PathBuf) -> Res<Arc<Mutex<FileBase>>> {
        if let Some(some) = self.file_bases.get(dir) {
            return Ok(some.clone());
        }
        match FileBase::open(dir, metadata_path) {
            Ok(new_base) => {
                let arc: Arc<Mutex<FileBase>> = Arc::new(Mutex::new(new_base));
                self.file_bases.insert(dir.clone(), arc.clone());
                Ok(arc)
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
        keyword: String::new(),
        display: String::new(),
        lighbar_display: String::new(),
        help: String::new(),
        auto_run: AutoRun::Disabled,
        autorun_time: 0,
        position: crate::icy_board::commands::Position::default(),
        actions: vec![CommandAction {
            command_type: cmd_type,
            parameter: String::new(),
            trigger: crate::icy_board::commands::ActionTrigger::default(),
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

#[cfg(test)]
mod node_status_tests {
    use super::*;

    /// The status column and the node record used to carry their own copy of this
    /// table, and reading bulletins was labelled as handling mail in one of them.
    #[test]
    fn every_state_has_its_own_line() {
        let states = [
            NodeStatus::NoCaller,
            NodeStatus::Available,
            NodeStatus::RunningDoor,
            NodeStatus::EnterMessage,
            NodeStatus::GroupChat,
            NodeStatus::HandlingMail,
            NodeStatus::LogoffPending,
            NodeStatus::NodeMessage,
            NodeStatus::RunningEvent,
            NodeStatus::LogIntoSystem,
            NodeStatus::PagingSysop,
            NodeStatus::ChatWithSysop,
            NodeStatus::RecycleBBS,
            NodeStatus::TakeSurvey,
            NodeStatus::Transfer,
            NodeStatus::Unavailable,
            NodeStatus::DropDOSDelayed,
            NodeStatus::DropDOSNow,
            NodeStatus::ReadBulletins,
        ];
        let mut seen = Vec::new();
        for state in states {
            let text = state.text();
            assert!(!seen.contains(&text), "two states share the line {text:?}");
            seen.push(text);
        }
    }

    #[test]
    fn reading_bulletins_is_not_handling_mail() {
        assert_ne!(NodeStatus::ReadBulletins.text(), NodeStatus::HandlingMail.text());
    }
}
