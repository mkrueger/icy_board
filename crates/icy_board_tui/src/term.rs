use std::
    time::Duration
;

use color_eyre::Result;

use crossterm::event::{self, Event};
use ratatui::{TerminalOptions, Viewport};

use crate::TerminalType;


pub fn init() -> Result<TerminalType> {
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
    ratatui::restore();
    Ok(())
}

pub fn next_event(timeout: Duration) -> Result<Option<Event>> {
    if !event::poll(timeout)? {
        return Ok(None);
    }
    let event = event::read()?;
    Ok(Some(event))
}
