pub mod about;
pub use about::*;

pub mod general;
use crossterm::event::KeyEvent;
pub use general::*;

pub mod commands;
use icy_board_engine::icy_board::menu::Menu;
pub use commands::*;

use ratatui::{buffer::Buffer, layout::Rect};

pub trait TabPage {
    fn prev_row(&mut self) {}

    fn next_row(&mut self) {}

    fn prev(&mut self) {}

    fn next(&mut self) {}

    fn render(&self, area: Rect, buf: &mut Buffer);

    fn handle_key_press(&mut self, _key: KeyEvent) -> Option<(u16, u16)> {
        None
    }

    fn request_edit_mode(&mut self) -> Option<(u16, u16)> {
        None
    }
}
