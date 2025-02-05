use crate::theme::get_tui_theme;
use crossterm::event::KeyCode;
use ratatui::{
    layout::Rect,
    text::Text,
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState, Widget},
    Frame,
};

pub struct MenuItem<T> {
    id: T,
    char: char,
    title: String,
}

impl<T> MenuItem<T> {
    pub fn new(id: T, char: char, title: String) -> Self {
        Self { id, char, title }
    }

    pub fn id(&self) -> &T {
        &self.id
    }

    pub fn char(&self) -> char {
        self.char
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    fn render_label(&self, area: Rect, frame: &mut Frame, selected: bool) {
        Text::from(self.char.to_string()).style(get_tui_theme().item).render(
            Rect {
                x: area.x,
                y: area.y,
                width: 1,
                height: 1,
            },
            frame.buffer_mut(),
        );
        Text::from(format!(" {:<25}", self.title))
            .style(if selected { get_tui_theme().selected_item } else { get_tui_theme().item })
            .render(
                Rect {
                    x: area.x + 2,
                    y: area.y,
                    width: area.width - 3,
                    height: 1,
                },
                frame.buffer_mut(),
            );
    }
}

#[derive(Default)]
pub struct SelectMenuState {
    pub selected: i32,
    pub scroll_state: ScrollbarState,
    pub first_row: u16,
    pub area_height: u16,
}

pub struct SelectMenu<T> {
    items: Vec<MenuItem<T>>,
}

impl<T> SelectMenu<T> {
    pub fn new(items: Vec<MenuItem<T>>) -> Self {
        Self { items }
    }

    pub fn render(&self, area: Rect, frame: &mut Frame, state: &mut SelectMenuState) {
        for (i, item) in self.items.iter().enumerate() {
            if i < state.first_row as usize {
                continue;
            }
            let display_area = Rect {
                x: area.x,
                y: area.y + i as u16 - state.first_row,
                width: area.width,
                height: 1,
            };
            if display_area.y >= area.y + area.height {
                break;
            }
            item.render_label(display_area, frame, state.selected as usize == i);
        }
        state.area_height = area.height;
        state.scroll_state = state
            .scroll_state
            .position(state.first_row as usize)
            .content_length(self.items.len().saturating_sub(state.area_height as usize));
        Self::render_scrollbar(&mut state.scroll_state, frame, area);
    }

    fn render_scrollbar(state: &mut ScrollbarState, frame: &mut Frame, mut area: Rect) {
        area.x += 1;
        frame.render_stateful_widget(
            Scrollbar::default()
                .style(get_tui_theme().content_box)
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .thumb_symbol("█")
                .track_symbol(Some("░"))
                .end_symbol(Some("▼")),
            area,
            state,
        );
    }

    pub fn handle_key_press(&self, key: crossterm::event::KeyEvent, state: &mut SelectMenuState) -> Option<&T> {
        match key.code {
            KeyCode::Up => {
                if state.selected > 0 {
                    state.selected -= 1;
                    if state.selected < state.first_row as i32 {
                        state.first_row = state.selected as u16;
                    }
                }
            }
            KeyCode::Down => {
                if state.selected as usize + 1 < self.items.len() {
                    state.selected += 1;
                    if state.selected >= state.first_row as i32 + state.area_height as i32 {
                        state.first_row += 1;
                    }
                }
            }
            KeyCode::Home | KeyCode::PageUp => {
                state.selected = 0;
                state.first_row = 0;
            }
            KeyCode::End | KeyCode::PageDown => {
                state.selected = self.items.len() as i32 - 1;
                state.first_row = (state.selected as u16).saturating_sub(state.area_height);
            }
            KeyCode::Char(ch) => {
                for item in &self.items {
                    if ch.to_ascii_uppercase() == item.char.to_ascii_uppercase() {
                        return Some(&item.id);
                    }
                }
            }
            KeyCode::Enter => {
                return Some(&self.items[state.selected as usize].id);
            }
            _ => {}
        }
        None
    }
}
