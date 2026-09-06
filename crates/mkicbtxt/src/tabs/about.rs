use super::TabPage;
use icy_board_tui::{about::render_about, theme::get_tui_theme};
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Widget},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AboutTab {}

impl TabPage for AboutTab {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        if get_tui_theme().swatch {
            icy_board_tui::colors::RgbSwatch.render(area, frame.buffer_mut());
        } else {
            Block::new()
                .style(get_tui_theme().background)
                .borders(Borders::NONE)
                .render(area, frame.buffer_mut());
        }

        render_about(frame, area, "app_mkicbtxt", &crate::VERSION.to_string());
    }
}
