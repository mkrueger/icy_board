//! Local-console modal. No event handled here is forwarded to the BBS connection.
use std::{io, path::PathBuf};

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use icy_board_engine::icy_board::state::local_transfer::{LocalFilePickerKind, LocalFilePickerRequest};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, FrameExt, Paragraph, Wrap},
};
use ratatui_explorer::{FileExplorer, FileExplorerBuilder, Input, Theme};
use tokio::sync::oneshot;

pub(super) struct LocalFilePicker {
    kind: LocalFilePickerKind,
    explorer: Option<FileExplorer>,
    response: Option<oneshot::Sender<Option<PathBuf>>>,
    error: Option<String>,
}

impl LocalFilePicker {
    pub(super) fn new(request: LocalFilePickerRequest) -> Self {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let explorer = home
            .map(|path| FileExplorerBuilder::default().working_dir(path).build())
            .unwrap_or_else(FileExplorer::new)
            .or_else(|_| FileExplorer::new());
        Self::from_explorer(request, explorer)
    }

    fn from_explorer(request: LocalFilePickerRequest, explorer: io::Result<FileExplorer>) -> Self {
        let (explorer, error) = match explorer {
            Ok(mut explorer) => {
                // The library constructors use Theme::new(), which has no selection
                // highlight. A picker must make the active entry visible.
                explorer.set_theme(Theme::default());
                (Some(explorer), None)
            }
            Err(error) => (None, Some(format!("Cannot open file picker: {error}"))),
        };
        Self {
            kind: request.kind,
            explorer,
            response: Some(request.response),
            error,
        }
    }

    pub(super) fn is_closed(&self) -> bool {
        self.response.as_ref().is_none_or(oneshot::Sender::is_closed)
    }

    fn finish(&mut self, path: Option<PathBuf>) {
        if let Some(response) = self.response.take() {
            let _ = response.send(path);
        }
    }

    /// Return true only when the modal closes. Mouse, paste, releases, hotkeys,
    /// and unrelated keys are deliberately consumed without reaching the session.
    pub(super) fn handle(&mut self, event: Event) -> bool {
        let Event::Key(key) = event else { return false };
        if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return false;
        }
        if key.code == KeyCode::Esc {
            self.finish(None);
            return true;
        }
        let Some(explorer) = &mut self.explorer else { return false };
        if key.modifiers == KeyModifiers::CONTROL && matches!(key.code, KeyCode::Char('s' | 'S')) {
            if self.kind == LocalFilePickerKind::DownloadDirectory {
                let path = explorer.cwd().clone();
                if path.is_dir() {
                    self.finish(Some(path));
                    return true;
                }
                self.error = Some("Directory no longer exists".into());
            }
            return false;
        }
        if key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER) {
            return false;
        }
        if explorer.files().is_empty() && !matches!(key.code, KeyCode::Left | KeyCode::Backspace | KeyCode::Char('.')) {
            self.error = Some("No entries; use Ctrl-S to select this directory or Esc to cancel".into());
            return false;
        }
        self.error = None;
        let result = match key.code {
            KeyCode::Enter | KeyCode::Right => {
                let path = explorer.current().path.clone();
                if path.is_dir() {
                    // Explorer's Input::Right removes the selected entry before a
                    // failed directory read; set_cwd preserves it on errors.
                    explorer.set_cwd(path)
                } else if key.code == KeyCode::Enter && self.kind == LocalFilePickerKind::UploadFile {
                    match std::fs::symlink_metadata(&path) {
                        Ok(metadata) if metadata.file_type().is_file() => {
                            self.finish(Some(path));
                            return true;
                        }
                        Ok(_) => Err(io::Error::new(io::ErrorKind::InvalidInput, "Select a regular file (not a symlink)")),
                        Err(error) => Err(error),
                    }
                } else {
                    self.error = Some("Enter opens directories; Ctrl-S selects the current directory".into());
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
        false
    }

    pub(super) fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        frame.render_widget(Clear, area);
        let title = match self.kind {
            LocalFilePickerKind::UploadFile => " Local upload — select a file ",
            LocalFilePickerKind::DownloadDirectory => " Local download — select a directory ",
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().fg(Color::White).bg(Color::Blue));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let [path, browser, help, error] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0), Constraint::Length(3), Constraint::Length(2)]).areas(inner);
        if let Some(explorer) = &self.explorer {
            frame.render_widget(Paragraph::new(explorer.cwd().display().to_string()), path);
            if explorer.files().is_empty() {
                frame.render_widget(Paragraph::new("Empty directory"), browser);
            } else {
                frame.render_widget_ref(explorer.widget(), browser);
            }
        }
        let action = match self.kind {
            LocalFilePickerKind::UploadFile => "Enter: upload file / open directory",
            LocalFilePickerKind::DownloadDirectory => "Enter: open directory | Ctrl-S: select CURRENT directory",
        };
        let hidden = if self.explorer.as_ref().is_some_and(FileExplorer::show_hidden) {
            "shown"
        } else {
            "hidden"
        };
        frame.render_widget(
            Paragraph::new(format!(
                "{action}\nArrows/PgUp/PgDn: browse | Backspace: parent\nEsc: cancel | .: toggle hidden files ({hidden})"
            ))
            .wrap(Wrap { trim: false }),
            help,
        );
        if let Some(message) = &self.error {
            frame.render_widget(
                Paragraph::new(message.as_str())
                    .style(Style::default().fg(Color::Yellow))
                    .wrap(Wrap { trim: false }),
                error,
            );
        }
    }
}

impl Drop for LocalFilePicker {
    fn drop(&mut self) {
        self.finish(None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
    use ratatui::{Terminal, backend::TestBackend};

    fn picker(kind: LocalFilePickerKind, path: &std::path::Path) -> (LocalFilePicker, oneshot::Receiver<Option<PathBuf>>) {
        let (response, receiver) = oneshot::channel();
        let explorer = FileExplorerBuilder::default().working_dir(path).build();
        (LocalFilePicker::from_explorer(LocalFilePickerRequest { kind, response }, explorer), receiver)
    }

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn draw(picker: &LocalFilePicker) -> String {
        let mut terminal = Terminal::new(TestBackend::new(90, 24)).unwrap();
        terminal.draw(|frame| picker.render(frame)).unwrap();
        terminal.backend().buffer().content.iter().map(|cell| cell.symbol()).collect::<String>()
    }

    #[test]
    fn upload_navigation_hidden_files_and_selection() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("nested");
        std::fs::create_dir(&nested).unwrap();
        let file = nested.join(".hidden.bin");
        std::fs::write(&file, b"upload").unwrap();
        let (mut picker, mut response) = picker(LocalFilePickerKind::UploadFile, dir.path());
        picker.explorer.as_mut().unwrap().set_working_file(&nested).unwrap();
        assert!(!picker.handle(key(KeyCode::Enter)));
        assert_eq!(picker.explorer.as_ref().unwrap().cwd(), &nested);
        assert!(draw(&picker).contains("Esc: cancel"));
        assert!(draw(&picker).contains(&nested.display().to_string()));
        let mut terminal = Terminal::new(TestBackend::new(90, 24)).unwrap();
        terminal.draw(|frame| picker.render(frame)).unwrap();
        assert!(terminal.backend().buffer().content.iter().any(|cell| cell.bg == Color::DarkGray));
        assert!(!draw(&picker).contains(".hidden.bin"));
        picker.handle(key(KeyCode::Char('.')));
        assert!(draw(&picker).contains(".hidden.bin"));
        picker.handle(key(KeyCode::End));
        assert!(picker.handle(key(KeyCode::Enter)));
        assert_eq!(response.try_recv().unwrap(), Some(file));
    }

    #[test]
    fn directory_requires_explicit_confirmation_and_blocks_paste_mouse_hotkeys() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("file.bin");
        std::fs::write(&file, b"download").unwrap();
        let (mut picker, mut response) = picker(LocalFilePickerKind::DownloadDirectory, dir.path());
        picker.explorer.as_mut().unwrap().set_working_file(&file).unwrap();
        assert!(!picker.handle(key(KeyCode::Enter)));
        assert!(draw(&picker).contains("Ctrl-S"));
        let cwd = picker.explorer.as_ref().unwrap().cwd().clone();
        for event in [
            Event::Paste("\x1b\r\x13".into()),
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 4,
                row: 4,
                modifiers: KeyModifiers::NONE,
            }),
            Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT)),
            key(KeyCode::F(10)),
        ] {
            assert!(!picker.handle(event));
            assert!(response.try_recv().is_err());
            assert_eq!(picker.explorer.as_ref().unwrap().cwd(), &cwd);
        }
        assert!(picker.handle(Event::Key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL))));
        assert_eq!(response.try_recv().unwrap(), Some(cwd));
    }

    #[test]
    fn escape_drop_and_initialization_error_cancel() {
        let dir = tempfile::tempdir().unwrap();
        let (mut picker, mut response) = picker(LocalFilePickerKind::UploadFile, dir.path());
        assert!(picker.handle(key(KeyCode::Esc)));
        assert_eq!(response.try_recv().unwrap(), None);
        let (response, mut receiver) = oneshot::channel();
        let picker = LocalFilePicker::from_explorer(
            LocalFilePickerRequest {
                kind: LocalFilePickerKind::UploadFile,
                response,
            },
            Err(io::Error::other("denied")),
        );
        assert!(draw(&picker).contains("denied"));
        drop(picker);
        assert_eq!(receiver.try_recv().unwrap(), None);
    }

    #[test]
    fn failed_navigation_preserves_selection_and_renders_error() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("vanished");
        std::fs::write(&file, b"gone").unwrap();
        let (mut picker, mut response) = picker(LocalFilePickerKind::UploadFile, dir.path());
        picker.explorer.as_mut().unwrap().set_working_file(&file).unwrap();
        std::fs::remove_file(&file).unwrap();
        assert!(!picker.handle(key(KeyCode::Enter)));
        assert!(picker.error.is_some());
        assert!(response.try_recv().is_err());
        // Tiny terminals must also render without underflows or panics.
        let mut terminal = Terminal::new(TestBackend::new(8, 3)).unwrap();
        terminal.draw(|frame| picker.render(frame)).unwrap();
    }
}
