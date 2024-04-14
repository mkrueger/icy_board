use std::{path::PathBuf, time::Duration};

use chrono::{Local, Timelike};
use color_eyre::{eyre::Context, Result};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use icy_board_engine::icy_board::icb_text::IcbTextFile;
use icy_board_tui::{term::next_event, theme::THEME, TerminalType};
use itertools::Itertools;
use ratatui::{prelude::*, widgets::*};
use strum::{Display, EnumIter, FromRepr, IntoEnumIterator};
use tui_input::Input;

use crate::tabs::*;

pub struct App {
    mode: Mode,
    tab: TabPageType,

    file: PathBuf,

    cursor: Option<(u16, u16)>,
    status_line: String,
    full_screen: bool,

    input: Input,

    general_tab: GeneralTab,
    about_tab: AboutTab,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Mode {
    #[default]
    Command,
    Edit,
    Filter,
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

impl App {
    pub fn new(icb_txt: IcbTextFile, file: PathBuf, full_screen: bool) -> Self {
        Self {
            full_screen,
            file,
            general_tab: GeneralTab::new(icb_txt),
            cursor: None,
            mode: Mode::default(),
            tab: TabPageType::General,
            about_tab: AboutTab::default(),
            status_line: String::new(),
            input: Input::default(),
        }
    }

    /// Run the app until the user quits.
    pub fn run(&mut self, terminal: &mut TerminalType) -> Result<()> {
        while self.is_running() {
            self.draw(terminal)?;
            self.handle_events(terminal)?;
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

                if let Some((x, y)) = self.cursor {
                    frame.set_cursor(x, y);
                }
            })
            .wrap_err("terminal.draw")?;
        Ok(())
    }

    /// Handle events from the terminal.
    ///
    /// This function is called once per frame, The events are polled from the stdin with timeout of
    /// 1/50th of a second. This was chosen to try to match the default frame rate of a GIF in VHS.
    fn handle_events(&mut self, terminal: &mut TerminalType) -> Result<()> {
        let timeout = Duration::from_secs_f64(1.0);
        match next_event(timeout)? {
            Some(Event::Key(key)) if key.kind == KeyEventKind::Press => self.handle_key_press(terminal, key),
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

    fn handle_key_press(&mut self, terminal: &mut TerminalType, key: KeyEvent) {
        if self.mode == Mode::Edit {
            let state = self.get_tab_mut().handle_key_press(key);
            self.cursor = state.cursor;
            self.status_line = state.status_line;
            if self.cursor.is_none() {
                self.mode = Mode::Command;
            }
            return;
        }

        use KeyCode::*;
        match key.code {
            Char('q') | Esc => self.mode = Mode::Quit,
            Char('l') | Tab => self.next_tab(),
            F(2) => self.mode = Mode::Filter,
            Char('d') | Enter => {
                if self.mode == Mode::Filter {
                    self.mode = Mode::Command;
                    return;
                }
                let full_screen = self.full_screen;
                let state = self.get_tab_mut().request_edit_mode(terminal, full_screen);
                self.status_line = state.status_line;
                self.cursor = state.cursor;
                if self.cursor.is_some() {
                    self.mode = Mode::Edit;
                }
            }

            _ => {
                let state = self.get_tab_mut().handle_key_press(key);
                self.status_line = state.status_line;
            }
        };
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
        App::render_key_help_view(key_bar, frame.buffer_mut());
        self.render_status_line(status_line, frame.buffer_mut());
    }

    fn update_state(&mut self) {
        let state = self.get_tab().request_status();
        self.status_line = state.status_line;
    }
}

impl App {
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

    fn render_key_help_view(area: Rect, buf: &mut Buffer) {
        let keys = [("H/←", "Left"), ("L/→", "Right"), ("K/↑", "Up"), ("J/↓", "Down"), ("Q/Esc", "Quit")];
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
        let next_index = (current_index + 1) % 2;
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
