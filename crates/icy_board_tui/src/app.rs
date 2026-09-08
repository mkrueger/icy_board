use std::time::Duration;

use chrono::{Local, Timelike};
use color_eyre::{Result, eyre::Context};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};

use ratatui::{prelude::*, widgets::*};

use crate::{
    TerminalType,
    chrome::{dim_background, dirty_title, status_line},
    colors::RgbSwatch,
    config_menu::EditMessage,
    get_text,
    help_view::HelpViewState,
    tab_page::TabPage,
    term::next_event,
    text_field::set_cursor_mode,
    theme::get_tui_theme,
};

pub struct App {
    pub mode: Mode,
    pub tab: usize,
    pub title: String,
    pub status_line: String,
    pub full_screen: bool,
    pub date_format: String,

    pub tabs: Vec<Box<dyn TabPage>>,
    pub help_state: HelpViewState,

    pub save: SaveChoice,
    /// Whether the exit dialog offers PCBSetup's third answer next to yes and no.
    pub offers_quick_save: bool,
}

/// The answers PCBSetup gave to "Save configuration files (Y/N/Q=Quick Save)".
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SaveChoice {
    /// Write the files and look over what they point at.
    Save,
    /// Write the files and ask nothing.
    QuickSave,
    #[default]
    Discard,
}

impl SaveChoice {
    pub fn writes(self) -> bool {
        self != SaveChoice::Discard
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Command,
    Quit,
    RequestQuit,
    ShowHelp,
}

impl App {
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
        // Somebody else wrote on the screen, so there is nothing left to compare against.
        if crate::term::take_needs_full_redraw() {
            terminal.clear()?;
        }
        terminal
            .draw(|frame| {
                let screen: Rect = get_screen_size(frame, self.full_screen);
                self.help_state.set_area(screen);
                self.ui(frame, screen);

                if self.mode == Mode::ShowHelp {
                    self.show_help(frame, screen);
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
        if self.mode == Mode::ShowHelp {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => self.mode = Mode::Command,
                _ => {
                    self.help_state.handle_key_press(key);
                }
            }
            return;
        }

        if self.mode == Mode::RequestQuit {
            self.handle_quit_dialog(key.code);
            return;
        }

        if self.get_tab().has_control() {
            let state = self.get_tab_mut().handle_key_press(key);
            match state.edit_msg {
                EditMessage::ExternalProgramStarted => {
                    let _ = terminal.clear();
                }
                EditMessage::DisplayHelp(help) => {
                    self.help_state.set_content(&help);
                    self.mode = Mode::ShowHelp;
                }
                _ => (),
            }
            self.status_line = state.status_line;
            return;
        }

        match key.code {
            KeyCode::Esc => {
                if self.tabs.iter().any(|t| t.is_dirty()) {
                    self.save = SaveChoice::Save;
                    self.mode = Mode::RequestQuit;
                } else {
                    self.mode = Mode::Quit;
                }
            }
            KeyCode::BackTab => self.prev_tab(),
            KeyCode::Tab => self.next_tab(),

            _ => {
                let state = self.get_tab_mut().handle_key_press(key);
                match state.edit_msg {
                    EditMessage::ExternalProgramStarted => {
                        let _ = terminal.clear();
                    }
                    EditMessage::DisplayHelp(help) => {
                        self.help_state.set_content(&help);
                        self.mode = Mode::ShowHelp;
                    }
                    _ => (),
                }
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
        self.render_status_line(status_line, frame.buffer_mut());
        self.render_selected_tab(frame, tab);

        if self.mode == Mode::RequestQuit {
            let backdrop = frame.area();
            dim_background(frame.buffer_mut(), backdrop);
            let theme = get_tui_theme();
            let save_text = format!("{} ", get_text("icbtext_save_changes"));
            let mut spans = vec![Span::styled(save_text, theme.item)];
            for (at, choice) in self.choices().iter().enumerate() {
                if at > 0 {
                    spans.push(Span::styled("/", theme.item));
                }
                let label = match choice {
                    SaveChoice::Save => get_text("yes"),
                    SaveChoice::QuickSave => get_text("quick_save"),
                    SaveChoice::Discard => get_text("no"),
                };
                let style = if *choice == self.save { theme.selected_item } else { theme.item };
                spans.push(Span::styled(format!(" {label} "), style));
            }
            let field = Line::from(spans);
            let width = field.width().saturating_add(4).min(area.width as usize) as u16;
            let height = area.height.min(3);
            let save_area = Rect::new(area.x + (area.width - width) / 2, area.y + (area.height - height) / 2, width, height);

            Clear.render(save_area, frame.buffer_mut());

            Block::new()
                .borders(Borders::ALL)
                .style(get_tui_theme().dialog_box)
                .border_type(BorderType::Double)
                .render(save_area, frame.buffer_mut());

            field.render(save_area.inner(Margin { horizontal: 2, vertical: 1 }), frame.buffer_mut());
        }
    }

    fn update_state(&mut self) {
        let state = self.get_tab().request_status();
        self.status_line = state.status_line;
    }

    fn handle_quit_dialog(&mut self, key: KeyCode) {
        match key {
            KeyCode::Left => self.save = self.previous_choice(),
            KeyCode::Right => self.save = self.next_choice(),
            KeyCode::Enter => self.mode = Mode::Quit,
            KeyCode::Esc => self.mode = Mode::Command,
            _ => {}
        }
    }

    fn choices(&self) -> &'static [SaveChoice] {
        if self.offers_quick_save {
            &[SaveChoice::Save, SaveChoice::QuickSave, SaveChoice::Discard]
        } else {
            &[SaveChoice::Save, SaveChoice::Discard]
        }
    }

    fn next_choice(&self) -> SaveChoice {
        let choices = self.choices();
        let at = choices.iter().position(|choice| *choice == self.save).unwrap_or(0);
        choices[(at + 1) % choices.len()]
    }

    fn previous_choice(&self) -> SaveChoice {
        let choices = self.choices();
        let at = choices.iter().position(|choice| *choice == self.save).unwrap_or(0);
        choices[(at + choices.len() - 1) % choices.len()]
    }
    fn show_help(&mut self, frame: &mut Frame, screen: Rect) {
        let area = screen.inner(Margin { horizontal: 2, vertical: 2 });
        Clear.render(area, frame.buffer_mut());

        self.help_state.draw(frame);
    }
}

impl App {
    fn render_title_bar(&self, area: Rect, buf: &mut Buffer) {
        let len = self
            .tabs
            .iter()
            .fold(1usize, |len, t| len.saturating_add(Line::from(t.title()).width()).saturating_add(1));
        let layout = Layout::horizontal([Constraint::Min(0), Constraint::Length(len.min(u16::MAX as usize) as u16)]);
        let [title, tabs] = layout.areas(area);

        Span::styled(dirty_title(&self.title, self.tabs.iter().any(|tab| tab.is_dirty())), get_tui_theme().app_title).render(title, buf);
        let titles = self.tabs.iter().enumerate().map(|(i, t)| {
            if i == self.tab {
                format!(" {} ", t.title())
            } else if i == self.tab + 1 {
                t.title().to_string()
            } else {
                format!(" {}", t.title())
            }
        });
        Tabs::new(titles)
            .style(get_tui_theme().tabs)
            .highlight_style(get_tui_theme().tabs_selected)
            .select(self.tab)
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
        if self.mode == Mode::ShowHelp {
            return;
        }
        self.get_tab_mut().render(frame, area);
    }

    fn render_status_line(&self, area: Rect, buf: &mut Buffer) {
        let now = Local::now();
        let clock = format!("{} {}", now.time().with_nanosecond(0).unwrap(), now.date_naive().format(&self.date_format));
        status_line(buf, area, &self.status_line, &clock);
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
        Rect::new(x, y, width, height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use std::{cell::Cell, rc::Rc};

    struct TestTab(Rc<Cell<bool>>);

    impl TabPage for TestTab {
        fn render(&mut self, frame: &mut Frame, area: Rect) {
            Line::styled("Background", get_tui_theme().item).render(area, frame.buffer_mut());
        }

        fn title(&self) -> String {
            "Tab".into()
        }

        fn is_dirty(&self) -> bool {
            self.0.get()
        }
    }

    fn row_text(buffer: &Buffer, y: u16) -> String {
        (0..buffer.area.width).map(|x| buffer[(x, y)].symbol()).collect()
    }

    #[test]
    fn header_tracks_all_tabs_and_status_keeps_context_first() {
        let dirty = Rc::new(Cell::new(false));
        let mut app = dialog(false);
        app.mode = Mode::Command;
        app.title = "Setup".into();
        app.status_line = "F1 Help: selected field".into();
        app.date_format = "%Y-%m-%d".into();
        app.tabs = vec![Box::new(TestTab(Rc::new(Cell::new(false)))), Box::new(TestTab(dirty.clone()))];
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        for changed in [false, true, false] {
            dirty.set(changed);
            terminal.draw(|frame| app.ui(frame, frame.area())).unwrap();
            let buffer = terminal.backend().buffer();
            assert!(row_text(buffer, 0).starts_with(&dirty_title("Setup", changed)));
            assert_eq!(row_text(buffer, 0).contains('*'), changed);
            assert!(row_text(buffer, 24).starts_with(" F1 Help: selected field"));
        }
        for (width, height) in [(0, 0), (1, 1), (2, 2), (8, 3), (20, 5)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| app.ui(frame, frame.area())).unwrap();
            if width >= 8 {
                assert!(row_text(terminal.backend().buffer(), height - 1).starts_with(" F1 Help"));
            }
        }
    }

    #[test]
    fn path_browser_covers_status_and_stays_inside_setup_window() {
        struct BrowserTab(crate::path_browser::PathBrowser);
        impl TabPage for BrowserTab {
            fn title(&self) -> String {
                "Browser".into()
            }
            fn render(&mut self, frame: &mut Frame, _area: Rect) {
                self.0.render(Rect::new(35, 12, 30, 4), frame);
            }
        }
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("visible.txt"), "test").unwrap();
        for (width, height) in [(80, 25), (120, 36), (160, 50)] {
            let browser = crate::path_browser::PathBrowser::new(std::path::Path::new(""), Some(root.path()));
            let mut app = dialog(false);
            app.mode = Mode::Command;
            app.status_line = "UNDERLYING-STATUS-MUST-NOT-SHOW".into();
            app.tabs = vec![Box::new(BrowserTab(browser))];
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            let mut area = Rect::default();
            terminal
                .draw(|frame| {
                    area = get_screen_size(frame, false);
                    app.ui(frame, area);
                })
                .unwrap();
            let buffer = terminal.backend().buffer();
            let text: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
            assert!(text.contains("visible.txt"));
            assert!(text.contains(&get_text("path_browser_title")));
            assert!(!text.contains("UNDERLYING-STATUS"));
            assert_eq!(area.width, 80);
            assert_eq!(area.height, 25);
            for y in 0..height {
                for x in 0..width {
                    if !area.contains(ratatui::layout::Position::new(x, y)) {
                        assert_eq!(buffer[(x, y)], ratatui::buffer::Cell::EMPTY, "outside modal at {x},{y}");
                    }
                }
            }
            let bottom = row_text(buffer, area.bottom() - 1);
            assert!(bottom.contains('─'));
        }
    }

    #[test]
    fn quit_popup_dims_background_and_pads_the_focused_choice() {
        for quick in [false, true] {
            let mut app = dialog(quick);
            app.tabs.push(Box::new(TestTab(Rc::new(Cell::new(true)))));
            let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
            app.mode = Mode::Command;
            terminal.draw(|frame| app.ui(frame, frame.area())).unwrap();
            let mut expected = terminal.backend().buffer().clone();
            let area = expected.area;
            dim_background(&mut expected, area);
            app.mode = Mode::RequestQuit;
            for choice in app.choices() {
                app.save = *choice;
                terminal.draw(|frame| app.ui(frame, frame.area())).unwrap();
                let buffer = terminal.backend().buffer();
                assert_eq!(buffer[(0, 1)], expected[(0, 1)]);
                let labels = [
                    (SaveChoice::Save, get_text("yes")),
                    (SaveChoice::QuickSave, get_text("quick_save")),
                    (SaveChoice::Discard, get_text("no")),
                ];
                for (value, label) in labels {
                    if !app.choices().contains(&value) {
                        continue;
                    }
                    let text = row_text(buffer, 12);
                    let label = format!(" {label} ");
                    let start = text.find(&label).unwrap();
                    let x = Line::raw(&text[..start]).width() as u16;
                    let style = if value == app.save {
                        get_tui_theme().selected_item
                    } else {
                        get_tui_theme().item
                    };
                    for offset in 0..Line::raw(&label).width() as u16 {
                        assert_eq!(
                            buffer[(x + offset, 12)].style(),
                            get_tui_theme().dialog_box.patch(style).underline_color(ratatui::style::Color::Reset)
                        );
                    }
                }
            }
            for (width, height) in [(0, 0), (1, 1), (2, 2), (8, 3), (20, 5)] {
                let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                terminal.draw(|frame| app.ui(frame, frame.area())).unwrap();
            }
        }
    }

    fn dialog(offers_quick_save: bool) -> App {
        App {
            mode: Mode::RequestQuit,
            tab: 0,
            title: String::new(),
            status_line: String::new(),
            full_screen: false,
            date_format: String::new(),
            tabs: Vec::new(),
            help_state: HelpViewState::new(),
            save: SaveChoice::Save,
            offers_quick_save,
        }
    }

    fn press(app: &mut App, code: KeyCode) {
        app.handle_quit_dialog(code);
    }

    #[test]
    fn without_the_offer_the_dialog_has_two_answers() {
        let mut app = dialog(false);
        press(&mut app, KeyCode::Right);
        assert_eq!(app.save, SaveChoice::Discard);
        press(&mut app, KeyCode::Right);
        assert_eq!(app.save, SaveChoice::Save);
    }

    #[test]
    fn the_quick_save_sits_between_yes_and_no() {
        let mut app = dialog(true);
        press(&mut app, KeyCode::Right);
        assert_eq!(app.save, SaveChoice::QuickSave);
        press(&mut app, KeyCode::Right);
        assert_eq!(app.save, SaveChoice::Discard);
        press(&mut app, KeyCode::Right);
        assert_eq!(app.save, SaveChoice::Save);
    }

    #[test]
    fn the_answers_can_be_walked_backwards() {
        let mut app = dialog(true);
        press(&mut app, KeyCode::Left);
        assert_eq!(app.save, SaveChoice::Discard);
        press(&mut app, KeyCode::Left);
        assert_eq!(app.save, SaveChoice::QuickSave);
    }

    #[test]
    fn enter_takes_the_answer_and_escape_goes_back() {
        let mut app = dialog(true);
        press(&mut app, KeyCode::Right);
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.mode, Mode::Quit);
        assert_eq!(app.save, SaveChoice::QuickSave);

        let mut app = dialog(true);
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.mode, Mode::Command);
    }

    #[test]
    fn only_discarding_leaves_the_files_alone() {
        assert!(SaveChoice::Save.writes());
        assert!(SaveChoice::QuickSave.writes());
        assert!(!SaveChoice::Discard.writes());
    }
}
