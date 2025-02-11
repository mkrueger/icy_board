use std::{collections::HashMap, path::PathBuf, time::Duration};

use chrono::{Local, Timelike};
use color_eyre::{eyre::Context, Result};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use icy_board_engine::icy_board::icb_text::{IcbTextFile, IcbTextStyle, TextEntry};
use icy_board_tui::{
    get_text, get_text_args,
    pcb_line::get_styled_pcb_line,
    term::next_event,
    text_field::{TextField, TextfieldState},
    theme::{get_tui_theme, DOS_DARK_GRAY, DOS_LIGHT_CYAN, DOS_LIGHT_GRAY, DOS_WHITE},
    TerminalType,
};
use itertools::Itertools;
use ratatui::{
    prelude::*,
    widgets::{block::Title, *},
};
use strum_macros::{Display, FromRepr};

use crate::tabs::*;

pub struct App<'a> {
    mode: Mode,
    tab: TabPageType,
    orig: IcbTextFile,
    file: PathBuf,

    status_line: String,
    full_screen: bool,

    filter: String,
    filter_state: TextfieldState,

    edit_state: TextfieldState,
    edit_entry: TextEntry,

    record_tab: RecordTab<'a>,
    about_tab: AboutTab,

    pub save: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Mode {
    #[default]
    Command,
    Edit,
    Filter,
    Jump,
    RequestQuit,
    Quit,
}

#[derive(Debug, Clone, Copy, Default, Display, FromRepr, PartialEq, Eq)]
enum TabPageType {
    #[default]
    Record,
    About,
}

impl TabPageType {
    pub fn iter() -> impl Iterator<Item = Self> {
        vec![TabPageType::Record, TabPageType::About].into_iter()
    }
}

#[derive(Default)]
pub struct ResultState {
    pub _cursor: Option<(u16, u16)>,
    pub status_line: String,
}

impl<'a> App<'a> {
    pub fn new(icb_txt: &'a mut IcbTextFile, file: PathBuf, full_screen: bool) -> Self {
        let orig = icb_txt.clone();
        Self {
            orig,
            full_screen,
            file,
            record_tab: RecordTab::new(icb_txt),
            mode: Mode::default(),
            tab: TabPageType::Record,
            about_tab: AboutTab::default(),
            status_line: String::new(),
            filter: String::new(),
            filter_state: TextfieldState::default(),

            edit_entry: TextEntry::default(),
            edit_state: TextfieldState::default(),
            save: false,
        }
    }

    /// Run the app until the user quits.
    pub fn run(&mut self, terminal: &mut TerminalType) -> Result<()> {
        self.update_state();
        while self.is_running() {
            self.draw(terminal)?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.mode != Mode::Quit
    }

    /// Draw a single frame of the app.
    fn draw(&mut self, terminal: &mut TerminalType) -> Result<()> {
        terminal
            .draw(|frame| {
                let screen = get_screen_size(&frame, self.full_screen);
                self.ui(frame, screen);
                match self.mode {
                    Mode::Jump | Mode::Edit => self.edit_state.set_cursor_position(frame),
                    Mode::Filter => self.filter_state.set_cursor_position(frame),
                    _ => {}
                }
            })
            .wrap_err("terminal.draw")?;
        Ok(())
    }

    /// Handle events from the terminal.
    ///
    /// This function is called once per frame, The events are polled from the stdin with timeout of
    /// 1/50th of a second. This was chosen to try to match the default frame rate of a GIF in VHS.
    fn handle_events(&mut self) -> Result<()> {
        let timeout = Duration::from_secs_f64(1.0);
        match next_event(timeout)? {
            Some(Event::Key(key)) if key.kind == KeyEventKind::Press => self.handle_key_press(key),
            _ => {}
        }
        Ok(())
    }

    fn get_tab(&self) -> &dyn TabPage {
        match self.tab {
            TabPageType::Record => &self.record_tab,
            TabPageType::About => &self.about_tab,
        }
    }

    fn get_tab_mut(&mut self) -> &mut dyn TabPage {
        match self.tab {
            TabPageType::Record => &mut self.record_tab,
            TabPageType::About => &mut self.about_tab,
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) {
        use KeyCode::*;
        match self.mode {
            Mode::Edit => {
                match key.code {
                    Esc => self.mode = Mode::Command,
                    F(2) => {
                        self.edit_entry.style = self.edit_entry.style.next();
                    }
                    F(3) => {
                        self.edit_entry.style = self.edit_entry.style.prev();
                    }
                    F(4) => {
                        if let Some(entry) = self.record_tab.get_original_entry() {
                            self.edit_entry = entry.clone();
                        }
                    }
                    Enter => {
                        if let Some(edit) = self.record_tab.get_selected_entry_mut() {
                            *edit = self.edit_entry.clone();
                        }
                        self.mode = Mode::Command;
                    }
                    _ => {
                        self.edit_state.handle_input(key, &mut self.edit_entry.text);
                    }
                };

                return;
            }
            Mode::Filter => {
                match key.code {
                    Enter | Esc => self.mode = Mode::Command,

                    _ => {
                        self.filter_state.handle_input(key, &mut self.filter);
                        self.record_tab.set_filter(&self.filter);
                    }
                };
                return;
            }

            Mode::Jump => {
                match key.code {
                    Esc => self.mode = Mode::Command,

                    Enter => {
                        if let Some(number) = self.edit_entry.text.parse::<usize>().ok() {
                            if number > 0 {
                                self.record_tab.set_filter("");
                                self.record_tab.jump(number - 1);
                                self.update_state();
                            }
                        }
                        self.mode = Mode::Command;
                    }

                    _ => {
                        self.edit_state.handle_input(key, &mut self.edit_entry.text);
                    }
                };
                return;
            }
            Mode::RequestQuit => {
                match key.code {
                    Left | Right => self.save = !self.save,
                    Enter => {
                        self.mode = Mode::Quit;
                    }
                    Esc => {
                        self.mode = Mode::Command;
                    }
                    _ => {}
                };

                return;
            }
            _ => {
                if self.get_tab().grab_focus() {
                    let state = self.get_tab_mut().handle_key_press(key);
                    self.status_line = state.status_line;
                    return;
                }

                match key.code {
                    Char('q') | Esc => {
                        if self.record_tab.is_dirty(&self.orig) {
                            self.mode = Mode::RequestQuit;
                        } else {
                            self.mode = Mode::Quit;
                        }
                    }
                    Char('h') | Left => self.prev_tab(),
                    Char('l') | Right => self.next_tab(),
                    F(2) => self.mode = Mode::Filter,
                    F(3) => {
                        self.edit_entry.text = "".to_string();
                        self.edit_state = TextfieldState::default().with_max_len(4).with_position(0).with_mask("0123456789".to_string());
                        self.mode = Mode::Jump;
                    }
                    F(4) => {
                        let orig_entry = self.record_tab.get_original_entry().unwrap().clone();
                        if let Some(entry) = self.record_tab.get_selected_entry_mut() {
                            *entry = orig_entry;
                        }
                    }
                    Char('d') | Enter => {
                        if let Some(edit) = self.record_tab.get_selected_entry_mut() {
                            self.edit_entry = edit.clone();
                            self.edit_state = TextfieldState::default().with_position(edit.text.len() as u16);
                            self.mode = Mode::Edit;
                        }
                    }

                    _ => {
                        let state = self.get_tab_mut().handle_key_press(key);
                        self.status_line = state.status_line;
                    }
                };
            }
        }
    }

    fn prev_tab(&mut self) {
        self.tab = self.tab.prev();
        self.update_state();
    }

    fn next_tab(&mut self) {
        self.tab = self.tab.next();
        self.update_state();
    }

    fn ui(&mut self, frame: &mut Frame, area: Rect) {
        let vertical = Layout::vertical([Constraint::Length(1), Constraint::Fill(1), Constraint::Length(1), Constraint::Length(1)]);
        let [title_bar, mut tab, key_bar, status_line] = vertical.areas(area);

        Block::new().style(get_tui_theme().title_bar).render(area, frame.buffer_mut());
        self.render_title_bar(title_bar, frame.buffer_mut());

        if !self.filter.is_empty() {
            let filter_area = Rect::new(tab.x, tab.y, tab.width, 1);
            self.render_filter_text(filter_area, frame.buffer_mut());
            tab.y += 1;
            tab.height -= 1;
        }

        self.render_selected_tab(frame, tab);
        self.render_key_help_view(key_bar, frame.buffer_mut());
        self.render_status_line(status_line, frame.buffer_mut());

        match self.mode {
            Mode::Edit => {
                let edit_height = 12;
                let edit_area = Rect::new(area.x + 1, area.y + (area.height - edit_height - 1) / 2, area.width - 3, edit_height);

                Clear.render(edit_area, frame.buffer_mut());
                let edit_title = get_text_args(
                    "icbtext_edit_title",
                    HashMap::from([("number".to_string(), self.record_tab.selected_record().to_string())]),
                );

                let record_length = get_text_args(
                    "icbtext_edit_record_length_title",
                    HashMap::from([("number".to_string(), self.edit_entry.text.len().to_string())]),
                );

                let justify = match self.edit_entry.justification {
                    icy_board_engine::icy_board::icb_text::IcbTextJustification::Left => get_text("icbtext_edit_justify_left"),
                    icy_board_engine::icy_board::icb_text::IcbTextJustification::Right => get_text("icbtext_edit_justify_right"),
                    icy_board_engine::icy_board::icb_text::IcbTextJustification::Center => get_text("icbtext_edit_justify_center"),
                };
                let justify_title = get_text_args("icbtext_edit_justify_title", HashMap::from([("justify".to_string(), justify)]));

                Block::new()
                    .borders(Borders::ALL)
                    .title(Title::from(Span::from(format!(" {} ", edit_title)).style(get_tui_theme().dialog_box_title)))
                    .title_position(block::Position::Bottom)
                    .title_alignment(Alignment::Center)
                    .title(Title::from(Span::from(format!(" {} ", record_length)).style(get_tui_theme().dialog_box_title)))
                    .title_position(block::Position::Top)
                    .title_alignment(Alignment::Right)
                    .title(Title::from(Span::from(format!(" {} ", justify_title)).style(get_tui_theme().dialog_box_title)))
                    .style(get_tui_theme().dialog_box)
                    .border_type(BorderType::Double)
                    .render(edit_area, frame.buffer_mut());

                let field = TextField::new().with_value(self.edit_entry.text.to_string());

                let mut area = edit_area.inner(Margin { horizontal: 1, vertical: 1 });
                area.height = 1;

                Line::from(get_text("icbtext_edit_original_text_title"))
                    .style(Style::default().fg(DOS_LIGHT_CYAN).italic())
                    .render(area, frame.buffer_mut());

                if let Some(entry) = self.record_tab.get_original_entry() {
                    let mut style_area = area;
                    style_area.x += 30;
                    style_area.width -= 30;

                    Line::from(vec![
                        Span::styled(get_text("icbtext_edit_style"), get_tui_theme().dialog_box_title),
                        Span::raw(" "),
                        Span::styled(Self::get_style_description(entry.style), convert_style(entry.style).not_italic().bold()),
                        Span::raw(" "),
                    ])
                    .alignment(Alignment::Right)
                    .render(style_area, frame.buffer_mut());
                    area.y += 1;

                    Line::from(entry.text.clone())
                        .style(convert_style(entry.style))
                        .render(area, frame.buffer_mut());
                }
                area.y += 2;
                Line::from(get_text("icbtext_edit_preview_text_title"))
                    .style(Style::default().fg(DOS_LIGHT_CYAN).italic())
                    .render(area, frame.buffer_mut());
                area.y += 1;
                Text::from(get_styled_pcb_line(&self.edit_entry.text))
                    .style(convert_style(self.edit_entry.style))
                    .render(area, frame.buffer_mut());
                area.y += 2;

                Line::from(get_text("icbtext_edit_edit_text_title"))
                    .style(Style::default().fg(DOS_LIGHT_CYAN).italic())
                    .render(area, frame.buffer_mut());

                let mut style_area = area;
                style_area.x += 30;
                style_area.width -= 30;

                Line::from(vec![
                    Span::styled(get_text("icbtext_edit_style"), get_tui_theme().dialog_box_title),
                    Span::raw(" "),
                    Span::styled(
                        Self::get_style_description(self.edit_entry.style),
                        convert_style(self.edit_entry.style).not_italic().bold(),
                    ),
                    Span::raw(" "),
                ])
                .alignment(Alignment::Right)
                .render(style_area, frame.buffer_mut());
                area.y += 1;

                frame.render_stateful_widget(field, area, &mut self.edit_state);

                area.y += 2;
                Line::from(get_text("icbtext_edit_hard_space_info"))
                    .style(get_tui_theme().description_text)
                    .alignment(Alignment::Center)
                    .render(area, frame.buffer_mut());
            }
            Mode::Filter => {
                let filter_area = Rect::new(area.x + 2, area.y + (area.height - 3) / 2, area.width - 5, 3);

                Clear.render(filter_area, frame.buffer_mut());

                Block::new()
                    .borders(Borders::ALL)
                    .title(Title::from(
                        Span::from(format!(" {} ", get_text("icbtext_filter_title"))).style(get_tui_theme().dialog_box_title),
                    ))
                    .style(get_tui_theme().dialog_box)
                    .border_type(BorderType::Double)
                    .render(filter_area, frame.buffer_mut());

                let field = TextField::new().with_value(self.filter.to_string());

                let area = filter_area.inner(Margin { horizontal: 2, vertical: 1 });
                frame.render_stateful_widget(field, area, &mut self.filter_state);
            }
            Mode::Jump => {
                let jump_size = 31;
                let jump_area = Rect::new(area.x + 3 + (area.width - jump_size) / 2, area.y + (area.height - 3) / 2, jump_size, 3);

                Clear.render(jump_area, frame.buffer_mut());

                Block::new()
                    .borders(Borders::ALL)
                    .title(Title::from(
                        Span::from(format!(" {} ", get_text("icbtext_jump_to_title"))).style(get_tui_theme().dialog_box_title),
                    ))
                    .style(get_tui_theme().dialog_box)
                    .border_type(BorderType::Double)
                    .render(jump_area, frame.buffer_mut());

                let field = TextField::new().with_value(self.edit_entry.text.to_string());

                let area = jump_area.inner(Margin { horizontal: 2, vertical: 1 });
                frame.render_stateful_widget(field, area, &mut self.edit_state);
            }
            Mode::RequestQuit => {
                let save_text = format!("{} ", get_text("icbtext_save_changes"));
                let mut save_area = Rect::new(
                    area.x + (area.width - (save_text.len() as u16 + 10)) / 2,
                    area.y + (area.height - 3) / 2,
                    save_text.len() as u16 + 10,
                    3,
                );

                Clear.render(save_area, frame.buffer_mut());

                Block::new()
                    .borders(Borders::ALL)
                    .style(get_tui_theme().dialog_box)
                    .border_type(BorderType::Double)
                    .render(save_area, frame.buffer_mut());

                let field = Line::from(vec![
                    Span::styled(save_text, Style::default().fg(DOS_LIGHT_GRAY)),
                    Span::styled(get_text("yes"), Style::default().fg(if self.save { DOS_WHITE } else { DOS_DARK_GRAY })),
                    Span::styled("/", Style::default().fg(DOS_LIGHT_GRAY)),
                    Span::styled(get_text("no"), Style::default().fg(if !self.save { DOS_WHITE } else { DOS_DARK_GRAY })),
                ]);
                save_area.y += 1;
                save_area.x += 1;
                field.render(save_area.inner(Margin { horizontal: 1, vertical: 0 }), frame.buffer_mut());
            }
            _ => {}
        }
    }

    fn get_style_description(style: IcbTextStyle) -> String {
        match style {
            IcbTextStyle::Plain => get_text("icbtext_style_plain"),
            IcbTextStyle::Red => get_text("icbtext_style_red"),
            IcbTextStyle::Green => get_text("icbtext_style_green"),
            IcbTextStyle::Yellow => get_text("icbtext_style_yellow"),
            IcbTextStyle::Blue => get_text("icbtext_style_blue"),
            IcbTextStyle::Purple => get_text("icbtext_style_purple"),
            IcbTextStyle::Cyan => get_text("icbtext_style_cyan"),
            IcbTextStyle::White => get_text("icbtext_style_white"),
        }
    }

    fn update_state(&mut self) {
        let state = self.get_tab().request_status();
        self.status_line = state.status_line;
    }

    fn render_title_bar(&self, area: Rect, buf: &mut Buffer) {
        let len: u16 = TabPageType::iter().map(|p| TabPageType::title(p).len() as u16).sum();
        let layout = Layout::horizontal([Constraint::Min(0), Constraint::Length(len)]);
        let [title, tabs] = layout.areas(area);

        Span::styled(
            format!(" ICBTEXT File Generator/Editor ({})", self.file.file_name().unwrap().to_string_lossy()),
            get_tui_theme().app_title,
        )
        .render(title, buf);
        let titles = TabPageType::iter().map(TabPageType::title);
        Tabs::new(titles)
            .style(get_tui_theme().tabs)
            .highlight_style(get_tui_theme().tabs_selected)
            .select(self.tab as usize)
            .divider("")
            .padding("", "")
            .render(tabs, buf);
    }

    fn render_filter_text(&self, area: Rect, buf: &mut Buffer) {
        Line::from(get_text_args(
            "icbtext_filter_text",
            HashMap::from([("filter".to_string(), self.filter.to_string())]),
        ))
        .style(get_tui_theme().filter_text.bold())
        .render(area, buf);
    }

    fn render_selected_tab(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        self.get_tab_mut().render(frame, area);
    }

    fn render_key_help_view(&self, area: Rect, buf: &mut Buffer) {
        let keys = match self.mode {
            Mode::RequestQuit => vec![("Enter", get_text("key_desc_quit")), ("Q/Esc", get_text("key_desc_back"))],
            Mode::Edit => vec![
                ("F2/F3", get_text("key_desc_next_prev_style")),
                ("F4", get_text("key_desc_restore")),
                ("Enter", get_text("key_desc_accept")),
                ("Esc", get_text("key_desc_cancel")),
            ],
            _ => vec![
                ("F2", get_text("key_desc_filter")),
                ("F3", get_text("key_desc_jump")),
                ("F4", get_text("key_desc_restore")),
                ("Enter", get_text("key_desc_edit")),
                ("Q/Esc", get_text("key_desc_quit")),
            ],
        };
        let spans = keys
            .iter()
            .flat_map(|(key, desc)| {
                let key = Span::styled(format!(" {key} "), get_tui_theme().key_binding);
                let desc = Span::styled(format!(" {desc} "), get_tui_theme().key_binding_description);
                [key, desc]
            })
            .collect_vec();
        Line::from(spans).centered().style((Color::Indexed(236), Color::Indexed(232))).render(area, buf);
    }

    fn render_status_line(&self, area: Rect, buf: &mut Buffer) {
        let now = Local::now();
        let time_status = format!(" {} {} |", now.time().with_nanosecond(0).unwrap(), now.date_naive().format("%m-%d-%y"));
        let time_len = time_status.len() as u16;
        Line::from(time_status).left_aligned().style(get_tui_theme().status_line).render(area, buf);

        if self.mode == Mode::RequestQuit {
            return;
        }
        let mut area = area;
        area.x += time_len + 1;
        area.width -= time_len + 1;
        Line::from(self.status_line.clone())
            .left_aligned()
            .style(get_tui_theme().status_line_text)
            .render(area, buf);
    }
}

impl TabPageType {
    fn next(self) -> Self {
        let current_index = self as usize;
        let next_index = current_index.saturating_add(1);
        Self::from_repr(next_index).unwrap_or(self)
    }

    fn prev(self) -> Self {
        let current_index = self as usize;
        let prev_index = current_index.saturating_sub(1);
        Self::from_repr(prev_index).unwrap_or(self)
    }

    fn title(self) -> String {
        let t = match self {
            Self::Record => get_text("icbtext_tab_record"),
            Self::About => get_text("icbtext_tab_about"),
        };
        format!(" {t} ")
    }
}

pub fn get_screen_size(frame: &Frame, is_full_screen: bool) -> Rect {
    if is_full_screen {
        frame.area()
    } else {
        let width = frame.area().width.min(80);
        let height = frame.area().height.min(25);

        let x = frame.area().x + (frame.area().width - width) / 2;
        let y = frame.area().y + (frame.area().height - height) / 2;
        Rect::new(frame.area().x + x, frame.area().y + y, width, height)
    }
}
