use std::{
    collections::{HashMap, VecDeque},
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Datelike, Local, Timelike};
use icy_engine::ansi::constants::COLOR_OFFSETS;
use icy_engine::TextAttribute;
use icy_engine::{ansi, Buffer, BufferParser, Caret};
use icy_net::Connection;
use icy_ppe::{datetime::IcbDate, executable::Executable, Res};

use crate::vm::{run, DiskIO, TerminalTarget};
pub mod functions;

use self::functions::display_flags;

use super::{
    bulletins::BullettinList,
    conferences::Conference,
    file_areas::FileAreaList,
    icb_config::IcbColor,
    icb_text::{IcbTextStyle, IceText},
    pcboard_data::Node,
    surveys::SurveyList,
    user_base::User,
    IcyBoard, IcyBoardSerializer,
};

#[derive(Clone, Copy, PartialEq)]
pub enum GraphicsMode {
    Off,
    Ansi,
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
    pub current_conference: Conference,
    pub current_file_areas: FileAreaList,
    pub login_date: DateTime<Local>,

    pub cur_user: i32,
    pub cur_security: u8,
    pub cur_groups: Vec<String>,

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

    pub node_num: i32,

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
}

impl Session {
    pub fn new() -> Self {
        Self {
            disp_options: DisplayOptions::default(),
            current_conference_number: 0,
            current_conference: Conference::default(),
            current_file_areas: FileAreaList::default(),
            login_date: Local::now(),
            cur_user: -1,
            cur_security: 0,
            cur_groups: Vec::new(),
            num_lines_printed: 0,
            security_violations: 0,
            current_message_area: 0,
            last_new_line_y: 0,
            node_num: 0,
            page_len: 24,
            is_sysop: false,
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
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

pub struct IcyBoardState {
    pub connection: Box<dyn Connection>,
    pub board: Arc<Mutex<IcyBoard>>,

    pub nodes: Vec<Node>,

    pub transfer_statistics: TransferStatistics,

    pub yes_char: char,
    pub no_char: char,

    pub current_user: Option<User>,

    pub session: Session,

    /// 0 = no debug, 1 - errors, 2 - errors and warnings, 3 - all
    pub debug_level: i32,
    pub env_vars: HashMap<String, String>,

    parser: ansi::Parser,
    caret: Caret,
    buffer: Buffer,

    char_buffer: VecDeque<char>,
}

impl IcyBoardState {
    pub fn new(board: Arc<Mutex<IcyBoard>>, connection: Box<dyn Connection>) -> Self {
        let buffer = Buffer::new((80, 25));
        let caret = Caret::default();
        let mut parser = ansi::Parser::default();
        parser.bs_is_ctrl_char = true;

        Self {
            board,
            connection,
            nodes: Vec::new(),
            current_user: None,
            yes_char: 'Y',
            no_char: 'N',
            debug_level: 0,
            env_vars: HashMap::new(),
            session: Session::new(),
            transfer_statistics: TransferStatistics::default(),
            parser,
            buffer,
            caret,
            char_buffer: VecDeque::new(),
        }
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
        self.session.disp_options.grapics_mode != GraphicsMode::Off
    }

    fn check_time_left(&self) {
        // TODO: Check time left.
    }

    pub fn reset_color(&mut self) -> Res<()> {
        let color = self.board.lock().unwrap().config.color_configuration.default.clone();
        self.set_color(color)
    }

    pub fn clear_screen(&mut self) -> Res<()> {
        self.session.num_lines_printed = 0;
        if self.use_ansi() {
            self.print(TerminalTarget::Both, "\x1B[2J\x1B[H")
        } else {
            // form feed character
            self.print(TerminalTarget::Both, "\x0C")
        }
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
        if self.use_ansi() {
            self.print(TerminalTarget::Both, "\x1B[K")
        } else {
            Ok(())
        }
    }

    pub fn set_current_user(&mut self, user: i32) {
        self.session.cur_user = user;
        if user >= 0 {
            let last_conference = if let Ok(board) = self.board.lock() {
                self.current_user = Some(board.users[user as usize].clone());
                self.session.cur_security = board.users[user as usize].security_level;
                self.session.page_len = board.users[user as usize].page_len;
                board.users[user as usize].last_conference as i32
            } else {
                return;
            };
            self.join_conference(last_conference);
        }
    }

    pub fn join_conference(&mut self, conference: i32) {
        if conference >= 0 && conference < self.board.lock().unwrap().conferences.len() as i32 {
            self.session.current_conference_number = conference;
            if let Ok(board) = self.board.lock() {
                self.session.current_conference = board.conferences[conference as usize].clone();
            }

            let file_area_file = self.resolve_path(&self.session.current_conference.file_area_file);
            self.session.current_file_areas = FileAreaList::load(&file_area_file).unwrap();
            // todo unwrap
        }
    }

    fn next_line(&mut self) -> Res<bool> {
        if self.session.disp_options.non_stop {
            return Ok(true);
        }
        let cur_y = self.caret.get_position().y;
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

    pub fn put_keyboard_buffer(&mut self, value: &str) -> Res<()> {
        let in_chars: Vec<char> = value.chars().collect();

        for (i, c) in in_chars.iter().enumerate() {
            if *c == '^' && i + 1 < in_chars.len() && in_chars[i + 1] >= 'A' && in_chars[i + 1] <= '[' {
                let c = in_chars[i + 1] as u8 - b'@';
                self.char_buffer.push_back(c as char);
            } else {
                self.char_buffer.push_back(*c);
            }
        }
        self.char_buffer.push_back('\n');
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
            let path = board.resolve_file(&board.config.paths.tmp_path);

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
        PathBuf::from(self.board.lock().unwrap().resolve_file(&file))
    }
}

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
        if self.use_ansi() {
            self.write_raw(target, format!("\x1B[{};{}H", y, x).chars().collect::<Vec<char>>().as_slice())
        } else {
            Ok(())
        }
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

    /// # Errors
    pub fn print(&mut self, target: TerminalTarget, str: &str) -> Res<()> {
        self.write_raw(target, str.chars().collect::<Vec<char>>().as_slice())
    }

    fn no_terminal(&self, terminal_target: TerminalTarget) -> bool {
        match terminal_target {
            TerminalTarget::Sysop => !self.is_sysop(),
            _ => false,
        }
    }

    fn write_char(&mut self, c: char) -> Res<()> {
        self.parser.print_char(&mut self.buffer, 0, &mut self.caret, c)?;
        self.connection.write_all(&[c as u8])?;
        if c == '\n' {
            self.next_line()?;
        }
        Ok(())
    }

    fn write_string(&mut self, data: &[char]) -> Res<()> {
        let mut b = Vec::new();
        for c in data {
            self.parser.print_char(&mut self.buffer, 0, &mut self.caret, *c)?;
            if *c == '\n' {
                self.next_line()?;
            }
            b.push(*c as u8);
        }
        self.connection.write_all(&b)?;
        Ok(())
    }

    /// # Errors
    pub fn write_raw(&mut self, target: TerminalTarget, data: &[char]) -> Res<()> {
        if self.no_terminal(target) {
            return Ok(());
        }
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
                            self.write_char(*c)?;
                        }
                    }
                    PcbState::GotAt => {
                        if *c == 'X' || *c == 'x' {
                            state = PcbState::ReadColor1;
                        } else {
                            state = PcbState::ReadAtSequence(c.to_string());
                        }
                    }
                    PcbState::ReadAtSequence(s) => {
                        if *c == '@' {
                            state = PcbState::Default;
                            if let Some(s) = self.translate_variable(&s) {
                                self.write_string(s.chars().collect::<Vec<char>>().as_slice())?;
                            }
                        } else {
                            state = PcbState::ReadAtSequence(s + &c.to_string());
                        }
                    }
                    PcbState::ReadColor1 => {
                        if c.is_ascii_hexdigit() {
                            state = PcbState::ReadColor2(*c);
                        } else {
                            self.write_char('@')?;
                            self.write_char(*c)?;
                            state = PcbState::Default;
                        }
                    }
                    PcbState::ReadColor2(ch1) => {
                        state = PcbState::Default;
                        if !c.is_ascii_hexdigit() {
                            self.write_char('@')?;
                            self.write_char(ch1)?;
                            self.write_char(*c)?;
                        } else {
                            let color = (c.to_digit(16).unwrap() | (ch1.to_digit(16).unwrap() << 4)) as u8;
                            self.set_color(color.into())?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn translate_variable(&mut self, input: &str) -> Option<String> {
        let mut split = input.split(':');
        let id = split.next().unwrap();
        let param = split.next();
        let mut result = String::new();
        match id {
            "ALIAS" => {
                if let Some(user) = &self.current_user {
                    result = user.alias.to_string();
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
            "BPS" => result = self.get_bps().to_string(),

            // TODO
            "BYTECREDIT" | "BYTELIMIT" | "BYTERATIO" | "BYTESLEFT" | "CARRIER" => {
                // todo
            }

            "CITY" => {
                if let Some(user) = &self.current_user {
                    result = user.city.to_string();
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
            "DAYBYTES" | "DELAY" | "DIRNAME" | "DIRNUM" | "DLBYTES" | "DLFILES" | "EVENT" => {}
            "EXPDATE" => {
                if let Some(user) = &self.current_user {
                    result = user.exp_date.to_string();
                } else {
                    result = IcbDate::default().to_country_date();
                }
            }
            "EXPDAYS" => {
                if self.board.lock().unwrap().config.subscription_info.is_enabled {
                    if let Some(user) = &self.current_user {
                        if user.exp_date.get_year() != 0 {
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
                    let entry = self.board.lock().unwrap().display_text.get_display_text(IceText::Unlimited).unwrap();
                    if entry.style != IcbTextStyle::Plain {
                        let _ = self.set_color(entry.style.to_color());
                    }
                    result = entry.text;
                }
            }
            "FBYTES" | "FFILES" | "FILECREDIT" | "FILERATIO" => {}
            "FIRST" => {
                if let Some(user) = &self.current_user {
                    result = user.get_first_name();
                }
            }
            "FIRSTU" => {
                if let Some(user) = &self.current_user {
                    result = user.get_first_name().to_uppercase();
                }
            }
            "FNUM" => {}
            "FREESPACE" => {}
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
            "NOCHAR" => result = self.no_char.to_string(),
            "NODE" => result = self.session.node_num.to_string(),
            "NUMBLT" => {
                if let Ok(bullettins) = self.load_bullettins() {
                    result = bullettins.len().to_string();
                }
            }
            "NUMCALLS" => {
                result = self.board.lock().unwrap().num_callers.to_string();
            }
            "NUMCONF" => result = self.board.lock().unwrap().conferences.len().to_string(),
            "NUMDIR" => {
                result = self.session.current_file_areas.len().to_string();
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
                let x = self.caret.get_position().x as usize;
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
            "SECURITY" => {}
            "SCPS" => result = self.transfer_statistics.get_cps_download().to_string(),
            "SBYTES" => result = self.transfer_statistics.downloaded_bytes.to_string(),
            "SFILES" => result = self.transfer_statistics.downloaded_files.to_string(),
            "SYSDATE" => {
                let now = Local::now();
                let d = now.date_naive();
                result = format!("{:02}/{:02}/{:02}", d.month0() + 1, d.day(), d.year_ce().1 % 100);
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
                            result = user.get_name().to_string();
                        } else {
                            result = user.alias.to_string();
                        }
                    } else {
                        result = user.get_name().to_string();
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
                self.session.disp_options.disable_color = false;
                return None;
            }
            "YESCHAR" => result = self.yes_char.to_string(),
            _ => {
                if id.to_ascii_uppercase().starts_with("ENV=") {
                    let key = &id[4..];
                    if let Some(value) = self.get_env(key) {
                        result = value.to_string();
                    }
                }
            }
        }

        if let Some(param) = param {
            if let Ok(i) = param.parse::<usize>() {
                while result.chars().count() < i {
                    result.push(' ');
                }
            }
        }

        Some(result)
    }

    /// # Errors
    pub fn get_char(&mut self) -> Res<Option<(bool, char)>> {
        if let Some(ch) = self.char_buffer.pop_front() {
            return Ok(Some((true, ch)));
        }
        let mut a = [0; 1];
        let size = self.connection.read(&mut a)?;
        if size == 1 {
            Ok(Some((true, a[0] as char)))
        } else {
            Ok(None)
        }
    }

    pub fn inbytes(&mut self) -> i32 {
        self.char_buffer.len() as i32
    }

    pub fn set_color(&mut self, color: IcbColor) -> Res<()> {
        if !self.use_graphics() {
            return Ok(());
        }
        let new_color = match color {
            IcbColor::None => {
                return Ok(());
            }
            IcbColor::Dos(color) => {
                if self.caret.get_attribute().as_u8(icy_engine::IceMode::Blink) == color {
                    return Ok(());
                }
                TextAttribute::from_u8(color, icy_engine::IceMode::Blink)
            }
            IcbColor::IcyEngine(_fg) => {
                todo!();
            }
        };

        let mut color_change = "\x1B[".to_string();
        let was_bold = self.caret.get_attribute().is_bold();
        let new_bold = new_color.is_bold() || new_color.get_foreground() > 7;
        let mut bg = self.caret.get_attribute().get_background();
        let mut fg = self.caret.get_attribute().get_foreground();
        if was_bold != new_bold {
            if new_bold {
                color_change += "1;";
            } else {
                color_change += "0;";
                fg = 7;
                bg = 0;
            }
        }

        if !self.caret.get_attribute().is_blinking() && new_color.is_blinking() {
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
        self.write_string(color_change.chars().collect::<Vec<char>>().as_slice())
    }

    pub fn get_caret_position(&mut self) -> (i32, i32) {
        (self.caret.get_position().x, self.caret.get_position().y)
    }

    /// # Errors
    pub fn hangup(&mut self) -> Res<()> {
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
        self.session.request_logoff = true;
        self.connection.shutdown()?;
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
            "YyNn",
            "HLPMORE",
            display_flags::YESNO | display_flags::UPCASE | display_flags::STACKED | display_flags::ERASELINE,
        )?;
        Ok(result != "N")
    }

    pub fn press_enter(&mut self) -> Res<()> {
        self.session.disable_auto_more = true;
        self.session.more_requested = false;
        self.input_field(IceText::PressEnter, 0, "", "", display_flags::ERASELINE)?;
        Ok(())
    }

    pub fn new_line(&mut self) -> Res<()> {
        self.write_raw(TerminalTarget::Both, &['\r', '\n'])
    }
}
