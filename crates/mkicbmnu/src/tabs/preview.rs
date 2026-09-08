//! Safe, static menu preview. There is deliberately no IcyBoardState, executor,
//! subprocess, keyboard stuffing, accounting, or writable file handle here.
//!
//! The background uses icy_engine's in-memory ANSI/PCBoard text import. The
//! palette/cell rendering follows PositionEditor, with a clipped, pannable
//! 80x25 viewport instead of unsafe centering arithmetic. Cells are NOT scaled.
//! PCB @X colors in menu strings use the shared PCB line renderer. Other PCB
//! macros and embedded runtime commands are not expanded; control characters
//! in labels/traces are made inert. The prompt has its own row, not the live
//! runtime caret position. Secondary prompts are shown only in the inspector.
//!
//! request_status marks tab entry for refresh; a snapshot comparison also catches
//! edits without that hook. Disk reads happen only on entry, changed inputs, or
//! F5, never on each unchanged frame. Background reads are bounded to 1 MiB and
//! restricted to regular files and text formats (no PPE/archive/image loading).

use std::{
    cell::Cell,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use icy_board_engine::{
    icy_board::{
        IcyBoard,
        commands::{ActionTrigger, Command, CommandType},
        menu::{Menu, MenuType},
        state::functions::MASK_COMMAND,
    },
    tokens::tokenize,
};
use icy_board_tui::{
    config_menu::{EditMessage, ResultState},
    get_text,
    hotkeys::{Hotkey, HotkeyBar},
    pcb_line::get_styled_pcb_line,
    tab_page::TabPage,
    theme::get_tui_theme,
};
use icy_engine::{AttributeColor, FileFormat, LoadData, TextBuffer, TextPane};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, ListState, Paragraph, Widget},
};

use crate::validation::{IssueSeverity, MenuIssue, PreviewAccess, preview_access, resolve_display_file, validate_menu};

const WIDTH: u16 = 80;
const HEIGHT: u16 = 25;
const MAX_BACKGROUND_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Panel {
    Details,
    Trace,
    Issues,
}

pub struct PreviewTab {
    board: Arc<Mutex<IcyBoard>>,
    menu: Arc<Mutex<Menu>>,
    snapshot: Option<Menu>,
    root: PathBuf,
    reload: Cell<bool>,
    security: u8,
    selected: Option<usize>,
    access: Vec<PreviewAccess>,
    background: TextBuffer,
    background_path: PathBuf,
    background_error: Option<String>,
    state_error: Option<String>,
    issues: Vec<MenuIssue>,
    issue_state: ListState,
    reveal_issue: bool,
    panel: Panel,
    scroll: usize,
    trace: Vec<String>,
    input: String,
    normal_only: bool,
    view_x: u16,
    view_y: u16,
    manual_view: bool,
}

fn inert(text: &str) -> String {
    text.chars().map(|ch| if ch.is_control() { '\u{fffd}' } else { ch }).collect()
}

fn access_text(access: PreviewAccess) -> String {
    get_text(match access {
        PreviewAccess::Allowed => "mnu_preview_allowed",
        PreviewAccess::Denied => "mnu_preview_denied",
        PreviewAccess::Unknown => "mnu_preview_unknown",
        PreviewAccess::Invalid => "mnu_preview_invalid",
    })
}

// Menu derives PartialEq, but NaN fees would invalidate the disk cache on every
// frame. Compare fee bit patterns separately and normalize the cloned values.
fn same_menu(left: &Menu, right: &Menu) -> bool {
    if left.commands.len() != right.commands.len() {
        return false;
    }
    if left
        .commands
        .iter()
        .zip(&right.commands)
        .any(|(a, b)| a.charge_per_use.to_bits() != b.charge_per_use.to_bits() || a.charge_per_minute.to_bits() != b.charge_per_minute.to_bits())
    {
        return false;
    }
    let mut left = left.clone();
    let mut right = right.clone();
    for command in left.commands.iter_mut().chain(right.commands.iter_mut()) {
        command.charge_per_use = 0.0;
        command.charge_per_minute = 0.0;
    }
    left == right
}

fn decode_background(path: &Path, bytes: &[u8]) -> Result<TextBuffer, String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let format = match ext.as_str() {
        "pcb" => FileFormat::PCBoard,
        "ans" | "asc" | "txt" | "diz" | "nfo" | "" => {
            // Extensionless/nonstandard PCBoard display files are common.
            // If @X appears, prefer the PCBoard parser. Mixed ANSI/PCB runtime
            // semantics are not guaranteed by this static file importer.
            if bytes.windows(2).any(|w| w.eq_ignore_ascii_case(b"@X")) {
                FileFormat::PCBoard
            } else {
                FileFormat::Ansi
            }
        }
        "avt" => FileFormat::Avatar,
        // An explicitly named .TOP is still a text display, but do not pass
        // known executable/binary formats to FileFormat's general loader.
        "ppe" | "pps" | "mnu" | "rip" | "icy" | "icyanim" | "bin" | "exe" | "com" | "zip" | "png" | "jpg" => {
            return Err(get_text("mnu_preview_format"));
        }
        _ => {
            if bytes.windows(2).any(|w| w.eq_ignore_ascii_case(b"@X")) {
                FileFormat::PCBoard
            } else {
                FileFormat::Ansi
            }
        }
    };
    let options = LoadData::new(None, Some(usize::from(WIDTH))).with_max_height(i32::from(HEIGHT));
    format
        .from_bytes(bytes, Some(options))
        .map(|document| document.screen.buffer)
        .map_err(|err| err.to_string())
}

pub(crate) fn load_background(path: &Path) -> Result<TextBuffer, String> {
    let metadata = std::fs::metadata(path).map_err(|err| err.to_string())?;
    if !metadata.is_file() {
        return Err(get_text("mnu_preview_regular_file"));
    }
    if metadata.len() > MAX_BACKGROUND_BYTES {
        return Err(get_text("mnu_preview_file_limit"));
    }
    let file = File::open(path).map_err(|err| err.to_string())?;
    let mut bytes = Vec::new();
    file.take(MAX_BACKGROUND_BYTES + 1).read_to_end(&mut bytes).map_err(|err| err.to_string())?;
    if bytes.len() as u64 > MAX_BACKGROUND_BYTES {
        return Err(get_text("mnu_preview_file_limit"));
    }
    decode_background(path, &bytes)
}

/// Static action description only: no executable capability is accepted.
fn action_trace(command: &Command, access: PreviewAccess, activation: bool) -> Vec<String> {
    let mut lines = vec![
        get_text("mnu_preview_read_only"),
        format!("{}: {}", get_text("mnu_preview_security"), access_text(access)),
        format!("{}: {:?} / {}", get_text("mnu_preview_autorun"), command.auto_run, command.autorun_time),
        format!("{}: {} / {}", get_text("mnu_preview_fees"), command.charge_per_use, command.charge_per_minute),
        get_text("mnu_preview_selection_note"),
        get_text("mnu_preview_trace_note"),
    ];
    if command.actions.is_empty() {
        lines.push(get_text("mnu_check_no_actions"));
    }
    for (i, action) in command.actions.iter().enumerate() {
        let trigger = match action.trigger {
            ActionTrigger::Activation => get_text("mnu_preview_activation"),
            ActionTrigger::Selection => get_text("mnu_preview_selection"),
        };
        let disposition = if matches!(action.command_type, CommandType::Disabled | CommandType::DisableMenuOption) {
            get_text("mnu_preview_disabled")
        } else if action.trigger == ActionTrigger::Selection {
            get_text("mnu_preview_selection_only")
        } else if !activation {
            get_text("mnu_preview_not_activated")
        } else {
            access_text(access)
        };
        lines.push(format!(
            "{}. {trigger} / {:?} / {disposition}: {}",
            i + 1,
            action.command_type,
            inert(&action.parameter)
        ));
    }
    lines
}

/// Wrap by terminal-cell width, not bytes; never feed escape/control characters
/// from a menu or error message back to the user's terminal.
fn wrapped(lines: &[String], width: u16) -> Vec<Line<'static>> {
    let width = usize::from(width.max(1));
    let mut result = Vec::new();
    for line in lines {
        let mut text = String::new();
        let mut used = 0;
        for ch in inert(line).chars() {
            let cell_width = Span::raw(ch.to_string()).width();
            if used + cell_width > width && !text.is_empty() {
                result.push(Line::from(std::mem::take(&mut text)));
                used = 0;
            }
            text.push(ch);
            used += cell_width;
        }
        result.push(Line::from(text));
    }
    result
}

impl PreviewTab {
    pub fn new(board: Arc<Mutex<IcyBoard>>, menu: Arc<Mutex<Menu>>) -> Self {
        Self {
            board,
            menu,
            snapshot: None,
            root: PathBuf::new(),
            reload: Cell::new(true),
            security: 0,
            selected: None,
            access: Vec::new(),
            background: TextBuffer::new((i32::from(WIDTH), i32::from(HEIGHT))),
            background_path: PathBuf::new(),
            background_error: None,
            state_error: None,
            issues: Vec::new(),
            issue_state: ListState::default(),
            reveal_issue: true,
            panel: Panel::Details,
            scroll: 0,
            trace: Vec::new(),
            input: String::new(),
            normal_only: false,
            view_x: 0,
            view_y: 0,
            manual_view: false,
        }
    }

    fn refresh(&mut self) {
        // Never hold both shared locks at once and never obtain a mutable guard.
        let menu = match self.menu.lock() {
            Ok(menu) => menu.clone(),
            Err(_) => {
                self.state_error = Some(get_text("mnu_preview_lock_error"));
                return;
            }
        };
        let board = match self.board.lock() {
            Ok(board) => board,
            Err(_) => {
                self.state_error = Some(get_text("mnu_preview_lock_error"));
                return;
            }
        };
        let changed = self.snapshot.as_ref().is_none_or(|old| !same_menu(old, &menu));
        let reload = self.reload.replace(false) || self.root != board.root_path;
        self.state_error = None;
        if !changed && !reload {
            return;
        }
        let background_changed = self.snapshot.as_ref().is_none_or(|old| old.display_file != menu.display_file);
        self.root = board.root_path.clone();
        self.issues = validate_menu(&board, &menu);
        self.access = menu.commands.iter().map(|cmd| preview_access(&cmd.security, self.security)).collect();
        self.selected = if menu.commands.is_empty() {
            None
        } else {
            Some(self.selected.unwrap_or(0).min(menu.commands.len() - 1))
        };
        self.issue_state.select(if self.issues.is_empty() {
            None
        } else {
            Some(self.issue_state.selected().unwrap_or(0).min(self.issues.len() - 1))
        });
        self.reveal_issue = true;
        if reload || background_changed {
            self.background_path = resolve_display_file(&board, &menu.display_file, self.security);
            self.background_error = None;
            self.background = TextBuffer::new((i32::from(WIDTH), i32::from(HEIGHT)));
            if !self.background_path.as_os_str().is_empty() {
                match load_background(&self.background_path) {
                    Ok(background) => self.background = background,
                    Err(error) => {
                        self.background_error = Some(format!(
                            "{}: {}: {}",
                            get_text("mnu_preview_load_error"),
                            self.background_path.display(),
                            inert(&error)
                        ))
                    }
                }
            }
        }
        drop(board);
        self.snapshot = Some(menu);
        // Never leave a stale plan visible after edits, security changes or entry.
        self.trace.clear();
        self.scroll = 0;
        self.input.clear();
    }

    fn select(&mut self, index: usize) {
        let Some(menu) = &self.snapshot else { return };
        let Some(command) = menu.commands.get(index) else { return };
        self.manual_view = false;
        if self.selected != Some(index) {
            self.selected = Some(index);
            self.trace = action_trace(command, self.access[index], false);
            self.scroll = 0;
        }
    }

    fn submit(&mut self) {
        let Some(menu) = &self.snapshot else { return };
        let typed = !self.input.is_empty();
        let index = if typed {
            let tokens = tokenize(&self.input);
            tokens
                .first()
                .and_then(|token| menu.commands.iter().position(|cmd| cmd.keyword.eq_ignore_ascii_case(token)))
        } else {
            self.selected
        };
        self.panel = Panel::Trace;
        self.scroll = 0;
        self.trace = if let Some(index) = index {
            self.selected = Some(index);
            self.manual_view = false;
            let command = &menu.commands[index];
            action_trace(command, self.access[index], typed || !command.lighbar_display.is_empty())
        } else {
            vec![get_text("mnu_preview_read_only"), get_text("mnu_preview_no_match")]
        };
        self.input.clear();
    }

    fn details(&self) -> Vec<String> {
        let Some(menu) = &self.snapshot else { return Vec::new() };
        let mut lines = vec![
            get_text("mnu_preview_scope"),
            get_text("mnu_preview_render_note"),
            format!("{}: {}", get_text("mnu_preview_background"), self.background_path.display()),
            format!("{}: {:?}", get_text("mnu_preview_type"), menu.menu_type),
        ];
        if let Some(index) = self.selected
            && let Some(command) = menu.commands.get(index)
        {
            lines.insert(
                0,
                format!(
                    "{} #{}: {} / {}",
                    get_text("mnu_preview_entry"),
                    index + 1,
                    inert(&command.keyword),
                    access_text(self.access[index])
                ),
            );
            lines.insert(1, format!("{}: {}", get_text("mnu_preview_normal"), inert(&command.display)));
            lines.insert(2, format!("{}: {}", get_text("mnu_preview_highlight"), inert(&command.lighbar_display)));
            lines.push(format!(
                "{}: {} / {}",
                get_text("mnu_preview_security"),
                command.security,
                access_text(self.access[index])
            ));
        }
        lines.push(format!("{}: {}", get_text("mnu_preview_prompt"), inert(&menu.prompt)));
        for (condition, prompt) in &menu.prompts {
            lines.push(format!("{}: {} / {}", get_text("mnu_preview_secondary"), inert(condition), inert(prompt)));
        }
        for (i, command) in menu.commands.iter().enumerate() {
            lines.push(format!(
                "#{} {} / {} / {:?}",
                i + 1,
                inert(&command.keyword),
                access_text(self.access[i]),
                command.auto_run
            ));
        }
        // The header is clipped on small screens; the inspector preserves the
        // complete, scrollable load/lock error and resolved filename.
        if let Some(error) = self.state_error.as_ref().or(self.background_error.as_ref()) {
            lines.insert(0, error.clone());
        }
        lines
    }

    fn canvas(&self) -> Buffer {
        let mut canvas = Buffer::empty(Rect::new(0, 0, WIDTH, HEIGHT));
        // Same palette/cell conversion as PositionEditor; do not invoke its UI
        // because it assumes an unclipped viewport and always sets a cursor.
        for y in 0..HEIGHT.min(self.background.height().max(0) as u16) {
            for x in 0..WIDTH.min(self.background.width().max(0) as u16) {
                let ch = self.background.char_at((i32::from(x), i32::from(y)).into());
                let color = |color: AttributeColor, bold: bool| match color {
                    AttributeColor::Palette(index) => {
                        let index = if bold && index < 8 { index + 8 } else { index };
                        let rgb = self.background.palette.rgb(u32::from(index));
                        Color::Rgb(rgb.0, rgb.1, rgb.2)
                    }
                    AttributeColor::ExtendedPalette(index) => Color::Indexed(index),
                    AttributeColor::Rgb(r, g, b) => Color::Rgb(r, g, b),
                    AttributeColor::Transparent => Color::Reset,
                };
                let mut style = Style::default()
                    .fg(color(ch.attribute.foreground_color(), ch.attribute.is_bold()))
                    .bg(color(ch.attribute.background_color(), false));
                if ch.attribute.is_blinking() {
                    style = style.add_modifier(Modifier::SLOW_BLINK);
                }
                let unicode = self.background.buffer_type.convert_to_unicode(ch.ch);
                canvas[(x, y)].set_symbol(&inert(&unicode.to_string())).set_style(style);
            }
        }
        if let Some(menu) = &self.snapshot {
            for (i, command) in menu.commands.iter().enumerate() {
                if command.position.x >= WIDTH || command.position.y >= HEIGHT {
                    continue;
                }
                let selected = self.selected == Some(i) && !self.normal_only;
                let text = if selected && !command.lighbar_display.is_empty() {
                    &command.lighbar_display
                } else {
                    &command.display
                };
                let mut line = get_styled_pcb_line(&inert(text));
                if self.access[i] != PreviewAccess::Allowed {
                    line = line.patch_style(Style::default().add_modifier(Modifier::DIM));
                }
                if selected {
                    line = line.patch_style(Style::default().add_modifier(Modifier::UNDERLINED));
                }
                line.render(Rect::new(command.position.x, command.position.y, WIDTH - command.position.x, 1), &mut canvas);
            }
        }
        canvas
    }

    fn render_canvas(&mut self, frame: &mut Frame, area: Rect) {
        let width = area.width.min(WIDTH);
        let height = area.height.min(HEIGHT);
        if width == 0 || height == 0 {
            return;
        }
        if !self.manual_view
            && let Some(menu) = &self.snapshot
            && let Some(index) = self.selected
        {
            let pos = menu.commands[index].position;
            self.view_x = follow(self.view_x, pos.x.min(WIDTH - 1), width, WIDTH);
            self.view_y = follow(self.view_y, pos.y.min(HEIGHT - 1), height, HEIGHT);
        }
        self.view_x = self.view_x.min(WIDTH - width);
        self.view_y = self.view_y.min(HEIGHT - height);
        let canvas = self.canvas();
        let left = area.x + (area.width - width) / 2;
        for y in 0..height {
            for x in 0..width {
                frame.buffer_mut()[(left + x, area.y + y)] = canvas[(self.view_x + x, self.view_y + y)].clone();
            }
        }
    }

    fn render_panel(&mut self, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let theme = get_tui_theme();
        let title = get_text(match self.panel {
            Panel::Details => "mnu_preview_details",
            Panel::Trace => "mnu_preview_trace",
            Panel::Issues => "mnu_preview_issues",
        });
        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(theme.dialog_box)
            .title(Line::styled(title, theme.config_title));
        let inner = block.inner(area);
        block.render(area, frame.buffer_mut());
        if self.panel == Panel::Issues && !self.issues.is_empty() {
            let mut lines = Vec::new();
            let mut selected_start = 0;
            for (i, issue) in self.issues.iter().enumerate() {
                let severity = get_text(match issue.severity {
                    IssueSeverity::Warning => "mnu_check_warning",
                    IssueSeverity::Error => "mnu_check_error",
                });
                let command = issue.command.map_or_else(|| get_text("mnu_check_menu"), |i| format!("#{}", i + 1));
                let selected = self.issue_state.selected() == Some(i);
                let marker = if selected { "> " } else { "  " };
                let item = wrapped(&[format!("{marker}{severity} {command}: {}", issue.message)], inner.width);
                if selected {
                    selected_start = lines.len();
                    lines.extend(item.into_iter().map(|line| line.patch_style(theme.selected_item)));
                } else {
                    lines.extend(item.into_iter().map(|line| {
                        line.patch_style(if issue.severity == IssueSeverity::Error {
                            theme.false_value
                        } else {
                            theme.item
                        })
                    }));
                }
            }
            if self.reveal_issue {
                self.scroll = selected_start;
                self.reveal_issue = false;
            }
            // Scroll physical lines, not just items: even one very long path
            // or diagnostic remains readable on a narrow/short terminal.
            self.scroll = self.scroll.min(lines.len().saturating_sub(usize::from(inner.height).max(1)));
            frame.render_widget(Paragraph::new(lines.into_iter().skip(self.scroll).collect::<Vec<_>>()).style(theme.item), inner);
        } else {
            let lines = match self.panel {
                Panel::Details => self.details(),
                Panel::Trace if self.trace.is_empty() => vec![get_text("mnu_preview_trace_empty")],
                Panel::Trace => self.trace.clone(),
                Panel::Issues => vec![get_text("mnu_check_ok")],
            };
            let lines = wrapped(&lines, inner.width);
            self.scroll = self.scroll.min(lines.len().saturating_sub(usize::from(inner.height).max(1)));
            frame.render_widget(Paragraph::new(lines.into_iter().skip(self.scroll).collect::<Vec<_>>()).style(theme.item), inner);
        }
    }

    fn hints(&self) -> HotkeyBar {
        HotkeyBar::new([
            Hotkey::new(KeyCode::F(1), get_text("mnu_prompts_help")),
            Hotkey::new(KeyCode::Enter, get_text("mnu_preview_trace")),
            Hotkey::new(KeyCode::F(2), get_text("mnu_preview_issues")),
            Hotkey::new(KeyCode::F(3), get_text("mnu_preview_details")),
            Hotkey::new(KeyCode::F(4), get_text("mnu_preview_highlight")),
            Hotkey::new(KeyCode::F(5), get_text("mnu_preview_reload")),
            Hotkey::alternatives([KeyCode::F(6), KeyCode::F(7)], get_text("mnu_preview_security")),
            Hotkey::alternatives([KeyCode::PageUp, KeyCode::PageDown], get_text("mnu_app_scroll")),
        ])
    }
}

fn follow(origin: u16, selected: u16, visible: u16, extent: u16) -> u16 {
    let visible = visible.min(extent).max(1);
    let origin = if selected < origin {
        selected
    } else if selected >= origin + visible {
        selected + 1 - visible
    } else {
        origin
    };
    origin.min(extent - visible)
}

impl TabPage for PreviewTab {
    fn title(&self) -> String {
        get_text("mnu_preview_title")
    }

    fn request_status(&self) -> ResultState {
        self.reload.set(true);
        ResultState::status_line(get_text("mnu_preview_scope"))
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.refresh();
        let theme = get_tui_theme();
        let area = area.intersection(frame.area());
        if area.is_empty() {
            return;
        }
        Clear.render(area, frame.buffer_mut());
        Block::default().style(theme.background).render(area, frame.buffer_mut());
        let status = format!(
            "{} | {} {} | {} {}",
            get_text("mnu_preview_read_only"),
            get_text("mnu_preview_security"),
            self.security,
            get_text("mnu_preview_issues"),
            self.issues.len()
        );
        frame.render_widget(Line::styled(status, theme.config_title), Rect::new(area.x, area.y, area.width, 1));
        if area.height < 2 {
            return;
        }
        let (note, note_style) = match self.state_error.as_ref().or(self.background_error.as_ref()) {
            Some(error) => (error.clone(), theme.false_value),
            None => (get_text("mnu_preview_scope"), theme.menu_label),
        };
        frame.render_widget(Line::styled(inert(&note), note_style), Rect::new(area.x, area.y + 1, area.width, 1));
        if area.height < 3 {
            return;
        }
        // On tiny terminals prioritize diagnostics/trace over a zero-sized image.
        if area.height < 10 {
            self.render_panel(frame, Rect::new(area.x, area.y + 2, area.width, area.height - 2));
            return;
        }
        let hint_rows = self.hints().rows(area.width);
        let hint_height = hint_rows.len().min(3) as u16;
        let panel_height = (area.height / 3).clamp(4, 9);
        let image_height = area.height.saturating_sub(panel_height + 3 + hint_height);
        self.render_canvas(frame, Rect::new(area.x, area.y + 2, area.width, image_height));
        let prompt_y = area.y + 2 + image_height;
        if let Some(menu) = &self.snapshot {
            frame.render_widget(
                get_styled_pcb_line(&format!("{} {}", inert(&menu.prompt), self.input)),
                Rect::new(area.x, prompt_y, area.width, 1),
            );
        }
        self.render_panel(frame, Rect::new(area.x, prompt_y + 1, area.width, panel_height));
        let footer = Rect::new(area.x, area.bottom() - hint_height, area.width, hint_height);
        for (line, row) in hint_rows.into_iter().zip(footer.rows()) {
            frame.render_widget(line, row);
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> ResultState {
        self.refresh();
        // A poisoned shared model must not produce a plan from stale data.
        if self.state_error.is_some() {
            return ResultState::status_line(get_text("mnu_preview_lock_error"));
        }
        match key.code {
            KeyCode::F(1) => {
                return ResultState {
                    edit_msg: EditMessage::DisplayHelp(format!(
                        "# {}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}",
                        get_text("mnu_preview_title"),
                        get_text("mnu_preview_help"),
                        get_text("mnu_preview_scope"),
                        get_text("mnu_preview_render_note"),
                        get_text("mnu_preview_selection_note"),
                        get_text("mnu_preview_trace_note"),
                    )),
                    status_line: get_text("mnu_preview_scope"),
                };
            }
            KeyCode::F(2) => {
                self.panel = if self.panel == Panel::Issues { Panel::Details } else { Panel::Issues };
                self.reveal_issue = true;
                self.scroll = 0;
            }
            KeyCode::F(3) => {
                self.panel = Panel::Details;
                self.scroll = 0;
            }
            KeyCode::F(4) => self.normal_only = !self.normal_only,
            KeyCode::F(5) => {
                self.reload.set(true);
                self.refresh();
            }
            KeyCode::F(6) | KeyCode::F(7) => {
                let step = if key.modifiers.contains(KeyModifiers::CONTROL) { 10 } else { 1 };
                self.security = if key.code == KeyCode::F(6) {
                    self.security.saturating_sub(step)
                } else {
                    self.security.saturating_add(step)
                };
                self.reload.set(true);
                self.refresh();
            }
            KeyCode::Up | KeyCode::Down | KeyCode::Home | KeyCode::End if self.panel == Panel::Issues => {
                let last = self.issues.len().saturating_sub(1);
                let cur = self.issue_state.selected().unwrap_or(0);
                let next = match key.code {
                    KeyCode::Up => cur.saturating_sub(1),
                    KeyCode::Down => cur.saturating_add(1).min(last),
                    KeyCode::Home => 0,
                    _ => last,
                };
                self.issue_state.select(if self.issues.is_empty() { None } else { Some(next) });
                self.reveal_issue = true;
            }
            KeyCode::Enter if self.panel == Panel::Issues => {
                if let Some(issue) = self.issue_state.selected().and_then(|i| self.issues.get(i))
                    && let Some(index) = issue.command
                {
                    self.select(index);
                    self.panel = Panel::Details;
                    self.scroll = 0;
                }
            }
            KeyCode::PageUp => {
                self.reveal_issue = false;
                self.scroll = self.scroll.saturating_sub(5);
            }
            KeyCode::PageDown => {
                self.reveal_issue = false;
                self.scroll = self.scroll.saturating_add(5);
            }
            KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right if key.modifiers.contains(KeyModifiers::ALT) => {
                self.manual_view = true;
                match key.code {
                    KeyCode::Up => self.view_y = self.view_y.saturating_sub(1),
                    KeyCode::Down => self.view_y = self.view_y.saturating_add(1).min(HEIGHT - 1),
                    KeyCode::Left => self.view_x = self.view_x.saturating_sub(1),
                    _ => self.view_x = self.view_x.saturating_add(1).min(WIDTH - 1),
                }
            }
            KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right | KeyCode::Home | KeyCode::End => {
                if let Some(menu) = &self.snapshot
                    && let Some(cur) = self.selected
                {
                    let last = menu.commands.len() - 1;
                    let next = if key.modifiers.contains(KeyModifiers::CONTROL) {
                        match key.code {
                            KeyCode::Up | KeyCode::Left => {
                                if cur == 0 {
                                    last
                                } else {
                                    cur - 1
                                }
                            }
                            _ => {
                                if cur == last {
                                    0
                                } else {
                                    cur + 1
                                }
                            }
                        }
                    } else {
                        match key.code {
                            KeyCode::Up => menu.up(cur),
                            KeyCode::Down => menu.down(cur),
                            KeyCode::Left => menu.left(cur),
                            KeyCode::Right => menu.right(cur),
                            KeyCode::Home => 0,
                            _ => last,
                        }
                    };
                    self.select(next);
                }
            }
            KeyCode::Enter => self.submit(),
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Char(ch) if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) && MASK_COMMAND.contains(ch) => {
                if self.input.len() < 13 {
                    self.input.push(ch);
                    if self.snapshot.as_ref().is_some_and(|m| m.menu_type == MenuType::Hotkey) {
                        self.submit();
                    }
                }
            }
            _ => {}
        }
        ResultState::status_line(get_text("mnu_preview_scope"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use icy_board_engine::icy_board::{
        commands::{CommandAction, Position},
        security_expr::SecurityExpression,
    };
    use ratatui::{Terminal, backend::TestBackend};

    fn fixture() -> (PreviewTab, Arc<Mutex<Menu>>) {
        let menu = Arc::new(Mutex::new(Menu {
            menu_type: MenuType::Command,
            prompt: "@X0FChoose:".into(),
            commands: vec![
                Command {
                    keyword: "A".into(),
                    display: "NORMAL".into(),
                    lighbar_display: "HIGHLIGHT".into(),
                    actions: vec![CommandAction {
                        command_type: CommandType::StuffText,
                        parameter: "DANGEROUS;COMMAND".into(),
                        ..CommandAction::default()
                    }],
                    ..Command::default()
                },
                Command {
                    keyword: "B".into(),
                    display: "BOTTOM".into(),
                    position: Position { x: 70, y: 24 },
                    security: SecurityExpression::from_req_security(100),
                    ..Command::default()
                },
            ],
            ..Menu::default()
        }));
        (PreviewTab::new(Arc::new(Mutex::new(IcyBoard::default())), menu.clone()), menu)
    }

    fn key(tab: &mut PreviewTab, code: KeyCode) {
        tab.handle_key_press(KeyEvent::new(code, KeyModifiers::NONE));
    }

    #[test]
    fn rendering_is_safe_at_80x25_and_tiny_sizes_and_offset_areas() {
        let (mut tab, _) = fixture();
        for (width, height) in [(80, 25), (40, 12), (8, 3), (1, 1), (0, 0), (120, 40)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            for panel in [Panel::Details, Panel::Issues, Panel::Trace] {
                tab.panel = panel;
                terminal.draw(|frame| tab.render(frame, frame.area())).unwrap();
            }
        }
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| tab.render(frame, Rect::new(3, 2, 70, 20))).unwrap();
        assert!(!tab.is_dirty());
        assert!(!tab.has_control());
    }

    #[test]
    fn canvas_shows_normal_highlight_and_keeps_coordinates_without_scaling() {
        let (mut tab, _) = fixture();
        tab.refresh();
        let canvas = tab.canvas();
        assert_eq!(canvas[(0, 0)].symbol(), "H");
        assert_eq!(canvas[(70, 24)].symbol(), "B");
        tab.normal_only = true;
        assert_eq!(tab.canvas()[(0, 0)].symbol(), "N");
        assert_eq!(follow(0, 24, 12, 25), 13);
        assert_eq!(follow(13, 0, 12, 25), 0);
        assert_eq!(follow(0, 79, 40, 80), 40);
        tab.handle_key_press(KeyEvent::new(KeyCode::Down, KeyModifiers::ALT));
        assert!(tab.manual_view);
        assert_eq!(tab.view_y, 1);
        tab.select(1);
        assert!(!tab.manual_view);
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| tab.render(frame, frame.area())).unwrap();
        // The viewport follows the selection; its height depends on the panel
        // and hint rows, so derive the expected screen row from the offset.
        assert!(tab.view_y <= 24);
        let row = 2 + (24 - tab.view_y);
        assert_eq!(terminal.backend().buffer()[(70, row)].symbol(), "B");
    }

    #[test]
    fn panel_text_is_readable_and_keys_are_shown_once_as_a_hotkey_bar() {
        let (mut tab, _) = fixture();
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| tab.render(frame, frame.area())).unwrap();
        let buffer = terminal.backend().buffer().clone();
        let theme = get_tui_theme();
        let row = |y: u16| (0..80).map(|x| buffer[(x, y)].symbol().to_string()).collect::<String>();
        let title = (3..24).find(|y| row(*y).contains(&get_text("mnu_preview_details"))).expect("inspector panel");
        let panel = title + 1;
        let text = (0..80).find(|x| buffer[(*x, panel)].symbol() != " ").expect("text cell");
        // Inheriting the frame colour made this dark blue on black before.
        assert_eq!(buffer[(text, panel)].style().fg, theme.item.fg);
        assert_ne!(buffer[(text, panel)].style().fg, theme.dialog_box.fg);
        let keys = (0..25).filter(|y| row(*y).contains(&get_text("mnu_preview_reload"))).count();
        assert_eq!(keys, 1, "the tab owns one hint bar, the app owns the status line");
    }

    #[test]
    fn activation_selection_autorun_and_all_action_types_are_only_data() {
        let (mut tab, menu) = fixture();
        {
            let mut menu = menu.lock().unwrap();
            menu.commands[0].auto_run = icy_board_engine::icy_board::commands::AutoRun::Loop;
            for command_type in [
                CommandType::RunPPE,
                CommandType::Menu,
                CommandType::Door,
                CommandType::Goodbye,
                CommandType::StuffFile,
                CommandType::Command,
            ] {
                menu.commands[0].actions.push(CommandAction {
                    command_type,
                    parameter: "preview-must-not-create-this".into(),
                    trigger: ActionTrigger::Selection,
                });
            }
        }
        let before = menu.lock().unwrap().clone();
        tab.refresh();
        key(&mut tab, KeyCode::Enter);
        assert!(tab.trace.iter().any(|line| line.contains("DANGEROUS;COMMAND")));
        assert!(tab.trace.iter().any(|line| line.contains("RunPPE")));
        key(&mut tab, KeyCode::End);
        key(&mut tab, KeyCode::Home);
        key(&mut tab, KeyCode::Enter);
        assert!(same_menu(&before, &menu.lock().unwrap()));
        assert!(!tab.is_dirty());
        // The trace builder accepts only immutable command data, not board or
        // runtime state. Every trigger is represented without an execution API.
        assert_eq!(
            action_trace(&before.commands[0], PreviewAccess::Denied, true).len(),
            6 + before.commands[0].actions.len()
        );
    }

    #[test]
    fn typed_input_is_exact_first_match_and_hotkey_is_immediate() {
        let (mut tab, menu) = fixture();
        key(&mut tab, KeyCode::Char('b'));
        assert_eq!(tab.selected, Some(0));
        key(&mut tab, KeyCode::Enter);
        assert_eq!(tab.selected, Some(1));
        assert_eq!(tab.access[1], PreviewAccess::Denied);
        menu.lock().unwrap().menu_type = MenuType::Hotkey;
        key(&mut tab, KeyCode::Char('a'));
        assert_eq!(tab.selected, Some(0));
        assert!(tab.input.is_empty());
        key(&mut tab, KeyCode::Char('z'));
        assert!(tab.trace.iter().any(|line| line == &get_text("mnu_preview_no_match")));
    }

    #[test]
    fn changes_tab_entry_security_and_removal_refresh_safely() {
        let (mut tab, menu) = fixture();
        tab.refresh();
        tab.security = 100;
        tab.request_status();
        tab.refresh();
        assert_eq!(tab.access[1], PreviewAccess::Allowed);
        key(&mut tab, KeyCode::End);
        menu.lock().unwrap().commands.clear();
        tab.refresh();
        assert_eq!(tab.selected, None);
        key(&mut tab, KeyCode::Enter);
        key(&mut tab, KeyCode::Up);
        assert!(!tab.issues.is_empty());
    }

    #[test]
    fn issues_can_select_commands_and_nan_does_not_invalidate_cache() {
        let (mut tab, menu) = fixture();
        menu.lock().unwrap().commands[1].charge_per_minute = f64::NAN;
        tab.refresh();
        assert!(same_menu(tab.snapshot.as_ref().unwrap(), &menu.lock().unwrap()));
        tab.panel = Panel::Issues;
        tab.issue_state.select(tab.issues.iter().position(|issue| issue.command == Some(1)));
        key(&mut tab, KeyCode::Enter);
        assert_eq!(tab.selected, Some(1));
        assert_eq!(tab.panel, Panel::Details);
        tab.trace = vec!["cache sentinel".into()];
        tab.refresh();
        assert_eq!(tab.trace, vec!["cache sentinel"]);
    }

    #[test]
    fn in_memory_ansi_and_pcb_are_parsed_and_executables_rejected() {
        let ansi = decode_background(Path::new("menu.ans"), b"\x1b[2J\x1b[3;4HANSI").unwrap();
        assert_eq!(ansi.char_at((3, 2).into()).ch, 'A');
        let pcb = decode_background(Path::new("menu.pcb"), b"@X0CRED").unwrap();
        assert_eq!(pcb.char_at((0, 0).into()).ch, 'R');
        assert_eq!(pcb.char_at((0, 0).into()).attribute.foreground(), 12);
        let cp437 = decode_background(Path::new("menu.ans"), &[0xdb]).unwrap();
        assert_eq!(cp437.buffer_type.convert_to_unicode(cp437.char_at((0, 0).into()).ch), '\u{2588}');
        assert!(decode_background(Path::new("menu.ppe"), b"anything").is_err());
        assert_eq!(inert("\x1b[2J\n"), "\u{fffd}[2J\u{fffd}");
    }

    #[test]
    fn background_load_error_is_visible_not_a_panic() {
        let (mut tab, menu) = fixture();
        // A child of a regular Rust source file cannot exist, independently of
        // the caller's working directory and without creating any fixture files.
        menu.lock().unwrap().display_file = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/src/tabs/preview.rs/missing"));
        tab.refresh();
        assert!(tab.background_error.is_some());
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| tab.render(frame, frame.area())).unwrap();
    }

    #[test]
    fn a_single_long_issue_can_be_scrolled_on_a_small_terminal() {
        let (mut tab, _) = fixture();
        tab.refresh();
        tab.panel = Panel::Issues;
        tab.issues = vec![MenuIssue {
            command: Some(1),
            severity: IssueSeverity::Warning,
            message: "0123456789".repeat(100),
        }];
        tab.issue_state.select(Some(0));
        let mut terminal = Terminal::new(TestBackend::new(30, 12)).unwrap();
        terminal.draw(|frame| tab.render(frame, frame.area())).unwrap();
        key(&mut tab, KeyCode::PageDown);
        terminal.draw(|frame| tab.render(frame, frame.area())).unwrap();
        assert_eq!(tab.scroll, 5);
        key(&mut tab, KeyCode::Enter);
        assert_eq!(tab.selected, Some(1));
    }
}
