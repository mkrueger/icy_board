use std::{collections::HashMap, path::PathBuf};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::icy_board::{commands::Position, icb_config::IcbColor};
use ratatui::{
    layout::Rect,
    style::Stylize,
    text::Text,
    widgets::{Block, BorderType, Borders, Clear, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget},
    Frame,
};

use crate::{
    text_field::{TextField, TextfieldState},
    theme::THEME,
};

#[derive(Default)]
pub enum EditMode {
    #[default]
    None,
    Open(String, PathBuf),
}

#[derive(Default)]
pub struct ResultState {
    pub edit_mode: EditMode,
    pub status_line: String,
}
impl ResultState {
    pub fn status_line(status_line: String) -> ResultState {
        ResultState {
            edit_mode: EditMode::None,
            status_line,
        }
    }
}

pub struct Value {
    pub display: String,
    pub value: String,
}
impl Value {
    pub fn new(display: &str, value: &str) -> Self {
        Self {
            display: display.to_string(),
            value: value.to_string(),
        }
    }
}

#[derive(Clone)]
pub struct ComboBoxValue {
    pub display: String,
    pub value: String,
}

impl ComboBoxValue {
    pub fn new(display: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            display: display.into(),
            value: value.into(),
        }
    }
}

pub struct ComboBox {
    pub values: Vec<ComboBoxValue>,
    pub cur_value: ComboBoxValue,
    pub first: usize,
    pub scroll_state: ScrollbarState,
}
impl ComboBox {
    fn handle_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Home => {
                self.first = 0;
                self.cur_value = self.values.get(0).unwrap().clone();
                self.scroll_state = self.scroll_state.position(0);
            }
            KeyCode::End => {
                self.first = self.values.len().saturating_sub(10);
                self.cur_value = self.values.get(self.values.len() - 1).unwrap().clone();
                self.scroll_state = self.scroll_state.position(self.values.len() - 1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let i = self.values.iter().position(|v| v.value == self.cur_value.value).unwrap_or(0).saturating_sub(1);
                self.cur_value = self.values.get(i).unwrap().clone();
                self.scroll_state = self.scroll_state.position(i);
                if i < self.first {
                    self.first = i;
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let i = self
                    .values
                    .iter()
                    .position(|v| v.value == self.cur_value.value)
                    .unwrap_or(0)
                    .saturating_add(1)
                    .min(self.values.len() - 1);
                self.cur_value = self.values.get(i).unwrap().clone();
                self.scroll_state = self.scroll_state.position(i);
                if i >= self.first + 10 {
                    self.first = i - 9;
                }
            }
            _ => {}
        }
    }
}

pub enum ListValue {
    ComboBox(ComboBox),
    Text(u16, String),
    Path(PathBuf),
    U32(u32, u32, u32),
    Bool(bool),
    Color(IcbColor),
    ValueList(String, Vec<Value>),
    Position(Box<dyn Fn(&mut Frame, &Position)>, Box<dyn Fn(KeyEvent, &Position) -> Position>, Position),
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

    pub fn with_label_width(mut self, width: u16) -> Self {
        while self.title.len() < width as usize {
            self.title.push(' ');
        }
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
            ListValue::ComboBox(v) => {
                Text::from(v.cur_value.display.clone()).style(THEME.value).render(area, frame.buffer_mut());
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
            ListValue::ValueList(cur_value, list) => {
                for l in list {
                    if l.value == *cur_value {
                        Text::from(l.display.clone()).style(THEME.value).render(area, frame.buffer_mut());
                        return;
                    }
                }
                Text::from(cur_value.clone()).style(THEME.value).render(area, frame.buffer_mut());
            }
            ListValue::Position(_, _, pos) => {
                Text::from(format!("x: {} y: {}", pos.x, pos.y))
                    .style(THEME.value)
                    .render(area, frame.buffer_mut());
            }
        }
    }

    fn render_editor(&mut self, area: Rect, frame: &mut Frame) -> bool {
        match &mut self.value {
            ListValue::Text(_edit_len, text) => {
                let field = TextField::new().with_value(text.to_string());
                frame.render_stateful_widget(field, area, &mut self.text_field_state);
            }

            ListValue::Path(text) => {
                let field = TextField::new().with_value(format!("{}", text.display()));
                frame.render_stateful_widget(field, area, &mut self.text_field_state);
            }
            ListValue::U32(value, _min, _max) => {
                let field = TextField::new().with_value(format!("{}", value));
                frame.render_stateful_widget(field, area, &mut self.text_field_state);
            }
            ListValue::Bool(_value) => {
                self.render_value(area, frame);
            }
            ListValue::Color(_value) => {
                self.render_value(area, frame);
            }
            ListValue::ValueList(cur_value, list) => {
                let mut area = area;
                area.width = list.iter().map(|l| l.display.len()).max().unwrap_or(0) as u16;

                for l in list {
                    if l.value == *cur_value {
                        Text::from(l.display.clone()).style(THEME.edit_value).render(area, frame.buffer_mut());
                        return true;
                    }
                }
                Text::from(cur_value.clone()).style(THEME.edit_value).render(area, frame.buffer_mut());
            }
            ListValue::ComboBox(c) => {
                let mut area = area;
                Clear.render(area, frame.buffer_mut());
                area.width = c.values.iter().map(|l| l.display.len()).max().unwrap_or(0) as u16 + 2;
                area.height = c.values.len().min(12) as u16;

                let block = Block::new()
                    //  .title(Title::from(Span::from(" Edit Action ").style(THEME.content_box_title)).alignment(Alignment::Center))
                    .style(THEME.content_box)
                    //  .padding(Padding::new(2, 2, 1, 1))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double);
                //     let area =  footer.inner(&Margin { vertical: 15, horizontal: 5 });
                block.render(area, frame.buffer_mut());

                let mut line = area;
                line.x += 1;
                line.width -= 2;
                line.y += 1;
                line.height = 1;
                for l in c.values.iter().skip(c.first).take(10) {
                    if l.value == c.cur_value.value {
                        Text::from(l.display.clone()).style(THEME.edit_value).render(line, frame.buffer_mut());
                    } else {
                        Text::from(l.display.clone()).style(THEME.value).render(line, frame.buffer_mut());
                    }
                    line.y += 1;
                }
                Scrollbar::new(ScrollbarOrientation::VerticalRight).render(area, frame.buffer_mut(), &mut c.scroll_state);
            }
            ListValue::Position(show_editor, _, pos) => {
                show_editor(frame, pos);
                return false;
            }
        }
        true
    }

    fn handle_key_press(&mut self, key: KeyEvent, state: &mut ConfigMenuState) -> ResultState {
        match key {
            KeyEvent { code: KeyCode::Enter, .. } => {
                state.in_edit = false;
                return ResultState::status_line(self.status.clone());
            }
            KeyEvent { code: KeyCode::Esc, .. } => {
                state.in_edit = false;
                return ResultState::status_line(self.status.clone());
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
            ListValue::U32(cur, min, max) => {
                let mut text = format!("{}", *cur);
                self.text_field_state.handle_input(key, &mut text);
                if let Ok(u) = text.parse::<u32>() {
                    *cur = u.clamp(*min, *max);
                }
            }
            ListValue::Bool(b) => {
                *b = !*b;
                return ResultState::default();
            }

            ListValue::ValueList(cur_value, list) => {
                for (i, l) in list.iter().enumerate() {
                    if l.value == *cur_value {
                        *cur_value = list[(i + 1) % list.len()].value.clone();
                        return ResultState::default();
                    }
                }
                *cur_value = list[0].value.clone();
                return ResultState::default();
            }

            ListValue::Color(_) => {}
            ListValue::ComboBox(combo) => {
                combo.handle_input(key);
            }
            ListValue::Position(_, input, pos) => {
                *pos = input(key, pos);
            }
        }
        return ResultState::status_line(self.status.clone());
    }

    fn request_edit_mode(&mut self, state: &mut ConfigMenuState) -> ResultState {
        match &mut self.value {
            ListValue::Bool(b) => {
                *b = !*b;
                state.in_edit = false;
                return ResultState::status_line(self.status.clone());
            }
            _ => ResultState::status_line(String::new()),
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
    fn _title_len(&self) -> u16 {
        match self {
            ConfigEntry::Item(item) => item.title.len() as u16,
            ConfigEntry::Group(_, items) => items.iter().map(|item| item._title_len()).max().unwrap_or(0),
            ConfigEntry::Table(_rows, _items) => 0,
            ConfigEntry::Separator => 0,
        }
    }
}

pub struct ConfigMenu {
    pub entry: Vec<ConfigEntry>,
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
        let mut y = 0;
        let mut x = 0;
        let mut i = 0;

        state.area_height = area.height;

        if !Self::display_list(&mut i, &mut self.entry, area, &mut y, &mut x, frame, state, false) {
            return;
        }

        let mut y = 0;
        let mut x = 0;
        let mut i = 0;
        if !Self::display_list(&mut i, &mut self.entry, area, &mut y, &mut x, frame, state, true) {
            return;
        }

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
        self.count_items(&self.entry, &mut len);
        len
    }

    pub fn get_item(&self, i: usize) -> Option<&ListItem> {
        let mut len = 0;
        Self::get_item_internal(&self.entry, &mut len, i)
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
        Self::get_item_internal_mut(&mut self.entry, &mut len, i)
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
        y: &mut u16,
        x: &mut u16,
        frame: &mut Frame,
        state: &mut ConfigMenuState,
        display_editor: bool,
    ) -> bool {
        let x1 = *x;
        let x2 = *x + area.width / 2;

        for (j, item) in items.iter_mut().enumerate() {
            match item {
                ConfigEntry::Item(item) => {
                    if *y >= state.first_row && *y < area.height + state.first_row {
                        let max = item.title.len() as u16;

                        let left_area = Rect {
                            x: area.x + *x,
                            y: area.y + *y - state.first_row,
                            width: max as u16 + 2,
                            height: 1,
                        };
                        if !display_editor {
                            item.render_label(left_area, frame, *i == state.selected, state.in_edit);
                        }
                        let xright = if *x >= x2 { area.right() - 1 } else { area.x + x2 };
                        let right_area = Rect {
                            x: left_area.right() + 1,
                            y: area.y + *y - state.first_row,
                            width: xright.saturating_sub(left_area.right() + 1),
                            height: 1,
                        };
                        if state.in_edit && *i == state.selected {
                            if display_editor {
                                if !item.render_editor(right_area, frame) {
                                    return false;
                                }
                            }
                        } else if !display_editor {
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
        true
    }
    pub fn display_list(
        i: &mut usize,
        items: &mut Vec<ConfigEntry>,
        area: Rect,
        y: &mut u16,
        x: &mut u16,
        frame: &mut Frame,
        state: &mut ConfigMenuState,
        display_editor: bool,
    ) -> bool {
        for item in items.iter_mut() {
            match item {
                ConfigEntry::Item(item) => {
                    if *y >= state.first_row && *y < area.height + state.first_row {
                        let max = item.title.len() as u16;
                        let left_area = Rect {
                            x: area.x + *x,
                            y: area.y + *y - state.first_row,
                            width: max + 2,
                            height: 1,
                        };
                        if !display_editor {
                            item.render_label(left_area, frame, *i == state.selected, state.in_edit);
                        }
                        let right_area = Rect {
                            x: area.x + *x + max + 3,
                            y: area.y + *y - state.first_row,
                            width: area.width - (*x + max + 3) - 2,
                            height: 1,
                        };
                        if state.in_edit && *i == state.selected {
                            if display_editor {
                                if !item.render_editor(right_area, frame) {
                                    return false;
                                }
                            }
                        } else if !display_editor {
                            item.render_value(right_area, frame);
                        }
                    }

                    state.item_pos.insert(*i, *y);
                    *y += 1;
                    *i += 1;
                }
                ConfigEntry::Group(title, items) => {
                    if !display_editor && !title.is_empty() {
                        if *y >= state.first_row && *y < area.height + state.first_row {
                            let left_area = Rect {
                                x: area.x + *x,
                                y: area.y + *y - state.first_row,
                                width: area.width - *x - 1,
                                height: 1,
                            };
                            if display_editor {
                                Text::from(format!(" {}", title.clone()))
                                    .alignment(ratatui::layout::Alignment::Left)
                                    .style(THEME.config_title.italic())
                                    .render(left_area, frame.buffer_mut());
                            }
                        }
                        *y += 1;
                    }
                    if !Self::display_list(i, items, area, y, x, frame, state, display_editor) {
                        return false;
                    }
                }
                ConfigEntry::Table(_cols, items) => {
                    if !Self::display_table(i, items, area, y, x, frame, state, display_editor) {
                        return false;
                    }
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
        true
    }

    pub fn handle_key_press(&mut self, key: KeyEvent, state: &mut ConfigMenuState) -> ResultState {
        if state.in_edit {
            return self.get_item_mut(state.selected).unwrap().handle_key_press(key, state);
        }

        match key.code {
            KeyCode::Home => {
                state.selected = 0;
                state.first_row = 0;
            }
            KeyCode::End => {
                state.selected = self.count() - 1;
                state.first_row = state.item_pos.get(&state.selected).unwrap_or(&0).saturating_sub(state.area_height) + 1;
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
                state.in_edit = !state.in_edit;
                return self.get_item_mut(state.selected).unwrap().request_edit_mode(state);
            }

            _ => {}
        }

        if let Some(item) = self.get_item(state.selected) {
            ResultState::status_line(item.status.clone())
        } else {
            log::error!("config_menu: no item found for index {}", state.selected);
            ResultState::default()
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

    pub fn iter(&self) -> impl Iterator<Item = &ListItem> {
        ConfigMenuIter { iter: vec![self.entry.iter()] }
    }
}

struct ConfigMenuIter<'a> {
    iter: Vec<std::slice::Iter<'a, ConfigEntry>>,
}
impl<'a> Iterator for ConfigMenuIter<'a> {
    type Item = &'a ListItem;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(mut l) = self.iter.pop() else {
            return None;
        };
        match l.next() {
            Some(a) => match a {
                ConfigEntry::Item(item) => {
                    self.iter.push(l);
                    Some(item)
                }
                ConfigEntry::Group(_, items) => {
                    self.iter.push(l);
                    self.iter.push(items.iter());
                    self.next()
                }
                ConfigEntry::Table(_, items) => {
                    self.iter.push(l);
                    self.iter.push(items.iter());
                    self.next()
                }
                ConfigEntry::Separator => {
                    self.iter.push(l);
                    self.next()
                }
            },
            None => self.next(),
        }
    }
}
