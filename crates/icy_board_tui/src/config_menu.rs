use std::{collections::HashMap, path::PathBuf, str::FromStr};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    datetime::{IcbDate, IcbDoW, IcbTime},
    icy_board::{commands::Position, icb_config::IcbColor, security_expr::SecurityExpression},
};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Stylize,
    text::Text,
    widgets::{Block, BorderType, Borders, Clear, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget},
};

use crate::{
    tab_page::PageMessage,
    text_field::{TextField, TextfieldState},
    theme::get_tui_theme,
};

#[derive(Default)]
pub enum EditMode {
    #[default]
    None,
    Open(String, PathBuf),
    ExternalProgramStarted,
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

#[derive(Clone, PartialEq)]
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
    /// value, min, max
    U32(u32, u32, u32),
    /// float, cur_edit_string
    Float(f64, String),
    Date(IcbDate, String),
    Time(IcbTime, String),
    DoW(IcbDoW, String),
    Bool(bool),
    Color(IcbColor),
    ValueList(String, Vec<Value>),
    Security(SecurityExpression, String),
    Position(Box<dyn Fn(&mut Frame, &Position)>, Box<dyn Fn(KeyEvent, &Position) -> Position>, Position),
}

pub struct ListItem<T> {
    title: String,
    label_width: u16,
    label_alignment: Alignment,
    edit_width: u16,
    editable: bool,
    pub status: String,
    pub text_field_state: TextfieldState,
    pub value: ListValue,
    pub help: String,

    pub update_value: Option<Box<dyn Fn(&T, &ListValue) -> ()>>,

    pub path_editor: Option<Box<dyn Fn(T, PathBuf) -> PageMessage>>,
}

impl<T> ListItem<T> {
    pub fn new(title: String, value: ListValue) -> Self {
        Self {
            status: format!("{}", title),
            text_field_state: TextfieldState::default(),
            label_width: title.len() as u16,
            label_alignment: Alignment::Left,
            title,
            value,
            update_value: None,
            help: String::new(),
            edit_width: 0,
            path_editor: None,
            editable: true,
        }
    }

    pub fn editable(&self) -> bool {
        self.editable
    }

    pub fn with_editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = status.into();
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = help.into();
        self
    }

    pub fn with_label_width(mut self, width: u16) -> Self {
        self.label_width = width;
        self
    }

    pub fn with_label_alignment(mut self, alignment: Alignment) -> Self {
        self.label_alignment = alignment;
        self
    }

    pub fn with_update_value(mut self, update_value: Box<dyn Fn(&T, &ListValue) -> ()>) -> Self {
        self.update_value = Some(update_value);
        self
    }

    pub fn with_edit_width(mut self, width: u16) -> Self {
        self.edit_width = width;
        self
    }

    pub fn with_path_editor(mut self, editor: Box<dyn Fn(T, PathBuf) -> PageMessage>) -> Self {
        self.path_editor = Some(editor);
        self
    }

    pub fn with_update_text_value(mut self, update_value: &'static dyn Fn(&T, String) -> ()) -> Self {
        let b: Box<dyn Fn(&T, &ListValue) -> ()> = Box::new(move |val: &T, value: &ListValue| {
            let ListValue::Text(_, text) = value else {
                return;
            };
            update_value(val, text.clone());
        });
        self.update_value = Some(b);
        self
    }

    pub fn with_update_combobox_value(mut self, update_value: &'static dyn Fn(&T, &ComboBox) -> ()) -> Self {
        let b: Box<dyn Fn(&T, &ListValue) -> ()> = Box::new(move |val: &T, value: &ListValue| {
            let ListValue::ComboBox(b) = value else {
                return;
            };
            update_value(val, b);
        });
        self.update_value = Some(b);
        self
    }

    pub fn with_update_bool_value(mut self, update_value: &'static dyn Fn(&T, bool) -> ()) -> Self {
        let b: Box<dyn Fn(&T, &ListValue) -> ()> = Box::new(move |val: &T, value: &ListValue| {
            let ListValue::Bool(b) = value else {
                return;
            };
            update_value(val, *b);
        });
        self.update_value = Some(b);
        self
    }

    pub fn with_update_u32_value(mut self, update_value: &'static dyn Fn(&T, u32) -> ()) -> Self {
        let b: Box<dyn Fn(&T, &ListValue) -> ()> = Box::new(move |val: &T, value: &ListValue| {
            let ListValue::U32(b, _, _) = value else {
                return;
            };
            update_value(val, *b);
        });
        self.update_value = Some(b);
        self
    }

    pub fn with_update_path_value(mut self, update_value: &'static dyn Fn(&T, PathBuf) -> ()) -> Self {
        let b: Box<dyn Fn(&T, &ListValue) -> ()> = Box::new(move |val: &T, value: &ListValue| {
            let ListValue::Path(b) = value else {
                return;
            };
            update_value(val, b.clone());
        });
        self.update_value = Some(b);
        self
    }

    pub fn with_update_float_value(mut self, update_value: &'static dyn Fn(&T, f64) -> ()) -> Self {
        let b: Box<dyn Fn(&T, &ListValue) -> ()> = Box::new(move |val: &T, value: &ListValue| {
            let ListValue::Float(f, _str) = value else {
                return;
            };
            update_value(val, *f);
        });
        self.update_value = Some(b);
        self
    }

    pub fn with_update_sec_value(mut self, update_value: &'static dyn Fn(&T, SecurityExpression) -> ()) -> Self {
        let b: Box<dyn Fn(&T, &ListValue) -> ()> = Box::new(move |val: &T, value: &ListValue| {
            let ListValue::Security(b, _) = value else {
                return;
            };
            update_value(val, b.clone());
        });
        self.update_value = Some(b);
        self
    }

    pub fn with_update_date_value(mut self, update_value: &'static dyn Fn(&T, IcbDate) -> ()) -> Self {
        let b: Box<dyn Fn(&T, &ListValue) -> ()> = Box::new(move |val: &T, value: &ListValue| {
            let ListValue::Date(b, _) = value else {
                return;
            };
            update_value(val, b.clone());
        });
        self.update_value = Some(b);
        self
    }

    pub fn with_update_time_value(mut self, update_value: &'static dyn Fn(&T, IcbTime) -> ()) -> Self {
        let b: Box<dyn Fn(&T, &ListValue) -> ()> = Box::new(move |val: &T, value: &ListValue| {
            let ListValue::Time(b, _) = value else {
                return;
            };
            update_value(val, b.clone());
        });
        self.update_value = Some(b);
        self
    }

    fn render_label(&self, left_area: Rect, frame: &mut Frame, selected: bool, in_edit: bool) {
        Text::from(self.title.clone())
            .alignment(self.label_alignment)
            .style(if selected && !in_edit {
                get_tui_theme().selected_item
            } else {
                get_tui_theme().item
            })
            .render(left_area, frame.buffer_mut());
    }

    fn measure_value(&self, area: Rect) -> usize {
        let a = if self.edit_width > 0 {
            self.edit_width as usize
        } else {
            match &self.value {
                ListValue::Text(len, _text) => *len as usize,
                ListValue::ComboBox(v) => v.cur_value.display.len(),

                ListValue::Path(_) => area.width as usize,

                ListValue::U32(_u, _min, max) => max.ilog10() as usize + 1,

                ListValue::Float(_u, _) => 5,

                ListValue::Color(_color) => 2,
                ListValue::Bool(_value) => 3,
                ListValue::Security(_e, _) => 6,
                ListValue::ValueList(_cur_value, _list) => {
                    /*
                    for l in list {
                        if l.value == *cur_value {
                            Text::from(l.display.clone()).style(get_tui_theme().value).render(area, frame.buffer_mut());
                            return;
                        }
                    }
                    Text::from(cur_value.clone()).style(get_tui_theme().value).render(area, frame.buffer_mut());*/
                    8
                }
                ListValue::Position(_, _, pos) => pos.x as usize,

                ListValue::Time(_, _) => 5, // 00:00
                ListValue::Date(_, _) => 8, // 00/00/00
                ListValue::DoW(_, _) => 7,  // SMDMDFS
            }
        };

        a + (self.label_width as usize) + 3
    }

    fn render_value(&self, area: Rect, frame: &mut Frame) {
        match &self.value {
            ListValue::Text(_, text) => {
                Text::from(text.clone()).style(get_tui_theme().value).render(area, frame.buffer_mut());
            }
            ListValue::ComboBox(v) => {
                Text::from(v.cur_value.display.clone())
                    .style(get_tui_theme().value)
                    .render(area, frame.buffer_mut());
            }

            ListValue::Path(text) => {
                Text::from(format!("{}", text.display()))
                    .style(get_tui_theme().value)
                    .render(area, frame.buffer_mut());
            }

            ListValue::U32(u, _min, _max) => {
                let val = u.to_string();
                let area = Rect {
                    x: area.x,
                    y: area.y,
                    width: val.len() as u16,
                    height: 1,
                };
                Text::from(val).style(get_tui_theme().value).render(area, frame.buffer_mut());
            }

            ListValue::Float(_val, str) => {
                Text::from(str.clone()).style(get_tui_theme().value).render(area, frame.buffer_mut());
            }

            ListValue::Time(_val, str) => {
                let area = Rect {
                    x: area.x,
                    y: area.y,
                    width: 5,
                    height: 1,
                };
                Text::from(str.clone()).style(get_tui_theme().value).render(area, frame.buffer_mut());
            }
            ListValue::Date(_val, str) => {
                let area = Rect {
                    x: area.x,
                    y: area.y,
                    width: 8,
                    height: 1,
                };
                Text::from(str.clone()).style(get_tui_theme().value).render(area, frame.buffer_mut());
            }

            ListValue::DoW(_val, str) => {
                let area = Rect {
                    x: area.x,
                    y: area.y,
                    width: 7,
                    height: 1,
                };
                Text::from(str.clone()).style(get_tui_theme().value).render(area, frame.buffer_mut());
            }

            ListValue::Security(_val, str) => {
                let area = Rect {
                    x: area.x,
                    y: area.y,
                    width: 6,
                    height: 1,
                };
                Text::from(str.clone()).style(get_tui_theme().value).render(area, frame.buffer_mut());
            }

            ListValue::Color(color) => match color {
                IcbColor::None => Text::from("Plain").style(get_tui_theme().value).render(area, frame.buffer_mut()),
                IcbColor::Dos(u8) => Text::from(format!("{:02X}", *u8)).style(get_tui_theme().value).render(area, frame.buffer_mut()),
                IcbColor::IcyEngine(_) => todo!(),
            },
            ListValue::Bool(value) => {
                let title: &str = if *value { " ✓ " } else { " ✗ " };
                let area = Rect {
                    x: area.x,
                    y: area.y,
                    width: 3,
                    height: 1,
                };
                Text::from(title)
                    .style(if *value { get_tui_theme().true_value } else { get_tui_theme().false_value })
                    .render(area, frame.buffer_mut());
            }
            ListValue::ValueList(cur_value, list) => {
                for l in list {
                    if l.value == *cur_value {
                        Text::from(l.display.clone()).style(get_tui_theme().value).render(area, frame.buffer_mut());
                        return;
                    }
                }
                Text::from(cur_value.clone()).style(get_tui_theme().value).render(area, frame.buffer_mut());
            }
            ListValue::Position(_, _, pos) => {
                Text::from(format!("x: {} y: {}", pos.x, pos.y))
                    .style(get_tui_theme().value)
                    .render(area, frame.buffer_mut());
            }
        }
    }

    fn render_editor(&mut self, val: &T, mut area: Rect, frame: &mut Frame) -> bool {
        match &mut self.value {
            ListValue::Text(edit_len, text) => {
                let field = TextField::new().with_value(text.to_string());
                area.width = area.width.min(*edit_len);
                frame.render_stateful_widget(field, area, &mut self.text_field_state);
                self.text_field_state.set_cursor_position(frame);
            }

            ListValue::Path(text) => {
                let field = TextField::new().with_value(format!("{}", text.display()));
                frame.render_stateful_widget(field, area, &mut self.text_field_state);
                self.text_field_state.set_cursor_position(frame);
            }
            ListValue::U32(value, _min, max) => {
                let area = Rect {
                    x: area.x,
                    y: area.y,
                    width: max.ilog10() as u16 + 1,
                    height: 1,
                };
                let field = TextField::new().with_value(format!("{}", value));
                frame.render_stateful_widget(field, area, &mut self.text_field_state);
                self.text_field_state.set_cursor_position(frame);
            }
            ListValue::Float(_value, str) => {
                let field = TextField::new().with_value(str.clone());
                frame.render_stateful_widget(field, area, &mut self.text_field_state);
                self.text_field_state.set_cursor_position(frame);
            }
            ListValue::Time(_val, str) => {
                let area = Rect {
                    x: area.x,
                    y: area.y,
                    width: 5,
                    height: 1,
                };
                let field = TextField::new().with_max_len(5).with_value(str.clone());
                frame.render_stateful_widget(field, area, &mut self.text_field_state);
                self.text_field_state.set_cursor_position(frame);
            }
            ListValue::Date(_val, str) => {
                let area = Rect {
                    x: area.x,
                    y: area.y,
                    width: 8,
                    height: 1,
                };
                let field = TextField::new().with_max_len(8).with_value(str.clone());
                frame.render_stateful_widget(field, area, &mut self.text_field_state);
                self.text_field_state.set_cursor_position(frame);
            }
            ListValue::DoW(_val, str) => {
                let area = Rect {
                    x: area.x,
                    y: area.y,
                    width: 7,
                    height: 1,
                };
                let field = TextField::new().with_max_len(7).with_value(str.clone());
                frame.render_stateful_widget(field, area, &mut self.text_field_state);
                self.text_field_state.set_cursor_position(frame);
            }

            ListValue::Security(_val, str) => {
                let area = Rect {
                    x: area.x,
                    y: area.y,
                    width: 6,
                    height: 1,
                };
                let field = TextField::new().with_value(str.clone());
                frame.render_stateful_widget(field, area, &mut self.text_field_state);
                self.text_field_state.set_cursor_position(frame);
            }
            ListValue::Bool(value) => {
                let title = if *value { " ✓ " } else { " ✗ " };
                Text::from(title).style(get_tui_theme().edit_value).render(
                    Rect {
                        x: area.x,
                        y: area.y,
                        width: 3,
                        height: 1,
                    },
                    frame.buffer_mut(),
                );
                frame.set_cursor_position((area.x, area.y));
            }
            ListValue::Color(value) => {
                if let IcbColor::Dos(value) = value {
                    let area = Rect {
                        x: area.x,
                        y: area.y,
                        width: 2,
                        height: 1,
                    };
                    let field = TextField::new().with_value(format!("{:02X}", value));
                    frame.render_stateful_widget(field, area, &mut self.text_field_state);
                    self.text_field_state.set_cursor_position(frame);
                }
            }
            ListValue::ValueList(cur_value, list) => {
                let mut area = area;
                area.width = list.iter().map(|l| l.display.len()).max().unwrap_or(0) as u16;

                for l in list {
                    if l.value == *cur_value {
                        Text::from(l.display.clone()).style(get_tui_theme().edit_value).render(area, frame.buffer_mut());
                        return true;
                    }
                }
                Text::from(cur_value.clone()).style(get_tui_theme().edit_value).render(area, frame.buffer_mut());
            }
            ListValue::ComboBox(c) => {
                let mut area = area;
                area.width = c.values.iter().map(|l| l.display.len()).max().unwrap_or(0) as u16 + 2;
                area.height = (c.values.len() + 2).min(12) as u16;
                Clear.render(area, frame.buffer_mut());

                let block = Block::new()
                    //  .title(Title::from(Span::from(" Edit Action ").style(THEME.content_box_title)).alignment(Alignment::Center))
                    .style(get_tui_theme().dialog_box)
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
                        Text::from(l.display.clone()).style(get_tui_theme().edit_value).render(line, frame.buffer_mut());
                    } else {
                        Text::from(l.display.clone()).style(get_tui_theme().value).render(line, frame.buffer_mut());
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

        if let Some(update) = self.update_value.as_ref() {
            update(val, &self.value);
        }
        true
    }

    fn handle_key_press(&mut self, key: KeyEvent, _state: &mut ConfigMenuState) -> ResultState {
        match key {
            KeyEvent { code: KeyCode::Enter, .. } => {
                return ResultState::status_line(self.status.clone());
            }
            KeyEvent { code: KeyCode::Esc, .. } => {
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
            ListValue::Float(val, str) => {
                self.text_field_state.handle_input(key, str);
                if let Ok(f) = str.parse::<f64>() {
                    *val = f;
                }
            }
            ListValue::Time(val, str) => {
                self.text_field_state.handle_input(key, str);
                *val = IcbTime::parse(str);
            }
            ListValue::Date(val, str) => {
                self.text_field_state.handle_input(key, str);
                *val = IcbDate::parse(str);
            }
            ListValue::DoW(val, str) => {
                self.text_field_state.handle_input(key, str);
                *val = IcbDoW::from(str.clone());
            }

            ListValue::Security(val, str) => {
                self.text_field_state.handle_input(key, str);
                if let Ok(res) = SecurityExpression::from_str(str) {
                    *val = res;
                }
            }
            ListValue::U32(cur, min, max) => {
                let mut text = format!("{}", *cur);
                self.text_field_state.handle_input(key, &mut text);
                if let Ok(u) = text.parse::<u32>() {
                    *cur = u.clamp(*min, *max);
                }
            }
            ListValue::Bool(b) => {
                match key.code {
                    KeyCode::BackTab | KeyCode::Left | KeyCode::Tab | KeyCode::Right | KeyCode::Char(' ') => {
                        *b = !*b;
                    }
                    _ => {}
                }
                return ResultState::default();
            }
            ListValue::ValueList(cur_value, list) => {
                for (i, l) in list.iter().enumerate() {
                    if l.value == *cur_value {
                        match key.code {
                            KeyCode::BackTab | KeyCode::Left => {
                                *cur_value = list[(i + list.len() - 1) % list.len()].value.clone();
                                return ResultState::default();
                            }
                            KeyCode::Tab | KeyCode::Right => {
                                *cur_value = list[(i + 1) % list.len()].value.clone();
                                return ResultState::default();
                            }
                            _ => {}
                        }
                        return ResultState::default();
                    }
                }
                *cur_value = list[0].value.clone();
                return ResultState::default();
            }

            ListValue::Color(col) => {
                if let IcbColor::Dos(u) = col {
                    let mut text = format!("{:02X}", u);
                    self.text_field_state.handle_input(key, &mut text);
                    if let Ok(u) = u8::from_str_radix(&text, 16) {
                        *col = IcbColor::Dos(u);
                    }
                }
            }
            ListValue::ComboBox(combo) => {
                combo.handle_input(key);
            }
            ListValue::Position(_, input, pos) => {
                *pos = input(key, pos);
            }
        }
        return ResultState::status_line(self.status.clone());
    }
    /*
    fn request_edit_mode(&mut self, _state: &mut ConfigMenuState) -> ResultState {
        match &mut self.value {
            ListValue::Bool(b) => {
                *b = !*b;
                return ResultState::status_line(self.status.clone());
            }
            _ => ResultState::status_line(String::new()),
        }
    }*/
}

pub enum ConfigEntry<T> {
    Item(ListItem<T>),
    Group(String, Vec<ConfigEntry<T>>),
    Table(usize, Vec<ConfigEntry<T>>),
    Label(String),
    Separator,
}

impl<T> ConfigEntry<T> {
    fn _title_len(&self) -> u16 {
        match self {
            ConfigEntry::Item(item) => item.title.len() as u16,
            ConfigEntry::Group(_, items) => items.iter().map(|item| item._title_len()).max().unwrap_or(0),
            ConfigEntry::Table(_rows, _items) => 0,
            ConfigEntry::Label(_) | ConfigEntry::Separator => 0,
        }
    }

    pub fn with_editable(mut self, editable: bool) -> Self {
        match &mut self {
            ConfigEntry::Item(item) => {
                item.editable = editable;
            }
            _ => {}
        }
        self
    }

    fn measure_value(&self, area: Rect) -> u16 {
        match self {
            ConfigEntry::Item(item) => item.measure_value(area) as u16,
            ConfigEntry::Group(_, items) => items.iter().map(|item| item.measure_value(area)).max().unwrap_or(0),
            ConfigEntry::Table(_, items) => items.iter().map(|item| item.measure_value(area)).max().unwrap_or(0),
            ConfigEntry::Label(_) | ConfigEntry::Separator => 0,
        }
    }
}

pub struct ConfigMenu<T> {
    pub obj: T,
    pub entry: Vec<ConfigEntry<T>>,
}

#[derive(Default)]
pub struct ConfigMenuState {
    pub selected: usize,
    pub first_row: u16,
    pub area_height: u16,

    pub item_pos: HashMap<usize, u16>,

    pub scroll_state: ScrollbarState,
}

impl<T> ConfigMenu<T> {
    pub fn render(&mut self, area: Rect, frame: &mut Frame, state: &mut ConfigMenuState) {
        let mut y = 0;
        let mut x = 0;
        let mut i = 0;

        state.area_height = area.height;

        let list_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width - 1,
            height: area.height,
        };

        if !Self::display_list(&self.obj, &mut i, &mut self.entry, list_area, &mut y, &mut x, frame, state, false) {
            return;
        }
        let content_length = y.saturating_sub(area.height);

        let mut y = 0;
        let mut x = 0;
        let mut i = 0;
        if !Self::display_list(&self.obj, &mut i, &mut self.entry, list_area, &mut y, &mut x, frame, state, true) {
            return;
        }

        state.scroll_state = ScrollbarState::new(content_length as usize).position(state.first_row as usize);
        Self::render_scrollbar(state, frame, area);
    }

    fn render_scrollbar(state: &mut ConfigMenuState, frame: &mut Frame, mut area: Rect) {
        area.x -= 1;

        frame.render_stateful_widget(
            Scrollbar::default()
                .style(get_tui_theme().dialog_box_scrollbar)
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

    pub fn get_item(&self, i: usize) -> Option<&ListItem<T>> {
        let mut len = 0;
        Self::get_item_internal(&self.entry, &mut len, i)
    }

    pub fn get_item_internal<'a>(items: &'a Vec<ConfigEntry<T>>, len: &mut usize, i: usize) -> Option<&'a ListItem<T>> {
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

    pub fn get_item_mut(&mut self, i: usize) -> Option<&mut ListItem<T>> {
        let mut len = 0;
        Self::get_item_internal_mut(&mut self.entry, &mut len, i)
    }

    pub fn get_item_internal_mut<'a>(items: &'a mut Vec<ConfigEntry<T>>, len: &mut usize, i: usize) -> Option<&'a mut ListItem<T>> {
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

    fn count_items(&self, items: &Vec<ConfigEntry<T>>, len: &mut usize) {
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
        val: &T,
        i: &mut usize,
        items: &mut Vec<ConfigEntry<T>>,
        area: Rect,
        y: &mut u16,
        x: &mut u16,
        frame: &mut Frame,
        state: &mut ConfigMenuState,
        display_editor: bool,
    ) -> bool {
        let x1 = *x;
        let mut x2 = 0;

        for (j, item) in items.iter_mut().enumerate() {
            if j % 2 == 0 {
                x2 = x2.max(item.measure_value(area) as u16);
            }
        }

        x2 += *x;

        for (j, item) in items.iter_mut().enumerate() {
            match item {
                ConfigEntry::Item(item) => {
                    if *y >= state.first_row && *y < area.height + state.first_row {
                        let left_area = Rect {
                            x: area.x + *x,
                            y: area.y + *y - state.first_row,
                            width: item.label_width,
                            height: 1,
                        };

                        if !display_editor {
                            item.render_label(left_area, frame, *i == state.selected, true);
                        }
                        let xright = if *x >= x2 { area.right() - 1 } else { area.x + x2 };

                        Text::from(":").style(get_tui_theme().item_separator).render(
                            Rect {
                                x: left_area.left() + item.label_width + 1,
                                y: area.y + *y - state.first_row,
                                width: 1,
                                height: 1,
                            },
                            frame.buffer_mut(),
                        );

                        let right_area = Rect {
                            x: left_area.left() + item.label_width + 3,
                            y: area.y + *y - state.first_row,
                            width: if item.edit_width > 0 {
                                item.edit_width
                            } else {
                                xright.saturating_sub(item.label_width + 1)
                            },
                            height: 1,
                        };

                        if *i == state.selected {
                            if display_editor {
                                if !item.render_editor(val, right_area, frame) {
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
        val: &T,
        i: &mut usize,
        items: &mut Vec<ConfigEntry<T>>,
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
                        let left_area = Rect {
                            x: area.x + *x,
                            y: (area.y + *y).saturating_sub(state.first_row),
                            width: item.label_width,
                            height: 1,
                        };
                        item.render_label(left_area, frame, *i == state.selected, true);

                        Text::from(":").style(get_tui_theme().item_separator).render(
                            Rect {
                                x: left_area.left() + item.label_width + 1,
                                y: area.y + *y - state.first_row,
                                width: 1,
                                height: 1,
                            },
                            frame.buffer_mut(),
                        );

                        let right_area = Rect {
                            x: left_area.left() + item.label_width + 3,
                            y: area.y + *y - state.first_row,
                            width: area.right().saturating_sub(left_area.right() + 5),
                            height: 1,
                        };
                        if *i == state.selected {
                            if display_editor {
                                if !item.render_editor(val, right_area, frame) {
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
                    if !title.is_empty() {
                        if !display_editor {
                            if *y >= state.first_row && *y < area.height + state.first_row {
                                let left_area = Rect {
                                    x: area.x + *x,
                                    y: area.y + y.saturating_sub(state.first_row),
                                    width: area.width.saturating_sub(*x + 1),
                                    height: 1,
                                };
                                Text::from(format!(" {}", title.clone()))
                                    .alignment(ratatui::layout::Alignment::Left)
                                    .style(get_tui_theme().config_title.italic())
                                    .render(left_area, frame.buffer_mut());
                            }
                        }
                        *y += 1;
                    }
                    if !Self::display_list(val, i, items, area, y, x, frame, state, display_editor) {
                        return false;
                    }
                }
                ConfigEntry::Table(_cols, items) => {
                    if !Self::display_table(val, i, items, area, y, x, frame, state, display_editor) {
                        return false;
                    }
                }
                ConfigEntry::Label(text) => {
                    let left_area = Rect {
                        x: area.x + *x,
                        y: area.y + y.saturating_sub(state.first_row),
                        width: area.width.saturating_sub(*x + 1),
                        height: 1,
                    };

                    Text::from(text.as_str())
                        .alignment(ratatui::layout::Alignment::Left)
                        .style(get_tui_theme().menu_label)
                        .render(left_area, frame.buffer_mut());
                    *y += 1;
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
        match key.code {
            KeyCode::Up => Self::prev(self.count(), state),
            KeyCode::Down | KeyCode::Enter => Self::next(self.count(), state),
            _ => {
                return self.get_item_mut(state.selected).unwrap().handle_key_press(key, state);
            }
        }

        if let Some(item) = self.get_item(state.selected) {
            ResultState::status_line(item.status.clone())
        } else {
            log::error!("config_menu: no item found for index {}", state.selected);
            ResultState::default()
        }
    }

    fn prev(count: usize, state: &mut ConfigMenuState) {
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
        } else {
            state.selected = count - 1;
            if let Some(y) = state.item_pos.get(&state.selected) {
                if *y >= state.area_height {
                    state.first_row = *y - state.area_height + 1;
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
        } else {
            state.selected = 0;

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

    pub fn iter(&self) -> impl Iterator<Item = &ListItem<T>> {
        ConfigMenuIter { iter: vec![self.entry.iter()] }
    }

    pub fn current_status_line(&self, state: &ConfigMenuState) -> String {
        if let Some(item) = self.get_item(state.selected) {
            return item.status.clone();
        }
        String::new()
    }
}

struct ConfigMenuIter<'a, T> {
    iter: Vec<std::slice::Iter<'a, ConfigEntry<T>>>,
}
impl<'a, T> Iterator for ConfigMenuIter<'a, T> {
    type Item = &'a ListItem<T>;

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
                ConfigEntry::Label(_) | ConfigEntry::Separator => {
                    self.iter.push(l);
                    self.next()
                }
            },
            None => self.next(),
        }
    }
}
