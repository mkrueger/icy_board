use icy_board_tui::theme::THEME;
use ratatui::{buffer::Buffer, layout::{Margin, Rect}, text::Line, widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Widget}};

use super::TabPage;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AboutTab {}

impl TabPage for AboutTab {
    fn render(&self, area: Rect, buf: &mut Buffer) {
/* 
        let area = area.inner(&Margin { vertical: 1, horizontal: 2 });

        Clear.render(area, buf);
        vec![
            IceText::from("MkICBMnu Configuration Utility v0.1.0"),
            IceText::from("written 2024 as part of the icy_board project"),
            IceText::from("visit "),
            Line::from("This is a line   ".red()),
            Line::from("This is a line".on_blue()),
            Line::from("This is a longer line".crossed_out()),
            Line::from(long_line.on_green()),
            Line::from("This is a line".green().italic()),
            Line::from(vec![
                "Masked text: ".into(),
                Span::styled(
                    Masked::new("password", '*'),
                    Style::default().fg(Color::Red),
                ),
            ]),
        ];
        Paragraph::new("About")
            .style(THEME.title)
            .alignment(Alignment::Center)
            .render(area, buf);

        let block = Block::new()
            .style(THEME.content_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        block.render(area, buf);
*/


    }
}
