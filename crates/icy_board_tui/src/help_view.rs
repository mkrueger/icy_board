use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    text::Text,
    widgets::{Block, BorderType, Borders},
};
use ratatui::{
    prelude::*,
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
};

use crate::theme::get_tui_theme;

/// necessary as ScrollbarState fields are private
#[derive(Debug, Default)]
pub struct HelpViewState<'a> {
    pub position: usize,
    pub view_size: usize,
    pub max: usize,
    pub text: Text<'a>,
}

impl<'a> HelpViewState<'a> {
    pub fn new(max: usize) -> HelpViewState<'a> {
        HelpViewState {
            position: 0,
            view_size: 1,
            max,
            text: Text::default(),
        }
    }

    fn scroll_down(&mut self) {
        self.position = self.position.saturating_add(1);
    }

    fn scroll_up(&mut self) {
        self.position = self.position.saturating_sub(1);
    }

    fn scroll_page_down(&mut self) {
        self.position = self.position.saturating_add(self.view_size);
    }

    fn scroll_page_up(&mut self) {
        self.position = self.position.saturating_sub(self.view_size);
    }

    fn scroll_top(&mut self) {
        self.position = 0;
    }

    fn scroll_bottom(&mut self) {
        self.position = self.max.saturating_sub(self.view_size);
    }

    pub(crate) fn handle_key_press(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll_down();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll_up();
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.scroll_top();
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.scroll_bottom();
            }
            KeyCode::PageDown => {
                self.scroll_page_down();
            }
            KeyCode::PageUp => {
                self.scroll_page_up();
            }
            _ => {}
        }
    }
}

impl From<&mut HelpViewState<'_>> for ScrollbarState {
    fn from(state: &mut HelpViewState) -> ScrollbarState {
        ScrollbarState::new(state.max.saturating_sub(state.view_size) as usize).position(state.position as usize)
    }
}

#[derive(Default, Clone, Copy)]
pub struct HelpView<'a> {
    pub phantom_data: &'a str,
}

impl<'a> StatefulWidget for HelpView<'a> {
    type State = HelpViewState<'a>;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        Block::new()
            .style(get_tui_theme().help_box)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .render(area, buf);
        let area = area.inner(Margin { horizontal: 1, vertical: 1 });
        let [body, scrollbar] = Layout::horizontal([Constraint::Fill(1), Constraint::Length(1)]).areas(area);
        let body = body.inner(Margin { horizontal: 1, vertical: 1 });
        state.view_size = body.height as usize;
        state.position = state.position.min(state.text.height().saturating_sub(state.view_size));
        let position = state.position.min(state.text.height().saturating_sub(state.view_size)) as u16;
        Paragraph::new(state.text.clone())
            .scroll((position, 0))
            .wrap(Wrap { trim: false })
            .render(body, buf);
        let mut scrollbar_state = state.into();
        Scrollbar::new(ScrollbarOrientation::VerticalRight).render(scrollbar, buf, &mut scrollbar_state);
    }
}
