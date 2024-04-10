use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use super::TabPage;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PromptsTab {}

impl TabPage for PromptsTab {}

impl Widget for PromptsTab {
    fn render(self, area: Rect, buf: &mut Buffer) {}
}
