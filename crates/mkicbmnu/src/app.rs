use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::{Local, Timelike};
use color_eyre::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use icy_board_engine::icy_board::{IcyBoard, menu::Menu};
use icy_board_tui::{
    TerminalType,
    app::{SaveChoice, get_screen_size, render_save_dialog},
    chrome::{dim_background, dirty_title, status_line},
    config_menu::EditMessage,
    get_text,
    help_view::HelpViewState,
    hotkeys::{Hotkey, HotkeyBar},
    tab_page::TabPage,
    term,
    theme::get_tui_theme,
};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Margin, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Tabs, Widget, Wrap},
};

use crate::{AboutTab, CommandsTab, GeneralTab, PreviewTab, PromptsTab, document::Document, validation::validate_menu};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Edit,
    Help,
    Quit,
    Review,
}

/// mkicbmnu writes one file, so PCBSetup's quick-save answer has no meaning here.
const QUIT_CHOICES: &[SaveChoice] = &[SaveChoice::Save, SaveChoice::Discard];

pub struct MenuApp {
    board: Arc<Mutex<IcyBoard>>,
    menu: Arc<Mutex<Menu>>,
    document: Document,
    title: String,
    date_format: String,
    tabs: Vec<Box<dyn TabPage>>,
    tab: usize,
    full_screen: bool,
    mode: Mode,
    help: HelpViewState,
    status: String,
    save: SaveChoice,
    review: Vec<String>,
    review_scroll: u16,
    close_after_save: bool,
    quit: bool,
}

pub fn new_main_window(board: IcyBoard, menu: Arc<Mutex<Menu>>, full_screen: bool, path: &Path, create: bool) -> Result<MenuApp> {
    let document = Document::new(path, &menu.lock().unwrap(), create)?;
    let date_format = board.config.board.date_format.clone();
    // The file name would crowd out the tabs here; the General page shows it.
    let title = format!(" {}", get_text("app_mkicbmnu"));
    let mut app = MenuApp {
        board: Arc::new(Mutex::new(board)),
        menu,
        document,
        title,
        date_format,
        tabs: Vec::new(),
        tab: 0,
        full_screen,
        mode: Mode::Edit,
        help: HelpViewState::new(),
        status: if create { get_text("mnu_app_new") } else { String::new() },
        save: SaveChoice::Save,
        review: Vec::new(),
        review_scroll: 0,
        close_after_save: false,
        quit: false,
    };
    app.rebuild_tabs();
    Ok(app)
}

impl MenuApp {
    fn rebuild_tabs(&mut self) {
        let root = self.board.lock().unwrap().root_path.clone();
        self.tabs = vec![
            Box::new(GeneralTab::new(self.menu.clone(), root, self.document.path.clone())),
            Box::new(CommandsTab::new(self.board.clone(), self.menu.clone())),
            Box::new(PromptsTab::new(self.board.clone(), self.menu.clone())),
            Box::new(PreviewTab::new(self.board.clone(), self.menu.clone())),
            Box::new(AboutTab::default()),
        ];
    }

    pub fn run(&mut self, terminal: &mut TerminalType) -> Result<()> {
        icy_board_tui::text_field::set_cursor_mode();
        while !self.quit {
            if term::take_needs_full_redraw() {
                terminal.clear()?;
            }
            terminal.draw(|frame| self.ui(frame))?;
            if let Some(Event::Key(key)) = term::next_event(Duration::from_millis(200))?
                && key.kind != KeyEventKind::Release
            {
                self.handle_key(key);
            }
        }
        Ok(())
    }

    fn dirty(&self) -> bool {
        self.document.is_dirty(&self.menu.lock().unwrap())
    }

    fn select_tab(&mut self, tab: usize) {
        self.tab = tab;
        self.status = self.tabs[tab].request_status().status_line;
    }

    fn dispatch(&mut self, key: KeyEvent) {
        let before = self.menu.lock().unwrap().clone();
        let state = self.tabs[self.tab].handle_key_press(key);
        self.status = state.status_line;
        if let EditMessage::DisplayHelp(text) = state.edit_msg {
            self.help.set_content(&text);
            self.mode = Mode::Help;
        }
        self.document.record(before, &self.menu.lock().unwrap());
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match self.mode {
            Mode::Help => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => self.mode = Mode::Edit,
                    _ => self.help.handle_key_press(key),
                }
                return;
            }
            Mode::Quit => {
                match key.code {
                    KeyCode::Left => self.save = self.save.step(QUIT_CHOICES, false),
                    KeyCode::Right => self.save = self.save.step(QUIT_CHOICES, true),
                    KeyCode::Enter => {
                        if self.save.writes() {
                            self.request_save(true);
                        } else {
                            self.quit = true;
                        }
                    }
                    KeyCode::Esc => self.mode = Mode::Edit,
                    _ => (),
                }
                return;
            }
            Mode::Review => {
                match key.code {
                    KeyCode::F(8) => self.save(),
                    KeyCode::Esc => {
                        self.mode = Mode::Edit;
                        self.close_after_save = false;
                    }
                    KeyCode::Up => self.review_scroll = self.review_scroll.saturating_sub(1),
                    KeyCode::Down => self.review_scroll = self.review_scroll.saturating_add(1),
                    KeyCode::PageUp => self.review_scroll = self.review_scroll.saturating_sub(8),
                    KeyCode::PageDown => self.review_scroll = self.review_scroll.saturating_add(8),
                    _ => (),
                }
                return;
            }
            Mode::Edit => (),
        }
        // Drafts, pickers and filters own their keys; never save stale parent data.
        if self.tabs[self.tab].has_control() {
            self.dispatch(key);
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('s') => {
                    self.request_save(false);
                    return;
                }
                KeyCode::Char('z') => {
                    self.document.undo(&mut self.menu.lock().unwrap());
                    self.rebuild_tabs();
                    return;
                }
                KeyCode::Char('y') => {
                    self.document.redo(&mut self.menu.lock().unwrap());
                    self.rebuild_tabs();
                    return;
                }
                _ => (),
            }
        }
        match key.code {
            KeyCode::Esc => {
                if self.dirty() {
                    self.save = SaveChoice::Save;
                    self.mode = Mode::Quit;
                } else {
                    self.quit = true;
                }
            }
            KeyCode::Tab => self.select_tab((self.tab + 1) % self.tabs.len()),
            KeyCode::BackTab => self.select_tab((self.tab + self.tabs.len() - 1) % self.tabs.len()),
            KeyCode::F(9) => self.select_tab(3),
            KeyCode::F(1) if self.tab != 3 => {
                self.help.set_content(&get_text("mnu_app_help"));
                self.mode = Mode::Help;
            }
            _ => self.dispatch(key),
        }
    }

    fn request_save(&mut self, close: bool) {
        self.close_after_save = close;
        self.review = validate_menu(&self.board.lock().unwrap(), &self.menu.lock().unwrap())
            .into_iter()
            .map(|issue| {
                format!(
                    "{}: {}",
                    issue.command.map_or_else(|| get_text("mnu_check_menu"), |i| format!("#{}", i + 1)),
                    issue.message
                )
            })
            .collect();
        if self.review.is_empty() {
            self.save();
        } else {
            self.mode = Mode::Review;
            self.review_scroll = 0;
        }
    }

    fn save(&mut self) {
        match self.document.save(&self.menu.lock().unwrap()) {
            Ok(()) => {
                self.status = get_text("mnu_app_saved");
                self.quit = self.close_after_save;
            }
            Err(err) => {
                self.status = format!("{}: {err}", get_text("mnu_app_save_error"));
                self.quit = false;
            }
        }
        self.close_after_save = false;
        self.mode = Mode::Edit;
    }

    fn hints(&self) -> HotkeyBar {
        HotkeyBar::new([
            Hotkey::modified(KeyModifiers::CONTROL, KeyCode::Char('s'), get_text("mnu_app_save")),
            Hotkey::modified(KeyModifiers::CONTROL, KeyCode::Char('z'), get_text("mnu_app_undo")),
            Hotkey::modified(KeyModifiers::CONTROL, KeyCode::Char('y'), get_text("mnu_app_redo")),
            Hotkey::new(KeyCode::F(9), get_text("mnu_preview_title")),
            Hotkey::new(KeyCode::Tab, get_text("mnu_app_tabs")),
            Hotkey::new(KeyCode::Esc, get_text("mnu_app_quit")),
        ])
    }

    /// PCBSetup's title row: application and file on the left, tabs on the right.
    fn render_title_bar(&self, area: Rect, buf: &mut Buffer) {
        let theme = get_tui_theme();
        let width = self
            .tabs
            .iter()
            .fold(1usize, |width, tab| width.saturating_add(Line::from(tab.title()).width()).saturating_add(1));
        let [title, tabs] = Layout::horizontal([Constraint::Min(0), Constraint::Length(width.min(u16::MAX as usize) as u16)]).areas(area);
        Span::styled(dirty_title(&self.title, self.dirty()), theme.app_title).render(title, buf);
        let titles = self.tabs.iter().enumerate().map(|(i, tab)| {
            if i == self.tab {
                format!(" {} ", tab.title())
            } else if i == self.tab + 1 {
                tab.title()
            } else {
                format!(" {}", tab.title())
            }
        });
        Tabs::new(titles)
            .style(theme.tabs)
            .highlight_style(theme.tabs_selected)
            .select(self.tab)
            .divider("")
            .padding("", "")
            .render(tabs, buf);
    }

    fn ui(&mut self, frame: &mut Frame) {
        let screen = get_screen_size(frame, self.full_screen);
        let theme = get_tui_theme();
        self.help.set_area(screen);
        Block::new().style(theme.title_bar).render(screen, frame.buffer_mut());
        let modal = self.tabs[self.tab].has_control();
        let hint_rows = self.hints().rows(screen.width);
        let rows = if modal { 0 } else { hint_rows.len().min(3) as u16 };
        let [title, body, status, footer] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(if modal { 0 } else { 1 }),
            Constraint::Length(rows),
        ])
        .areas(screen);
        self.render_title_bar(title, frame.buffer_mut());
        Block::new().style(theme.background).render(body, frame.buffer_mut());
        if self.mode == Mode::Edit && !modal {
            let now = Local::now();
            let clock = format!("{} {}", now.time().with_nanosecond(0).unwrap(), now.date_naive().format(&self.date_format));
            status_line(frame.buffer_mut(), status, &self.status, &clock);
            for (line, area) in hint_rows.into_iter().zip(footer.rows()) {
                line.render(area, frame.buffer_mut());
            }
        }
        // Paint last so full-screen position/path controls cover parent chrome.
        self.tabs[self.tab].render(frame, body);
        if self.mode == Mode::Help {
            let area = screen.inner(Margin::new(2, 2));
            Clear.render(area, frame.buffer_mut());
            self.help.draw(frame);
            return;
        }
        if self.mode == Mode::Quit {
            dim_background(frame.buffer_mut(), screen);
            render_save_dialog(frame.buffer_mut(), screen, QUIT_CHOICES, self.save);
            return;
        }
        if self.mode == Mode::Review {
            dim_background(frame.buffer_mut(), screen);
            let popup = screen.inner(Margin::new(2, 2));
            Clear.render(popup, frame.buffer_mut());
            let [content, help] = Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).areas(popup.inner(Margin::new(1, 1)));
            let hints = HotkeyBar::new([
                Hotkey::new(KeyCode::F(8), get_text("mnu_app_save_anyway")),
                Hotkey::new(KeyCode::Esc, get_text("mnu_app_back")),
                Hotkey::alternatives([KeyCode::Up, KeyCode::Down], get_text("mnu_app_scroll")),
            ]);
            Block::new()
                .title(Line::styled(get_text("mnu_app_confirm_title"), theme.dialog_box_title))
                .title_bottom(hints.line())
                .borders(Borders::ALL)
                .style(theme.dialog_box)
                .render(popup, frame.buffer_mut());
            let lines = self.review.iter().map(|text| Line::from(text.clone())).collect::<Vec<_>>();
            let paragraph = Paragraph::new(lines).style(theme.item).wrap(Wrap { trim: false });
            self.review_scroll = self.review_scroll.min(
                paragraph
                    .line_count(content.width)
                    .saturating_sub(content.height as usize)
                    .min(u16::MAX as usize) as u16,
            );
            paragraph.scroll((self.review_scroll, 0)).render(content, frame.buffer_mut());
            Paragraph::new(get_text("mnu_app_issues"))
                .style(theme.menu_label)
                .wrap(Wrap { trim: true })
                .render(help, frame.buffer_mut());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }
    fn app(dir: &Path) -> MenuApp {
        new_main_window(IcyBoard::default(), Arc::new(Mutex::new(Menu::default())), false, &dir.join("test.mnu"), true).unwrap()
    }

    #[test]
    fn dialog_owns_tab_and_escape_and_cancels_new_command() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app(dir.path());
        app.tab = 1;
        app.handle_key(key(KeyCode::Insert));
        assert!(app.tabs[1].has_control());
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.tab, 1);
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|f| app.ui(f)).unwrap();
        app.handle_key(key(KeyCode::Esc));
        assert!(!app.tabs[1].has_control());
        assert!(app.menu.lock().unwrap().commands.is_empty());
        assert!(!app.quit);
    }

    #[test]
    fn review_cancel_save_and_exit_work_without_losing_document() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app(dir.path());
        app.request_save(false);
        assert!(app.mode == Mode::Review);
        app.handle_key(key(KeyCode::Esc));
        assert!(app.mode == Mode::Edit);
        assert!(!app.document.path.exists());
        app.request_save(false);
        app.handle_key(key(KeyCode::F(8)));
        assert!(app.document.path.exists());
        assert!(!app.dirty());
        assert!(!app.quit);
        app.handle_key(key(KeyCode::Esc));
        assert!(app.quit);
    }

    #[test]
    fn the_exit_prompt_is_the_shared_lightbar_answer_bar() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app(dir.path());
        app.handle_key(key(KeyCode::Esc));
        assert!(app.mode == Mode::Quit, "an unsaved document asks first");
        assert_eq!(app.save, SaveChoice::Save, "yes is preselected like in the other tools");
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|f| app.ui(f)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        let row = |y: u16| (0..buffer.area.width).map(|x| buffer[(x, y)].symbol().to_string()).collect::<String>();
        let y = (0..25)
            .find(|y| row(*y).contains(&get_text("icbtext_save_changes")))
            .expect("the shared question");
        let line = row(y);
        assert!(line.contains(&get_text("yes")) && line.contains(&get_text("no")), "{line}");
        let theme = get_tui_theme();
        let lit = |y: u16| (0..buffer.area.width).filter(|x| buffer[(*x, y)].style().bg == theme.selected_item.bg).count();
        assert!(lit(y) > 0, "the selected answer carries the lightbar");

        app.handle_key(key(KeyCode::Right));
        assert_eq!(app.save, SaveChoice::Discard);
        app.handle_key(key(KeyCode::Right));
        assert_eq!(app.save, SaveChoice::Save, "the answer bar wraps around");
        app.handle_key(key(KeyCode::Left));
        assert_eq!(app.save, SaveChoice::Discard);
        app.handle_key(key(KeyCode::Esc));
        assert!(app.mode == Mode::Edit && !app.quit, "escape returns to editing");

        app.handle_key(key(KeyCode::Esc));
        app.handle_key(key(KeyCode::Enter));
        assert!(app.mode == Mode::Review, "yes still runs the checks before saving");
        app.handle_key(key(KeyCode::Esc));
        app.handle_key(key(KeyCode::Esc));
        app.handle_key(key(KeyCode::Right));
        app.handle_key(key(KeyCode::Enter));
        assert!(app.quit && !app.document.path.exists(), "no discards without writing");
    }

    #[test]
    fn failed_save_returns_to_edit_and_keeps_changes() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app(&dir.path().join("missing"));
        app.request_save(true);
        app.handle_key(key(KeyCode::F(8)));
        assert!(app.mode == Mode::Edit);
        assert!(!app.quit);
        assert!(app.dirty());
        assert!(app.status.contains(&get_text("mnu_app_save_error")));
    }

    #[test]
    fn the_title_row_leaves_the_tabs_room_and_the_general_page_names_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app(dir.path());
        app.status = "Status".into();
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|f| app.ui(f)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        let row = |y: u16| (0..buffer.area.width).map(|x| buffer[(x, y)].symbol().to_string()).collect::<String>();
        let title = row(0);
        assert!(title.contains(&get_text("app_mkicbmnu")), "{title}");
        assert!(title.contains('*'), "a new document is unsaved: {title}");
        for tab in &app.tabs {
            assert!(title.contains(&tab.title()), "every tab stays readable: {title}");
        }
        assert!(!title.contains("test.mnu"), "the file name belongs on the General page: {title}");
        let page = (1..24).map(row).collect::<String>();
        assert!(page.contains("test.mnu"), "{page}");
        let theme = get_tui_theme();
        assert_eq!(buffer[(1, 0)].style().bg, theme.app_title.bg);
        let status = (1..25).find(|y| row(*y).contains("Status")).expect("status row");
        assert_eq!(buffer[(1, status)].style().bg, theme.status_line_text.bg);
        assert!(row(status).contains(':'), "the shared status line also shows the clock");
    }

    #[test]
    fn help_uses_the_shared_markdown_view_like_the_other_tools() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app(dir.path());
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|f| app.ui(f)).unwrap();
        app.handle_key(key(KeyCode::F(1)));
        assert!(app.mode == Mode::Help);
        assert!(app.help.markdown.is_some(), "the shared view parses the help as markdown");
        terminal.draw(|f| app.ui(f)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        let rendered = buffer.content.iter().map(|cell| cell.symbol().to_string()).collect::<String>();
        let help = get_text("mnu_app_help");
        let heading = help.lines().find_map(|line| line.strip_prefix("# ")).expect("a markdown heading");
        assert!(rendered.contains(heading), "{rendered}");
        assert!(!rendered.contains(&format!("# {heading}")), "markdown is rendered, not printed raw");
        let theme = get_tui_theme();
        let row = |y: u16| (0..buffer.area.width).map(|x| buffer[(x, y)].symbol().to_string()).collect::<String>();
        let y = (0..buffer.area.height).find(|y| row(*y).contains(heading)).expect("heading row");
        let x = (0..buffer.area.width).find(|x| buffer[(*x, y)].symbol() != " ").expect("heading cell");
        assert_eq!(buffer[(x, y)].style().bg, theme.help_header.bg, "the shared help palette is used");
        app.handle_key(key(KeyCode::Esc));
        assert!(app.mode == Mode::Edit);
    }

    #[test]
    fn all_tabs_render_in_normal_and_small_terminals() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app(dir.path());
        for (w, h) in [(80, 25), (40, 15), (1, 1)] {
            let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
            for tab in 0..app.tabs.len() {
                app.tab = tab;
                terminal.draw(|f| app.ui(f)).unwrap();
            }
        }
        assert_eq!(app.tabs[0].title(), get_text("mnu_general_tab"));
        assert_eq!(app.tabs[3].title(), get_text("mnu_preview_title"));
    }
}
