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
use ratatui::{TerminalOptions, Viewport};
use terminfo::{Database, capability as cap};

use crate::{TerminalType, theme::DOS_ANSI_INDEX};

static PALETTE_ACTIVE: AtomicBool = AtomicBool::new(false);

fn write_dos_palette(mut writer: impl Write) -> io::Result<()> {
    writer.write_all(b"\x1b]4")?;
    for (ansi_index, dos_index) in DOS_ANSI_INDEX.into_iter().enumerate() {
        let color = &DOS_DEFAULT_PALETTE[usize::from(dos_index)];
        let (red, green, blue) = color.rgb();
        write!(writer, ";{ansi_index};rgb:{red:02X}/{green:02X}/{blue:02X}")?;
    }
    writer.write_all(b"\x1b\\")?;
    writer.flush()
}

fn reset_dos_palette(mut writer: impl Write) -> io::Result<()> {
    writer.write_all(b"\x1b]104;0;1;2;3;4;5;6;7;8;9;10;11;12;13;14;15\x1b\\")?;
    writer.flush()
}

fn supports_osc_palette(terminfo_supports_osc4: bool, terminal_program: Option<&str>, known_terminal_host: bool) -> bool {
    let terminal_program_supports_osc4 =
        terminal_program.is_some_and(|value| matches!(value.to_ascii_lowercase().as_str(), "vscode" | "wezterm" | "iterm.app" | "hyper"));

    terminfo_supports_osc4 || terminal_program_supports_osc4 || known_terminal_host
}

fn terminal_supports_osc_palette() -> bool {
    let terminfo_supports_osc4 = Database::from_env()
        .ok()
        .and_then(|info| info.get::<cap::InitializeColor>().map(|init| init.as_ref().starts_with(b"\x1b]4")))
        .unwrap_or(false);
    let terminal_program = std::env::var("TERM_PROGRAM").ok();
    let known_terminal_host = ["VTE_VERSION", "KITTY_WINDOW_ID", "ALACRITTY_WINDOW_ID", "KONSOLE_VERSION", "WT_SESSION"]
        .into_iter()
        .any(|name| std::env::var_os(name).is_some());

    supports_osc_palette(terminfo_supports_osc4, terminal_program.as_deref(), known_terminal_host)
}

pub fn apply_dos_palette() -> io::Result<()> {
    if !terminal_supports_osc_palette() {
        return Ok(());
    }
    write_dos_palette(std::io::stdout())?;
    PALETTE_ACTIVE.store(true, Ordering::Release);
    Ok(())
}

pub fn restore_palette() -> io::Result<()> {
    if !PALETTE_ACTIVE.swap(false, Ordering::AcqRel) {
        return Ok(());
    }
    reset_dos_palette(std::io::stdout())
}

pub fn init() -> Result<TerminalType> {
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    apply_dos_palette()?;

    color_eyre::install()?;

    // this size is to match the size of the terminal when running the demo
    // using vhs in a 1280x640 sized window (github social preview size)
    let options = TerminalOptions {
        viewport: Viewport::Fullscreen,
    };
    let mut terminal = ratatui::init_with_options(options);
    terminal.clear()?;
    Ok(terminal)
}

pub fn restore() -> Result<()> {
    let mut stdout = std::io::stdout();
    execute!(stdout, Clear(terminal::ClearType::All))?;
    ratatui::restore();
    restore_palette()?;
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
    let _ = restore_palette();

    let result = run();

    let _ = execute!(stdout, EnterAlternateScreen);
    let _ = apply_dos_palette();
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
    fn dos_palette_programs_the_ansi_slots_with_dos_rgb_values() {
        let mut output = Vec::new();

        write_dos_palette(&mut output).unwrap();

        assert!(output.starts_with(b"\x1b]4;0;rgb:00/00/00;1;rgb:AA/00/00"));
        assert!(output.windows(b";4;rgb:00/00/AA".len()).any(|window| window == b";4;rgb:00/00/AA"));
        assert!(output.ends_with(b";15;rgb:FF/FF/FF\x1b\\"));
    }

    #[test]
    fn palette_reset_only_restores_the_slots_icy_board_changed() {
        let mut output = Vec::new();

        reset_dos_palette(&mut output).unwrap();

        assert_eq!(output, b"\x1b]104;0;1;2;3;4;5;6;7;8;9;10;11;12;13;14;15\x1b\\");
    }

    #[test]
    fn palette_support_requires_osc4_terminfo_or_a_known_terminal_host() {
        assert!(supports_osc_palette(true, None, false));
        assert!(supports_osc_palette(false, Some("vscode"), false));
        assert!(supports_osc_palette(false, None, true));
        assert!(!supports_osc_palette(false, None, false));
        assert!(!supports_osc_palette(false, Some("unknown"), false));
    }
}
