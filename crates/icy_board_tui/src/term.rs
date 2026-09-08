use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use color_eyre::Result;

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{self, Clear, EnterAlternateScreen, LeaveAlternateScreen},
};
use icy_engine::DOS_DEFAULT_PALETTE;
use ratatui::{
    Terminal, TerminalOptions, Viewport,
    backend::{Backend, ClearType, CrosstermBackend, WindowSize},
    buffer::Cell,
    layout::{Position, Size},
    style::Color,
};
use terminfo::{Database, capability as cap};

use crate::{TerminalType, theme::DOS_ANSI_INDEX};

static RESTORE_HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);

fn install_restore_hook() {
    if RESTORE_HOOK_INSTALLED.swap(true, Ordering::AcqRel) {
        return;
    }
    let original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = restore();
        original(panic_info);
    }));
}

fn rgb_capability(
    override_mode: Option<&str>,
    color_term: Option<&str>,
    term: Option<&str>,
    terminal_program: Option<&str>,
    known_truecolor_host: bool,
    terminfo_truecolor: bool,
) -> bool {
    match override_mode.map(str::to_ascii_lowercase).as_deref() {
        Some("rgb" | "truecolor" | "24bit") => return true,
        Some("indexed" | "ansi" | "16color") => return false,
        _ => {}
    }
    if color_term.is_some_and(|value| matches!(value.to_ascii_lowercase().as_str(), "truecolor" | "24bit")) {
        return true;
    }
    if term.is_some_and(|value| {
        let value = value.to_ascii_lowercase();
        value.contains("truecolor") || value.contains("24bit") || value.ends_with("-direct")
    }) {
        return true;
    }
    let terminal_program_supports_rgb =
        terminal_program.is_some_and(|value| matches!(value.to_ascii_lowercase().as_str(), "vscode" | "wezterm" | "iterm.app" | "hyper"));
    terminfo_truecolor || terminal_program_supports_rgb || known_truecolor_host
}

pub fn terminal_supports_rgb() -> bool {
    let terminfo_truecolor = Database::from_env().ok().is_some_and(|info| {
        info.get::<cap::TrueColor>().is_some() || (info.get::<cap::SetTrueColorForeground>().is_some() && info.get::<cap::SetTrueColorBackground>().is_some())
    });
    let terminal_program = std::env::var("TERM_PROGRAM").ok();
    let known_truecolor_host = ["KITTY_WINDOW_ID", "ALACRITTY_WINDOW_ID", "KONSOLE_VERSION", "WT_SESSION"]
        .into_iter()
        .any(|name| std::env::var_os(name).is_some())
        || std::env::var("VTE_VERSION")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .is_some_and(|version| version >= 3600);

    rgb_capability(
        std::env::var("ICY_BOARD_COLOR_MODE").ok().as_deref(),
        std::env::var("COLORTERM").ok().as_deref(),
        std::env::var("TERM").ok().as_deref(),
        terminal_program.as_deref(),
        known_truecolor_host,
        terminfo_truecolor,
    )
}

fn dos_rgb_for_ansi_color(color: Color) -> Color {
    let Color::Indexed(index @ 0..=15) = color else {
        return color;
    };
    let (red, green, blue) = DOS_DEFAULT_PALETTE[usize::from(DOS_ANSI_INDEX[usize::from(index)])].rgb();
    Color::Rgb(red, green, blue)
}

pub struct IcyBoardBackend<W: Write> {
    inner: CrosstermBackend<W>,
    rgb: bool,
}

impl<W: Write> IcyBoardBackend<W> {
    pub fn new(writer: W) -> Self {
        Self::with_rgb(writer, terminal_supports_rgb())
    }

    fn with_rgb(writer: W, rgb: bool) -> Self {
        Self {
            inner: CrosstermBackend::new(writer),
            rgb,
        }
    }
}

impl<W: Write> Backend for IcyBoardBackend<W> {
    type Error = io::Error;

    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        if !self.rgb {
            return self.inner.draw(content);
        }
        let cells: Vec<_> = content
            .map(|(x, y, cell)| {
                let mut cell = cell.clone();
                cell.fg = dos_rgb_for_ansi_color(cell.fg);
                cell.bg = dos_rgb_for_ansi_color(cell.bg);
                (x, y, cell)
            })
            .collect();
        self.inner.draw(cells.iter().map(|(x, y, cell)| (*x, *y, cell)))
    }

    fn append_lines(&mut self, n: u16) -> io::Result<()> {
        self.inner.append_lines(n)
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.inner.hide_cursor()
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.inner.show_cursor()
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        self.inner.get_cursor_position()
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> io::Result<()> {
        self.inner.set_cursor_position(position)
    }

    fn clear(&mut self) -> io::Result<()> {
        self.inner.clear()
    }

    fn clear_region(&mut self, clear_type: ClearType) -> io::Result<()> {
        self.inner.clear_region(clear_type)
    }

    fn size(&self) -> io::Result<Size> {
        self.inner.size()
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        self.inner.window_size()
    }

    fn flush(&mut self) -> io::Result<()> {
        Backend::flush(&mut self.inner)
    }
}

pub fn init() -> Result<TerminalType> {
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    color_eyre::install()?;
    install_restore_hook();
    terminal::enable_raw_mode()?;

    // this size is to match the size of the terminal when running the demo
    // using vhs in a 1280x640 sized window (github social preview size)
    let options = TerminalOptions {
        viewport: Viewport::Fullscreen,
    };
    let backend = IcyBoardBackend::new(stdout);
    let mut terminal = Terminal::with_options(backend, options)?;
    terminal.clear()?;
    Ok(terminal)
}

pub fn restore() -> Result<()> {
    let mut stdout = std::io::stdout();
    execute!(stdout, Clear(terminal::ClearType::All))?;
    ratatui::restore();
    Ok(())
}

/// Set while an external program had the terminal, so the next frame is drawn
/// in full instead of as a difference against a screen somebody else wrote on.
static NEEDS_FULL_REDRAW: AtomicBool = AtomicBool::new(false);

/// Hands the terminal to a program that wants it for itself - an editor, say -
/// and takes it back afterwards.
///
/// Without this the editor draws into the alternate screen the TUI is holding,
/// in raw mode, and whatever it leaves behind stays on screen. The hand-back
/// happens whether the program ran, failed to start or failed while running.
pub fn with_terminal<T>(run: impl FnOnce() -> T) -> T {
    let mut stdout = std::io::stdout();
    let _ = terminal::disable_raw_mode();
    let _ = execute!(stdout, LeaveAlternateScreen);

    let result = run();

    let _ = execute!(stdout, EnterAlternateScreen);
    let _ = terminal::enable_raw_mode();
    NEEDS_FULL_REDRAW.store(true, Ordering::Relaxed);
    result
}

/// Whether the screen has to be thrown away before the next frame. Answers true
/// once per hand-back.
pub fn take_needs_full_redraw() -> bool {
    NEEDS_FULL_REDRAW.swap(false, Ordering::Relaxed)
}

pub fn next_event(timeout: Duration) -> Result<Option<Event>> {
    if !event::poll(timeout)? {
        return Ok(None);
    }
    let event = event::read()?;
    Ok(Some(event))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_support_uses_explicit_capabilities_without_guessing_from_256_colors() {
        assert!(rgb_capability(None, Some("truecolor"), None, None, false, false));
        assert!(rgb_capability(None, None, Some("xterm-direct"), None, false, false));
        assert!(rgb_capability(None, None, None, Some("vscode"), false, false));
        assert!(rgb_capability(None, None, None, None, true, false));
        assert!(rgb_capability(None, None, None, None, false, true));
        assert!(!rgb_capability(None, None, Some("xterm-256color"), None, false, false));
        assert!(!rgb_capability(None, None, None, Some("unknown"), false, false));
    }

    #[test]
    fn color_mode_override_wins_over_detected_capabilities() {
        assert!(rgb_capability(Some("rgb"), None, None, None, false, false));
        assert!(!rgb_capability(
            Some("indexed"),
            Some("truecolor"),
            Some("xterm-direct"),
            Some("vscode"),
            true,
            true
        ));
    }

    #[test]
    fn rgb_backend_translates_dos_slots_without_programming_the_terminal_palette() {
        let mut output = Vec::new();
        let mut backend = IcyBoardBackend::with_rgb(&mut output, true);
        let mut cell = Cell::new("X");
        cell.fg = Color::Indexed(4);
        cell.bg = Color::Indexed(1);

        backend.draw(std::iter::once((0, 0, &cell))).unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("38;2;0;0;170"));
        assert!(output.contains("48;2;170;0;0"));
        assert!(!output.contains("]4;"));
    }

    #[test]
    fn indexed_backend_keeps_ansi_colors_for_limited_terminals() {
        let mut output = Vec::new();
        let mut backend = IcyBoardBackend::with_rgb(&mut output, false);
        let mut cell = Cell::new("X");
        cell.fg = Color::Indexed(4);

        backend.draw(std::iter::once((0, 0, &cell))).unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("38;5;4"));
        assert!(!output.contains("38;2;"));
    }
}
