use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use color_eyre::Result;

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{self, Clear, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{TerminalOptions, Viewport};

use crate::TerminalType;

pub fn init() -> Result<TerminalType> {
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    color_eyre::install()?;

    // this size is to match the size of the terminal when running the demo
    // using vhs in a 1280x640 sized window (github social preview size)
    let options = TerminalOptions {
        viewport: Viewport::Fullscreen,
    };
    let terminal = ratatui::init_with_options(options);
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
