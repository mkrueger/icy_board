use std::{path::PathBuf, time::Duration};

use chrono::{Local, Timelike};
use color_eyre::{eyre::Context, Result};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use icy_board_engine::icy_board::icb_text::IcbTextFile;
use icy_board_tui::{
    term::next_event,
    text_field::{TextField, TextfieldState},
    theme::{DOS_DARK_GRAY, DOS_LIGHT_GRAY, DOS_WHITE, THEME},
    TerminalType,
};
use itertools::Itertools;
use ratatui::{
    prelude::*,
    widgets::{block::Title, *},
};
use strum::{Display, EnumIter, FromRepr, IntoEnumIterator};

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
    edit_text: String,

    general_tab: GeneralTab<'a>,
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

#[derive(Debug, Clone, Copy, Default, Display, EnumIter, FromRepr, PartialEq, Eq)]
enum TabPageType {
    #[default]
    General,
    About,
}

#[derive(Default)]
pub struct ResultState {
    pub cursor: Option<(u16, u16)>,
    pub status_line: String,
}

impl<'a> App<'a> {
    pub fn new(icb_txt: &'a mut IcbTextFile, file: PathBuf, full_screen: bool) -> Self {
        let orig = icb_txt.clone();
        Self {
            orig,
            full_screen,
            file,
            general_tab: GeneralTab::new(icb_txt),
            mode: Mode::default(),
            tab: TabPageType::General,
            about_tab: AboutTab::default(),
            status_line: String::new(),
            filter: String::new(),
            filter_state: TextfieldState::default(),

            edit_text: String::new(),
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
            TabPageType::General => &self.general_tab,
            TabPageType::About => &self.about_tab,
        }
    }

    fn get_tab_mut(&mut self) -> &mut dyn TabPage {
        match self.tab {
            TabPageType::General => &mut self.general_tab,
            TabPageType::About => &mut self.about_tab,
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) {
        use KeyCode::*;
        match self.mode {
            Mode::Edit => {
                match key.code {
                    Esc => self.mode = Mode::Command,
                    Enter => {
                        if let Some(edit) = self.general_tab.get_selected_text_mut() {
                            edit.text = self.edit_text.clone();
                        }
                        self.mode = Mode::Command;
                    }
                    _ => {
                        self.edit_state.handle_input(key, &mut self.edit_text);
                    }
                };

                return;
            }
            Mode::Filter => {
                match key.code {
                    Enter | Esc => self.mode = Mode::Command,

                    _ => {
                        self.filter_state.handle_input(key, &mut self.filter);
                        self.general_tab.set_filter(&self.filter);
                    }
                };
                return;
            }

            Mode::Jump => {
                match key.code {
                    Esc => self.mode = Mode::Command,

                    Enter => {
                        if let Some(number) = self.edit_text.parse::<usize>().ok() {
                            if number > 0 {
                                self.general_tab.set_filter("");
                                self.general_tab.jump(number - 1);
                                self.update_state();
                            }
                        }
                        self.mode = Mode::Command;
                    }

                    _ => {
                        self.edit_state.handle_input(key, &mut self.edit_text);
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
                match key.code {
                    Char('q') | Esc => {
                        if self.general_tab.is_dirty(&self.orig) {
                            self.mode = Mode::RequestQuit;
                        } else {
                            self.mode = Mode::Quit;
                        }
                    }
                    Char('h') | Left => self.prev_tab(),
                    Char('l') | Right => self.next_tab(),
                    F(2) => self.mode = Mode::Filter,
                    F(3) => {
                        self.edit_text = "".to_string();
                        self.edit_state = TextfieldState::default().with_max_len(4).with_position(0).with_mask("0123456789".to_string());
                        self.mode = Mode::Jump;
                    }
                    F(4) => {
                        let txt = self.general_tab.get_original_text().unwrap().text.clone();
                        if let Some(edit) = self.general_tab.get_selected_text_mut() {
                            edit.text = txt;
                        }
                    }
                    Char('d') | Enter => {
                        if let Some(edit) = self.general_tab.get_selected_text_mut() {
                            self.edit_text = edit.text.to_string();
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
        let [title_bar, tab, key_bar, status_line] = vertical.areas(area);

        Block::new().style(THEME.title_bar).render(area, frame.buffer_mut());
        self.render_title_bar(title_bar, frame.buffer_mut());
        self.render_selected_tab(frame, tab);
        self.render_key_help_view(key_bar, frame.buffer_mut());
        self.render_status_line(status_line, frame.buffer_mut());

        match self.mode {
            Mode::Edit => {
                let edit_area = Rect::new(area.x + 2, area.y + (area.height - 3) / 2, area.width - 5, 4);

                Clear.render(edit_area, frame.buffer_mut());

                Block::new()
                    .borders(Borders::ALL)
                    .title(Title::from(Span::from(" Edit ").style(THEME.content_box_title)))
                    .style(THEME.content_box)
                    .border_type(BorderType::Double)
                    .render(edit_area, frame.buffer_mut());

                let field = TextField::new().with_value(self.edit_text.to_string());

                let mut area = edit_area.inner(&Margin { horizontal: 1, vertical: 1 });

                if let Some(entry) = self.general_tab.get_original_text() {
                    Line::from(entry.text.clone())
                        .style(Style::default().fg(DOS_LIGHT_GRAY))
                        .render(area, frame.buffer_mut());
                }
                area.y += 1;
                frame.render_stateful_widget(field, area, &mut self.edit_state);
            }
            Mode::Filter => {
                let filter_area = Rect::new(area.x + 2, area.y + (area.height - 3) / 2, area.width - 5, 3);

                Clear.render(filter_area, frame.buffer_mut());

                Block::new()
                    .borders(Borders::ALL)
                    .title(Title::from(Span::from(" Filter ").style(THEME.content_box_title)))
                    .style(THEME.content_box)
                    .border_type(BorderType::Double)
                    .render(filter_area, frame.buffer_mut());

                let field = TextField::new().with_value(self.filter.to_string());

                let area = filter_area.inner(&Margin { horizontal: 2, vertical: 1 });
                frame.render_stateful_widget(field, area, &mut self.filter_state);
            }
            Mode::Jump => {
                let jump_size = 21;
                let jump_area = Rect::new(area.x + 3 + (area.width - jump_size) / 2, area.y + (area.height - 3) / 2, jump_size, 3);

                Clear.render(jump_area, frame.buffer_mut());

                Block::new()
                    .borders(Borders::ALL)
                    .title(Title::from(Span::from(" Jump to Record # ").style(THEME.content_box_title)))
                    .style(THEME.content_box)
                    .border_type(BorderType::Double)
                    .render(jump_area, frame.buffer_mut());

                let field = TextField::new().with_value(self.edit_text.to_string());

                let area = jump_area.inner(&Margin { horizontal: 2, vertical: 1 });
                frame.render_stateful_widget(field, area, &mut self.edit_state);
            }
            Mode::RequestQuit => {
                let save_text = "Save changes? ";
                let mut save_area = Rect::new(
                    area.x + (area.width - (save_text.len() as u16 + 10)) / 2,
                    area.y + (area.height - 3) / 2,
                    save_text.len() as u16 + 10,
                    3,
                );

                Clear.render(save_area, frame.buffer_mut());

                Block::new()
                    .borders(Borders::ALL)
                    .style(THEME.content_box)
                    .border_type(BorderType::Double)
                    .render(save_area, frame.buffer_mut());

                let field = Line::from(vec![
                    Span::styled(save_text, Style::default().fg(DOS_LIGHT_GRAY)),
                    Span::styled("Yes", Style::default().fg(if self.save { DOS_WHITE } else { DOS_DARK_GRAY })),
                    Span::styled("/", Style::default().fg(DOS_LIGHT_GRAY)),
                    Span::styled("No", Style::default().fg(if !self.save { DOS_WHITE } else { DOS_DARK_GRAY })),
                ]);
                save_area.y += 1;
                save_area.x += 1;
                field.render(save_area.inner(&Margin { horizontal: 1, vertical: 0 }), frame.buffer_mut());
            }
            _ => {}
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
            THEME.app_title,
        )
        .render(title, buf);
        let titles = TabPageType::iter().map(TabPageType::title);
        Tabs::new(titles)
            .style(THEME.tabs)
            .highlight_style(THEME.tabs_selected)
            .select(self.tab as usize)
            .divider("")
            .padding("", "")
            .render(tabs, buf);
    }

    fn render_selected_tab(&mut self, frame: &mut Frame, area: Rect) {
        Clear.render(area, frame.buffer_mut());
        self.get_tab_mut().render(frame, area);
    }

    fn render_key_help_view(&self, area: Rect, buf: &mut Buffer) {
        let keys = match self.mode {
            Mode::RequestQuit => vec![("Enter", "Quit"), ("Q/Esc", "Back")],
            _ => vec![
                ("F2", "Filter"),
                ("F3", "Jump"),
                ("F4", "Restore Default"),
                ("Enter", "Edit"),
                ("Q/Esc", "Quit"),
            ],
        };
        let spans = keys
            .iter()
            .flat_map(|(key, desc)| {
                let key = Span::styled(format!(" {key} "), THEME.key_binding);
                let desc = Span::styled(format!(" {desc} "), THEME.key_binding_description);
                [key, desc]
            })
            .collect_vec();
        Line::from(spans).centered().style((Color::Indexed(236), Color::Indexed(232))).render(area, buf);
    }

    fn render_status_line(&self, area: Rect, buf: &mut Buffer) {
        let now = Local::now();
        let time_status = format!(" {} {} |", now.time().with_nanosecond(0).unwrap(), now.date_naive().format("%m-%d-%y"));
        let time_len = time_status.len() as u16;
        Line::from(time_status).left_aligned().style(THEME.status_line).render(area, buf);

        if self.mode == Mode::RequestQuit {
            return;
        }
        let mut area = area;
        area.x += time_len + 1;
        area.width -= time_len + 1;
        Line::from(self.status_line.clone())
            .left_aligned()
            .style(THEME.status_line_text)
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
        format!(" {self} ")
    }
}

pub fn get_screen_size(frame: &Frame, is_full_screen: bool) -> Rect {
    if is_full_screen {
        frame.size()
    } else {
        let width = frame.size().width.min(80);
        let height = frame.size().height.min(25);

        let x = frame.size().x + (frame.size().width - width) / 2;
        let y = frame.size().y + (frame.size().height - height) / 2;
        Rect::new(frame.size().x + x, frame.size().y + y, width, height)
    }
}
