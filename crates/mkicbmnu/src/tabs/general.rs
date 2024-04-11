use icy_board_engine::icy_board::menu::Menu;
use icy_board_tui::theme::THEME;
use ratatui::{
    buffer::Buffer,
    layout::{Margin, Rect},
    style::{Style, Styled, Stylize},
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, List, Padding, StatefulWidget, Widget},
};

use crate::TabPage;

#[derive(Clone)]
pub struct GeneralTab {
    state: ratatui::widgets::ListState,
}

impl Default for GeneralTab {
    fn default() -> Self {
        Self {
            state: ratatui::widgets::ListState::default().with_selected(Some(0)),
        }
    }
}

impl GeneralTab {}

impl TabPage for GeneralTab {
    fn prev(&mut self) {
        let selected = self.state.selected().unwrap_or_default();
        let selected = (selected + 3) % 4;
        self.state = self.state.clone().with_selected(Some(selected));
    }

    fn next(&mut self) {
        let selected = self.state.selected().unwrap_or_default();
        let selected = (selected + 1) % 4;
        self.state = self.state.clone().with_selected(Some(selected));
    }

    fn render(&self, mnu: &Menu, area: Rect, buf: &mut Buffer) {
        let area = area.inner(&Margin { vertical: 1, horizontal: 2 });

        Clear.render(area, buf);

        let block = Block::new()
            .style(THEME.content_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        block.render(area, buf);
        let area = area.inner(&Margin { vertical: 1, horizontal: 2 });

        let mut state = self.state.clone();

        let list = List::new(["Title:", "Display File:", "Help File:", "Prompt:"])
            .style(THEME.item)
            .highlight_style(THEME.selected_item);

        StatefulWidget::render(list, area, buf, &mut state);
    }
}
