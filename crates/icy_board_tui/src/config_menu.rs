use std::{collections::HashMap, path::PathBuf, str::FromStr};

use chrono::{DateTime, Datelike, Utc};
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

#[derive(Default, PartialEq)]
pub enum EditMessage {
    #[default]
    None,
    PrevItem,
    NextItem,
    Close,
    Open(String, PathBuf),
    ExternalProgramStarted,
    DisplayHelp(String),
}

#[derive(Default)]
pub struct ResultState {
    pub edit_msg: EditMessage,
    pub status_line: String,
}
impl ResultState {
    pub fn status_line(status_line: String) -> ResultState {
        ResultState {
            edit_msg: EditMessage::None,
            status_line,
        }
    }
    pub fn up() -> ResultState {
        ResultState {
            edit_msg: EditMessage::PrevItem,
            status_line: String::new(),
        }
    }
    pub fn down() -> ResultState {
        ResultState {
            edit_msg: EditMessage::NextItem,
            status_line: String::new(),
        }
    }
    pub fn close() -> ResultState {
        ResultState {
            edit_msg: EditMessage::Close,
            status_line: String::new(),
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
    pub is_edit_open: bool,
    pub values: Vec<ComboBoxValue>,
    pub cur_value: ComboBoxValue,
    pub selected_item: usize,
    pub first_item: usize,
}

impl ComboBox {
    fn handle_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Home => {
                self.selected_item = 0;
                self.first_item = 0;
            }
            KeyCode::End => {
                self.selected_item = self.values.len().saturating_sub(1);
                self.first_item = self.values.len().saturating_sub(4);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected_item > 0 {
                    self.selected_item = self.selected_item.saturating_sub(1);
                    if self.selected_item < self.first_item {
                        self.first_item = self.selected_item;
                    }
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.selected_item < self.values.len().saturating_sub(1) {
                    self.selected_item = self.selected_item + 1;
                    if self.selected_item >= self.first_item + 4 {
                        self.first_item = self.selected_item - 3;
                    }
                }
            }
            _ => {}
        }
    }
}

pub enum TextFlags {
    None,
    Password,
}

pub enum ListValue {
    ComboBox(ComboBox),
    Text(u16, TextFlags, String),
    Path(PathBuf),
    /// value, min, max
    U32(u32, u32, u32),
    /// float, cur_edit_string
    Float(f64, String),
    Date(DateTime<Utc>, DateEditState),
    Time(IcbTime, String),
    DoW(IcbDoW, String),
    Bool(bool),
    Color(IcbColor),
    ValueList(String, Vec<Value>),
    Security(SecurityExpression, String),
    Position(Box<dyn Fn(&mut Frame, &Position)>, Box<dyn Fn(KeyEvent, &Position) -> Position>, Position),
}

#[derive(Clone, Debug)]
pub struct DateEditState {
    pub text_buffer: String,
    pub cursor_field: DateField, // Month, Day, Year
    pub editing_mode: DateEditMode,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DateField {
    Month,
    Day,
    Year,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DateEditMode {
    Text,   // Current text entry mode
    Picker, // Interactive picker mode
}

impl DateEditState {
    pub fn from_date(dt: &DateTime<Utc>) -> Self {
        Self {
            text_buffer: IcbDate::from_utc(dt).to_pcb_str(),
            cursor_field: DateField::Month,
            editing_mode: DateEditMode::Picker,
        }
    }

    pub fn increment_field(&mut self, dt: &mut DateTime<Utc>) {
        let date = dt.date_naive();
        *dt = match self.cursor_field {
            DateField::Month => {
                if date.month() == 12 {
                    date.with_month(1).unwrap_or(date)
                } else {
                    date.with_month(date.month() + 1).unwrap_or(date)
                }
            }
            DateField::Day => (*dt + chrono::Duration::days(1)).date_naive(),
            DateField::Year => date.with_year(date.year() + 1).unwrap_or(date),
        }
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();
        self.text_buffer = IcbDate::from_utc(dt).to_pcb_str();
    }

    pub fn decrement_field(&mut self, dt: &mut DateTime<Utc>) {
        let date = dt.date_naive();
        *dt = match self.cursor_field {
            DateField::Month => {
                if date.month() == 1 {
                    date.with_month(12).unwrap_or(date)
                } else {
                    date.with_month(date.month() - 1).unwrap_or(date)
                }
            }
            DateField::Day => (*dt - chrono::Duration::days(1)).date_naive(),
            DateField::Year => date.with_year(date.year() - 1).unwrap_or(date),
        }
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();
        self.text_buffer = IcbDate::from_utc(dt).to_pcb_str();
    }
}

impl ListValue {
    pub fn date(date: DateTime<Utc>) -> Self {
        ListValue::Date(date, DateEditState::from_date(&date))
    }
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
    need_update: bool,

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
            need_update: false,
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
            let ListValue::Text(_, _, text) = value else {
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

    pub fn with_update_date_value(mut self, update_value: &'static dyn Fn(&T, DateTime<Utc>) -> ()) -> Self {
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
                ListValue::Text(len, _, _text) => *len as usize,
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
            ListValue::Text(_, flags, text) => {
                let text = if matches!(flags, TextFlags::Password) {
                    "*".repeat(text.len().min(6))
                } else {
                    text.clone()
                };
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
            ListValue::Date(_val, state) => {
                let area = Rect {
                    x: area.x,
                    y: area.y,
                    width: 8,
                    height: 1,
                };
                Text::from(state.text_buffer.clone())
                    .style(get_tui_theme().value)
                    .render(area, frame.buffer_mut());
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
            ListValue::Text(edit_len, _, text) => {
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
            ListValue::Date(_val, state) => {
                let area = Rect {
                    x: area.x,
                    y: area.y,
                    width: 10, // MM-DD-YY
                    height: 1,
                };

                if state.editing_mode == DateEditMode::Picker {
                    // Render with field highlighting
                    let date_str = &state.text_buffer;
                    let parts: Vec<&str> = date_str.split('-').collect();

                    let mut x = area.x;
                    for (i, (part, is_selected)) in [
                        (parts.get(0).unwrap_or(&"??"), state.cursor_field == DateField::Month),
                        (parts.get(1).unwrap_or(&"??"), state.cursor_field == DateField::Day),
                        (parts.get(2).unwrap_or(&"??"), state.cursor_field == DateField::Year),
                    ]
                    .iter()
                    .enumerate()
                    {
                        let style = if *is_selected {
                            get_tui_theme().edit_value.reversed() // Highlight current field
                        } else {
                            get_tui_theme().edit_value
                        };

                        Text::from(part.to_string()).style(style).render(
                            Rect {
                                x,
                                y: area.y,
                                width: part.len() as u16,
                                height: 1,
                            },
                            frame.buffer_mut(),
                        );

                        x += part.len() as u16;
                        if i < 2 {
                            Text::from("-").style(get_tui_theme().edit_value).render(
                                Rect {
                                    x,
                                    y: area.y,
                                    width: 1,
                                    height: 1,
                                },
                                frame.buffer_mut(),
                            );
                            x += 1;
                        }
                    }
                    frame.set_cursor_position((area.x, area.y));
                } else {
                    // Fallback to text mode
                    let field = TextField::new().with_max_len(8).with_value(state.text_buffer.clone());
                    frame.render_stateful_widget(field, area, &mut self.text_field_state);
                    self.text_field_state.set_cursor_position(frame);
                }
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
                if !c.is_edit_open {
                    Text::from(c.cur_value.display.clone())
                        .style(get_tui_theme().edit_value)
                        .render(area, frame.buffer_mut());

                    if let Some(update) = self.update_value.as_ref() {
                        update(val, &self.value);
                    }
                    return true;
                }
                let mut area = area;
                area.width = c.values.iter().map(|l| l.display.len()).max().unwrap_or(0) as u16 + 2;
                area.height = (c.values.len() + 2).clamp(3, 6) as u16;
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
                for (i, l) in c.values.iter().skip(c.first_item).take((area.height as usize).saturating_sub(2)).enumerate() {
                    if i + c.first_item == c.selected_item {
                        Text::from(l.display.clone()).style(get_tui_theme().edit_value).render(line, frame.buffer_mut());
                    } else {
                        Text::from(l.display.clone()).style(get_tui_theme().value).render(line, frame.buffer_mut());
                    }
                    line.y += 1;
                }
                let mut scroll_state = ScrollbarState::new(c.values.len()).position(c.first_item);
                Scrollbar::new(ScrollbarOrientation::VerticalRight).render(area, frame.buffer_mut(), &mut scroll_state);
            }
            ListValue::Position(show_editor, _, pos) => {
                show_editor(frame, pos);
                return false;
            }
        }
        if self.need_update {
            if let Some(update) = self.update_value.as_ref() {
                update(val, &self.value);
                self.need_update = false;
            }
        }
        true
    }

    fn handle_key_press(&mut self, key: KeyEvent, _state: &mut ConfigMenuState) -> ResultState {
        match key {
            KeyEvent { code: KeyCode::F(1), .. } => {
                if !self.help.is_empty() {
                    return ResultState {
                        edit_msg: EditMessage::DisplayHelp(self.help.clone()),
                        status_line: self.status.clone(),
                    };
                }
            }
            _ => {}
        }

        if !matches!(self.value, ListValue::ComboBox(_)) {
            match key.code {
                KeyCode::Up => return ResultState::up(),
                KeyCode::Down | KeyCode::Enter => return ResultState::down(),
                KeyCode::Esc => return ResultState::close(),
                _ => {}
            }
        }

        match &mut self.value {
            ListValue::Text(_edit_len, _, text) => {
                self.need_update |= self.text_field_state.handle_input(key, text);
            }

            ListValue::Path(path) => {
                let mut text = format!("{}", path.display());
                self.need_update |= self.text_field_state.handle_input(key, &mut text);
                *path = PathBuf::from(text);
            }
            ListValue::Float(val, str) => {
                self.need_update |= self.text_field_state.handle_input(key, str);
                if let Ok(f) = str.parse::<f64>() {
                    *val = f;
                }
            }
            ListValue::Time(val, str) => {
                self.need_update |= self.text_field_state.handle_input(key, str);
                *val = IcbTime::parse(str);
            }
            ListValue::Date(val, state) => {
                match state.editing_mode {
                    DateEditMode::Picker => {
                        match key.code {
                            KeyCode::Left => {
                                state.cursor_field = match state.cursor_field {
                                    DateField::Month => DateField::Year,
                                    DateField::Day => DateField::Month,
                                    DateField::Year => DateField::Day,
                                };
                            }
                            KeyCode::Right | KeyCode::Tab => {
                                state.cursor_field = match state.cursor_field {
                                    DateField::Month => DateField::Day,
                                    DateField::Day => DateField::Year,
                                    DateField::Year => DateField::Month,
                                };
                            }
                            KeyCode::Char('+') | KeyCode::PageUp => {
                                state.increment_field(val);
                                self.need_update = true;
                            }
                            KeyCode::Char('-') | KeyCode::PageDown => {
                                state.decrement_field(val);
                                self.need_update = true;
                            }
                            KeyCode::Char('t') | KeyCode::Char('T') => {
                                // Set to today
                                *val = Utc::now();
                                state.text_buffer = IcbDate::from_utc(val).to_pcb_str();
                                self.need_update = true;
                            }
                            KeyCode::Char(' ') => {
                                // Toggle to text mode
                                state.editing_mode = DateEditMode::Text;
                            }
                            KeyCode::Char(c) if c.is_ascii_digit() => {
                                // Start typing - switch to text mode
                                state.editing_mode = DateEditMode::Text;
                                state.text_buffer = c.to_string();
                                // self.text_field_state.cursor_position = 1;
                            }
                            _ => {}
                        }
                    }
                    DateEditMode::Text => {
                        if key.code == KeyCode::Esc {
                            // Return to picker mode
                            state.editing_mode = DateEditMode::Picker;
                            state.text_buffer = IcbDate::from_utc(val).to_pcb_str();
                        } else {
                            self.need_update |= self.text_field_state.handle_input(key, &mut state.text_buffer);
                            if let Some(parsed) = IcbDate::try_parse(&state.text_buffer) {
                                *val = parsed.to_utc_date_time();
                            }
                        }
                    }
                }
            }
            ListValue::DoW(val, str) => {
                self.need_update |= self.text_field_state.handle_input(key, str);
                *val = IcbDoW::from(str.clone());
            }

            ListValue::Security(val, str) => {
                self.need_update |= self.text_field_state.handle_input(key, str);
                if let Ok(res) = SecurityExpression::from_str(str) {
                    *val = res;
                }
            }
            ListValue::U32(cur, min, max) => {
                let mut text = format!("{}", *cur);
                self.need_update |= self.text_field_state.handle_input(key, &mut text);
                if let Ok(u) = text.parse::<u32>() {
                    *cur = u.clamp(*min, *max);
                }
            }
            ListValue::Bool(b) => {
                match key.code {
                    KeyCode::BackTab | KeyCode::Left | KeyCode::Tab | KeyCode::Right | KeyCode::Char(' ') => {
                        *b = !*b;
                        self.need_update = true;
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
                                self.need_update = true;
                                return ResultState::default();
                            }
                            KeyCode::Tab | KeyCode::Right => {
                                *cur_value = list[(i + 1) % list.len()].value.clone();
                                self.need_update = true;
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
                    self.need_update |= self.text_field_state.handle_input(key, &mut text);
                    if let Ok(u) = u8::from_str_radix(&text, 16) {
                        *col = IcbColor::Dos(u);
                    }
                }
            }
            ListValue::ComboBox(combo) => {
                if !combo.is_edit_open {
                    match key.code {
                        KeyCode::Up => return ResultState::up(),
                        KeyCode::Down => return ResultState::down(),
                        KeyCode::Esc => return ResultState::close(),
                        KeyCode::Enter => {
                            combo.is_edit_open = true;
                            for (i, val) in combo.values.iter().enumerate() {
                                if val.value == combo.cur_value.value {
                                    combo.selected_item = i;
                                    combo.first_item = i.saturating_sub(2);
                                    self.need_update = true;
                                }
                            }
                            return ResultState::default();
                        }
                        _ => {}
                    }
                } else {
                    if key.code == KeyCode::Esc {
                        combo.is_edit_open = false;
                        return ResultState::default();
                    }
                    if key.code == KeyCode::Enter {
                        combo.is_edit_open = false;
                        combo.cur_value = combo.values[combo.selected_item].clone();
                        self.need_update = true;
                        return ResultState::default();
                    }
                    combo.handle_input(key);
                }
                return ResultState::default();
            }
            ListValue::Position(_, input, pos) => {
                *pos = input(key, pos);
                self.need_update = true;
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
            ConfigEntry::Label(_) | ConfigEntry::Separator => 01,
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
        cols: usize, // Renamed from 'rows' for clarity
    ) -> bool {
        let cols = cols.max(1); // Guard against 0
        if items.is_empty() {
            return true;
        }

        // First pass: calculate column widths
        let mut col_widths: Vec<u16> = vec![0; cols];
        for (idx, item) in items.iter().enumerate() {
            let col = idx % cols;
            let item_width = item.measure_value(area) as u16;
            col_widths[col] = col_widths[col].max(item_width);
        }

        // Second pass: render items
        let start_x = *x;
        let start_y = *y;

        for (idx, item) in items.iter_mut().enumerate() {
            let col = idx % cols;
            let row = idx / cols;

            // Calculate x position for this column
            let mut col_x = start_x;
            for c in 0..col {
                col_x += col_widths[c] + 1; // +1 for spacing between columns
            }

            // Calculate y position for this row
            let item_y = start_y + row as u16;

            match item {
                ConfigEntry::Item(item) => {
                    if item_y >= state.first_row && item_y < area.height + state.first_row {
                        let left_area = Rect {
                            x: area.x + col_x,
                            y: area.y + item_y - state.first_row,
                            width: item.label_width,
                            height: 1,
                        };

                        if !display_editor || *i != state.selected {
                            item.render_label(left_area, frame, *i == state.selected, true);
                        }

                        // Render separator
                        Text::from(":").style(get_tui_theme().item_separator).render(
                            Rect {
                                x: left_area.x + item.label_width + 1,
                                y: left_area.y,
                                width: 1,
                                height: 1,
                            },
                            frame.buffer_mut(),
                        );

                        // Calculate value area
                        let value_x = left_area.x + item.label_width + 3;
                        let value_width = if item.edit_width > 0 {
                            item.edit_width
                        } else {
                            // Calculate remaining width in this column
                            let col_end = area.x + col_x + col_widths[col];
                            col_end.saturating_sub(value_x)
                        };

                        let right_area = Rect {
                            x: value_x,
                            y: left_area.y,
                            width: value_width,
                            height: 1,
                        };

                        if *i == state.selected && display_editor {
                            if !item.render_editor(val, right_area, frame) {
                                return false;
                            }
                        } else if !display_editor {
                            item.render_value(right_area, frame);
                        }
                    }

                    state.item_pos.insert(*i, item_y);
                    *i += 1;
                }
                _ => {
                    todo!()
                }
            }
        }

        // Update y to point after the table
        let total_rows = (items.len() + cols - 1) / cols;
        *y = start_y + total_rows as u16;
        *x = start_x;

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
                ConfigEntry::Table(rows, items) => {
                    if !Self::display_table(val, i, items, area, y, x, frame, state, display_editor, *rows) {
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
        let res = self.get_item_mut(state.selected).unwrap().handle_key_press(key, state);
        match res.edit_msg {
            EditMessage::PrevItem => Self::prev(self.count(), state),
            EditMessage::NextItem => Self::next(self.count(), state),
            _ => {
                return res;
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
