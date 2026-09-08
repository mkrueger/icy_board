//! Read-only, bounded local logs for the call-wait screen. No scheduler leases,
//! logger reconfiguration, or log writes belong here.
use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

use codepages::tables::CP437_TO_UNICODE;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use icy_board_engine::icy_board::{IcyBoard, bbs::BBS};
use icy_board_tui::{app::get_screen_size, chrome::key_hint, get_text as text, theme::get_tui_theme};
use ratatui::{
    Frame, Terminal,
    backend::Backend,
    layout::{Constraint, Layout},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Widget},
};
use tokio::{sync::Mutex, task::JoinHandle};

use crate::{Res, event_screen::RuntimeStatus};

const MAX_BYTES: usize = 256 * 1024;
const MAX_LINES: usize = 2000;
const MAX_LINE_CHARS: usize = 2048;
const MAX_QUERY_CHARS: usize = 128;
const REFRESH_INTERVAL: Duration = Duration::from_secs(1);
// Nonzero: the shared crossterm reader drops keys on a zero-timeout poll.
const POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Source {
    #[default]
    Application,
    Caller,
    File,
}

#[derive(Debug)]
enum ReadFailure {
    Unconfigured,
    NotRegular,
    Io(io::Error),
    Worker(String),
}

impl From<io::Error> for ReadFailure {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl ReadFailure {
    fn message(&self) -> String {
        match self {
            Self::Unconfigured => text("log_view_unconfigured"),
            Self::NotRegular => text("log_view_regular_only"),
            Self::Io(error) => display_text(&error.to_string()),
            Self::Worker(error) => display_text(error),
        }
    }
}

#[derive(Debug)]
struct LogLine {
    text: String,
    warning: bool,
    redacted: bool,
}

/// Match main.rs's fern prefix, not words in the message or a continuation.
/// Level's Display spelling is uppercase: [YYYY-MM-DD HH:MM:SS LEVEL target].
fn is_warning(line: &str) -> bool {
    let Some(header) = line.strip_prefix('[').and_then(|s| s.split_once("] ")).map(|(header, _)| header) else {
        return false;
    };
    let Some(timestamp) = header.get(..19) else {
        return false;
    };
    if chrono::NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%d %H:%M:%S").is_err() {
        return false;
    }
    let Some((level, target)) = header.get(19..).and_then(|s| s.strip_prefix(' ')).and_then(|s| s.split_once(' ')) else {
        return false;
    };
    matches!(level, "WARN" | "ERROR") && !target.is_empty() && !target.chars().any(char::is_whitespace)
}

/// Strip CSI, OSC, DCS and other terminal strings, including C1 forms. Preserve
/// no terminal controls or bidi formatting. State is reset at each physical log
/// line, so a malformed escape cannot hide the rest of the file.
pub(crate) fn sanitize(input: &str) -> String {
    #[derive(Clone, Copy)]
    enum Escape {
        None,
        Start,
        Intermediate,
        Csi,
        String,
        StringEscape,
    }
    let mut state = Escape::None;
    let mut output = String::with_capacity(input.len());
    for ch in input.chars() {
        match state {
            Escape::Start => {
                state = match ch {
                    '[' => Escape::Csi,
                    ']' | 'P' | 'X' | '^' | '_' => Escape::String,
                    '\x20'..='\x2f' => Escape::Intermediate,
                    _ => Escape::None,
                };
            }
            Escape::Intermediate => {
                if ('\x30'..='\x7e').contains(&ch) {
                    state = Escape::None;
                }
            }
            Escape::Csi => {
                if ('\x40'..='\x7e').contains(&ch) {
                    state = Escape::None;
                }
            }
            Escape::String => match ch {
                '\x07' | '\u{009c}' => state = Escape::None,
                '\x1b' => state = Escape::StringEscape,
                _ => {}
            },
            Escape::StringEscape => {
                state = if matches!(ch, '\\' | '\x07' | '\u{009c}') {
                    Escape::None
                } else if ch == '\x1b' {
                    Escape::StringEscape
                } else {
                    Escape::String
                };
            }
            Escape::None => match ch {
                '\x1b' => state = Escape::Start,
                '\u{009b}' => state = Escape::Csi,
                '\u{0090}' | '\u{0098}' | '\u{009d}' | '\u{009e}' | '\u{009f}' => state = Escape::String,
                '\t' | '\r' | '\n' => output.push(' '),
                ch if ch.is_control()
                    || matches!(ch, '\u{061c}' | '\u{200b}'..='\u{200f}' | '\u{2028}'..='\u{202e}' | '\u{2060}'..='\u{206f}' | '\u{feff}') => {}
                _ => output.push(ch),
            },
        }
    }
    output
}

fn is_redacted(input: &str, clean: &str) -> bool {
    input.to_ascii_lowercase().contains("web admin token:") || clean.to_ascii_lowercase().contains("web admin token:")
}

fn display_text(input: &str) -> String {
    // Redact before clipping, and after stripping escapes that might split the
    // marker. Drop the entire line: connections.rs logs the generated token as
    // "web admin token: {}". Never expose its value through search or errors.
    let clean = sanitize(input);
    if is_redacted(input, &clean) {
        return text("log_view_redacted");
    }
    if clean.chars().count() > MAX_LINE_CHARS {
        let mut clipped: String = clean.chars().take(MAX_LINE_CHARS - 1).collect();
        clipped.push('…');
        clipped
    } else {
        clean
    }
}

fn decode_line(bytes: &[u8], source: Source) -> LogLine {
    let decoded = match source {
        Source::Application | Source::File => String::from_utf8_lossy(bytes).into_owned(),
        // Native caller logs are UTF-8. Imported DOS logs can contain CP437;
        // fall back only for records that are not valid UTF-8. Preserve C0
        // bytes for the sanitizer rather than decoding them as pictographs.
        Source::Caller if std::str::from_utf8(bytes).is_ok() => String::from_utf8_lossy(bytes).into_owned(),
        Source::Caller => bytes
            .iter()
            .map(|&byte| {
                if byte < 32 || byte == 127 {
                    char::from(byte)
                } else {
                    CP437_TO_UNICODE[byte as usize]
                }
            })
            .collect(),
    };
    let warning = source == Source::Application && is_warning(&decoded);
    LogLine {
        text: display_text(decoded.trim_end_matches('\r')),
        warning,
        redacted: is_redacted(&decoded, &sanitize(&decoded)),
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct FileInfo {
    size: u64,
    modified: Option<SystemTime>,
    identity: Option<(u64, u64)>,
}

impl FileInfo {
    fn from_metadata(metadata: &fs::Metadata) -> Self {
        #[cfg(unix)]
        let identity = {
            use std::os::unix::fs::MetadataExt;
            Some((metadata.dev(), metadata.ino()))
        };
        #[cfg(not(unix))]
        let identity = None;
        Self {
            size: metadata.len(),
            modified: metadata.modified().ok(),
            identity,
        }
    }

    fn modified_text(&self) -> String {
        self.modified
            .map(|time| chrono::DateTime::<chrono::Utc>::from(time).format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| text("log_view_unknown"))
    }
}

#[derive(Debug)]
struct Tail {
    lines: Vec<LogLine>,
    metadata: FileInfo,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum FileChange {
    #[default]
    None,
    Rotated,
    Truncated,
}

/// Reopen for every sample: no stale descriptor after rotation or truncation.
/// Size is sampled once; a growing file cannot make a read chase its writer.
fn read_sample(path: &Path, source: Source) -> Result<Tail, ReadFailure> {
    if path.as_os_str().is_empty() {
        return Err(ReadFailure::Unconfigured);
    }
    // Check BEFORE opening, otherwise opening a FIFO could block indefinitely.
    // Reject a final symlink as well as directories, sockets and devices.
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        return Err(ReadFailure::NotRegular);
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // Linux UAPI O_NONBLOCK | O_NOFOLLOW: close the metadata/open FIFO and
        // final-symlink replacement races without introducing a dependency.
        // These flags do not change regular-file reads. Other platforms still
        // have the pre-open and descriptor metadata checks.
        options.custom_flags(0o4000 | 0o400000);
    }
    let mut file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(ReadFailure::NotRegular);
    }
    // All displayed metadata and identity describe the opened descriptor, not
    // the pre-open path check (which may already name a replacement file).
    let metadata = FileInfo::from_metadata(&metadata);
    let size = metadata.size;
    let start = size.saturating_sub(MAX_BYTES as u64);
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::with_capacity((size - start) as usize);
    file.take(size - start).read_to_end(&mut bytes)?;
    let mut data = bytes.as_slice();
    if start > 0 {
        // Never display a leading fragment: it could be the suffix of a token
        // line whose redaction marker lies outside our bounded byte window.
        data = match data.iter().position(|&byte| byte == b'\n') {
            Some(index) => &data[index + 1..],
            None => &[],
        };
    }
    if data.is_empty() {
        return Ok(Tail { lines: Vec::new(), metadata });
    }
    // Avoid a phantom empty row for the terminating newline, but keep actual
    // blank lines and an unterminated last record. Decode only the newest rows.
    data = data.strip_suffix(b"\n").unwrap_or(data);
    let mut lines: Vec<_> = data
        .rsplit(|&byte| byte == b'\n')
        .take(MAX_LINES)
        .map(|line| decode_line(line, source))
        .collect();
    lines.reverse();
    Ok(Tail { lines, metadata })
}

#[cfg(test)]
fn read_tail(path: &Path, source: Source) -> Result<Vec<LogLine>, ReadFailure> {
    read_sample(path, source).map(|tail| tail.lines)
}

struct Snapshot {
    path: PathBuf,
    result: Result<Tail, ReadFailure>,
}

struct ReadJob {
    source: Source,
    generation: u64,
    task: JoinHandle<Option<Snapshot>>,
}

impl ReadJob {
    fn start(board: Arc<Mutex<IcyBoard>>, source: Source, generation: u64, fixed_path: Option<PathBuf>) -> Self {
        Self {
            source,
            generation,
            task: tokio::task::spawn_blocking(move || {
                let path = if let Some(path) = fixed_path {
                    path
                } else {
                    // Retry on the next tick rather than waiting for a board
                    // writer. resolve_file itself can perform filesystem IO,
                    // so path resolution belongs on this worker too.
                    let board = board.try_lock().ok()?;
                    match source {
                        // main loads config_file unchanged into file_name,
                        // after opening config_file.with_extension("log").
                        Source::Application => board.file_name.with_extension("log"),
                        Source::Caller if board.config.paths.caller_log.as_os_str().is_empty() => PathBuf::new(),
                        Source::Caller => board.resolve_file(&board.config.paths.caller_log),
                        Source::File => PathBuf::new(),
                    }
                };
                // Neither board nor BBS is locked while reading the tail.
                let result = read_sample(&path, source);
                Some(Snapshot { path, result })
            }),
        }
    }
}

impl Drop for ReadJob {
    fn drop(&mut self) {
        // Cancels queued work. An already running blocking read may finish in
        // the background; never await it on exit and block the owner handshake.
        self.task.abort();
    }
}

#[derive(Default)]
struct LogScreen {
    source: Source,
    fixed_path: Option<PathBuf>,
    generation: u64,
    path: PathBuf,
    lines: Vec<LogLine>,
    metadata: Option<FileInfo>,
    // Comparison baseline only; never rendered on an error. Keeping it across
    // a missing-file interval lets the next reopen detect rename rotation.
    last_good: Option<FileInfo>,
    change: FileChange,
    visible: Vec<usize>,
    matches: Vec<usize>,
    selected: Option<usize>,
    context: bool,
    error: Option<ReadFailure>,
    loaded: bool,
    follow: bool,
    warnings_only: bool,
    query: String,
    editing: Option<String>,
    top: usize,
    horizontal: u16,
    page: usize,
    runtime: RuntimeStatus,
}

enum Action {
    Nothing,
    Switch,
    Refresh,
    Exit,
}

impl LogScreen {
    fn new() -> Self {
        Self {
            follow: true,
            page: 1,
            ..Self::default()
        }
    }

    fn file(path: PathBuf) -> Self {
        Self {
            source: Source::File,
            fixed_path: Some(path.clone()),
            path,
            ..Self::new()
        }
    }

    fn wants_read(&self) -> bool {
        // An explicitly selected source gets one initial snapshot even while
        // paused. A read error counts as a snapshot, too: no paused retries.
        self.follow || !self.loaded
    }

    fn set_follow(&mut self, follow: bool) {
        if self.follow != follow {
            self.follow = follow;
            // Invalidate even an already completed job. Pausing then resuming
            // before its delivery must not apply a pre-pause tail on resume.
            self.generation = self.generation.wrapping_add(1);
        }
    }

    fn accept(&mut self, source: Source, generation: u64, snapshot: Snapshot) -> bool {
        if self.source != source || self.generation != generation || !self.wants_read() {
            return false;
        }
        self.apply(snapshot);
        true
    }

    fn max_top(&self) -> usize {
        self.visible.len().saturating_sub(self.page.max(1))
    }

    fn clamp(&mut self) {
        self.top = if self.follow { self.max_top() } else { self.top.min(self.max_top()) };
    }

    fn rebuild(&mut self) {
        let query = self.query.to_lowercase();
        self.visible.clear();
        self.matches.clear();
        for (index, line) in self.lines.iter().enumerate() {
            if self.warnings_only && self.source == Source::Application && !line.warning {
                continue;
            }
            // Search only the displayed, bounded, sanitized text. Even the
            // translated redaction placeholder must never be a search match.
            let matched = !query.is_empty() && !line.redacted && line.text.to_lowercase().contains(&query);
            if matched {
                self.matches.push(index);
            }
            if self.context || query.is_empty() || matched {
                self.visible.push(index);
            }
        }
        if self.selected.is_some_and(|index| self.matches.binary_search(&index).is_err()) {
            self.selected = None;
        }
        self.clamp();
    }

    fn reveal_selection(&mut self) {
        if let Some(position) = self.selected.and_then(|index| self.visible.binary_search(&index).ok()) {
            self.top = position.saturating_sub(self.page / 2).min(self.max_top());
        }
    }

    fn jump_match(&mut self, backwards: bool) {
        if self.matches.is_empty() {
            return;
        }
        let count = self.matches.len();
        let position = self.selected.and_then(|index| self.matches.binary_search(&index).ok());
        let next = match (position, backwards) {
            (Some(position), true) => (position + count - 1) % count,
            (Some(position), false) => (position + 1) % count,
            (None, true) => count - 1,
            (None, false) => 0,
        };
        self.set_follow(false);
        self.selected = Some(self.matches[next]);
        self.horizontal = 0;
        self.reveal_selection();
    }

    fn apply(&mut self, snapshot: Snapshot) {
        if !self.wants_read() {
            return;
        }
        if self.path != snapshot.path {
            self.metadata = None;
            self.last_good = None;
            self.change = FileChange::None;
        }
        self.path = snapshot.path;
        self.loaded = true;
        self.selected = None;
        match snapshot.result {
            Ok(tail) => {
                if let Some(previous) = &self.last_good {
                    if previous.identity.is_some() && tail.metadata.identity.is_some() && previous.identity != tail.metadata.identity {
                        self.change = FileChange::Rotated;
                    } else if tail.metadata.size < previous.size {
                        self.change = FileChange::Truncated;
                    }
                    // Retain the last detected change across normal refreshes
                    // so a one-second redraw cannot hide the notification.
                }
                self.last_good = Some(tail.metadata.clone());
                self.metadata = Some(tail.metadata);
                self.lines = tail.lines;
                self.error = None;
            }
            Err(error) => {
                // Do not keep showing a previous file's content on an error.
                self.lines.clear();
                self.metadata = None;
                self.change = FileChange::None;
                self.error = Some(error);
            }
        }
        self.rebuild();
    }

    fn handle_key(&mut self, key: KeyCode) -> Action {
        if let Some(edit) = &mut self.editing {
            match key {
                KeyCode::Esc => self.editing = None,
                KeyCode::Enter => {
                    self.query = self.editing.take().unwrap_or_default();
                    self.selected = None;
                    self.top = 0;
                    self.rebuild();
                    if self.context {
                        self.jump_match(false);
                    }
                }
                KeyCode::Backspace => {
                    edit.pop();
                }
                KeyCode::Char(ch) if !ch.is_control() && edit.chars().count() < MAX_QUERY_CHARS => edit.push(ch),
                _ => {}
            }
            return Action::Nothing;
        }
        match key {
            KeyCode::Esc => return Action::Exit,
            KeyCode::Tab if self.source != Source::File => {
                self.source = if self.source == Source::Application {
                    Source::Caller
                } else {
                    Source::Application
                };
                self.lines.clear();
                self.visible.clear();
                self.matches.clear();
                self.selected = None;
                self.metadata = None;
                self.last_good = None;
                self.change = FileChange::None;
                self.generation = self.generation.wrapping_add(1);
                self.path.clear();
                self.error = None;
                self.loaded = false;
                self.top = 0;
                self.horizontal = 0;
                self.query.clear();
                return Action::Switch;
            }
            KeyCode::Char('f' | 'F') => {
                self.set_follow(!self.follow);
                if self.follow {
                    self.selected = None;
                    self.clamp();
                    return Action::Refresh;
                }
            }
            KeyCode::Char('t' | 'T') => {
                self.context = !self.context;
                self.selected = None;
                self.top = 0;
                self.rebuild();
                if self.context {
                    self.jump_match(false);
                }
            }
            KeyCode::Char('n') => self.jump_match(false),
            KeyCode::Char('N') => self.jump_match(true),
            KeyCode::Char('e' | 'E') if self.source == Source::Application => {
                self.warnings_only = !self.warnings_only;
                self.top = 0;
                self.rebuild();
            }
            KeyCode::Char('/') => self.editing = Some(self.query.clone()),
            KeyCode::Up | KeyCode::PageUp | KeyCode::Home => {
                self.set_follow(false);
                self.top = match key {
                    KeyCode::Home => 0,
                    KeyCode::PageUp => self.top.saturating_sub(self.page.max(1)),
                    _ => self.top.saturating_sub(1),
                };
            }
            KeyCode::Down | KeyCode::PageDown | KeyCode::End => {
                self.set_follow(false);
                self.top = match key {
                    KeyCode::End => self.max_top(),
                    KeyCode::PageDown => self.top.saturating_add(self.page.max(1)),
                    _ => self.top.saturating_add(1),
                };
            }
            KeyCode::Left => self.horizontal = self.horizontal.saturating_sub(4),
            KeyCode::Right => self.horizontal = self.horizontal.saturating_add(4).min((MAX_LINE_CHARS * 2) as u16),
            _ => {}
        }
        self.clamp();
        Action::Nothing
    }

    fn keys_id(&self) -> &'static str {
        match self.source {
            Source::Application => "log_view_keys",
            Source::Caller => "log_view_caller_keys",
            Source::File => "log_view_file_keys",
        }
    }

    fn metadata_text(&self) -> String {
        let Some(metadata) = &self.metadata else {
            return text("log_view_metadata_unavailable");
        };
        let change = match self.change {
            FileChange::None => String::new(),
            FileChange::Rotated => format!("{} | ", text("log_view_rotated")),
            FileChange::Truncated => format!("{} | ", text("log_view_truncated")),
        };
        format!(
            "{change}{}: {} B | {}: {}",
            text("log_view_size"),
            metadata.size,
            text("log_view_mtime"),
            metadata.modified_text(),
        )
    }

    fn ui(&mut self, frame: &mut Frame, full_screen: bool) {
        let theme = get_tui_theme();
        let area = get_screen_size(frame, full_screen);
        frame.render_widget(Clear, area);
        Block::new().style(theme.background).render(area, frame.buffer_mut());
        let block = Block::bordered()
            .title(Line::styled(format!(" {} ", text("log_view_title")), theme.dialog_box_title))
            .title_bottom(key_hint(text(self.keys_id())))
            .border_style(theme.dialog_box)
            .style(theme.background);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let [tabs, path, metadata, status, runtime, body, search, search_keys, bounds, keys] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(inner);
        let page = usize::from(body.height).max(1);
        if self.page != page {
            self.page = page;
            self.reveal_selection();
        }
        self.clamp();
        if self.source == Source::File {
            frame.render_widget(Line::styled(text("log_view_file"), theme.selected_item), tabs);
        } else {
            frame.render_widget(
                Line::from(vec![
                    Span::styled(
                        format!(" {} ", text("log_view_app")),
                        if self.source == Source::Application {
                            theme.selected_item
                        } else {
                            theme.value
                        },
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!(" {} ", text("log_view_caller")),
                        if self.source == Source::Caller { theme.selected_item } else { theme.value },
                    ),
                ]),
                tabs,
            );
        }
        frame.render_widget(Paragraph::new(display_text(&self.path.to_string_lossy())).style(theme.description_text), path);
        frame.render_widget(Paragraph::new(self.metadata_text()).style(theme.description_text), metadata);
        let first = if self.visible.is_empty() { 0 } else { self.top + 1 };
        let last = (self.top + self.page).min(self.visible.len());
        frame.render_widget(
            Paragraph::new(format!(
                "{} | {} | {first}-{last}/{}",
                text(if self.follow { "log_view_follow" } else { "log_view_paused" }),
                text(if self.warnings_only && self.source == Source::Application {
                    "log_view_warnings"
                } else {
                    "log_view_all"
                }),
                self.visible.len(),
            ))
            .style(theme.value),
            status,
        );
        frame.render_widget(
            Paragraph::new(display_text(&self.runtime.online_text().unwrap_or_default())).style(theme.description_text),
            runtime,
        );
        if let Some(error) = &self.error {
            frame.render_widget(
                Paragraph::new(format!("{}: {}", text("log_view_read_error"), error.message())).style(theme.false_value),
                body,
            );
        } else if self.visible.is_empty() {
            frame.render_widget(
                Paragraph::new(text(if self.loaded { "log_view_empty" } else { "log_view_loading" })).style(theme.description_text),
                body,
            );
        } else {
            let rows: Vec<_> = self
                .visible
                .iter()
                .skip(self.top)
                .take(self.page)
                .map(|&index| {
                    let line = &self.lines[index];
                    let style = if self.selected == Some(index) {
                        theme.selected_item
                    } else if self.matches.binary_search(&index).is_ok() {
                        theme.value.add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                    } else if line.warning {
                        theme.false_value
                    } else {
                        theme.value
                    };
                    Line::styled(line.text.as_str(), style)
                })
                .collect();
            // No wrapping: navigation is in physical lines, horizontal pan is
            // in display cells, and redraw touches only one viewport of text.
            frame.render_widget(Paragraph::new(rows).scroll((0, self.horizontal)), body);
        }
        let (label, query) = match &self.editing {
            Some(edit) => ("log_view_search_edit", edit.as_str()),
            None => (if self.context { "log_view_context" } else { "log_view_search" }, self.query.as_str()),
        };
        let selected = self
            .selected
            .and_then(|index| self.matches.binary_search(&index).ok())
            .map_or(0, |index| index + 1);
        frame.render_widget(
            Paragraph::new(format!(
                "{} {selected}/{} | {}: {}{}",
                text("log_view_matches"),
                self.matches.len(),
                text(label),
                display_text(query),
                if self.editing.is_some() { "▏" } else { "" }
            ))
            .style(if self.editing.is_some() {
                theme.selected_item
            } else {
                theme.description_text
            }),
            search,
        );
        if self.editing.is_none() {
            frame.render_widget(key_hint(text("log_view_mode_keys")), search_keys);
        }
        frame.render_widget(Paragraph::new(text("log_view_tail")).style(theme.description_text), bounds);
        frame.render_widget(
            key_hint(text(if self.editing.is_some() {
                "log_view_search_keys"
            } else {
                "log_view_navigation"
            })),
            keys,
        );
    }
}

/// Integrate as an operator-only CWS screen. The owner must resume its normal
/// scheduler/listener handshake after this returns, just as for event_screen.
pub(crate) async fn run<B: Backend>(terminal: &mut Terminal<B>, board: &Arc<Mutex<IcyBoard>>, bbs: &Arc<Mutex<BBS>>, full_screen: bool) -> Res<()>
where
    B::Error: Send + Sync + 'static,
{
    run_screen(terminal, board, bbs, full_screen, LogScreen::new()).await
}

/// Read one validated event-log path as UTF-8, without source or severity
/// switching. The event monitor owns validation beneath board.root/event_logs;
/// this viewer only enforces the same regular-file and bounded-read rules.
pub(crate) async fn run_file<B: Backend>(
    terminal: &mut Terminal<B>,
    board: &Arc<Mutex<IcyBoard>>,
    bbs: &Arc<Mutex<BBS>>,
    full_screen: bool,
    path: PathBuf,
) -> Res<()>
where
    B::Error: Send + Sync + 'static,
{
    run_screen(terminal, board, bbs, full_screen, LogScreen::file(path)).await
}

async fn run_screen<B: Backend>(
    terminal: &mut Terminal<B>,
    board: &Arc<Mutex<IcyBoard>>,
    bbs: &Arc<Mutex<BBS>>,
    full_screen: bool,
    mut screen: LogScreen,
) -> Res<()>
where
    B::Error: Send + Sync + 'static,
{
    let mut pending: Option<ReadJob> = None;
    let mut next_refresh = Instant::now();
    loop {
        screen.runtime.refresh(bbs);
        if screen.runtime.offline || screen.runtime.restart {
            return Ok(());
        }
        if pending.as_ref().is_some_and(|job| job.task.is_finished()) {
            let mut job = pending.take().unwrap();
            // Only await a completed handle. Slow IO never prevents runtime
            // refresh, keys, or return to the scheduler's owner.
            let result = (&mut job.task).await;
            if job.source == screen.source && job.generation == screen.generation {
                match result {
                    Ok(Some(snapshot)) => {
                        screen.accept(job.source, job.generation, snapshot);
                    }
                    Ok(None) => {} // Board busy; retry at the normal interval.
                    Err(error) => {
                        screen.accept(
                            job.source,
                            job.generation,
                            Snapshot {
                                path: screen.path.clone(),
                                result: Err(ReadFailure::Worker(error.to_string())),
                            },
                        );
                    }
                }
            } else {
                next_refresh = Instant::now();
            }
        }
        // At most one outstanding read, even if the operator switches tabs
        // while a filesystem is slow. Failed reads are UI state, never logged.
        if pending.is_none() && screen.wants_read() && Instant::now() >= next_refresh {
            pending = Some(ReadJob::start(board.clone(), screen.source, screen.generation, screen.fixed_path.clone()));
            next_refresh = Instant::now() + REFRESH_INTERVAL;
        }
        terminal.draw(|frame| screen.ui(frame, full_screen))?;
        if event::poll(POLL_INTERVAL)?
            && let Event::Key(key) = event::read()?
            && matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
        {
            match screen.handle_key(key.code) {
                Action::Exit => return Ok(()),
                Action::Switch | Action::Refresh => next_refresh = Instant::now(),
                Action::Nothing => {}
            }
        }
        tokio::task::yield_now().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use std::io::Write;

    fn sample(path: &Path, source: Source) -> Snapshot {
        Snapshot {
            path: path.to_path_buf(),
            result: read_sample(path, source),
        }
    }

    fn screen(count: usize) -> LogScreen {
        let mut screen = LogScreen::new();
        screen.page = 3;
        screen.apply(Snapshot {
            path: PathBuf::from("board.log"),
            result: Ok(Tail {
                lines: (0..count)
                    .map(|index| decode_line(format!("row {index}").as_bytes(), Source::Application))
                    .collect(),
                metadata: FileInfo::default(),
            }),
        });
        screen
    }

    #[test]
    fn regular_tail_is_bounded_and_keeps_newest_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("board.log");
        let content: String = (0..4000).map(|index| format!("{index:04} {}\n", "x".repeat(100))).collect();
        fs::write(&path, content).unwrap();
        let lines = read_tail(&path, Source::Application).unwrap();
        assert_eq!(lines.len(), MAX_LINES);
        assert!(lines.first().unwrap().text.starts_with("2000 "));
        assert!(lines.last().unwrap().text.starts_with("3999 "));
        assert_eq!(fs::metadata(&path).unwrap().len(), 4000 * 106, "viewer must not modify its input");
    }

    #[test]
    fn byte_cut_discards_partial_records_and_long_lines_are_clipped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("board.log");
        fs::write(&path, format!("web admin token: {}\nsafe\n", "s".repeat(MAX_BYTES))).unwrap();
        let lines = read_tail(&path, Source::Application).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "safe");
        fs::write(&path, "x".repeat(MAX_BYTES + 1)).unwrap();
        assert!(read_tail(&path, Source::Application).unwrap().is_empty());
        fs::write(&path, "ä".repeat(MAX_LINE_CHARS + 1)).unwrap();
        let lines = read_tail(&path, Source::Application).unwrap();
        assert_eq!(lines[0].text.chars().count(), MAX_LINE_CHARS);
        assert!(lines[0].text.ends_with('…'));
    }

    #[test]
    fn missing_empty_unconfigured_and_nonregular_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing.log");
        assert!(matches!(read_tail(&path, Source::Application), Err(ReadFailure::Io(error)) if error.kind() == io::ErrorKind::NotFound));
        assert!(matches!(read_tail(Path::new(""), Source::Caller), Err(ReadFailure::Unconfigured)));
        assert!(matches!(read_tail(dir.path(), Source::Caller), Err(ReadFailure::NotRegular)));
        fs::write(&path, "").unwrap();
        assert!(read_tail(&path, Source::Application).unwrap().is_empty());
        fs::write(&path, "a\r\n\nlast").unwrap();
        let lines = read_tail(&path, Source::Application).unwrap();
        assert_eq!(lines.iter().map(|line| line.text.as_str()).collect::<Vec<_>>(), ["a", "", "last"]);
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_and_sockets_are_rejected_before_open() {
        use std::os::unix::{fs::symlink, net::UnixListener};
        let dir = tempfile::tempdir().unwrap();
        let regular = dir.path().join("regular");
        fs::write(&regular, "safe").unwrap();
        let link = dir.path().join("link");
        symlink(&regular, &link).unwrap();
        assert!(matches!(read_tail(&link, Source::Application), Err(ReadFailure::NotRegular)));
        let socket = dir.path().join("socket");
        let _listener = UnixListener::bind(&socket).unwrap();
        assert!(matches!(read_tail(&socket, Source::Application), Err(ReadFailure::NotRegular)));
    }

    #[test]
    fn reopening_handles_truncation_rotation_and_recovery() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("board.log");
        fs::write(&path, "old\nmore\n").unwrap();
        assert_eq!(read_tail(&path, Source::Application).unwrap().len(), 2);
        fs::write(&path, "short\n").unwrap();
        assert_eq!(read_tail(&path, Source::Application).unwrap()[0].text, "short");
        fs::rename(&path, dir.path().join("rotated.log")).unwrap();
        assert!(read_tail(&path, Source::Application).is_err());
        fs::write(&path, "new\n").unwrap();
        assert_eq!(read_tail(&path, Source::Application).unwrap()[0].text, "new");
        let mut screen = screen(5);
        screen.apply(Snapshot {
            path: path.clone(),
            result: Err(ReadFailure::NotRegular),
        });
        assert!(screen.lines.is_empty() && screen.visible.is_empty() && screen.error.is_some());
        screen.apply(Snapshot {
            path: path.clone(),
            result: read_sample(&path, Source::Application),
        });
        assert!(screen.error.is_none());
        assert_eq!(screen.lines[0].text, "new");
    }

    #[tokio::test]
    async fn pending_completion_cannot_replace_paused_snapshot_and_resume_requires_fresh_read() {
        for completed_before_pause in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("board.log");
            fs::write(&path, "frozen row\n").unwrap();
            let mut screen = LogScreen::new();
            screen.apply(sample(&path, Source::Application));
            let metadata = screen.metadata.clone();
            let (sender, receiver) = tokio::sync::oneshot::channel();
            let mut job = ReadJob {
                source: screen.source,
                generation: screen.generation,
                task: tokio::spawn(async move { Some(receiver.await.unwrap()) }),
            };
            fs::write(&path, "replacement row\nanother row\n").unwrap();
            let snapshot = if completed_before_pause {
                assert!(sender.send(sample(&path, Source::Application)).is_ok());
                let snapshot = (&mut job.task).await.unwrap().unwrap();
                screen.handle_key(KeyCode::Char('F'));
                snapshot
            } else {
                screen.handle_key(KeyCode::Char('F'));
                assert!(sender.send(sample(&path, Source::Application)).is_ok());
                (&mut job.task).await.unwrap().unwrap()
            };
            assert!(!screen.wants_read());
            assert!(!screen.accept(job.source, job.generation, snapshot));
            assert_eq!(screen.lines[0].text, "frozen row");
            assert_eq!(screen.metadata, metadata);
            assert_eq!(screen.path, path);
            // A paused error cannot discard the frozen content or metadata.
            screen.apply(Snapshot {
                path: PathBuf::from("other.log"),
                result: Err(ReadFailure::NotRegular),
            });
            assert!(screen.error.is_none());
            assert_eq!(screen.metadata, metadata);
            assert_eq!(screen.path, path);
            assert!(matches!(screen.handle_key(KeyCode::Char('f')), Action::Refresh));
            assert!(screen.follow && screen.wants_read());
            assert!(!screen.accept(job.source, job.generation, sample(&path, Source::Application)));
            assert!(screen.accept(screen.source, screen.generation, sample(&path, Source::Application)));
            assert_eq!(screen.lines[0].text, "replacement row");
            assert_ne!(screen.metadata, metadata);
            assert_eq!(screen.top, screen.max_top());
        }
    }

    #[test]
    fn paused_source_switch_loads_once_and_rejects_old_source_generations() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("calls.log");
        fs::write(&path, "caller snapshot\n").unwrap();
        for initial_error in [false, true] {
            let mut screen = screen(4);
            let old_generation = screen.generation;
            screen.handle_key(KeyCode::Up);
            assert!(matches!(screen.handle_key(KeyCode::Tab), Action::Switch));
            assert!(!screen.follow && screen.wants_read());
            assert!(screen.metadata.is_none() && screen.last_good.is_none());
            assert!(!screen.accept(Source::Application, old_generation, sample(&path, Source::Application)));
            let snapshot = if initial_error {
                Snapshot {
                    path: path.clone(),
                    result: Err(ReadFailure::NotRegular),
                }
            } else {
                sample(&path, Source::Caller)
            };
            assert!(screen.accept(Source::Caller, screen.generation, snapshot));
            assert!(screen.loaded && !screen.wants_read());
            assert_eq!(screen.error.is_some(), initial_error);
            assert!(!screen.accept(Source::Caller, screen.generation, sample(&path, Source::Caller)));
            if !initial_error {
                assert_eq!(screen.lines[0].text, "caller snapshot");
            }
            screen.handle_key(KeyCode::Tab);
            assert_eq!(screen.source, Source::Application);
            assert!(!screen.accept(Source::Application, old_generation, sample(&path, Source::Application)));
            assert!(screen.accept(Source::Application, screen.generation, sample(&path, Source::Application)));
            assert!(!screen.wants_read());
        }
    }

    #[test]
    fn sampled_size_mtime_append_truncation_and_error_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("board.log");
        fs::write(&path, "first\n").unwrap();
        let modified = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        fs::File::open(&path).unwrap().set_modified(modified).unwrap();
        let mut screen = LogScreen::new();
        screen.apply(sample(&path, Source::Application));
        assert_eq!(screen.metadata.as_ref().unwrap().size, 6);
        assert_eq!(screen.metadata.as_ref().unwrap().modified, Some(modified));
        assert_eq!(screen.metadata.as_ref().unwrap().modified_text(), "2023-11-14 22:13:20");
        assert_eq!(screen.change, FileChange::None);
        OpenOptions::new().append(true).open(&path).unwrap().write_all(b"second\n").unwrap();
        screen.apply(sample(&path, Source::Application));
        assert_eq!(screen.lines.len(), 2);
        assert_eq!(screen.metadata.as_ref().unwrap().size, 13);
        assert_eq!(screen.change, FileChange::None);
        fs::write(&path, "x\n").unwrap();
        screen.apply(sample(&path, Source::Application));
        assert_eq!(screen.lines[0].text, "x");
        assert_eq!(screen.metadata.as_ref().unwrap().size, 2);
        assert_eq!(screen.change, FileChange::Truncated);
        screen.apply(sample(&path, Source::Application));
        assert_eq!(screen.change, FileChange::Truncated, "last change stays visible on later ticks");
        screen.apply(sample(&dir.path().join("missing"), Source::Application));
        assert!(screen.error.is_some() && screen.metadata.is_none() && screen.last_good.is_none());
        assert_eq!(screen.metadata_text(), text("log_view_metadata_unavailable"));
        assert!(screen.lines.is_empty() && screen.matches.is_empty());
        assert_eq!(screen.change, FileChange::None);
        assert_eq!(FileInfo::default().modified_text(), text("log_view_unknown"));
    }

    #[cfg(unix)]
    #[test]
    fn rotation_uses_descriptor_device_and_inode_even_with_equal_size_and_mtime() {
        use std::os::unix::fs::MetadataExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("board.log");
        fs::write(&path, "old\n").unwrap();
        let old = fs::File::open(&path).unwrap();
        let old_metadata = old.metadata().unwrap();
        let mut screen = LogScreen::new();
        screen.apply(sample(&path, Source::Application));
        assert_eq!(screen.metadata.as_ref().unwrap().identity, Some((old_metadata.dev(), old_metadata.ino())));
        fs::rename(&path, dir.path().join("rotated.log")).unwrap();
        screen.apply(sample(&path, Source::Application));
        assert!(screen.error.is_some() && screen.metadata.is_none());
        assert_eq!(screen.metadata_text(), text("log_view_metadata_unavailable"));
        fs::write(&path, "new\n").unwrap();
        let new = fs::File::open(&path).unwrap();
        new.set_modified(old_metadata.modified().unwrap()).unwrap();
        let new_metadata = new.metadata().unwrap();
        let tail = read_sample(&path, Source::Application).unwrap();
        assert_eq!(tail.metadata.identity, Some((new_metadata.dev(), new_metadata.ino())));
        assert_ne!(tail.metadata.identity, Some((old_metadata.dev(), old_metadata.ino())));
        assert_eq!(tail.metadata.size, old_metadata.len());
        assert_eq!(tail.metadata.modified, old_metadata.modified().ok());
        assert_eq!(
            FileInfo::from_metadata(&old.metadata().unwrap()).identity,
            Some((old_metadata.dev(), old_metadata.ino()))
        );
        screen.apply(Snapshot { path, result: Ok(tail) });
        assert_eq!(screen.change, FileChange::Rotated);
        assert_eq!(screen.lines[0].text, "new");
        assert!(screen.error.is_none());
    }

    #[test]
    fn only_actual_fern_warning_and_error_headers_match() {
        for level in ["WARN", "ERROR"] {
            assert!(is_warning(&format!("[2026-09-07 12:34:56 {level} icboard::connections] message")));
        }
        for line in [
            "[2026-09-07 12:34:56 INFO icboard] ERROR WARN warning error",
            "ERROR: not a fern record",
            " [2026-09-07 12:34:56 ERROR icboard] continuation",
            "[2026-99-99 99:99:99 ERROR icboard] bad timestamp",
            "[2026-09-07 12:34:56 ERROR ] no target",
            "[2026-09-07 12:34:56 warn icboard] wrong case",
            "[é] ERROR",
        ] {
            assert!(!is_warning(line), "{line}");
        }
        let mut screen = screen(0);
        screen.lines = ["[2026-09-07 12:34:56 ERROR icboard] actual", "[2026-09-07 12:34:56 INFO icboard] error text"]
            .iter()
            .map(|line| decode_line(line.as_bytes(), Source::Application))
            .collect();
        screen.handle_key(KeyCode::Char('E'));
        assert_eq!(screen.visible, [0]);
        screen.source = Source::Caller;
        screen.rebuild();
        screen.handle_key(KeyCode::Char('e'));
        assert_eq!(screen.visible, [0, 1], "caller content has no severity filter");
    }

    #[test]
    fn decoding_controls_ansi_and_token_redaction() {
        assert_eq!(decode_line(b"\x82\x81\x07\x1b[31m!", Source::Caller).text, "éü!");
        assert_eq!(decode_line("Grüße".as_bytes(), Source::Application).text, "Grüße");
        assert_eq!(decode_line("Grüße".as_bytes(), Source::Caller).text, "Grüße");
        assert_eq!(decode_line(b"bad\xff", Source::Application).text, "bad�");
        assert_eq!(sanitize("a\x1b[31mb\x1b[0m\x1b]52;c;hidden\x07c\x1bPpayload\x1b\\d\t\r\x08\u{202e}"), "abcd  ");
        assert_eq!(sanitize("a\u{009b}31mb\u{009d}hidden\u{009c}c"), "abc");
        for value in [
            "[2026-09-07 12:34:56 INFO icboard::connections] web admin token: SECRET".to_string(),
            "web admin \x1b[31mtoken: SECRET".to_string(),
            format!("{}web admin token: SECRET", "x".repeat(MAX_LINE_CHARS + 1)),
        ] {
            assert_eq!(display_text(&value), text("log_view_redacted"));
            assert!(!display_text(&value).contains("SECRET"));
        }
        let mut screen = screen(0);
        screen.lines.push(decode_line(b"web admin token: SECRET", Source::Application));
        screen.query = "secret".into();
        screen.rebuild();
        assert!(screen.visible.is_empty(), "search must not reveal redacted values");
    }

    #[test]
    fn navigation_follow_resize_and_tab() {
        let mut screen = screen(10);
        assert_eq!(screen.top, 7);
        screen.handle_key(KeyCode::Up);
        assert_eq!(screen.top, 6);
        assert!(!screen.follow);
        screen.handle_key(KeyCode::PageUp);
        assert_eq!(screen.top, 3);
        screen.handle_key(KeyCode::Home);
        assert_eq!(screen.top, 0);
        screen.handle_key(KeyCode::Down);
        screen.handle_key(KeyCode::PageDown);
        assert_eq!(screen.top, 4);
        screen.handle_key(KeyCode::End);
        assert_eq!(screen.top, 7);
        screen.handle_key(KeyCode::Down);
        assert_eq!(screen.top, 7);
        screen.handle_key(KeyCode::Right);
        assert_eq!(screen.horizontal, 4);
        screen.handle_key(KeyCode::Left);
        screen.handle_key(KeyCode::Left);
        assert_eq!(screen.horizontal, 0);
        screen.handle_key(KeyCode::Char('F'));
        assert!(screen.follow);
        screen.lines.push(decode_line(b"new row", Source::Application));
        screen.rebuild();
        assert_eq!(screen.top, 8);
        screen.page = 50;
        screen.clamp();
        assert_eq!(screen.top, 0);
        assert!(matches!(screen.handle_key(KeyCode::Tab), Action::Switch));
        assert_eq!(screen.source, Source::Caller);
        assert!(screen.lines.is_empty() && screen.visible.is_empty() && !screen.loaded);
        assert!(matches!(screen.handle_key(KeyCode::Esc), Action::Exit));
    }

    #[test]
    fn search_commits_cancels_and_limits_input() {
        let mut screen = screen(10);
        screen.handle_key(KeyCode::Char('/'));
        for ch in "ROW 1".chars() {
            screen.handle_key(KeyCode::Char(ch));
        }
        assert_eq!(screen.visible.len(), 10, "editing does not change the committed filter");
        screen.handle_key(KeyCode::Enter);
        assert_eq!(screen.visible, [1]);
        screen.handle_key(KeyCode::Char('/'));
        screen.handle_key(KeyCode::Backspace);
        assert!(matches!(screen.handle_key(KeyCode::Esc), Action::Nothing));
        assert_eq!(screen.query, "ROW 1");
        assert_eq!(screen.visible, [1]);
        screen.handle_key(KeyCode::Char('/'));
        for _ in 0..5 {
            screen.handle_key(KeyCode::Backspace);
        }
        screen.handle_key(KeyCode::Enter);
        assert_eq!(screen.visible.len(), 10);
        screen.handle_key(KeyCode::Char('/'));
        for _ in 0..MAX_QUERY_CHARS + 50 {
            screen.handle_key(KeyCode::Char('ü'));
        }
        assert_eq!(screen.editing.as_ref().unwrap().chars().count(), MAX_QUERY_CHARS);
        screen.handle_key(KeyCode::Esc);
        assert!(matches!(screen.handle_key(KeyCode::Esc), Action::Exit));
    }

    #[test]
    fn context_search_preserves_surroundings_and_wraps_first_last_unicode_matches() {
        let mut screen = screen(0);
        screen.lines = [
            "ÄPFEL first",
            "before middle",
            "more context",
            "äpfel middle",
            "after middle",
            "before last",
            "Äpfel last",
        ]
        .iter()
        .map(|line| decode_line(line.as_bytes(), Source::Application))
        .collect();
        screen.rebuild();
        screen.handle_key(KeyCode::Char('/'));
        for ch in "Äpfel".chars() {
            screen.handle_key(KeyCode::Char(ch));
        }
        assert_eq!(screen.visible.len(), 7, "draft must not change the committed search");
        screen.handle_key(KeyCode::Enter);
        assert_eq!(screen.visible, [0, 3, 6]);
        assert_eq!(screen.matches, [0, 3, 6]);
        screen.handle_key(KeyCode::Char('T'));
        assert!(screen.context && !screen.follow);
        assert_eq!(screen.query, "Äpfel");
        assert_eq!(screen.visible, [0, 1, 2, 3, 4, 5, 6]);
        assert_eq!(screen.selected, Some(0));
        assert_eq!(screen.top, 0);
        screen.handle_key(KeyCode::Char('N'));
        assert_eq!(screen.selected, Some(6));
        assert_eq!(screen.top, 4);
        screen.handle_key(KeyCode::Char('n'));
        assert_eq!(screen.selected, Some(0));
        screen.handle_key(KeyCode::Char('n'));
        assert_eq!(screen.selected, Some(3));
        assert_eq!(screen.top, 2);
        screen.handle_key(KeyCode::Char('t'));
        assert!(!screen.context);
        assert_eq!(screen.visible, [0, 3, 6]);
        screen.handle_key(KeyCode::Char('N'));
        assert_eq!(screen.selected, Some(6), "without a selection N starts at the last match");
        screen.handle_key(KeyCode::Char('n'));
        assert_eq!(screen.selected, Some(0), "match navigation also works in filter mode");

        // Unicode lowercase expansion changes byte lengths: no byte indexing
        // into the original text is used for selection or highlighting.
        screen.lines = vec![decode_line("İSTANBUL".as_bytes(), Source::Application)];
        screen.query = "i\u{0307}stanbul".into();
        screen.selected = None;
        screen.rebuild();
        screen.handle_key(KeyCode::Char('T'));
        assert_eq!(screen.selected, Some(0));
        screen.handle_key(KeyCode::Char('n'));
        screen.handle_key(KeyCode::Char('N'));
        assert_eq!(screen.selected, Some(0), "single match wraps to itself");
        screen.query = "no match".into();
        screen.rebuild();
        screen.handle_key(KeyCode::Char('n'));
        screen.handle_key(KeyCode::Char('N'));
        assert_eq!(screen.selected, None);
        assert_eq!(screen.visible, [0], "no-match context still shows surrounding content");
        screen.query.clear();
        screen.rebuild();
        assert!(screen.matches.is_empty(), "empty search is not a match on every row");
    }

    #[test]
    fn context_search_commit_cancel_redaction_and_clipping() {
        let mut screen = screen(0);
        screen.lines = vec![
            decode_line(b"safe first", Source::Application),
            decode_line(b"web admin \x1b[31mtoken: SECRET", Source::Application),
            decode_line(format!("{}HIDDEN_SUFFIX", "x".repeat(MAX_LINE_CHARS)).as_bytes(), Source::Application),
            decode_line(b"safe last", Source::Application),
        ];
        screen.handle_key(KeyCode::Char('T'));
        for query in ["secret", "web admin", &text("log_view_redacted"), "HIDDEN_SUFFIX"] {
            screen.query = query.into();
            screen.rebuild();
            assert!(screen.matches.is_empty(), "hidden data and placeholders must not be searchable");
            assert_eq!(screen.visible, [0, 1, 2, 3]);
        }
        screen.query.clear();
        screen.handle_key(KeyCode::Char('/'));
        for ch in "SAFE".chars() {
            screen.handle_key(KeyCode::Char(ch));
        }
        screen.handle_key(KeyCode::Enter);
        assert_eq!(screen.matches, [0, 3]);
        assert_eq!(screen.selected, Some(0));
        assert!(!screen.follow);
        screen.handle_key(KeyCode::Char('/'));
        for ch in "TnNEF".chars() {
            screen.handle_key(KeyCode::Char(ch));
        }
        screen.handle_key(KeyCode::Tab);
        screen.handle_key(KeyCode::Esc);
        assert!(screen.context && !screen.follow && !screen.warnings_only);
        assert_eq!(screen.source, Source::Application);
        assert_eq!(screen.query, "SAFE", "editing treats command letters as query input; Esc cancels");
        assert_eq!(screen.matches, [0, 3]);
    }

    #[tokio::test]
    async fn fixed_file_disables_tab_and_severity_and_reads_utf8_without_board_lock() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("event.log");
        fs::write(&path, b"[2026-09-07 12:34:56 ERROR command] text\nDOS byte \x82\n\x1b[31msafe\x1b[0m\n").unwrap();
        let board = Arc::new(Mutex::new(IcyBoard::new()));
        let _guard = board.lock().await;
        let mut screen = LogScreen::file(path.clone());
        let mut job = ReadJob::start(board.clone(), screen.source, screen.generation, screen.fixed_path.clone());
        let snapshot = (&mut job.task).await.unwrap().unwrap();
        assert!(screen.accept(job.source, job.generation, snapshot));
        assert_eq!(screen.lines[1].text, "DOS byte �", "fixed files never fall back to CP437");
        assert_eq!(screen.lines[2].text, "safe");
        assert!(screen.lines.iter().all(|line| !line.warning), "fixed files are not fern severity records");
        let generation = screen.generation;
        screen.query = "safe".into();
        screen.rebuild();
        assert!(matches!(screen.handle_key(KeyCode::Tab), Action::Nothing));
        screen.handle_key(KeyCode::Char('E'));
        screen.handle_key(KeyCode::Char('e'));
        assert_eq!(screen.source, Source::File);
        assert_eq!(screen.fixed_path, Some(path.clone()));
        assert_eq!(screen.path, path);
        assert_eq!(screen.generation, generation);
        assert!(!screen.warnings_only);
        assert_eq!(screen.query, "safe");
        assert_eq!(screen.visible, [2]);
        assert_eq!(screen.keys_id(), "log_view_file_keys");
        screen.handle_key(KeyCode::Char('T'));
        assert!(!screen.follow && screen.context);
        screen.handle_key(KeyCode::Tab);
        assert!(screen.loaded && !screen.wants_read());
        assert_eq!(screen.visible, [0, 1, 2]);
        assert!(matches!(screen.handle_key(KeyCode::Char('F')), Action::Refresh));
        assert!(matches!(screen.handle_key(KeyCode::Esc), Action::Exit));
    }

    #[test]
    fn context_and_fixed_file_metadata_errors_render_at_normal_and_small_sizes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("event.log");
        fs::write(&path, "before context\nÄPFEL first\nbetween context\näpfel last\nafter context\n").unwrap();
        fs::File::open(&path)
            .unwrap()
            .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000))
            .unwrap();
        for (width, height) in [(80, 25), (40, 10), (12, 5), (1, 1)] {
            for full_screen in [false, true] {
                let mut screen = LogScreen::file(path.clone());
                screen.apply(sample(&path, Source::File));
                screen.query = "Äpfel".into();
                screen.handle_key(KeyCode::Char('T'));
                let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                terminal.draw(|frame| screen.ui(frame, full_screen)).unwrap();
                if width == 80 && height == 25 {
                    let buffer = terminal.backend().buffer();
                    let rendered = buffer.content().iter().map(|cell| cell.symbol()).collect::<String>();
                    for expected in [
                        "before context",
                        "ÄPFEL first",
                        "between context",
                        "äpfel last",
                        "after context",
                        "2023-11-14 22:13:20",
                        &text("log_view_paused"),
                        &text("log_view_context"),
                        &text("log_view_size"),
                        &text("log_view_mtime"),
                        &text("log_view_file_keys"),
                        &text("log_view_mode_keys"),
                    ] {
                        assert!(rendered.contains(expected), "missing {expected:?}");
                    }
                    let selected = buffer.content().iter().find(|cell| cell.symbol() == "Ä").unwrap();
                    assert_eq!(selected.fg, get_tui_theme().selected_item.fg.unwrap());
                    assert_eq!(selected.bg, get_tui_theme().selected_item.bg.unwrap());
                    let row = buffer
                        .content()
                        .chunks(usize::from(width))
                        .find(|row| row.iter().map(|cell| cell.symbol()).collect::<String>().contains("äpfel last"))
                        .unwrap();
                    let other_match = row.iter().find(|cell| cell.symbol() == "ä").unwrap();
                    assert!(other_match.modifier.contains(Modifier::BOLD | Modifier::UNDERLINED));
                }
                screen.handle_key(KeyCode::Char('N'));
                terminal.draw(|frame| screen.ui(frame, full_screen)).unwrap();
                assert_eq!(screen.selected, Some(3));
                if height == 25 {
                    assert!(screen.top <= 3 && screen.top + screen.page > 3);
                }
                screen.handle_key(KeyCode::Char('F'));
                screen.apply(Snapshot {
                    path: PathBuf::from("event\x1b[31m.log"),
                    result: Err(ReadFailure::Io(io::Error::other("broken\x1b[31m\x07\u{202e}"))),
                });
                assert!(screen.metadata.is_none() && screen.lines.is_empty());
                terminal.draw(|frame| screen.ui(frame, full_screen)).unwrap();
                let rendered = terminal.backend().buffer().content().iter().map(|cell| cell.symbol()).collect::<String>();
                assert!(!rendered.contains('\x1b') && !rendered.contains('\x07') && !rendered.contains('\u{202e}'));
                assert!(!rendered.contains("2023-11-14"), "old mtime must not survive a read error");
                if width == 80 {
                    assert!(rendered.contains(&text("log_view_metadata_unavailable")));
                    assert!(rendered.contains(&text("log_view_read_error")) && rendered.contains("broken"));
                }
            }
        }
    }

    #[test]
    fn rendering_at_80x25_and_tiny_sizes_is_bounded() {
        for (width, height) in [(80, 25), (40, 10), (1, 1), (120, 40)] {
            let mut screen = screen(100);
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| screen.ui(frame, false)).unwrap();
            if width >= 80 && height >= 25 {
                let rendered = terminal.backend().buffer().content().iter().map(|cell| cell.symbol()).collect::<String>();
                assert!(rendered.contains("row 99"));
                assert!(rendered.contains(&text("log_view_title")));
                assert!(rendered.contains(&text("log_view_keys")));
                assert!(rendered.contains(&text("log_view_navigation")));
            }
            screen.handle_key(KeyCode::Char('/'));
            terminal.draw(|frame| screen.ui(frame, true)).unwrap();
            screen.error = Some(ReadFailure::NotRegular);
            terminal.draw(|frame| screen.ui(frame, false)).unwrap();
        }
    }

    #[tokio::test]
    async fn offline_and_restart_return_without_reading_or_waiting_for_keys() {
        for restart in [false, true] {
            let board = Arc::new(Mutex::new(IcyBoard::new()));
            let mut state = BBS::new(1);
            state.event_restart_requested = restart;
            state.event_maintenance = !restart;
            let bbs = Arc::new(Mutex::new(state));
            let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
            run(&mut terminal, &board, &bbs, false).await.unwrap();
            run_file(&mut terminal, &board, &bbs, false, PathBuf::from("missing-event.log")).await.unwrap();
        }
    }

    #[tokio::test]
    async fn worker_uses_actual_config_log_and_resolved_caller_paths() {
        let dir = tempfile::tempdir().unwrap();
        let mut board = IcyBoard::new();
        board.root_path = dir.path().to_path_buf();
        board.file_name = dir.path().join("custom.toml");
        board.config.paths.caller_log = PathBuf::from("calls.log");
        fs::write(board.file_name.with_extension("log"), "application\n").unwrap();
        fs::write(dir.path().join("calls.log"), b"caller \x82\n").unwrap();
        let board = Arc::new(Mutex::new(board));
        for (source, filename, expected) in [(Source::Application, "custom.log", "application"), (Source::Caller, "calls.log", "caller é")] {
            let mut job = ReadJob::start(board.clone(), source, 0, None);
            let snapshot = (&mut job.task).await.unwrap().unwrap();
            assert_eq!(snapshot.path, dir.path().join(filename));
            assert_eq!(snapshot.result.unwrap().lines[0].text, expected);
        }
        let _guard = board.lock().await;
        let mut job = ReadJob::start(board.clone(), Source::Caller, 0, None);
        assert!((&mut job.task).await.unwrap().is_none(), "worker must not wait for the board lock");
    }
}
