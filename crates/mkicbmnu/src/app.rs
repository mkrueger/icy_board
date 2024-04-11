use std::{arch::x86_64, time::Duration};

use color_eyre::{eyre::Context, Result};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use icy_board_tui::{term::next_event, theme::THEME};
use itertools::Itertools;
use ratatui::{prelude::*, widgets::*};
use strum::{Display, EnumIter, FromRepr, IntoEnumIterator};

use crate::{tabs::*, TabPage};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct App {
    mode: Mode,
    tab: Tab,

    full_screen: bool,

    about_tab: AboutTab,
    general_tab: GeneralTab,
    keywords_tab: KeywordsTab,
    prompts_tab: PromptsTab,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Mode {
    #[default]
    Running,
    Destroy,
    Quit,
}

#[derive(Debug, Clone, Copy, Default, Display, EnumIter, FromRepr, PartialEq, Eq)]
enum Tab {
    #[default]
    About,
    General,
    Keywords,
    Prompts,
}

impl App {
    pub fn new(full_screen: bool) -> Self {
        Self {
            full_screen,
            ..Default::default()
        }
    }

    /// Run the app until the user quits.
    pub fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
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
    fn draw(&self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
        terminal
            .draw(|frame| {

                let width = frame.size().width.min(80);
                let height = frame.size().height.min(25);

                let x = frame.size().x + (frame.size().width - width) / 2;
                let y = frame.size().y + (frame.size().height - height) / 2;

                frame.render_widget(self, Rect::new(frame.size().x + x, frame.size().y + y, width, height));
            })
            .wrap_err("terminal.draw")?;
        Ok(())
    }

    /// Handle events from the terminal.
    ///
    /// This function is called once per frame, The events are polled from the stdin with timeout of
    /// 1/50th of a second. This was chosen to try to match the default frame rate of a GIF in VHS.
    fn handle_events(&mut self) -> Result<()> {
        let timeout = Duration::from_secs_f64(1.0 / 50.0);
        match next_event(timeout)? {
            Some(Event::Key(key)) if key.kind == KeyEventKind::Press => self.handle_key_press(key),
            _ => {}
        }
        Ok(())
    }

    fn handle_key_press(&mut self, key: KeyEvent) {
        use KeyCode::*;
        match key.code {
            Char('q') | Esc => self.mode = Mode::Quit,
            Char('h') | Left => self.prev_tab(),
            Char('l') | Right => self.next_tab(),
            Char('k') | Up => self.prev(),
            Char('j') | Down => self.next(),
            Char('d') | Delete => self.destroy(),
            _ => {}
        };
    }

    fn prev(&mut self) {
        self.tab.prev();
    }

    fn next(&mut self) {
        self.tab.next();
    }

    fn prev_tab(&mut self) {
        self.tab = self.tab.prev();
    }

    fn next_tab(&mut self) {
        self.tab = self.tab.next();
    }

    fn destroy(&mut self) {
        self.mode = Mode::Destroy;
    }
}

/// Implement Widget for &App rather than for App as we would otherwise have to clone or copy the
/// entire app state on every frame. For this example, the app state is small enough that it doesn't
/// matter, but for larger apps this can be a significant performance improvement.
impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let vertical = Layout::vertical([Constraint::Length(1), Constraint::Fill(1), Constraint::Length(1)]);
        let [title_bar, tab, bottom_bar] = vertical.areas(area);

        Block::new().style(THEME.root).render(area, buf);
        self.render_title_bar(title_bar, buf);
        self.render_selected_tab(tab, buf);
        App::render_bottom_bar(bottom_bar, buf);
    }
}

impl App {
    fn render_title_bar(&self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::horizontal([Constraint::Min(0), Constraint::Length(35)]);
        let [title, tabs] = layout.areas(area);

        Span::styled("MNU File Editor", THEME.app_title).render(title, buf);
        let titles = Tab::iter().map(Tab::title);
        Tabs::new(titles)
            .style(THEME.tabs)
            .highlight_style(THEME.tabs_selected)
            .select(self.tab as usize)
            .divider("")
            .padding("", "")
            .render(tabs, buf);
    }

    fn render_selected_tab(&self, area: Rect, buf: &mut Buffer) {
        icy_board_tui::colors::RgbSwatch.render(area, buf);

        match self.tab {
            Tab::About => self.about_tab.render(area, buf),
            Tab::General => self.general_tab.render(area, buf),
            Tab::Keywords => self.keywords_tab.render(area, buf),
            Tab::Prompts => self.prompts_tab.render(area, buf),
        };
    }

    fn render_bottom_bar(area: Rect, buf: &mut Buffer) {
        let keys = [("H/←", "Left"), ("L/→", "Right"), ("K/↑", "Up"), ("J/↓", "Down"), ("Q/Esc", "Quit")];
        let spans = keys
            .iter()
            .flat_map(|(key, desc)| {
                let key = Span::styled(format!(" {key} "), THEME.key_binding.key);
                let desc = Span::styled(format!(" {desc} "), THEME.key_binding.description);
                [key, desc]
            })
            .collect_vec();
        Line::from(spans).centered().style((Color::Indexed(236), Color::Indexed(232))).render(area, buf);
    }
}

impl Tab {
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
