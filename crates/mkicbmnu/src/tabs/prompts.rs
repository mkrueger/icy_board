use icy_board_engine::icy_board::menu::Menu;
use ratatui::{buffer::Buffer, layout::Rect};

use super::TabPage;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PromptsTab {}

impl TabPage for PromptsTab {
    fn render(&self, mnu: &Menu, area: Rect, buf: &mut Buffer) {}
}
