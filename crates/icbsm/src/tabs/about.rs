use icy_board_tui::{about::render_about, get_text, tab_page::TabPage};
use ratatui::{Frame, layout::Rect};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AboutTab {}

impl TabPage for AboutTab {
    fn title(&self) -> String {
        get_text("tui_tab_about")
    }
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        render_about(frame, area, "app_icbsm", &crate::VERSION.to_string());
    }
}
