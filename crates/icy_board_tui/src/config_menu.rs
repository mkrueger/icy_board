use std::{collections::HashMap, path::PathBuf};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::icy_board::icb_config::IcbColor;
use ratatui::{
    layout::Rect,
    style::Stylize,
    text::Text,
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState, Widget},
    Frame,
};

use crate::{
    text_field::{TextField, TextfieldState},
    theme::THEME,
};

#[derive(Default)]
pub struct ResultState {
    pub in_edit_mode: bool,
    pub status_line: String,
}

pub enum ListValue {
    Text(u16, String),
    Path(PathBuf),
    U32(u32, u32, u32),
    Bool(bool),
    Color(IcbColor),
    ValueList(String, Vec<String>),
}

pub struct ListItem {
    pub id: String,
    title: String,
    pub status: String,
    pub text_field_state: TextfieldState,
    pub value: ListValue,
}

impl ListItem {
    pub fn new(id: &str, title: String, value: ListValue) -> Self {
        Self {
            id: id.to_string(),
            status: format!("{}", title),
            text_field_state: TextfieldState::default(),
            title,
            value,
        }
    }

    pub fn with_status(mut self, status: &str) -> Self {
        self.status = status.to_string();
        self
    }

    fn render_label(&self, left_area: Rect, frame: &mut Frame, selected: bool, in_edit: bool) {
        match &self.value {
            ListValue::Bool(value) => {
                let title = if *value {
                    format!(" ✓ {}", self.title)
                } else {
                    format!(" ☐ {}", self.title)
                };
                let area = Rect {
                    x: left_area.x,
                    y: left_area.y,
                    width: title.len() as u16 + 1,
                    height: left_area.height,
                };
                Text::from(title)
                    .alignment(ratatui::layout::Alignment::Right)
                    .style(if selected { THEME.selected_item } else { THEME.item })
                    .render(area, frame.buffer_mut());
            }
            _ => {
                Text::from(format!(" {}:", self.title.clone()))
                    .alignment(ratatui::layout::Alignment::Right)
                    .style(if selected && !in_edit { THEME.selected_item } else { THEME.item })
                    .render(left_area, frame.buffer_mut());
            }
        }
    }

    fn render_value(&self, area: Rect, frame: &mut Frame) {
        match &self.value {
            ListValue::Text(_, text) => {
                Text::from(text.clone()).style(THEME.value).render(area, frame.buffer_mut());
            }

            ListValue::Path(text) => {
                Text::from(format!("{}", text.display())).style(THEME.value).render(area, frame.buffer_mut());
            }

            ListValue::U32(u, _min, _max) => {
                Text::from(u.to_string()).style(THEME.value).render(area, frame.buffer_mut());
            }

            ListValue::Color(color) => match color {
                IcbColor::None => Text::from("Plain").style(THEME.value).render(area, frame.buffer_mut()),
                IcbColor::Dos(u8) => Text::from(format!("@X{:02}", *u8)).style(THEME.value).render(area, frame.buffer_mut()),
                IcbColor::IcyEngine(_) => todo!(),
            },
            ListValue::Bool(_) => {}
            ListValue::ValueList(cur_value, _) => {
                Text::from(cur_value.clone()).style(THEME.value).render(area, frame.buffer_mut());
            }
        }
    }

    fn render_editor(&mut self, area: Rect, frame: &mut Frame) {
        match &self.value {
            ListValue::Text(_edit_len, text) => {
                let field = TextField::new().with_value(text.to_string());
                frame.render_stateful_widget(field, area, &mut self.text_field_state);
            }

            ListValue::Path(text) => {
                let field = TextField::new().with_value(format!("{}", text.display()));
                frame.render_stateful_widget(field, area, &mut self.text_field_state);
            }
            ListValue::U32(_value, _min, _max) => {
                self.render_value(area, frame);
            }
            ListValue::Bool(_value) => {
                self.render_value(area, frame);
            }
            ListValue::Color(_value) => {
                self.render_value(area, frame);
            }
            ListValue::ValueList(_cur_value, _) => {
                self.render_value(area, frame);
            }
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> ResultState {
        match key {
            KeyEvent { code: KeyCode::Enter, .. } => {
                return ResultState {
                    in_edit_mode: false,
                    status_line: self.status.clone(),
                };
            }
            KeyEvent { code: KeyCode::Esc, .. } => {
                return ResultState {
                    in_edit_mode: false,
                    status_line: self.status.clone(),
                };
            }
            _ => {}
        }

        match &mut self.value {
            ListValue::Text(_edit_len, text) => {
                self.text_field_state.handle_input(key, text);
            }
            ListValue::Path(path) => {
                let mut text = format!("{}", path.display());
                self.text_field_state.handle_input(key, &mut text);
                *path = PathBuf::from(text);
            }
            ListValue::Bool(b) => {
                *b = !*b;
                return ResultState::default();
            }
            ListValue::Color(_) | ListValue::U32(_, _, _) | ListValue::ValueList(_, _) => {}
        }
        ResultState {
            in_edit_mode: true,
            status_line: self.status.clone(),
        }
    }

    fn request_edit_mode(&mut self) -> ResultState {
        match &mut self.value {
            ListValue::Bool(b) => {
                *b = !*b;
                ResultState {
                    in_edit_mode: false,
                    status_line: self.status.clone(),
                }
            }
            _ => ResultState {
                in_edit_mode: true,
                status_line: "".to_string(),
            },
        }
    }
}

pub enum ConfigEntry {
    Item(ListItem),
    Group(String, Vec<ConfigEntry>),
    Table(usize, Vec<ConfigEntry>),
    Separator,
}
impl ConfigEntry {
    fn title_len(&self) -> u16 {
        match self {
            ConfigEntry::Item(item) => item.title.len() as u16,
            ConfigEntry::Group(_, items) => items.iter().map(|item| item.title_len()).max().unwrap_or(0),
            ConfigEntry::Table(_rows, _items) => 0,
            ConfigEntry::Separator => 0,
        }
    }
}

pub struct ConfigMenu {
    pub items: Vec<ConfigEntry>,
}

#[derive(Default)]
pub struct ConfigMenuState {
    pub selected: usize,
    pub in_edit: bool,
    pub first_row: u16,
    pub area_height: u16,

    pub item_pos: HashMap<usize, u16>,

    pub scroll_state: ScrollbarState,
}

impl ConfigMenu {
    pub fn render(&mut self, area: Rect, frame: &mut Frame, state: &mut ConfigMenuState) {
        let max = self.items.iter().map(|item| item.title_len()).max().unwrap_or(0);

        let mut y = 0;
        let mut x = 0;
        let mut i = 0;

        state.area_height = area.height;

        Self::display_list(&mut i, &mut self.items, area, max, &mut y, &mut x, frame, state);

        state.scroll_state = state.scroll_state.position(state.first_row as usize).content_length(state.area_height as usize);
        Self::render_scrollbar(state, frame, area);
    }

    fn render_scrollbar(state: &mut ConfigMenuState, frame: &mut Frame, mut area: Rect) {
        area.x += 1;

        frame.render_stateful_widget(
            Scrollbar::default()
                .style(THEME.content_box)
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .thumb_symbol("█")
                .track_symbol(Some("░"))
                .end_symbol(Some("▼")),
            area,
            &mut state.scroll_state,
        );
    }

    pub fn count(&self) -> usize {
        let mut len = 0;
        self.count_items(&self.items, &mut len);
        len
    }

    pub fn get_item(&self, i: usize) -> Option<&ListItem> {
        let mut len = 0;
        Self::get_item_internal(&self.items, &mut len, i)
    }

    pub fn get_item_internal<'a>(items: &'a Vec<ConfigEntry>, len: &mut usize, i: usize) -> Option<&'a ListItem> {
        for item in items.iter() {
            match item {
                ConfigEntry::Item(item) => {
                    if *len == i {
                        return Some(item);
                    }
                    *len += 1;
                }
                ConfigEntry::Group(_t, items) => {
                    let res = Self::get_item_internal(items, len, i);
                    if res.is_some() {
                        return res;
                    }
                }
                ConfigEntry::Table(_, items) => {
                    let res = Self::get_item_internal(items, len, i);
                    if res.is_some() {
                        return res;
                    }
                }
                _ => {}
            }
        }

        None
    }

    pub fn get_item_mut(&mut self, i: usize) -> Option<&mut ListItem> {
        let mut len = 0;
        Self::get_item_internal_mut(&mut self.items, &mut len, i)
    }

    pub fn get_item_internal_mut<'a>(items: &'a mut Vec<ConfigEntry>, len: &mut usize, i: usize) -> Option<&'a mut ListItem> {
        for item in items.iter_mut() {
            match item {
                ConfigEntry::Item(item) => {
                    if *len == i {
                        return Some(item);
                    }
                    *len += 1;
                }
                ConfigEntry::Group(_t, items) => {
                    let res = Self::get_item_internal_mut(items, len, i);
                    if res.is_some() {
                        return res;
                    }
                }
                ConfigEntry::Table(_, items) => {
                    let res = Self::get_item_internal_mut(items, len, i);
                    if res.is_some() {
                        return res;
                    }
                }
                _ => {}
            }
        }

        None
    }

    fn count_items(&self, items: &Vec<ConfigEntry>, len: &mut usize) {
        for item in items.iter() {
            match item {
                ConfigEntry::Item(_) => {
                    *len += 1;
                }
                ConfigEntry::Group(_t, items) => {
                    self.count_items(items, len);
                }
                ConfigEntry::Table(_rows, items) => {
                    self.count_items(items, len);
                }
                _ => {}
            }
        }
    }

    fn display_table(
        i: &mut usize,
        items: &mut Vec<ConfigEntry>,
        area: Rect,
        max: u16,
        y: &mut u16,
        x: &mut u16,
        frame: &mut Frame,
        state: &mut ConfigMenuState,
    ) {
        let x1 = *x;
        let x2 = *x + area.width / 2;

        for (j, item) in items.iter_mut().enumerate() {
            match item {
                ConfigEntry::Item(item) => {
                    if *y >= state.first_row && *y < area.height + state.first_row {
                        let left_area = Rect {
                            x: area.x + *x,
                            y: area.y + *y - state.first_row,
                            width: max as u16 + 2,
                            height: 1,
                        };

                        item.render_label(left_area, frame, *i == state.selected, state.in_edit);

                        let right_area = Rect {
                            x: area.x + *x + max + 3,
                            y: area.y + *y - state.first_row,
                            width: area.width - (*x + max + 3) - 2,
                            height: 1,
                        };
                        if state.in_edit && *i == state.selected {
                            item.render_editor(right_area, frame);
                        } else {
                            item.render_value(right_area, frame);
                        }
                    }

                    state.item_pos.insert(*i, *y);
                    if j % 2 == 0 {
                        *x = x2;
                    } else {
                        *x = x1;
                        *y += 1;
                    }
                    *i += 1;
                }
                _ => {
                    todo!()
                }
            }
        }
        *x = x1;
    }
    pub fn display_list(
        i: &mut usize,
        items: &mut Vec<ConfigEntry>,
        area: Rect,
        max: u16,
        y: &mut u16,
        x: &mut u16,
        frame: &mut Frame,
        state: &mut ConfigMenuState,
    ) {
        for item in items.iter_mut() {
            match item {
                ConfigEntry::Item(item) => {
                    if *y >= state.first_row && *y < area.height + state.first_row {
                        let left_area = Rect {
                            x: area.x + *x,
                            y: area.y + *y - state.first_row,
                            width: max as u16 + 2,
                            height: 1,
                        };

                        item.render_label(left_area, frame, *i == state.selected, state.in_edit);

                        let right_area = Rect {
                            x: area.x + *x + max + 3,
                            y: area.y + *y - state.first_row,
                            width: area.width - (*x + max + 3) - 2,
                            height: 1,
                        };
                        if state.in_edit && *i == state.selected {
                            item.render_editor(right_area, frame);
                        } else {
                            item.render_value(right_area, frame);
                        }
                    }

                    state.item_pos.insert(*i, *y);
                    *y += 1;
                    *i += 1;
                }
                ConfigEntry::Group(title, items) => {
                    if *y >= state.first_row && *y < area.height + state.first_row {
                        let left_area = Rect {
                            x: area.x + *x,
                            y: area.y + *y - state.first_row,
                            width: area.width - *x - 1,
                            height: 1,
                        };
                        Text::from(format!(" {}", title.clone()))
                            .alignment(ratatui::layout::Alignment::Left)
                            .style(THEME.config_title.italic())
                            .render(left_area, frame.buffer_mut());
                    }
                    *y += 1;
                    Self::display_list(i, items, area, max, y, x, frame, state);
                }
                ConfigEntry::Table(_cols, items) => {
                    Self::display_table(i, items, area, max, y, x, frame, state);
                }
                ConfigEntry::Separator => {
                    /*
                    let left_area = Rect {
                        x: area.x,
                        y: area.y + *y - state.top_row,
                        width: area.width - 2,
                        height: 1,
                    };

                    Text::from("-".repeat(area.width as usize))
                    .alignment(ratatui::layout::Alignment::Right)
                    .style(
                        THEME.item
                    )
                    .render(left_area, buf);*/
                    *y += 1;
                }
            }
        }
    }

    pub fn handle_key_press(&mut self, key: KeyEvent, state: &mut ConfigMenuState) -> ResultState {
        if state.in_edit {
            return self.get_item_mut(state.selected).unwrap().handle_key_press(key);
        }

        match key.code {
            KeyCode::Home => {
                state.selected = 0;
                state.first_row = 0;
            }
            KeyCode::End => {
                state.selected = self.count() - 1;
                state.first_row = state.item_pos.get(&state.selected).unwrap_or(&0) - state.area_height + 1;
            }
            KeyCode::PageDown => {
                state.selected += state.area_height as usize;
                if state.selected >= self.count() {
                    state.selected = self.count() - 1;
                }
                if let Some(y) = state.item_pos.get(&state.selected) {
                    if *y >= state.area_height {
                        state.first_row = *y - state.area_height + 1;
                    }
                }
            }
            KeyCode::PageUp => {
                if state.selected >= state.area_height as usize {
                    state.selected -= state.area_height as usize;
                } else {
                    state.selected = 0;
                }
                if let Some(y) = state.item_pos.get(&state.selected) {
                    if *y < state.first_row {
                        state.first_row = *y;
                        if state.first_row == 1 {
                            state.first_row = 0;
                        }
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => Self::prev(state),
            KeyCode::Char('j') | KeyCode::Down => Self::next(self.count(), state),
            KeyCode::Char('d') | KeyCode::Enter => {
                return self.get_item_mut(state.selected).unwrap().request_edit_mode();
            }

            _ => {}
        }
        ResultState {
            in_edit_mode: false,
            status_line: self.get_item(state.selected).unwrap().status.clone(),
        }
    }

    fn prev(state: &mut ConfigMenuState) {
        if state.selected > 0 {
            state.selected -= 1;

            if let Some(y) = state.item_pos.get(&state.selected) {
                if *y < state.first_row {
                    state.first_row = *y;
                    if state.first_row == 1 {
                        state.first_row = 0;
                    }
                }
            }
        }
    }

    fn next(count: usize, state: &mut ConfigMenuState) {
        if state.selected < count - 1 {
            state.selected += 1;
            if let Some(y) = state.item_pos.get(&state.selected) {
                if *y >= state.area_height {
                    state.first_row = *y - state.area_height + 1;
                }
            }
        }
    }
}
