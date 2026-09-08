//! Read-only configuration path selection. Browsing never creates or copies files.
use std::{
    io,
    path::{Path, PathBuf},
};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Borders, Clear, FrameExt, Paragraph, Wrap},
};
use ratatui_explorer::{FileExplorer, FileExplorerBuilder, Input, Theme as ExplorerTheme};

use crate::{get_text, theme::get_tui_theme};

fn explorer_theme() -> ExplorerTheme {
    let theme = get_tui_theme();
    ExplorerTheme::new()
        .with_style(theme.description_text)
        .with_block(
            Block::default()
                .borders(Borders::ALL)
                .style(theme.description_text)
                .border_style(theme.dialog_box),
        )
        // Plain files recede; only directories and the selection are emphasised.
        .with_item_style(theme.description_text)
        .with_dir_style(theme.menu_title)
        .with_highlight_item_style(theme.selected_item)
        .with_highlight_dir_style(theme.selected_item)
}

pub(crate) enum PathBrowserResult {
    Pending,
    Cancelled,
    Selected(PathBuf),
}

pub(crate) struct PathBrowser {
    explorer: Option<FileExplorer>,
    base: PathBuf,
    relative: bool,
    error: Option<String>,
}

impl PathBrowser {
    pub(crate) fn new(value: &Path, base: Option<&Path>) -> Self {
        let mut browser = Self {
            explorer: None,
            base: PathBuf::new(),
            relative: value.is_relative(),
            error: None,
        };
        let result = (|| -> io::Result<FileExplorer> {
            browser.base = std::path::absolute(base.filter(|path| !path.as_os_str().is_empty()).unwrap_or_else(|| Path::new(".")))?;
            let target = if value.is_absolute() { value.to_path_buf() } else { browser.base.join(value) };
            // An unset/new filename starts at its nearest existing directory.
            let directory = target
                .ancestors()
                .find(|path| path.is_dir())
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, get_text("path_browser_missing")))?;
            let mut explorer = FileExplorerBuilder::default().working_dir(directory).build()?;
            explorer.set_theme(explorer_theme());
            if target.file_name().is_some_and(|name| name.to_string_lossy().starts_with('.')) && target.is_file() {
                explorer.handle(Input::ToggleShowHidden)?;
            }
            if let Some(index) = explorer.files().iter().position(|file| file.path == target) {
                explorer.set_selected_idx(index);
            }
            Ok(explorer)
        })();
        match result {
            Ok(explorer) => browser.explorer = Some(explorer),
            Err(error) => browser.error = Some(error.to_string()),
        }
        browser
    }

    fn selected(&self, path: PathBuf) -> PathBrowserResult {
        let path = if self.relative {
            path.strip_prefix(&self.base)
                .map(|relative| {
                    if relative.as_os_str().is_empty() {
                        PathBuf::from(".")
                    } else {
                        relative.to_path_buf()
                    }
                })
                .unwrap_or(path)
        } else {
            path
        };
        PathBrowserResult::Selected(path)
    }

    pub(crate) fn handle(&mut self, key: KeyEvent) -> PathBrowserResult {
        if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return PathBrowserResult::Pending;
        }
        if key.code == KeyCode::Esc {
            return PathBrowserResult::Cancelled;
        }
        let Some(explorer) = &mut self.explorer else {
            return PathBrowserResult::Pending;
        };
        if key.modifiers == KeyModifiers::CONTROL && matches!(key.code, KeyCode::Char('s' | 'S')) {
            let path = explorer.cwd().clone();
            if path.is_dir() {
                return self.selected(path);
            }
            self.error = Some(get_text("path_browser_missing"));
            return PathBrowserResult::Pending;
        }
        if key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER) {
            return PathBrowserResult::Pending;
        }
        if explorer.files().is_empty() && !matches!(key.code, KeyCode::Left | KeyCode::Backspace | KeyCode::Char('.')) {
            return PathBrowserResult::Pending;
        }
        self.error = None;
        let result = match key.code {
            KeyCode::Enter | KeyCode::Right => {
                let path = explorer.current().path.clone();
                if path.is_dir() {
                    // Unlike Input::Right, set_cwd retains the list on read errors.
                    explorer.set_cwd(path)
                } else if key.code == KeyCode::Enter {
                    if path.is_file() {
                        return self.selected(path);
                    }
                    Err(io::Error::new(io::ErrorKind::InvalidInput, get_text("path_browser_missing")))
                } else {
                    Ok(())
                }
            }
            KeyCode::Left | KeyCode::Backspace => explorer.handle(Input::Left),
            KeyCode::Up => explorer.handle(Input::Up),
            KeyCode::Down => explorer.handle(Input::Down),
            KeyCode::Home => explorer.handle(Input::Home),
            KeyCode::End => explorer.handle(Input::End),
            KeyCode::PageUp => explorer.handle(Input::PageUp),
            KeyCode::PageDown => explorer.handle(Input::PageDown),
            KeyCode::Char('.') => explorer.handle(Input::ToggleShowHidden),
            _ => Ok(()),
        };
        if let Err(error) = result {
            self.error = Some(error.to_string());
        }
        PathBrowserResult::Pending
    }

    pub(crate) fn render(&self, area: Rect, frame: &mut Frame) {
        let theme = get_tui_theme();
        let area = Self::modal_area(frame.area(), area);
        frame.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .title(get_text("path_browser_title"))
            .style(theme.dialog_box)
            .border_style(theme.dialog_box)
            .title_style(theme.dialog_box_title);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let [path, files, help, error] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(3),
            Constraint::Length(if self.error.is_some() { 2 } else { 0 }),
        ])
        .areas(inner);
        if let Some(explorer) = &self.explorer {
            frame.render_widget(Paragraph::new(explorer.cwd().display().to_string()).style(theme.value), path);
            if explorer.files().is_empty() {
                frame.render_widget(Paragraph::new(get_text("path_browser_empty")).style(theme.description_text), files);
            } else if !files.is_empty() {
                frame.render_widget_ref(explorer.widget(), files);
            }
        }
        frame.render_widget(
            Paragraph::new(get_text("path_browser_keys"))
                .style(theme.description_text)
                .wrap(Wrap { trim: false }),
            help,
        );
        if let Some(message) = &self.error {
            frame.render_widget(Paragraph::new(message.as_str()).style(theme.false_value).wrap(Wrap { trim: false }), error);
        }
    }

    /// Browsing needs room, so this is a modal over the whole frame rather than
    /// the popup rectangle its caller owns. The app's title bar and status line
    /// stay readable; a frame too small for them keeps the caller's area.
    fn modal_area(frame: Rect, area: Rect) -> Rect {
        if frame.height > 6 && frame.width > 8 {
            Rect::new(frame.x, frame.y + 1, frame.width, frame.height - 2)
        } else {
            area
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend, style::Style};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn selected(result: PathBrowserResult) -> PathBuf {
        match result {
            PathBrowserResult::Selected(path) => path,
            _ => panic!("expected a selected path"),
        }
    }

    #[test]
    fn file_selection_preserves_relative_and_absolute_paths() {
        let root = tempfile::tempdir().unwrap();
        let filename = "café 界.txt";
        let file = root.path().join(filename);
        std::fs::write(&file, "unchanged").unwrap();
        let cwd = std::env::current_dir().unwrap();
        for value in [PathBuf::from(filename), file.clone()] {
            let mut browser = PathBrowser::new(&value, Some(root.path()));
            assert_eq!(browser.explorer.as_ref().unwrap().current().path, file);
            assert_eq!(selected(browser.handle(key(KeyCode::Enter))), value);
        }
        assert_eq!(std::env::current_dir().unwrap(), cwd);
        assert_eq!(std::fs::read_to_string(file).unwrap(), "unchanged");
    }

    #[test]
    fn missing_paths_start_at_existing_ancestor_and_directory_selection_is_explicit() {
        let root = tempfile::tempdir().unwrap();
        let nested = root.path().join("nested");
        std::fs::create_dir(&nested).unwrap();
        let mut browser = PathBrowser::new(Path::new("missing/sub/new.txt"), Some(root.path()));
        assert_eq!(browser.explorer.as_ref().unwrap().cwd(), root.path());
        browser.handle(key(KeyCode::End));
        assert!(matches!(browser.handle(key(KeyCode::Enter)), PathBrowserResult::Pending));
        assert_eq!(browser.explorer.as_ref().unwrap().cwd(), &nested);
        assert_eq!(
            selected(browser.handle(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL))),
            PathBuf::from("nested")
        );
        browser.handle(key(KeyCode::Backspace));
        assert_eq!(
            selected(browser.handle(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL))),
            PathBuf::from(".")
        );
        assert!(!root.path().join("missing").exists());
    }

    #[test]
    fn hidden_files_cancellation_and_paths_outside_board() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let file = outside.path().join(".hidden");
        std::fs::write(&file, "test").unwrap();
        let mut browser = PathBrowser::new(Path::new(""), Some(root.path()));
        let explorer = browser.explorer.as_mut().unwrap();
        explorer.set_cwd(outside.path()).unwrap();
        assert!(!explorer.files().iter().any(|entry| entry.path == file));
        browser.handle(key(KeyCode::Char('.')));
        browser.handle(key(KeyCode::End));
        assert_eq!(selected(browser.handle(key(KeyCode::Enter))), file);
        assert!(matches!(browser.handle(key(KeyCode::Esc)), PathBrowserResult::Cancelled));
        let browser = PathBrowser::new(&file, Some(root.path()));
        assert_eq!(browser.explorer.unwrap().current().path, file);
    }

    #[test]
    fn vanished_entry_and_key_release_do_not_confirm() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("selected");
        std::fs::write(&file, "test").unwrap();
        let mut browser = PathBrowser::new(&file, Some(root.path()));
        assert!(matches!(
            browser.handle(KeyEvent::new_with_kind(KeyCode::Enter, KeyModifiers::NONE, KeyEventKind::Release)),
            PathBrowserResult::Pending
        ));
        std::fs::remove_file(&file).unwrap();
        assert!(matches!(browser.handle(key(KeyCode::Enter)), PathBrowserResult::Pending));
        assert!(browser.error.is_some());
        assert!(matches!(browser.handle(key(KeyCode::Esc)), PathBrowserResult::Cancelled));
    }

    #[test]
    fn picker_uses_active_theme_colors() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("selected.txt");
        std::fs::write(&file, "test").unwrap();
        std::fs::write(root.path().join("other.txt"), "test").unwrap();
        let mut browser = PathBrowser::new(&file, Some(root.path()));
        browser.error = Some("Example error".into());
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        for select_directory in [false, true] {
            if select_directory {
                browser.handle(key(KeyCode::Home));
                browser.error = Some("Example error".into());
            }
            terminal
                .draw(|frame| {
                    let area = frame.area();
                    browser.render(area, frame);
                })
                .unwrap();
            let buffer = terminal.backend().buffer();
            let theme = get_tui_theme();
            let assert_text = |text: &str, style: Style| {
                for y in 0..25 {
                    let row: String = (0..100).map(|x| buffer[(x, y)].symbol()).collect();
                    if let Some(offset) = row.find(text) {
                        let x = row[..offset].chars().count() as u16;
                        assert_eq!(buffer[(x, y)].fg, style.fg.unwrap(), "{text}");
                        assert_eq!(buffer[(x, y)].bg, style.bg.unwrap(), "{text}");
                        return;
                    }
                }
                panic!("missing text: {text}");
            };
            assert_text(&get_text("path_browser_title"), theme.dialog_box_title);
            assert_text("Enter:", theme.description_text);
            assert_text("Example error", theme.false_value);
            assert_text("other.txt", theme.description_text);
            assert_text("selected.txt", if select_directory { theme.description_text } else { theme.selected_item });
            assert_text("../", if select_directory { theme.selected_item } else { theme.menu_title });
            assert_ne!(theme.description_text.fg, theme.menu_title.fg, "directories stay distinguishable");
            // The modal starts below the title bar and ends above the status line.
            assert_eq!(
                (buffer[(0, 1)].fg, buffer[(0, 1)].bg),
                (theme.dialog_box.fg.unwrap(), theme.dialog_box.bg.unwrap())
            );
            assert_eq!((buffer[(1, 2)].fg, buffer[(1, 2)].bg), (theme.value.fg.unwrap(), theme.value.bg.unwrap()));
            assert_eq!(buffer[(0, 0)], ratatui::buffer::Cell::EMPTY);
            assert_eq!(buffer[(0, 24)], ratatui::buffer::Cell::EMPTY);
        }
    }

    #[test]
    fn renders_help_and_handles_small_areas() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("visible.txt");
        std::fs::write(&file, "test").unwrap();
        let browser = PathBrowser::new(&file, Some(root.path()));
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        terminal.draw(|frame| browser.render(frame.area(), frame)).unwrap();
        let text: String = terminal.backend().buffer().content.iter().map(|cell| cell.symbol()).collect();
        assert!(text.contains("visible.txt"));
        assert!(text.contains(&get_text("path_browser_title")));
        assert!(text.contains("Esc:"));
        // A caller's small popup no longer limits the browser.
        terminal.draw(|frame| browser.render(Rect::new(30, 10, 20, 6), frame)).unwrap();
        let buffer = terminal.backend().buffer();
        let row: String = (0..100).map(|x| buffer[(x, 1)].symbol()).collect();
        assert!(row.contains(&get_text("path_browser_title")), "{row}");
        assert!((0..100).map(|x| buffer[(x, 23)].symbol()).any(|symbol| symbol != " "));
        for width in 0..10 {
            for height in 0..10 {
                let mut tiny = Terminal::new(TestBackend::new(width.max(1), height.max(1))).unwrap();
                tiny.draw(|frame| browser.render(Rect::new(0, 0, width, height), frame)).unwrap();
            }
        }
    }
}
