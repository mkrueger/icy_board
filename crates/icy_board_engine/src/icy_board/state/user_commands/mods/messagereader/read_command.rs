//! Port of `PCBoard`'s read command parser.
//!
//! R, Q and REPLY all funnel their input through this one parser, so the set of
//! words it accepts and the order in which it consumes them is part of the
//! contract a PPE relies on when it stuffs a read command.

pub const MAX_GROUPS: usize = 20;
pub const LAST_MESSAGE: i64 = 0x7FFF_FFFF;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MsgFunc {
    #[default]
    None,
    Stop,
    Redisplay,
    EditHeader,
    EnterMessage,
    Goodbye,
    Join,
    Kill,
    Protect,
    Unprotect,
    QuickScan,
    ViewFile,
    Export,
    FindTo,
    FindFrom,
    Chat,
    Copy,
    Move,
    Forward,
    DeselectConference,
    SelectConference,
    EditMessage,
    FlagFile,
    JumpOut,
    Reply,
    ReplyOther,
    Skip,
    Who,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadLoop {
    Inside,
    Outside,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeaderLength {
    Short,
    Long,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MsgRange {
    pub first: i64,
    pub last: i64,
}

impl MsgRange {
    fn new(first: i64, last: i64) -> Self {
        Self { first, last }
    }
}

pub mod user_search {
    pub const NONE: u8 = 0;
    pub const TO: u8 = 1;
    pub const FROM: u8 = 2;
    pub const USER: u8 = 4;
}

/// Everything the parser needs to know about the session, so that parsing
/// itself stays free of I/O and can be tested directly.
#[derive(Clone, Debug, Default)]
pub struct ParseContext {
    pub cur_msg_number: i64,
    pub memorized: Option<i64>,
    pub reply_to: i64,
    pub may_join: bool,
    pub may_quick_scan: bool,
    pub may_read_only: bool,
    pub alias_support: bool,
    pub qwk_support: bool,
    pub reply_command: bool,
    pub quick_scan: bool,
    /// How many conferences the board has; MOVE and COPY only accept a number below it.
    pub num_conferences: u16,
}

/// Mirrors `PCBoard`'s `readtype`: the accumulated result of one command line.
#[derive(Clone, Debug)]
pub struct ReadCommand {
    pub func: MsgFunc,
    pub numbers: Vec<MsgRange>,
    pub all_conf: bool,
    pub mail_wait_conf: bool,
    pub since: bool,
    pub new_msgs: bool,
    pub new_date: Option<String>,
    pub from_msgs: bool,
    pub your_msgs: bool,
    pub msgs_to_all: bool,
    pub full_range: bool,
    pub keep_going: bool,
    pub threading: bool,
    pub thread_forward: bool,
    pub any_msgs: bool,
    pub unread_only: bool,
    pub stay_in_conf: bool,
    pub check_user_scan: bool,
    pub update_pointers: bool,
    pub update_msg_status: bool,
    pub quick_scan: bool,
    pub net: bool,
    pub search_text: String,
    pub user_name_to: String,
    pub user_name_from: String,
    pub user_search: u8,
    pub do_text_search: bool,
    pub do_user_search: bool,
    pub header_len: Option<HeaderLength>,
    pub move_conf: Option<u16>,
    pub open_capture: bool,
    pub open_qwk: bool,
    pub capture_single: bool,
    pub cap_ask: bool,
    pub cap_bye: bool,
    pub zip_cap: bool,
    pub valid_cmd: bool,
    /// `A`/`ALL` asks whether to resume where the last ALL scan stopped.
    pub ask_resume_all: bool,
    pub show_help: bool,
    pub memorize: bool,
    pub toggle_alias: Option<AliasToggle>,
    pub set_last_read: bool,
    pub new_last_read: Option<i64>,
    pub not_memorized: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AliasToggle {
    On,
    Off,
    Flip,
}

impl Default for ReadCommand {
    fn default() -> Self {
        Self {
            func: MsgFunc::None,
            numbers: Vec::new(),
            all_conf: false,
            mail_wait_conf: false,
            since: false,
            new_msgs: false,
            new_date: None,
            from_msgs: false,
            your_msgs: false,
            msgs_to_all: false,
            full_range: false,
            keep_going: false,
            threading: false,
            thread_forward: true,
            any_msgs: true,
            unread_only: false,
            stay_in_conf: false,
            check_user_scan: true,
            update_pointers: true,
            update_msg_status: true,
            quick_scan: false,
            net: false,
            search_text: String::new(),
            user_name_to: String::new(),
            user_name_from: String::new(),
            user_search: user_search::NONE,
            do_text_search: false,
            do_user_search: false,
            header_len: None,
            move_conf: None,
            open_capture: false,
            open_qwk: false,
            capture_single: false,
            cap_ask: true,
            cap_bye: false,
            zip_cap: false,
            valid_cmd: false,
            ask_resume_all: false,
            show_help: false,
            memorize: false,
            toggle_alias: None,
            set_last_read: false,
            new_last_read: None,
            not_memorized: false,
        }
    }
}

const OPTIONS: [&str; 42] = [
    "ALL", "ALIAS", "BYE", "CHAT", "COPY", "DESELECT", "EDIT", "FF", "FT", "FLAG", "FORWARD", "FROM", "GB", "HELP", "JUMP", "KILL", "LONG", "MOVE", "NEXT",
    "NET", "PREV", "QWK", "REPLY", "RM", "RM+", "RM-", "RO", "RR", "RR+", "RR-", "SELECT", "SET", "SHORT", "SKIP", "T+", "T-", "TO", "TS", "USER", "WAIT",
    "WHO", "YA",
];

const O_ALL: usize = 0;
const O_ALIAS: usize = 1;
const O_BYE: usize = 2;
const O_CHAT: usize = 3;
const O_COPY: usize = 4;
const O_DESEL: usize = 5;
const O_EDIT: usize = 6;
const O_FINDF: usize = 7;
const O_FINDT: usize = 8;
const O_FLAG: usize = 9;
const O_FORW: usize = 10;
const O_FROM: usize = 11;
const O_GB: usize = 12;
const O_HELP: usize = 13;
const O_JUMP: usize = 14;
const O_KILL: usize = 15;
const O_LONG: usize = 16;
const O_MOVE: usize = 17;
const O_NEXT: usize = 18;
const O_NET: usize = 19;
const O_PREV: usize = 20;
const O_QWK: usize = 21;
const O_REPLY: usize = 22;
const O_RM: usize = 23;
const O_RMF: usize = 24;
const O_RMB: usize = 25;
const O_RO: usize = 26;
const O_RR: usize = 27;
const O_RRF: usize = 28;
const O_RRB: usize = 29;
const O_SEL: usize = 30;
const O_SET: usize = 31;
const O_SHORT: usize = 32;
const O_SKIP: usize = 33;
const O_TF: usize = 34;
const O_TB: usize = 35;
const O_TO: usize = 36;
const O_TS: usize = 37;
const O_USER: usize = 38;
const O_WAIT: usize = 39;
const O_WHO: usize = 40;
const O_YA: usize = 41;

/// Keyword lookup: first prefix match wins, and two characters are
/// enough for any word of three or more.
fn option(input: &str) -> Option<usize> {
    if input.is_empty() {
        return None;
    }
    OPTIONS
        .iter()
        .position(|word| word.starts_with(input) && (input.len() + 1 >= word.len() || (input.len() >= 2 && word.len() >= 3)))
}

/// Digits, with `+` or `-` allowed once a digit
/// has been seen, so `123`, `123+` and `100-50` all qualify but `+5` does not.
fn is_number(s: &str) -> bool {
    let mut seen_digit = false;
    for ch in s.chars() {
        if ch.is_ascii_digit() {
            seen_digit = true;
        } else if !(seen_digit && (ch == '+' || ch == '-')) {
            return false;
        }
    }
    true
}

/// Roll the tokenizer's split words back into one string.
fn add_text(dest: &mut String, text: &str, max_chars: usize) {
    if dest.len() + text.len() + usize::from(!dest.is_empty()) > max_chars {
        return;
    }
    if !dest.is_empty() {
        dest.push(' ');
    }
    dest.push_str(text);
}

impl ReadCommand {
    fn push_range(&mut self, range: MsgRange) {
        if self.numbers.len() < MAX_GROUPS {
            self.numbers.push(range);
        }
    }

    fn push_memorized(&mut self, memorized: Option<i64>, last: impl Fn(i64) -> i64) {
        match memorized {
            Some(num) => self.push_range(MsgRange::new(num, last(num))),
            None => self.not_memorized = true,
        }
        self.func = MsgFunc::None;
        self.valid_cmd = true;
    }
}

/// Consume one command line. `flag` says whether we are already displaying a
/// message, which changes what several single letters mean.
pub fn parse(tokens: &[String], flag: ReadLoop, ctx: &ParseContext) -> ReadCommand {
    let mut cmd = ReadCommand {
        quick_scan: ctx.quick_scan,
        ..Default::default()
    };
    let mut flag = flag;
    let mut last_search_cmd: Option<usize> = None;
    let mut iter = tokens.iter().enumerate().peekable();

    while let Some((index, token)) = iter.next() {
        let remaining = tokens.len() - index;
        let token = token.as_str();

        if is_number(token) {
            if cmd.new_msgs && token.len() == 6 && token.chars().all(|c| c.is_ascii_digit()) {
                cmd.new_date = Some(token.to_string());
                continue;
            }
            let base: i64 = token[..token.find(|c: char| !c.is_ascii_digit()).unwrap_or(token.len())].parse().unwrap_or(0);
            let mut range = MsgRange::new(base, base);
            if token.contains('+') || (ctx.quick_scan && !token.contains('-')) {
                range.last = LAST_MESSAGE;
                cmd.keep_going = true;
            } else if let Some(rest) = token.split_once('-').map(|(_, rest)| rest) {
                range.last = if rest.is_empty() { 1 } else { rest.parse().unwrap_or(1) };
            }
            cmd.func = MsgFunc::None;
            cmd.push_range(range);
            cmd.valid_cmd = true;
            continue;
        }

        if ctx.reply_command {
            continue;
        }

        if token.len() == 1 {
            let ch = token.chars().next().unwrap();
            match ch {
                'L' | '-' => {
                    if ch == 'L' {
                        flag = ReadLoop::Outside;
                    }
                    let first = if flag == ReadLoop::Inside { ctx.cur_msg_number - 1 } else { LAST_MESSAGE };
                    cmd.push_range(MsgRange::new(first, 1));
                    cmd.func = MsgFunc::None;
                    cmd.valid_cmd = true;
                }
                '+' => {
                    if flag == ReadLoop::Inside {
                        cmd.push_range(MsgRange::new(ctx.cur_msg_number + 1, LAST_MESSAGE));
                        cmd.func = MsgFunc::None;
                    } else {
                        cmd.since = true;
                    }
                    cmd.keep_going = true;
                    cmd.valid_cmd = true;
                }
                'A' => {
                    if !ctx.may_join {
                        continue;
                    }
                    cmd.valid_cmd = true;
                    cmd.all_conf = true;
                    cmd.ask_resume_all = true;
                    if flag == ReadLoop::Inside {
                        cmd.stay_in_conf = true;
                    }
                }
                'Z' | 'D' | 'C' => {
                    if ch == 'Z' {
                        cmd.zip_cap = true;
                    }
                    if ch == 'Z' || ch == 'D' {
                        cmd.cap_ask = false;
                    }
                    cmd.open_capture = true;
                    cmd.valid_cmd = true;
                    if !cmd.numbers.is_empty() {
                        continue;
                    }
                    if flag == ReadLoop::Inside {
                        cmd.capture_single = true;
                        cmd.func = MsgFunc::Redisplay;
                        continue;
                    }
                    // else fall through to a read-since with the capture
                    cmd.since = true;
                    cmd.keep_going = true;
                }
                '*' | 'S' => {
                    cmd.since = true;
                    cmd.keep_going = true;
                    cmd.valid_cmd = true;
                }
                'E' => {
                    cmd.func = if flag == ReadLoop::Inside {
                        MsgFunc::EditHeader
                    } else {
                        MsgFunc::EnterMessage
                    };
                    cmd.valid_cmd = true;
                }
                'F' => {
                    if flag == ReadLoop::Inside {
                        cmd.func = MsgFunc::FindFrom;
                    } else {
                        cmd.any_msgs = false;
                        cmd.from_msgs = true;
                    }
                    cmd.valid_cmd = true;
                }
                'G' => {
                    if cmd.func == MsgFunc::None {
                        cmd.func = MsgFunc::Goodbye;
                        cmd.valid_cmd = true;
                    }
                }
                'H' => {
                    cmd.show_help = true;
                    cmd.valid_cmd = true;
                }
                'J' => {
                    cmd.func = MsgFunc::Join;
                    cmd.valid_cmd = true;
                    break;
                }
                'K' => {
                    cmd.func = MsgFunc::Kill;
                    cmd.valid_cmd = true;
                }
                'M' => {
                    cmd.memorize = true;
                    cmd.valid_cmd = true;
                }
                'N' => {
                    if flag == ReadLoop::Inside {
                        cmd.func = MsgFunc::Stop;
                    } else {
                        cmd.new_msgs = true;
                        cmd.since = false;
                    }
                    cmd.valid_cmd = true;
                }
                'O' => {
                    cmd.update_pointers = false;
                    cmd.valid_cmd = true;
                    if ctx.may_read_only {
                        cmd.update_msg_status = false;
                    }
                }
                'P' => {
                    cmd.func = MsgFunc::Protect;
                    cmd.valid_cmd = true;
                }
                'Q' => {
                    if ctx.may_quick_scan {
                        cmd.quick_scan = true;
                        cmd.valid_cmd = true;
                        if flag == ReadLoop::Inside {
                            cmd.func = MsgFunc::QuickScan;
                        }
                    }
                }
                'T' => {
                    if flag == ReadLoop::Inside {
                        cmd.threading = true;
                        cmd.thread_forward = true;
                        cmd.valid_cmd = true;
                    }
                }
                'U' => {
                    if flag == ReadLoop::Inside {
                        cmd.func = MsgFunc::Unprotect;
                    } else {
                        cmd.unread_only = true;
                    }
                    cmd.valid_cmd = true;
                }
                'V' => {
                    cmd.func = MsgFunc::ViewFile;
                    cmd.valid_cmd = true;
                }
                'X' => {
                    if flag == ReadLoop::Inside {
                        cmd.func = MsgFunc::Export;
                        cmd.valid_cmd = true;
                    }
                }
                'Y' => {
                    cmd.any_msgs = false;
                    cmd.your_msgs = true;
                    cmd.valid_cmd = true;
                }
                '/' => {
                    if flag == ReadLoop::Inside {
                        cmd.func = MsgFunc::Redisplay;
                        cmd.valid_cmd = true;
                    }
                }
                _ => append_search_text(&mut cmd, last_search_cmd, token),
            }
            continue;
        }

        let Some(opt) = option(token) else {
            append_search_text(&mut cmd, last_search_cmd, token);
            continue;
        };

        match opt {
            O_ALL => {
                cmd.check_user_scan = false;
                cmd.valid_cmd = true;
                if !ctx.may_join {
                    continue;
                }
                cmd.all_conf = true;
                cmd.ask_resume_all = true;
                if flag == ReadLoop::Inside {
                    cmd.stay_in_conf = true;
                }
            }
            O_ALIAS => {
                if ctx.alias_support {
                    cmd.toggle_alias = Some(if remaining > 1 {
                        match iter.next().map(|(_, t)| t.as_str()) {
                            Some("ON") => AliasToggle::On,
                            Some("OFF") => AliasToggle::Off,
                            _ => AliasToggle::Flip,
                        }
                    } else {
                        AliasToggle::Flip
                    });
                }
                if flag == ReadLoop::Inside {
                    cmd.func = MsgFunc::Redisplay;
                }
                cmd.valid_cmd = true;
            }
            O_BYE | O_GB => {
                cmd.cap_ask = false;
                cmd.cap_bye = true;
                cmd.valid_cmd = true;
            }
            O_HELP => {
                cmd.show_help = true;
                cmd.valid_cmd = true;
                if flag == ReadLoop::Inside {
                    cmd.func = MsgFunc::Redisplay;
                }
            }
            O_CHAT => {
                cmd.func = MsgFunc::Chat;
                cmd.valid_cmd = true;
            }
            O_COPY | O_MOVE | O_FORW => {
                cmd.func = match opt {
                    O_COPY => MsgFunc::Copy,
                    O_MOVE => MsgFunc::Move,
                    _ => MsgFunc::Forward,
                };
                cmd.valid_cmd = true;
                cmd.move_conf = None;
                if remaining > 1
                    && let Some((_, next)) = iter.next()
                    && let Ok(num) = next.parse::<u16>()
                    && num < ctx.num_conferences
                {
                    cmd.move_conf = Some(num);
                }
            }
            O_DESEL => {
                cmd.func = MsgFunc::DeselectConference;
                cmd.valid_cmd = true;
            }
            O_EDIT => {
                cmd.func = MsgFunc::EditMessage;
                cmd.valid_cmd = true;
            }
            O_FINDF => {
                cmd.func = MsgFunc::FindFrom;
                cmd.valid_cmd = true;
            }
            O_FINDT => {
                cmd.func = MsgFunc::FindTo;
                cmd.valid_cmd = true;
            }
            O_FLAG => {
                cmd.func = MsgFunc::FlagFile;
                cmd.valid_cmd = true;
            }
            O_FROM => {
                cmd.do_user_search = true;
                last_search_cmd = Some(O_FROM);
                cmd.user_search |= user_search::FROM;
                cmd.valid_cmd = true;
            }
            O_JUMP => {
                cmd.func = MsgFunc::JumpOut;
                cmd.valid_cmd = true;
            }
            O_KILL => {
                cmd.func = MsgFunc::Kill;
                cmd.valid_cmd = true;
            }
            O_LONG => {
                cmd.header_len = Some(HeaderLength::Long);
                if flag == ReadLoop::Inside {
                    cmd.func = MsgFunc::Redisplay;
                }
                cmd.valid_cmd = true;
            }
            O_SHORT => {
                cmd.header_len = Some(HeaderLength::Short);
                if flag == ReadLoop::Inside {
                    cmd.func = MsgFunc::Redisplay;
                }
                cmd.valid_cmd = true;
            }
            O_PREV => {
                let first = if flag == ReadLoop::Inside { ctx.cur_msg_number - 1 } else { LAST_MESSAGE };
                cmd.push_range(MsgRange::new(first, 1));
                cmd.func = MsgFunc::None;
                cmd.valid_cmd = true;
            }
            O_NEXT => {
                let first = if flag == ReadLoop::Inside { ctx.cur_msg_number + 1 } else { 1 };
                cmd.push_range(MsgRange::new(first, LAST_MESSAGE));
                cmd.keep_going = true;
                cmd.func = MsgFunc::None;
                cmd.valid_cmd = true;
            }
            O_NET => {
                if ctx.qwk_support {
                    cmd.net = true;
                }
                cmd.valid_cmd = true;
            }
            O_QWK => {
                cmd.open_qwk = true;
                cmd.zip_cap = true;
                cmd.cap_ask = false;
                if !cmd.numbers.is_empty() || cmd.new_msgs {
                    continue;
                }
                if flag == ReadLoop::Inside {
                    cmd.capture_single = true;
                    cmd.func = MsgFunc::Redisplay;
                    continue;
                }
                cmd.since = true;
                cmd.keep_going = true;
                cmd.valid_cmd = true;
            }
            O_REPLY => {
                cmd.func = MsgFunc::Reply;
                cmd.valid_cmd = true;
            }
            O_RO => {
                cmd.func = MsgFunc::ReplyOther;
                cmd.valid_cmd = true;
            }
            O_RM => cmd.push_memorized(ctx.memorized, |num| num),
            O_RMF => {
                if ctx.memorized.is_some() {
                    cmd.keep_going = true;
                }
                cmd.push_memorized(ctx.memorized, |_| LAST_MESSAGE);
            }
            O_RMB => cmd.push_memorized(ctx.memorized, |_| 1),
            O_RR | O_RRF | O_RRB => {
                cmd.valid_cmd = true;
                if ctx.reply_to == 0 {
                    cmd.func = MsgFunc::Redisplay;
                } else {
                    let last = match opt {
                        O_RRF => LAST_MESSAGE,
                        O_RRB => 1,
                        _ => ctx.reply_to,
                    };
                    if opt == O_RRF {
                        cmd.keep_going = true;
                    }
                    cmd.push_range(MsgRange::new(ctx.reply_to, last));
                    cmd.func = MsgFunc::None;
                }
            }
            O_SEL => {
                cmd.func = MsgFunc::SelectConference;
                cmd.valid_cmd = true;
            }
            O_SET => {
                cmd.func = MsgFunc::None;
                cmd.valid_cmd = true;
                cmd.set_last_read = true;
                // The next token is only swallowed when it is the number itself.
                if iter
                    .peek()
                    .is_some_and(|(_, next)| !next.is_empty() && next.chars().all(|c| c.is_ascii_digit()))
                {
                    let (_, next) = iter.next().unwrap();
                    cmd.new_last_read = next.parse::<i64>().ok();
                }
            }
            O_SKIP => {
                cmd.func = MsgFunc::Skip;
                cmd.valid_cmd = true;
            }
            O_TF | O_TB => {
                if flag == ReadLoop::Inside {
                    cmd.threading = true;
                    cmd.thread_forward = opt == O_TF;
                    cmd.valid_cmd = true;
                }
            }
            O_TS => {
                cmd.do_text_search = true;
                last_search_cmd = Some(O_TS);
                cmd.valid_cmd = true;
                cmd.search_text.clear();
            }
            O_TO => {
                cmd.do_user_search = true;
                last_search_cmd = Some(O_TO);
                cmd.user_search |= user_search::TO;
                cmd.valid_cmd = true;
            }
            O_USER => {
                cmd.do_user_search = true;
                last_search_cmd = Some(O_USER);
                cmd.user_search = user_search::USER;
                cmd.valid_cmd = true;
            }
            O_WAIT => {
                cmd.check_user_scan = false;
                cmd.mail_wait_conf = true;
                cmd.all_conf = true;
                cmd.valid_cmd = true;
            }
            O_WHO => {
                cmd.func = MsgFunc::Who;
                cmd.valid_cmd = true;
            }
            O_YA => {
                cmd.any_msgs = false;
                cmd.your_msgs = true;
                cmd.msgs_to_all = true;
                cmd.valid_cmd = true;
            }
            _ => append_search_text(&mut cmd, last_search_cmd, token),
        }
    }

    if cmd.threading {
        if cmd.thread_forward {
            let first = if flag == ReadLoop::Inside { ctx.cur_msg_number + 1 } else { 1 };
            cmd.push_range(MsgRange::new(first, LAST_MESSAGE));
            cmd.keep_going = true;
        } else {
            let first = if flag == ReadLoop::Inside { ctx.cur_msg_number - 1 } else { LAST_MESSAGE };
            cmd.push_range(MsgRange::new(first, 1));
        }
        cmd.func = MsgFunc::None;
    }

    cmd
}

fn append_search_text(cmd: &mut ReadCommand, last_search_cmd: Option<usize>, token: &str) {
    match last_search_cmd {
        Some(O_FROM) => add_text(&mut cmd.user_name_from, token, 25),
        Some(O_TO | O_USER) => add_text(&mut cmd.user_name_to, token, 25),
        _ => add_text(&mut cmd.search_text, token, 40),
    }
}

/// Fold in the defaults `PCBoard` applies once the whole line has
/// been read and any missing search terms have been prompted for.
pub fn finalize(cmd: &mut ReadCommand) {
    if !cmd.numbers.is_empty() {
        if cmd.all_conf {
            cmd.numbers.truncate(1);
        }
        return;
    }
    if cmd.all_conf && !cmd.new_msgs {
        cmd.since = true;
    }
    if cmd.since || cmd.new_date.is_some() || cmd.from_msgs || cmd.your_msgs || cmd.unread_only {
        cmd.full_range = true;
        cmd.keep_going = true;
        cmd.numbers.push(MsgRange::new(1, LAST_MESSAGE));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_ascii_uppercase).collect()
    }

    fn ctx() -> ParseContext {
        ParseContext {
            cur_msg_number: 100,
            may_join: true,
            may_quick_scan: true,
            alias_support: true,
            qwk_support: true,
            num_conferences: 10,
            ..Default::default()
        }
    }

    fn parse_outside(line: &str) -> ReadCommand {
        let mut cmd = parse(&tokens(line), ReadLoop::Outside, &ctx());
        finalize(&mut cmd);
        cmd
    }

    fn parse_inside(line: &str) -> ReadCommand {
        parse(&tokens(line), ReadLoop::Inside, &ctx())
    }

    #[test]
    fn option_takes_two_letter_abbreviations() {
        assert_eq!(option("AL"), Some(O_ALL));
        assert_eq!(option("ALL"), Some(O_ALL));
        assert_eq!(option("SE"), Some(O_SEL));
        assert_eq!(option("RM"), Some(O_RM));
        assert_eq!(option("RM+"), Some(O_RMF));
        assert_eq!(option("RM-"), Some(O_RMB));
        assert_eq!(option("ZZ"), None);
    }

    #[test]
    fn a_single_number_reads_just_that_message() {
        let cmd = parse_outside("42");
        assert_eq!(cmd.numbers, vec![MsgRange { first: 42, last: 42 }]);
        assert!(!cmd.keep_going);
    }

    #[test]
    fn trailing_plus_reads_forward_to_the_end() {
        let cmd = parse_outside("42+");
        assert_eq!(cmd.numbers, vec![MsgRange { first: 42, last: LAST_MESSAGE }]);
        assert!(cmd.keep_going);
    }

    #[test]
    fn trailing_minus_reads_backwards_to_the_first() {
        let cmd = parse_outside("42-");
        assert_eq!(cmd.numbers, vec![MsgRange { first: 42, last: 1 }]);
    }

    #[test]
    fn a_hyphen_makes_an_explicit_range() {
        let cmd = parse_outside("10-20");
        assert_eq!(cmd.numbers, vec![MsgRange { first: 10, last: 20 }]);
    }

    #[test]
    fn several_numbers_make_several_groups() {
        let cmd = parse_outside("5 9 12");
        assert_eq!(
            cmd.numbers,
            vec![MsgRange { first: 5, last: 5 }, MsgRange { first: 9, last: 9 }, MsgRange { first: 12, last: 12 }]
        );
    }

    #[test]
    fn quick_scan_treats_a_bare_number_as_open_ended() {
        let mut context = ctx();
        context.quick_scan = true;
        let cmd = parse(&tokens("42"), ReadLoop::Outside, &context);
        assert_eq!(cmd.numbers, vec![MsgRange { first: 42, last: LAST_MESSAGE }]);
    }

    #[test]
    fn plus_and_minus_are_relative_inside_the_read_loop() {
        assert_eq!(
            parse_inside("+").numbers,
            vec![MsgRange {
                first: 101,
                last: LAST_MESSAGE
            }]
        );
        assert_eq!(parse_inside("-").numbers, vec![MsgRange { first: 99, last: 1 }]);
    }

    #[test]
    fn plus_outside_the_read_loop_means_since() {
        let cmd = parse_outside("+");
        assert!(cmd.since);
        assert!(cmd.full_range);
    }

    #[test]
    fn n_stops_inside_the_loop_but_means_new_outside() {
        assert_eq!(parse_inside("N").func, MsgFunc::Stop);
        assert!(parse_outside("N").new_msgs);
    }

    #[test]
    fn e_edits_the_header_inside_the_loop_and_enters_a_message_outside() {
        assert_eq!(parse_inside("E").func, MsgFunc::EditHeader);
        assert_eq!(parse_outside("E").func, MsgFunc::EnterMessage);
    }

    #[test]
    fn u_unprotects_inside_the_loop_and_limits_to_unread_outside() {
        assert_eq!(parse_inside("U").func, MsgFunc::Unprotect);
        assert!(parse_outside("U").unread_only);
    }

    #[test]
    fn since_produces_a_full_range() {
        let cmd = parse_outside("S");
        assert!(cmd.since);
        assert!(cmd.keep_going);
        assert_eq!(cmd.numbers, vec![MsgRange { first: 1, last: LAST_MESSAGE }]);
    }

    #[test]
    fn y_limits_to_your_messages() {
        let cmd = parse_outside("Y");
        assert!(cmd.your_msgs);
        assert!(!cmd.any_msgs);
        assert!(!cmd.msgs_to_all);
    }

    #[test]
    fn ya_also_includes_messages_to_all() {
        let cmd = parse_outside("YA");
        assert!(cmd.your_msgs);
        assert!(cmd.msgs_to_all);
    }

    #[test]
    fn ts_collects_the_rest_of_the_line_as_search_text() {
        let cmd = parse_outside("TS SOME WORDS");
        assert!(cmd.do_text_search);
        assert_eq!(cmd.search_text, "SOME WORDS");
    }

    #[test]
    fn from_and_to_collect_separate_names() {
        let cmd = parse_outside("FROM JOHN DOE TO JANE ROE");
        assert!(cmd.do_user_search);
        assert_eq!(cmd.user_name_from, "JOHN DOE");
        assert_eq!(cmd.user_name_to, "JANE ROE");
        assert_eq!(cmd.user_search, user_search::FROM | user_search::TO);
    }

    #[test]
    fn a_bare_word_defaults_to_a_text_search_term() {
        let cmd = parse_outside("HELLO");
        assert_eq!(cmd.search_text, "HELLO");
        assert!(!cmd.valid_cmd);
    }

    #[test]
    fn rm_needs_a_memorized_message() {
        assert!(parse_outside("RM").not_memorized);

        let mut context = ctx();
        context.memorized = Some(77);
        let cmd = parse(&tokens("RM"), ReadLoop::Outside, &context);
        assert_eq!(cmd.numbers, vec![MsgRange { first: 77, last: 77 }]);

        let cmd = parse(&tokens("RM+"), ReadLoop::Outside, &context);
        assert_eq!(cmd.numbers, vec![MsgRange { first: 77, last: LAST_MESSAGE }]);
        assert!(cmd.keep_going);

        let cmd = parse(&tokens("RM-"), ReadLoop::Outside, &context);
        assert_eq!(cmd.numbers, vec![MsgRange { first: 77, last: 1 }]);
    }

    #[test]
    fn rr_follows_the_reply_pointer_and_redisplays_when_there_is_none() {
        assert_eq!(parse_inside("RR").func, MsgFunc::Redisplay);

        let mut context = ctx();
        context.reply_to = 55;
        let cmd = parse(&tokens("RR"), ReadLoop::Inside, &context);
        assert_eq!(cmd.numbers, vec![MsgRange { first: 55, last: 55 }]);
        let cmd = parse(&tokens("RR+"), ReadLoop::Inside, &context);
        assert_eq!(cmd.numbers, vec![MsgRange { first: 55, last: LAST_MESSAGE }]);
        let cmd = parse(&tokens("RR-"), ReadLoop::Inside, &context);
        assert_eq!(cmd.numbers, vec![MsgRange { first: 55, last: 1 }]);
    }

    #[test]
    fn move_takes_an_optional_conference_number() {
        let cmd = parse_outside("MOVE 3");
        assert_eq!(cmd.func, MsgFunc::Move);
        assert_eq!(cmd.move_conf, Some(3));

        let cmd = parse_outside("MOVE");
        assert_eq!(cmd.func, MsgFunc::Move);
        assert_eq!(cmd.move_conf, None);

        // out of range conference numbers are dropped so the user is asked
        let cmd = parse_outside("COPY 99");
        assert_eq!(cmd.func, MsgFunc::Copy);
        assert_eq!(cmd.move_conf, None);
    }

    #[test]
    fn threading_adds_a_range_in_the_chosen_direction() {
        let cmd = parse_inside("T+");
        assert!(cmd.threading);
        assert_eq!(
            cmd.numbers,
            vec![MsgRange {
                first: 101,
                last: LAST_MESSAGE
            }]
        );

        let cmd = parse_inside("T-");
        assert!(cmd.threading);
        assert_eq!(cmd.numbers, vec![MsgRange { first: 99, last: 1 }]);
    }

    #[test]
    fn threading_only_applies_inside_the_read_loop() {
        assert!(!parse_outside("T").threading);
        assert!(parse_inside("T").threading);
    }

    #[test]
    fn a_needs_join_access() {
        let mut context = ctx();
        context.may_join = false;
        let cmd = parse(&tokens("A"), ReadLoop::Outside, &context);
        assert!(!cmd.all_conf);
        assert!(!cmd.valid_cmd);
    }

    #[test]
    fn all_conferences_implies_since() {
        let cmd = parse_outside("A");
        assert!(cmd.all_conf);
        assert!(cmd.ask_resume_all);
        assert!(cmd.since);
    }

    #[test]
    fn capture_letters_set_the_matching_flags() {
        let cmd = parse_outside("D");
        assert!(cmd.open_capture);
        assert!(!cmd.cap_ask);
        assert!(!cmd.zip_cap);

        let cmd = parse_outside("Z");
        assert!(cmd.open_capture);
        assert!(cmd.zip_cap);

        let cmd = parse_outside("C");
        assert!(cmd.open_capture);
        assert!(cmd.cap_ask);
    }

    #[test]
    fn a_capture_inside_the_loop_grabs_the_current_message_only() {
        let cmd = parse_inside("C");
        assert!(cmd.capture_single);
        assert_eq!(cmd.func, MsgFunc::Redisplay);
    }

    #[test]
    fn a_capture_with_a_range_keeps_the_range() {
        let cmd = parse_outside("10-20 C");
        assert!(cmd.open_capture);
        assert!(!cmd.capture_single);
        assert_eq!(cmd.numbers, vec![MsgRange { first: 10, last: 20 }]);
    }

    #[test]
    fn n_followed_by_six_digits_is_a_date_not_a_message_number() {
        let cmd = parse_outside("N 010295");
        assert!(cmd.new_msgs);
        assert_eq!(cmd.new_date.as_deref(), Some("010295"));
        assert!(cmd.numbers.is_empty() || cmd.full_range);
    }

    #[test]
    fn reply_mode_only_accepts_numbers() {
        let mut context = ctx();
        context.reply_command = true;
        let cmd = parse(&tokens("5 G"), ReadLoop::Outside, &context);
        assert_eq!(cmd.numbers, vec![MsgRange { first: 5, last: 5 }]);
        assert_eq!(cmd.func, MsgFunc::None);
    }

    #[test]
    fn j_stops_reading_the_rest_of_the_line() {
        let cmd = parse_outside("J 5 TS FOO");
        assert_eq!(cmd.func, MsgFunc::Join);
        assert!(cmd.search_text.is_empty());
    }

    #[test]
    fn alias_takes_an_explicit_state() {
        assert_eq!(parse_outside("ALIAS ON").toggle_alias, Some(AliasToggle::On));
        assert_eq!(parse_outside("ALIAS OFF").toggle_alias, Some(AliasToggle::Off));
        assert_eq!(parse_outside("ALIAS").toggle_alias, Some(AliasToggle::Flip));
    }

    #[test]
    fn o_suppresses_the_pointer_update() {
        let cmd = parse_outside("O");
        assert!(!cmd.update_pointers);
        assert!(cmd.update_msg_status);

        let mut context = ctx();
        context.may_read_only = true;
        let cmd = parse(&tokens("O"), ReadLoop::Outside, &context);
        assert!(!cmd.update_msg_status);
    }
}
