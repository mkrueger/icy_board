use icy_board_tui::{colors::RgbSwatch, theme::THEME};
use ratatui::{
    buffer::Buffer,
    layout::{Margin, Rect},
    style::Styled,
    text::Line,
    widgets::{ListItem, Paragraph, Widget},
};

use super::TabPage;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GeneralTab {}

impl TabPage for GeneralTab {}

impl Widget for GeneralTab {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = THEME.email;

        let area = area.inner(&Margin { vertical: 1, horizontal: 2 });
        let lists = vec![Line::from(vec!["Title: ".set_style(theme.header), "test".set_style(theme.header_value)])];

        Paragraph::new(lists).style(theme.body).render(area, buf);

        /*            ListItem::new(Line::from(vec!["Title:".to_string(), "test".to_string()])),
                  ListItem::new(Line::from(vec!["Display File:", "test"])),
                  ListItem::new(Line::from(vec!["Help File:", "test"])),
                  ListItem::new(Line::from(vec!["Force Display:", "test"])),
                  ListItem::new(Line::from(vec!["Use Hot Keys:", "test"])),
                  ListItem::new(Line::from(vec!["Pass Through:", "test"]))]
        */
    }
}
