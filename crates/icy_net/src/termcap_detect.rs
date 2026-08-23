use std::{collections::HashSet, time::Duration};

use regex::Regex;
use tokio::time::sleep;

use crate::Connection;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalProgram {
    IcyTerm,
    SyncTerm,
    Unknown,
    Name(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TerminalCaps {
    pub program: TerminalProgram,
    /// Raw primary device-attributes reply, retained for diagnostics.
    pub device_attributes: Option<String>,
    pub term_size: (u16, u16),
    pub is_utf8: bool,
    pub rip_version: Option<String>,

    /// What the terminal can draw.
    pub gfx: GfxCapabilities,

    /// Whether it can play a sound file.
    pub sound: bool,

    /// `Some` when DECRQM explicitly reported whether synchronized output is supported.
    pub synchronized_output: Option<bool>,

    /// `Some` when DECMSR explicitly reported whether terminal macro storage is available.
    pub terminal_macros: Option<bool>,

    /// Whether any query at all came back. Nothing may be waited on without this,
    /// or a terminal that answers nothing costs a timeout every time it is asked.
    pub answered: bool,
}

impl TerminalCaps {
    pub const LOCAL: TerminalCaps = TerminalCaps {
        program: TerminalProgram::Unknown,
        device_attributes: None,
        term_size: (80, 25),
        is_utf8: true,
        rip_version: None,
        gfx: GfxCapabilities::DEFAULT,
        sound: false,
        synchronized_output: None,
        terminal_macros: None,
        answered: false,
    };

    /// Asks the caller's terminal everything worth knowing, in one go.
    ///
    /// The probe result comes back alongside, because it carries what the caller already
    /// has cached and the bytes that were not answers - those are the caller typing.
    pub async fn detect(com: &mut dyn Connection) -> crate::Result<(Self, MediaProbeResult)> {
        let mut buf = [0; 1024];

        com.send(b"\x1B[999;999H\x1B[6n").await?;
        let instant = std::time::Instant::now();
        let mut term_size = (80, 25);
        let mut answered = false;
        while instant.elapsed().as_millis() < 100 {
            let size = com.try_read(&mut buf).await?;
            if size == 0 {
                sleep(Duration::from_millis(10)).await;
                continue;
            }
            let result = String::from_utf8_lossy(&buf[0..size]).to_string();
            if result.ends_with("R") {
                term_size = parse_cursor_pos(result);
                answered = true;
            }
            break;
        }
        com.send(b"\x1B[!\x07\x07\x07").await?;
        let mut rip_version = None;
        while instant.elapsed().as_millis() < 100 {
            let size = com.try_read(&mut buf).await?;
            if size == 0 {
                sleep(Duration::from_millis(10)).await;
                continue;
            }
            let result = String::from_utf8_lossy(&buf[0..size]).to_string();
            rip_version = parse_rip_version(&result);
            break;
        }

        com.send(b"\x1B[1;1H\x01\xF6\x1C\x1B[6n").await?;
        let instant = std::time::Instant::now();
        let mut is_utf8 = false;
        while instant.elapsed().as_millis() < 100 {
            let size = com.try_read(&mut buf).await?;
            if size == 0 {
                sleep(Duration::from_millis(10)).await;
                continue;
            }
            let result = String::from_utf8_lossy(&buf[0..size]).to_string();
            if result.ends_with("R") {
                is_utf8 = parse_cursor_pos(result).0 == 1;
                answered = true;
            }
            break;
        }

        // The device attributes reply names the program, so the same answer settles
        // both that and the drawing capabilities.
        let media = probe_media(com, &mut TerminalProbe::default()).await?;
        let program = match media.device_attributes.as_deref() {
            Some(reply) if reply.contains("73;99;121;84;101;114;109") => TerminalProgram::IcyTerm,
            Some(reply) if reply.contains("67;84;101;114") => TerminalProgram::SyncTerm,
            Some(reply) => TerminalProgram::Name(reply.to_string()),
            None => TerminalProgram::Unknown,
        };

        Ok((
            Self {
                program,
                device_attributes: media.device_attributes.clone(),
                term_size,
                is_utf8,
                rip_version,
                gfx: media.gfx,
                sound: media.sound,
                synchronized_output: media.synchronized_output,
                terminal_macros: media.terminal_macros,
                answered: answered || media.answered,
            },
            media,
        ))
    }
}

/// What one pass of the media queries turned up.
pub struct MediaProbeResult {
    pub gfx: GfxCapabilities,
    pub sound: bool,
    pub synchronized_output: Option<bool>,
    pub terminal_macros: Option<bool>,
    /// The raw device attributes reply, which also names the terminal program.
    pub device_attributes: Option<String>,
    /// What the caller's media cache already holds, each name carrying its own prefix.
    pub cache_listing: HashSet<String>,
    pub answered: bool,
    /// Bytes that were not answers and still belong to the caller.
    pub leftover: Vec<u8>,
}

/// How long a terminal is given to answer a question. The capability queries all go out
/// together, so this is the cost of asking a terminal that ignores every one.
const MEDIA_PROBE_TIMEOUT: Duration = Duration::from_millis(500);

/// Reads until the probe reports what is being waited for, or the terminal stays quiet
/// for too long. Anything that is not an answer is appended to `leftover`.
async fn collect_replies(
    com: &mut dyn Connection,
    probe: &mut TerminalProbe,
    leftover: &mut Vec<u8>,
    answered: fn(&TerminalProbe) -> bool,
) -> crate::Result<()> {
    let deadline = std::time::Instant::now() + MEDIA_PROBE_TIMEOUT;
    let mut buf = [0; 256];
    while !answered(probe) && std::time::Instant::now() < deadline {
        let read = com.try_read(&mut buf).await?;
        if read == 0 {
            sleep(Duration::from_millis(5)).await;
            continue;
        }
        for byte in &buf[..read] {
            leftover.extend(probe.feed(*byte));
        }
    }
    Ok(())
}

/// Asks what the terminal can draw and play, and what it already has cached.
///
/// Every query is answered by a terminal that has the feature and ignored by one that
/// does not, so the wait is bounded and a silent terminal ends up with the defaults.
pub async fn probe_media(com: &mut dyn Connection, probe: &mut TerminalProbe) -> crate::Result<MediaProbeResult> {
    probe.start();
    com.send(DEVICE_ATTRIBUTES_QUERY).await?;
    com.send(CTERM_ATTRIBUTES_QUERY).await?;
    com.send(CELL_SIZE_QUERY).await?;
    com.send(PIXEL_SIZE_QUERY).await?;
    com.send(JXL_QUERY).await?;
    com.send(SOUND_QUERY).await?;
    com.send(SYNCHRONIZED_OUTPUT_QUERY).await?;
    com.send(TERMINAL_MACRO_QUERY).await?;

    let mut leftover = Vec::new();
    collect_replies(com, probe, &mut leftover, TerminalProbe::media_answered).await?;

    // Both listings answer with the same header, so they have to be asked one at a
    // time. The names carry their own prefix, which is what tells them apart.
    let mut cache_listing = HashSet::new();
    if probe.capabilities().jxl || probe.sound() {
        for query in [GFX_CACHE_LIST_QUERY, SOUND_CACHE_LIST_QUERY] {
            com.send(query).await?;
            collect_replies(com, probe, &mut leftover, TerminalProbe::cache_listed).await?;
            cache_listing.extend(probe.take_cache_listing().unwrap_or_default());
        }
    }

    let answered = probe.answered();
    let sound = probe.sound();
    let synchronized_output = probe.synchronized_output();
    let terminal_macros = probe.terminal_macros();
    let device_attributes = probe.take_device_attributes();
    let (gfx, listing, pending) = probe.finish();
    cache_listing.extend(listing.unwrap_or_default());
    leftover.extend(pending);
    Ok(MediaProbeResult {
        gfx,
        sound,
        synchronized_output,
        terminal_macros,
        device_attributes,
        cache_listing,
        answered,
        leftover,
    })
}

/// Queries the terminal answers before a graphics backend is chosen.
/// `syncterm_extensions.md` asks for JPEG XL to be probed rather than inferred, and the
/// cell size is what turns a text coordinate into the pixel destination an image APC wants.
pub const DEVICE_ATTRIBUTES_QUERY: &[u8] = b"\x1b[c";
pub const CTERM_ATTRIBUTES_QUERY: &[u8] = b"\x1b[<0c";
pub const CELL_SIZE_QUERY: &[u8] = b"\x1b[16t";
pub const PIXEL_SIZE_QUERY: &[u8] = b"\x1b[14t";
pub const JXL_QUERY: &[u8] = b"\x1b_SyncTERM:Q;JXL\x1b\\";
pub const SOUND_QUERY: &[u8] = b"\x1b_SyncTERM:Q;libsndfile\x1b\\";
pub const SYNCHRONIZED_OUTPUT_QUERY: &[u8] = b"\x1b[?2026$p";
pub const TERMINAL_MACRO_QUERY: &[u8] = b"\x1b[?62n";
pub const GFX_CACHE_LIST_QUERY: &[u8] = b"\x1b_SyncTERM:C;L;gfx/*\x1b\\";
pub const SOUND_CACHE_LIST_QUERY: &[u8] = b"\x1b_SyncTERM:C;L;snd/*\x1b\\";

/// `CTerm` revision that introduced the inline `*Blob` verbs, which draw a changing
/// frame without writing it to the caller's disk cache first.
pub const CTERM_INLINE_BLOB_REVISION: u32 = 1329;

const MAX_REPLY_BYTES: usize = 1024 * 1024;

/// What the caller's terminal turned out to be able to draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GfxCapabilities {
    pub sixel: bool,
    pub jxl: bool,
    pub physical_keys: bool,
    pub cterm_revision: Option<u32>,
    pub cell_width: i32,
    pub cell_height: i32,
    pub screen_width: i32,
    pub screen_height: i32,
}

impl Default for GfxCapabilities {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl GfxCapabilities {
    pub const DEFAULT: Self = Self {
        sixel: false,
        jxl: false,
        physical_keys: false,
        cterm_revision: None,
        cell_width: 8,
        cell_height: 16,
        screen_width: 0,
        screen_height: 0,
    };

    /// Inline blobs are only safe once the terminal has named a revision that has them.
    pub fn inline_blobs(&self) -> bool {
        self.cterm_revision.is_some_and(|revision| revision >= CTERM_INLINE_BLOB_REVISION)
    }
}

/// Collects the capability answers out of the caller's input while a probe is running.
///
/// Replies arrive interleaved with ordinary typing, so anything that is not one of the
/// answers being waited for is handed back and reaches the keyboard unchanged.
#[derive(Default)]
pub struct TerminalProbe {
    active: bool,
    pending: Vec<u8>,
    capabilities: GfxCapabilities,
    jxl_answered: bool,
    sound: bool,
    sound_answered: bool,
    synchronized_output: Option<bool>,
    synchronized_output_answered: bool,
    terminal_macros: Option<bool>,
    terminal_macros_answered: bool,
    device_attributes: Option<String>,
    cache_listing: Option<HashSet<String>>,
}

impl TerminalProbe {
    pub fn start(&mut self) {
        self.active = true;
        self.pending.clear();
        self.capabilities = GfxCapabilities::default();
        self.jxl_answered = false;
        self.sound = false;
        self.sound_answered = false;
        self.synchronized_output = None;
        self.synchronized_output_answered = false;
        self.terminal_macros = None;
        self.terminal_macros_answered = false;
        self.device_attributes = None;
        self.cache_listing = None;
    }

    pub fn capabilities(&self) -> GfxCapabilities {
        self.capabilities
    }

    pub fn jxl_answered(&self) -> bool {
        self.jxl_answered
    }

    pub fn sound(&self) -> bool {
        self.sound
    }

    pub fn synchronized_output(&self) -> Option<bool> {
        self.synchronized_output
    }

    pub fn terminal_macros(&self) -> Option<bool> {
        self.terminal_macros
    }

    /// Both media questions are settled, so there is nothing left to wait for.
    pub fn media_answered(&self) -> bool {
        self.jxl_answered && self.sound_answered && self.synchronized_output_answered && self.terminal_macros_answered
    }

    /// Whether the terminal said anything at all.
    pub fn answered(&self) -> bool {
        self.jxl_answered || self.sound_answered || self.synchronized_output_answered || self.terminal_macros_answered || self.device_attributes.is_some()
    }

    pub fn take_device_attributes(&mut self) -> Option<String> {
        self.device_attributes.take()
    }

    pub fn cache_listed(&self) -> bool {
        self.cache_listing.is_some()
    }

    pub fn take_cache_listing(&mut self) -> Option<HashSet<String>> {
        self.cache_listing.take()
    }

    /// Ends the probe and reports what was learned, along with any half finished
    /// sequence that turned out not to be an answer.
    pub fn finish(&mut self) -> (GfxCapabilities, Option<HashSet<String>>, Vec<u8>) {
        self.active = false;
        let leftover = std::mem::take(&mut self.pending);
        (self.capabilities, self.cache_listing.take(), leftover)
    }

    pub fn feed(&mut self, byte: u8) -> Vec<u8> {
        if !self.active {
            return vec![byte];
        }
        if self.pending.is_empty() {
            if byte == 0x1B {
                self.pending.push(byte);
                return Vec::new();
            }
            return vec![byte];
        }

        self.pending.push(byte);
        if self.pending.len() == 2 {
            if byte != b'[' && byte != b'_' {
                return std::mem::take(&mut self.pending);
            }
            return Vec::new();
        }
        if self.pending.len() > MAX_REPLY_BYTES {
            // A cache listing can be long; replaying one as keystrokes helps nobody.
            log::warn!("Discarding an overlong terminal reply while probing graphics support");
            self.pending.clear();
            return Vec::new();
        }
        if self.pending[1] == b'[' {
            if !(0x40..=0x7E).contains(&byte) {
                return Vec::new();
            }
            let reply = std::mem::take(&mut self.pending);
            if self.parse_csi(&reply) { Vec::new() } else { reply }
        } else {
            if !self.pending.ends_with(b"\x1b\\") {
                return Vec::new();
            }
            let reply = std::mem::take(&mut self.pending);
            if self.parse_apc(&reply) { Vec::new() } else { reply }
        }
    }

    fn parse_csi(&mut self, reply: &[u8]) -> bool {
        let Ok(text) = std::str::from_utf8(reply) else {
            return false;
        };
        let Some(body) = text.strip_prefix("\x1b[") else {
            return false;
        };
        let (body, final_byte) = body.split_at(body.len() - 1);
        match final_byte {
            "{" => {
                let Some(space) = body.strip_suffix('*').and_then(|value| value.parse::<u32>().ok()) else {
                    return false;
                };
                self.terminal_macros = Some(space > 0);
                self.terminal_macros_answered = true;
                true
            }
            "y" => {
                let Some(values) = body.strip_prefix("?2026;").and_then(|value| value.strip_suffix('$')) else {
                    return false;
                };
                let Ok(status) = values.parse::<u8>() else {
                    return false;
                };
                self.synchronized_output = match status {
                    0 => Some(false),
                    1..=4 => Some(true),
                    _ => return false,
                };
                self.synchronized_output_answered = true;
                true
            }
            // CSI = 1 ; <0-or-1> - n answers the JPEG XL query, CSI = 7 ; 100 ; <0-or-1> n
            // the one about sound.
            "n" => {
                let Some(values) = body.strip_prefix('=') else {
                    return false;
                };
                let mut parts = values.trim_end_matches('-').split(';');
                match parts.next() {
                    Some("1") => {
                        let Some(state) = parts.next() else {
                            return false;
                        };
                        self.capabilities.jxl = state == "1";
                        self.jxl_answered = true;
                        true
                    }
                    Some("7") => {
                        if parts.next() != Some("100") {
                            return false;
                        }
                        let Some(state) = parts.next() else {
                            return false;
                        };
                        self.sound = state == "1";
                        self.sound_answered = true;
                        true
                    }
                    _ => false,
                }
            }
            // CSI = 67;84;101;114;109;MAJOR;MINOR c spells "CTerm" and its revision.
            "c" => {
                if let Some(values) = body.strip_prefix('<') {
                    let features = values.split(';').filter_map(|value| value.parse::<i32>().ok()).collect::<Vec<_>>();
                    self.capabilities.sixel = features.contains(&4);
                    self.capabilities.physical_keys = features.contains(&8);
                    return true;
                }
                // Whatever else it says, a device attributes reply is never typing, and
                // the raw text is what names an unrecognised terminal.
                self.device_attributes = Some(text.to_string());
                let Some(values) = body.strip_prefix('=') else {
                    return true;
                };
                let numbers: Vec<&str> = values.split(';').collect();
                // Icy Term names itself instead of CTerm. It carries the inline blob
                // verbs from 0.8.4, which is what the revision stands in for here.
                if numbers.len() >= 10 && numbers[..7] == ["73", "99", "121", "84", "101", "114", "109"] {
                    let (Ok(major), Ok(minor), Ok(patch)) = (numbers[7].parse::<u32>(), numbers[8].parse::<u32>(), numbers[9].parse::<u32>()) else {
                        return true;
                    };
                    if (major, minor, patch) >= (0, 8, 4) {
                        self.capabilities.cterm_revision = Some(CTERM_INLINE_BLOB_REVISION);
                    }
                    return true;
                }
                if numbers.len() < 7 || numbers[..5] != ["67", "84", "101", "114", "109"] {
                    return true;
                }
                let (Ok(major), Ok(minor)) = (numbers[5].parse::<u32>(), numbers[6].parse::<u32>()) else {
                    return true;
                };
                self.capabilities.cterm_revision = Some(major * 1000 + minor);
                true
            }
            // CSI 6 ; height ; width t answers the cell size request.
            "t" => {
                let mut parts = body.split(';');
                let Some(report) = parts.next() else {
                    return false;
                };
                let (Some(Ok(height)), Some(Ok(width))) = (parts.next().map(str::parse::<i32>), parts.next().map(str::parse::<i32>)) else {
                    return false;
                };
                if height <= 0 || width <= 0 {
                    return false;
                }
                match report {
                    "4" => {
                        self.capabilities.screen_height = height;
                        self.capabilities.screen_width = width;
                        true
                    }
                    "6" => {
                        self.capabilities.cell_height = height;
                        self.capabilities.cell_width = width;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn parse_apc(&mut self, reply: &[u8]) -> bool {
        let Ok(text) = std::str::from_utf8(reply) else {
            return false;
        };
        let Some(payload) = text.strip_prefix("\x1b_").and_then(|rest| rest.strip_suffix("\x1b\\")) else {
            return false;
        };
        let Some(body) = payload.strip_prefix("SyncTERM:C;L") else {
            return false;
        };
        // The header line carries the command back; every line after it is name TAB md5.
        let entries = body.split_once('\n').map_or("", |(_, rest)| rest);
        // Listings are additive: each query answers for one prefix, and a caller can
        // hold both pictures and sound.
        self.cache_listing.get_or_insert_default().extend(
            entries
                .lines()
                .filter_map(|line| line.split('\t').next())
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_string),
        );
        true
    }
}

lazy_static::lazy_static! {
    static ref RIP_REGEX:Regex = Regex::new("RIPSCRIP(\\d+)").unwrap();
}
fn parse_rip_version(data: &str) -> Option<String> {
    if let Some(caps) = RIP_REGEX.captures(data)
        && let Some(r) = caps.get(1)
    {
        return Some(r.as_str().to_string());
    }
    None
}

fn parse_cursor_pos(result: String) -> (u16, u16) {
    let mut y = 0;
    let mut x = 0;
    let mut parse_x = false;
    for b in result.chars() {
        if let Some(digit) = b.to_digit(10) {
            if parse_x {
                x = x * 10 + digit as u16;
            } else {
                y = y * 10 + digit as u16;
            }
        }
        if b == ';' {
            parse_x = true;
        }
    }
    (x, y)
}

#[cfg(test)]
mod test {
    use crate::termcap_detect::{TerminalProbe, parse_rip_version};

    #[test]
    fn test_parse_rip() {
        assert_eq!(parse_rip_version("NEEALEG"), None);
        assert_eq!(parse_rip_version("RIPSCRIP015410\0"), Some("015410".to_string()));
    }

    #[test]
    fn collects_the_capability_answers_and_passes_typing_through() {
        let mut probe = TerminalProbe::default();
        probe.start();
        let mut typed = Vec::new();
        for byte in b"\x1b[=67;84;101;114;109;1;332c\x1b[6;20;10tA\x1b[=1;1-n" {
            typed.extend(probe.feed(*byte));
        }

        assert!(probe.jxl_answered());
        let (capabilities, _, leftover) = probe.finish();
        assert_eq!(typed, b"A");
        assert!(leftover.is_empty());
        assert!(capabilities.jxl);
        assert_eq!(capabilities.cterm_revision, Some(1332));
        assert!(capabilities.inline_blobs());
        assert_eq!((capabilities.cell_width, capabilities.cell_height), (10, 20));
    }

    #[test]
    fn a_denied_answer_keeps_jpeg_xl_off() {
        let mut probe = TerminalProbe::default();
        probe.start();
        for byte in b"\x1b[=1;0-n" {
            assert!(probe.feed(*byte).is_empty());
        }

        assert!(probe.jxl_answered());
        assert!(!probe.capabilities().jxl);
    }

    #[test]
    fn synchronized_output_support_is_tri_state() {
        let parse = |reply: &[u8]| {
            let mut probe = TerminalProbe::default();
            probe.start();
            for byte in reply {
                assert!(probe.feed(*byte).is_empty());
            }
            probe.synchronized_output()
        };

        assert_eq!(parse(b"\x1b[?2026;0$y"), Some(false));
        assert_eq!(parse(b"\x1b[?2026;2$y"), Some(true));
        assert_eq!(TerminalProbe::default().synchronized_output(), None);
    }

    #[test]
    fn terminal_macro_support_is_tri_state() {
        let parse = |reply: &[u8]| {
            let mut probe = TerminalProbe::default();
            probe.start();
            for byte in reply {
                assert!(probe.feed(*byte).is_empty());
            }
            probe.terminal_macros()
        };

        assert_eq!(parse(b"\x1b[32767*{"), Some(true));
        assert_eq!(parse(b"\x1b[0*{"), Some(false));
        assert_eq!(TerminalProbe::default().terminal_macros(), None);
    }

    /// What Icy Term answers: it names itself rather than a `CTerm` revision, and from
    /// 0.8.4 it carries the inline blob verbs that keep frames out of the disk cache.
    #[test]
    fn icy_term_is_served_jpeg_xl_and_inline_blobs_from_0_8_4() {
        let probe_icy_term = |identity: &[u8]| {
            let mut probe = TerminalProbe::default();
            probe.start();
            let mut typed = Vec::new();
            for byte in identity {
                typed.extend(probe.feed(*byte));
            }
            for byte in b"\x1b[<1;2;3;4;5;6;7c\x1b[6;16;8t\x1b[4;400;640t\x1b[=1;1-n" {
                typed.extend(probe.feed(*byte));
            }
            let (capabilities, _, _) = probe.finish();
            (capabilities, typed)
        };

        let (capabilities, typed) = probe_icy_term(b"\x1b[=73;99;121;84;101;114;109;0;8;4c");
        assert!(capabilities.jxl);
        assert!(capabilities.sixel);
        assert!(capabilities.inline_blobs());
        assert_eq!((capabilities.cell_width, capabilities.cell_height), (8, 16));
        // The identity is an answer now, so it no longer reaches the keyboard.
        assert!(typed.is_empty());

        let (older, typed) = probe_icy_term(b"\x1b[=73;99;121;84;101;114;109;0;8;3c");
        assert!(older.jxl);
        assert!(!older.inline_blobs());
        assert!(typed.is_empty());
    }

    #[test]
    fn a_cache_listing_names_what_the_caller_already_holds() {
        let mut probe = TerminalProbe::default();
        probe.start();
        for byte in b"\x1b_SyncTERM:C;L\ngfx/abc.jxl\td41d8c\ngfx/def.jxl\t0cc175\n\x1b\\" {
            assert!(probe.feed(*byte).is_empty());
        }

        assert!(probe.cache_listed());
        let (_, listing, _) = probe.finish();
        let listing = listing.unwrap();
        assert!(listing.contains("gfx/abc.jxl"));
        assert!(listing.contains("gfx/def.jxl"));
    }

    #[test]
    fn an_unrelated_escape_sequence_reaches_the_keyboard() {
        let mut probe = TerminalProbe::default();
        probe.start();
        let mut typed = Vec::new();
        for byte in b"\x1b[D" {
            typed.extend(probe.feed(*byte));
        }

        assert_eq!(typed, b"\x1b[D");
    }
}
