pub mod about;
pub use about::*;

pub mod general;
pub use general::*;

pub mod keywords;
use icy_board_engine::icy_board::menu::Menu;
pub use keywords::*;

pub mod prompts;
pub use prompts::*;
use ratatui::{buffer::Buffer, layout::Rect};

pub trait TabPage {
    fn prev_row(&mut self) {}

    fn next_row(&mut self) {}

    fn prev(&mut self) {}

    fn next(&mut self) {}

    fn render(&self, mnu: &Menu, area: Rect, buf: &mut Buffer);
}
