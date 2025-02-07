use std::time::Duration;

use chrono::{Local, Timelike};
use color_eyre::{eyre::Context, Result};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};

use ratatui::{prelude::*, widgets::*};

use crate::{
    colors::RgbSwatch,
    config_menu::EditMode,
    get_text,
    help_view::{HelpView, HelpViewState},
    tab_page::TabPage,
    term::next_event,
    text_field::set_cursor_mode,
    theme::{get_tui_theme, DOS_DARK_GRAY, DOS_LIGHT_GRAY, DOS_WHITE},
    TerminalType,
};

pub struct App<'a> {
    pub mode: Mode,
    pub tab: usize,
    pub title: String,
    pub status_line: String,
    pub full_screen: bool,
    pub date_format: String,

    pub tabs: Vec<Box<dyn TabPage>>,
    pub help_state: HelpViewState<'a>,

    pub save: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Command,
    Quit,
    RequestQuit,
    ShowHelp,
}

impl<'a> App<'a> {
    /// Run the app until the user quits.
    pub fn run(&mut self, terminal: &mut TerminalType) -> Result<()> {
        set_cursor_mode();
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
                let screen: Rect = get_screen_size(&frame, self.full_screen);
                self.ui(frame, screen);

                match self.mode {
                    Mode::ShowHelp => {
                        self.show_help(frame, screen);
                    }
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
    fn handle_events(&mut self, terminal: &mut TerminalType) -> Result<()> {
        let timeout = Duration::from_secs_f64(1.0);
        match next_event(timeout)? {
            Some(Event::Key(key)) if key.kind == KeyEventKind::Press => self.handle_key_press(terminal, key),
            _ => {}
        }
        Ok(())
    }

    fn get_tab(&self) -> &dyn TabPage {
        self.tabs[self.tab].as_ref()
    }

    fn get_tab_mut(&mut self) -> &mut dyn TabPage {
        self.tabs[self.tab].as_mut()
    }

    fn handle_key_press(&mut self, terminal: &mut TerminalType, key: KeyEvent) {
        if self.mode == Mode::RequestQuit {
            match key.code {
                KeyCode::Left | KeyCode::Right => self.save = !self.save,
                KeyCode::Enter => {
                    self.mode = Mode::Quit;
                }
                KeyCode::Esc => {
                    self.mode = Mode::Command;
                }
                _ => {}
            };
            return;
        }

        if self.get_tab().has_control() {
            let state = self.get_tab_mut().handle_key_press(key);
            if let EditMode::ExternalProgramStarted = &state.edit_mode {
                let _ = terminal.clear();
            }
            self.status_line = state.status_line;
            return;
        }

        if self.mode == Mode::ShowHelp {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => self.mode = Mode::Command,
                _ => {
                    self.help_state.handle_key_press(key);
                }
            }
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.tabs.iter().any(|t| t.is_dirty()) {
                    self.mode = Mode::RequestQuit;
                } else {
                    self.mode = Mode::Quit;
                }
            }
            KeyCode::Char('h') | KeyCode::BackTab => self.prev_tab(),
            KeyCode::Char('l') | KeyCode::Tab => self.next_tab(),
            KeyCode::F(1) => {
                self.help_state.text = self.get_tab().get_help();
                self.mode = Mode::ShowHelp;
            }
            _ => {
                let state = self.get_tab_mut().handle_key_press(key);

                self.status_line = state.status_line;
            }
        };
    }

    fn prev_tab(&mut self) {
        self.tab = (self.tab + self.tabs.len() - 1) % self.tabs.len();
        self.update_state();
    }

    fn next_tab(&mut self) {
        self.tab = (self.tab + 1) % self.tabs.len();
        self.update_state();
    }

    fn ui(&mut self, frame: &mut Frame, area: Rect) {
        let vertical = Layout::vertical([Constraint::Length(1), Constraint::Fill(1), Constraint::Length(1)]);
        let [title_bar, tab, status_line] = vertical.areas(area);

        Block::new().style(get_tui_theme().title_bar).render(area, frame.buffer_mut());
        self.render_title_bar(title_bar, frame.buffer_mut());
        self.render_selected_tab(frame, tab);
        self.render_status_line(status_line, frame.buffer_mut());

        if self.mode == Mode::RequestQuit {
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
                .style(get_tui_theme().content_box)
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
    }

    fn update_state(&mut self) {
        let state = self.get_tab().request_status();
        self.status_line = state.status_line;
    }

    fn show_help(&mut self, frame: &mut Frame, screen: Rect) {
        let area = screen.inner(Margin { horizontal: 2, vertical: 2 });
        Clear.render(area, frame.buffer_mut());
        HelpView::default().render(area, frame.buffer_mut(), &mut self.help_state);
    }
}

impl<'a> App<'a> {
    fn render_title_bar(&self, area: Rect, buf: &mut Buffer) {
        let len: u16 = self.tabs.iter().map(|t| t.title().len() as u16 + 1).sum();
        let layout = Layout::horizontal([Constraint::Min(0), Constraint::Length(1 + len)]);
        let [title, tabs] = layout.areas(area);

        Span::styled(&self.title, get_tui_theme().app_title).render(title, buf);
        let titles = self.tabs.iter().enumerate().map(|(i, t)| {
            if i == self.tab {
                format!(" {} ", t.title())
            } else if i == self.tab + 1 {
                format!("{}", t.title())
            } else {
                format!(" {}", t.title())
            }
        });
        Tabs::new(titles)
            .style(get_tui_theme().tabs)
            .highlight_style(get_tui_theme().tabs_selected)
            .select(self.tab as usize)
            .divider("")
            .padding("", "")
            .render(tabs, buf);
    }

    fn render_selected_tab(&mut self, frame: &mut Frame, area: Rect) {
        if get_tui_theme().swatch {
            RgbSwatch.render(area, frame.buffer_mut());
        } else {
            Block::new()
                .style(get_tui_theme().background)
                .borders(Borders::NONE)
                .render(area, frame.buffer_mut());
        }
        self.get_tab_mut().render(frame, area);
    }

    fn render_status_line(&self, area: Rect, buf: &mut Buffer) {
        let now = Local::now();
        let time_status = format!(" {} {} |", now.time().with_nanosecond(0).unwrap(), now.date_naive().format(&self.date_format));
        let time_len = time_status.len() as u16;
        Line::from(time_status).left_aligned().style(get_tui_theme().status_line).render(area, buf);
        let mut area = area;
        area.x += time_len + 1;
        area.width -= time_len + 1;
        Line::from(self.status_line.clone())
            .left_aligned()
            .style(get_tui_theme().status_line_text)
            .render(area, buf);
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
