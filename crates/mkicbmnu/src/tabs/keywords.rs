use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use super::TabPage;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeywordsTab {}

impl TabPage for KeywordsTab {}

impl Widget for KeywordsTab {
    fn render(self, area: Rect, buf: &mut Buffer) {}
}