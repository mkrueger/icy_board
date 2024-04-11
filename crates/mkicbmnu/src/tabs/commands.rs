use icy_board_engine::icy_board::menu::Menu;
use ratatui::{buffer::Buffer, layout::Rect};

use super::TabPage;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeywordsTab {}

impl TabPage for KeywordsTab {
    fn render(&self, area: Rect, buf: &mut Buffer) {

    }
}
