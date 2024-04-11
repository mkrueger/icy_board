pub mod about;
use std::io;

pub use about::*;

pub mod general;
use crossterm::event::KeyEvent;
pub use general::*;

pub mod commands;
pub use commands::*;

use icy_board_tui::TerminalType;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::Rect,
    Frame, Terminal,
};

pub trait TabPage {
    fn prev_row(&mut self) {}

    fn next_row(&mut self) {}

    fn prev(&mut self) {}

    fn next(&mut self) {}

    fn render(&mut self, frame: &mut Frame, area: Rect);

    fn handle_key_press(&mut self, _key: KeyEvent) -> Option<(u16, u16)> {
        None
    }

    fn request_edit_mode(&mut self, terminal: &mut TerminalType) -> Option<(u16, u16)> {
        None
    }

    fn insert(&mut self) {}

    fn delete(&mut self) {}
}
