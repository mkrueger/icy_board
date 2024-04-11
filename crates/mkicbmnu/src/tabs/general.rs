use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::icy_board::menu::Menu;
use icy_board_tui::{theme::THEME, TerminalType};
use ratatui::{
    buffer::Buffer,
    layout::{Margin, Rect},
    text::Text,
    widgets::{Block, BorderType, Borders, Clear, Padding, Widget},
    Frame,
};

use crate::TabPage;

pub struct GeneralTab {
    state: ConfigMenuState,
    config: ConfigMenu,
}

pub enum ListValue {
    Text(u16, String),
    File(u16, String),
    Bool(bool),
    ValueList(String, Vec<String>),
}

pub struct ListItem {
    id: String,
    title: String,
    cursor_pos: u16,
    edit_pos: Arc<Mutex<(u16, u16)>>,
    value: ListValue,
}

impl ListItem {
    fn new(id: &str, title: String, value: ListValue) -> Self {
        let cursor_pos = match &value {
            ListValue::Text(_, text) | ListValue::File(_, text) => text.len() as u16,
            _ => 0,
        };
        Self {
            id: id.to_string(),
            title,
            cursor_pos,
            edit_pos: Arc::new(Mutex::new((0, 0))),
            value,
        }
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.edit_pos.lock().unwrap().0 = area.x;
        self.edit_pos.lock().unwrap().1 = area.y;

        match &self.value {
            ListValue::Text(edit_len, text) | ListValue::File(edit_len, text) => {
                let mut area = area;
                area.width = *edit_len;
                Text::from(text.clone()).style(THEME.value).render(area, buf);
            }
            ListValue::Bool(value) => {
                Text::from(if *value { "Yes" } else { "No" }).style(THEME.value).render(area, buf);
            }
            ListValue::ValueList(cur_value, _) => {
                Text::from(cur_value.clone()).style(THEME.value).render(area, buf);
            }
        }
    }

    fn render_editor(&self, area: Rect, buf: &mut Buffer) {
        match &self.value {
            ListValue::Text(edit_len, text) | ListValue::File(edit_len, text) => {
                let mut area = area;
                area.width = *edit_len;
                Text::from(text.clone()).style(THEME.edit_value).render(area, buf);
            }
            ListValue::Bool(_value) => {
                self.render(area, buf);
            }
            ListValue::ValueList(_cur_value, _) => {
                self.render(area, buf);
            }
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> Option<(u16, u16)> {
        match &mut self.value {
            ListValue::File(_edit_len, text) | ListValue::Text(_edit_len, text) => {
                match key.code {
                    KeyCode::Enter => {
                        return None;
                    }
                    KeyCode::Esc => {
                        return None;
                    }
                    KeyCode::Char(c) => {
                        text.push(c);
                        self.cursor_pos += 1;
                    }
                    _ => {}
                }

                let mut e = (*self.edit_pos).lock().unwrap().clone();
                e.0 += self.cursor_pos;
                return Some(e);
            }
            ListValue::Bool(_value) => None,
            ListValue::ValueList(_cur_value, _) => None,
        }
    }
}

pub struct ConfigMenu {
    items: Vec<ListItem>,
}

#[derive(Default)]
pub struct ConfigMenuState {
    selected: usize,
    in_edit: bool,
}

impl ConfigMenu {
    pub fn render(&self, area: Rect, buf: &mut Buffer, state: &ConfigMenuState) {
        let mut area = area;
        area.height = self.items.len() as u16 - 3;
        let max = self.items.iter().map(|item| item.title.len()).max().unwrap_or(0);
        for (i, item) in self.items.iter().enumerate() {
            let mut left_area = area.clone();
            left_area.width = max as u16 + 2;

            Text::from(format!(" {}:", item.title.clone()))
                .alignment(ratatui::layout::Alignment::Right)
                .style(if i == state.selected && !state.in_edit {
                    THEME.selected_item
                } else {
                    THEME.item
                })
                .render(left_area, buf);

            let mut right_area = area.clone();
            right_area.x = right_area.x + max as u16 + 3;
            right_area.width = area.right() - right_area.x;
            if state.in_edit && i == state.selected {
                item.render_editor(right_area, buf);
            } else {
                item.render(right_area, buf);
            }

            area.y += 1;
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent, state: &ConfigMenuState) -> Option<(u16, u16)> {
        if !state.in_edit {
            return None;
        }
        self.items[state.selected].handle_key_press(key)
    }

    fn request_edit_mode(&self, state: &ConfigMenuState) -> Option<(u16, u16)> {
        let e = self.items[state.selected].edit_pos.lock().unwrap().clone();
        Some((e.0 + self.items[state.selected].cursor_pos, e.1))
    }
}

impl GeneralTab {
    pub fn new(menu: Arc<Menu>) -> Self {
        let items = vec![
            ListItem::new("title", "Title".to_string(), ListValue::Text(25, menu.title.clone())),
            ListItem::new(
                "display_file",
                "Display File".to_string(),
                ListValue::Text(25, menu.display_file.to_string_lossy().to_string()),
            ),
            ListItem::new(
                "help_file",
                "Help File".to_string(),
                ListValue::File(25, menu.help_file.to_string_lossy().to_string()),
            ),
            ListItem::new("prompt", "Prompt".to_string(), ListValue::Text(25, menu.prompt.clone())),
        ];

        Self {
            state: ConfigMenuState::default(),
            config: ConfigMenu { items },
        }
    }
}

impl TabPage for GeneralTab {
    fn prev(&mut self) {
        let selected = self.state.selected;
        self.state.selected = (selected + 3) % 4;
    }

    fn next(&mut self) {
        let selected = self.state.selected;
        self.state.selected = (selected + 1) % 4;
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let width = (2 + 50 + 2).min(area.width) as u16;

        let lines = (6).min(area.height) as u16;
        let area = Rect::new(area.x + (area.width - width) / 2, (area.y + area.height - lines) / 2, width + 2, lines);

        Clear.render(area, frame.buffer_mut());

        let block = Block::new()
            .style(THEME.content_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        block.render(area, frame.buffer_mut());

        let area = area.inner(&Margin { vertical: 1, horizontal: 1 });
        self.config.render(area, frame.buffer_mut(), &self.state);
    }

    fn request_edit_mode(&mut self, _t: &mut TerminalType) -> Option<(u16, u16)> {
        self.state.in_edit = true;
        self.config.request_edit_mode(&self.state)
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> Option<(u16, u16)> {
        let res = self.config.handle_key_press(key, &self.state);
        if res.is_none() {
            self.state.in_edit = false;
        }
        res
    }
}
