use std::{
    io::{self, stdout},
    time::Duration,
};

use color_eyre::config::HookBuilder;
use color_eyre::{eyre::WrapErr, Result};

use crossterm::{
    event::{self, Event},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;

pub fn init_hooks() -> Result<()> {
    let (panic, error) = HookBuilder::default().into_hooks();
    let panic = panic.into_panic_hook();
    let error = error.into_eyre_hook();
    color_eyre::eyre::set_hook(Box::new(move |e| {
        let _ = restore();
        error(e)
    }))?;
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore();
        panic(info);
    }));
    Ok(())
}

pub fn init(full_screen: bool) -> Result<Terminal<impl Backend>> {
    init_hooks()?;

    // this size is to match the size of the terminal when running the demo
    // using vhs in a 1280x640 sized window (github social preview size)
    let options = TerminalOptions {
        viewport: if full_screen {
            Viewport::Fullscreen
        } else {
            Viewport::Fixed(Rect::new(0, 0, 80, 25))
        },
    };
    let terminal = Terminal::with_options(CrosstermBackend::new(io::stdout()), options)?;
    enable_raw_mode().context("enable raw mode")?;
    stdout().execute(EnterAlternateScreen).wrap_err("enter alternate screen")?;
    Ok(terminal)
}

pub fn restore() -> Result<()> {
    disable_raw_mode().context("disable raw mode")?;
    stdout().execute(LeaveAlternateScreen).wrap_err("leave alternate screen")?;
    Ok(())
}

pub fn next_event(timeout: Duration) -> Result<Option<Event>> {
    if !event::poll(timeout)? {
        return Ok(None);
    }
    let event = event::read()?;
    Ok(Some(event))
}
