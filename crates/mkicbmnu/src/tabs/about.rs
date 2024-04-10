use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use super::TabPage;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AboutTab {}

impl TabPage for AboutTab {}

impl Widget for AboutTab {
    fn render(self, area: Rect, buf: &mut Buffer) {}
}
