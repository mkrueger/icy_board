pub mod about;
pub use about::*;

pub mod general;
pub use general::*;

pub mod keywords;
pub use keywords::*;

pub mod prompts;
pub use prompts::*;
use ratatui::widgets::Widget;

pub trait TabPage: Widget {
    fn prev_row(&mut self) {}

    fn next_row(&mut self) {}

    fn prev(&mut self) {}

    fn next(&mut self) {}
}
