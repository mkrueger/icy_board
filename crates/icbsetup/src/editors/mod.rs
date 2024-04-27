pub mod door;

use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};

pub trait Editor {
    fn render(&mut self, frame: &mut Frame, area: Rect);
    fn handle_key_press(&mut self, _key: KeyEvent) -> bool;
}
