use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use ratatui::{
    layout::{Margin, Rect},
    text::Line,
    widgets::{Block, Borders, Padding, Widget},
};

use crate::{
    BORDER_SET,
    config_menu::{ConfigMenu, ConfigMenuState, EditMessage, ListValue, ResultState},
    get_text,
    hotkeys::HotkeyBar,
    tab_page::{InfoState, PageMessage},
    theme::get_tui_theme,
};

pub struct ICBConfigMenuUI {
    state: ConfigMenuState,
    title: String,
    menu: ConfigMenu<Arc<Mutex<IcyBoard>>>,
}

/// Paint the standard setup form chrome and return its content viewport.
pub fn render_config_menu_frame(frame: &mut ratatui::Frame, disp_area: Rect, title: &str, footer: Option<Line<'static>>) -> Rect {
    let area = Rect {
        x: disp_area.x + 1,
        y: disp_area.y + 1,
        width: disp_area.width.saturating_sub(2),
        height: disp_area.height.saturating_sub(1),
    };
    let mut block: Block<'_> = Block::new()
        .style(get_tui_theme().background)
        .padding(Padding::new(2, 2, 5, 0))
        .borders(Borders::ALL)
        .border_set(BORDER_SET)
        .title_alignment(ratatui::layout::Alignment::Center)
        .border_style(get_tui_theme().dialog_box);
    if let Some(footer) = footer {
        block = block.title_bottom(footer);
    }
    block.render(area, frame.buffer_mut());

    let title_area = area.inner(Margin { horizontal: 1, vertical: 1 });
    let width = Line::raw(title).width().min(title_area.width as usize) as u16;
    Line::raw(title).style(get_tui_theme().dialog_box_title).render(
        Rect {
            x: (area.x + 1 + area.width.saturating_sub(width) / 2).min(title_area.right().saturating_sub(width)),
            y: title_area.y,
            width,
            height: title_area.height.min(1),
        },
        frame.buffer_mut(),
    );

    if area.height > 3 && area.width > 2 {
        frame.buffer_mut().set_string(
            area.x + 1,
            area.y + 2,
            "─".repeat((area.width as usize).saturating_sub(2)),
            get_tui_theme().dialog_box,
        );
    }

    Rect {
        x: disp_area.x + 3,
        y: area.y + 3,
        width: disp_area.width.saturating_sub(5),
        height: area.height.saturating_sub(4),
    }
}

impl ICBConfigMenuUI {
    pub fn new(title: String, menu: ConfigMenu<Arc<Mutex<IcyBoard>>>) -> Self {
        let mut state = ConfigMenuState::default();
        state.path_base = Some(menu.obj.lock().unwrap().root_path.clone());
        Self { state, title, menu }
    }

    pub fn render(&mut self, frame: &mut ratatui::Frame, disp_area: ratatui::prelude::Rect) {
        let mut preset_id = "icb_setup_key_menu_help";
        if let Some(item) = self.menu.get_item(self.state.selected)
            && let ListValue::Path(path) = &item.value
            && item.editable()
        {
            preset_id = "icb_setup_key_menu_browse_help";
            let path = self.menu.obj.lock().unwrap().resolve_file(path);
            if !path.as_os_str().is_empty() && !path.is_dir() && item.editable() {
                preset_id = if path.is_file() {
                    "icb_setup_key_menu_edit_help"
                } else if can_create_file(&path) {
                    "icb_setup_key_menu_create_help"
                } else {
                    preset_id
                };
            }
        }

        let modal = self.state.is_path_browser_open()
            || self
                .menu
                .get_item(self.state.selected)
                .is_some_and(|item| matches!(&item.value, ListValue::ComboBox(combo) if combo.is_edit_open));
        let footer = (!modal).then(|| HotkeyBar::for_id(preset_id).line());
        let area = render_config_menu_frame(frame, disp_area, &self.title, footer);
        if area.width > 0 && area.height > 0 {
            self.menu.render(area, frame, &mut self.state);
        }
    }

    pub fn request_status(&self) -> ResultState {
        ResultState {
            edit_msg: EditMessage::None,
            status_line: self.menu.current_status_line(&self.state),
        }
    }

    pub fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if self.state.is_path_browser_open() {
            return PageMessage::ResultState(self.menu.handle_key_press(key, &mut self.state));
        }
        if let Some(item) = self.menu.get_item(self.state.selected)
            && let ListValue::Path(path) = &item.value
            && item.editable()
            && matches!(key.code, crossterm::event::KeyCode::F(2) | crossterm::event::KeyCode::F(3))
        {
            let path = self.menu.obj.lock().unwrap().resolve_file(path);
            if path.as_os_str().is_empty() {
                return PageMessage::InfoBox(InfoState::Warning, get_text("no_file_name_given"));
            }
            if key.code == crossterm::event::KeyCode::F(3) && !path.exists() && can_create_file(&path) {
                return match create_file_with_content(&path, item.path_initial_content.as_deref().unwrap_or("")) {
                    Ok(()) => PageMessage::ResultState(self.request_status()),
                    Err(e) => {
                        log::error!("Error creating {}: {}", path.display(), e);
                        PageMessage::InfoBox(InfoState::Error, format!("{}\n\n{}", path.display(), e))
                    }
                };
            }
            if key.code != crossterm::event::KeyCode::F(2) || !path.is_file() {
                return PageMessage::ResultState(self.request_status());
            }
            if let Some(editor) = &item.path_editor {
                return editor(self.menu.obj.clone(), path);
            }

            let editor = {
                let board = self.menu.obj.lock().unwrap();
                if uses_graphics_editor(&path) {
                    board.config.sysop.graphics_editor.clone()
                } else {
                    board.config.sysop.external_editor.clone()
                }
            };
            let started = crate::term::with_terminal(|| {
                std::process::Command::new(&editor)
                    .arg(format!("{}", path.display()))
                    .spawn()
                    .and_then(|mut child| child.wait())
            });
            match started {
                Ok(_) => {
                    return PageMessage::ExternalProgramStarted;
                }
                Err(e) => {
                    log::error!("Error opening editor: {}", e);
                    return PageMessage::InfoBox(InfoState::Error, format!("{}\n\n{}", editor, e));
                }
            }
        }

        let res = self.menu.handle_key_press(key, &mut self.state);
        if res.edit_msg == EditMessage::Close {
            return PageMessage::Close;
        }
        PageMessage::ResultState(res)
    }
}

fn can_create_file(path: &std::path::Path) -> bool {
    !path.as_os_str().is_empty() && !path.exists() && path.parent().is_some_and(std::path::Path::is_dir)
}

fn uses_graphics_editor(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|extension| matches!(extension.to_ascii_lowercase().as_str(), "ans" | "ansi" | "icy" | "pcb" | "rip"))
}

fn create_file_with_content(path: &std::path::Path, content: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{can_create_file, uses_graphics_editor};

    #[test]
    fn config_footer_preserves_text_geometry_and_help_and_hides_during_browse() {
        use super::*;
        use crate::config_menu::{ConfigEntry, ListItem};
        use crossterm::event::KeyCode;
        use ratatui::{Terminal, backend::TestBackend};

        let root = tempfile::tempdir().unwrap();
        let mut board = IcyBoard::default();
        board.root_path = root.path().to_path_buf();
        let mut ui = ICBConfigMenuUI::new(
            "Files".into(),
            ConfigMenu {
                obj: Arc::new(Mutex::new(board)),
                entry: vec![ConfigEntry::Item(
                    ListItem::new("Path".into(), ListValue::Path("missing.txt".into())).with_help("Path help"),
                )],
            },
        );
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| ui.render(frame, frame.area())).unwrap();
        let row = |buffer: &ratatui::buffer::Buffer, y| (0..80).map(|x| buffer[(x, y)].symbol()).collect::<String>();
        let buffer = terminal.backend().buffer();
        assert!(row(buffer, 2).contains("Files"));
        assert_eq!(buffer[(3, 4)].symbol(), "P");
        assert_eq!(row(buffer, 3).chars().filter(|ch| *ch == '─').count(), 76);
        assert!(row(buffer, 24).contains(&HotkeyBar::for_id("icb_setup_key_menu_create_help").line().to_string()));
        assert!(
            matches!(ui.handle_key_press(KeyCode::F(1).into()), PageMessage::ResultState(state) if matches!(state.edit_msg, EditMessage::DisplayHelp(ref text) if text == "Path help"))
        );
        ui.handle_key_press(KeyCode::F(4).into());
        terminal.draw(|frame| ui.render(frame, frame.area())).unwrap();
        assert!(!row(terminal.backend().buffer(), 24).contains("F1"));
        ui.handle_key_press(KeyCode::Esc.into());
        terminal.draw(|frame| ui.render(frame, frame.area())).unwrap();
        assert!(row(terminal.backend().buffer(), 24).contains(&HotkeyBar::for_id("icb_setup_key_menu_create_help").line().to_string()));
        assert!(matches!(&ui.menu.get_item(0).unwrap().value, ListValue::Path(path) if path == std::path::Path::new("missing.txt")));
        assert!(!root.path().join("missing.txt").exists());
    }

    #[test]
    fn a_settings_page_is_not_framed_like_a_menu() {
        use super::*;
        use crate::config_menu::{ConfigEntry, ListItem};
        use ratatui::{Terminal, backend::TestBackend};

        let root = tempfile::tempdir().unwrap();
        let mut board = IcyBoard::default();
        board.root_path = root.path().to_path_buf();
        let mut ui = ICBConfigMenuUI::new(
            "Files".into(),
            ConfigMenu {
                obj: Arc::new(Mutex::new(board)),
                entry: vec![ConfigEntry::Item(ListItem::new("Path".into(), ListValue::Path("missing.txt".into())))],
            },
        );
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| ui.render(frame, frame.area())).unwrap();
        let buffer = terminal.backend().buffer();
        let theme = get_tui_theme();
        assert_ne!(theme.dialog_box.fg, theme.menu_box.fg);
        for cell in [&buffer[(1, 1)], &buffer[(40, 1)], &buffer[(3, 3)]] {
            assert_eq!(cell.fg, theme.dialog_box.fg.unwrap());
        }
        assert_eq!(buffer[(38, 2)].fg, theme.dialog_box_title.fg.unwrap());
    }

    #[test]
    fn config_chrome_is_bounded_at_small_sizes() {
        use super::*;
        use crate::config_menu::{ConfigEntry, ListItem};
        use ratatui::{Terminal, backend::TestBackend};

        for (width, height) in [(0, 0), (1, 1), (2, 2), (4, 10), (8, 3), (20, 5), (80, 25)] {
            let mut ui = ICBConfigMenuUI::new(
                "界 e\u{301} Configuration".into(),
                ConfigMenu {
                    obj: Arc::new(Mutex::new(IcyBoard::default())),
                    entry: vec![ConfigEntry::Item(ListItem::new("Value".into(), ListValue::Bool(false)))],
                },
            );
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| ui.render(frame, frame.area())).unwrap();
        }
    }

    #[test]
    fn browse_uses_board_root_and_blocks_editor_and_create_shortcuts() {
        use super::*;
        use crate::config_menu::{ConfigEntry, ListItem};
        use crossterm::event::KeyCode;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("selected.txt"), "unchanged").unwrap();
        let mut board = IcyBoard::default();
        board.root_path = root.path().to_path_buf();
        board.config.paths.statistics_file = "missing.txt".into();
        let board = Arc::new(Mutex::new(board));
        let editor_calls = Arc::new(AtomicUsize::new(0));
        let calls = editor_calls.clone();
        let mut ui = ICBConfigMenuUI::new(
            "Files".into(),
            ConfigMenu {
                obj: board.clone(),
                entry: vec![ConfigEntry::Item(
                    ListItem::new("Statistics".into(), ListValue::Path("missing.txt".into()))
                        .with_update_path_value(&|board: &Arc<Mutex<IcyBoard>>, path| board.lock().unwrap().config.paths.statistics_file = path)
                        .with_path_editor(Box::new(move |_, _| {
                            calls.fetch_add(1, Ordering::Relaxed);
                            PageMessage::None
                        })),
                )],
            },
        );
        for code in [KeyCode::F(4), KeyCode::F(3), KeyCode::End, KeyCode::Enter] {
            ui.handle_key_press(KeyEvent::from(code));
        }
        assert!(!root.path().join("missing.txt").exists());
        assert_eq!(board.lock().unwrap().config.paths.statistics_file, std::path::PathBuf::from("selected.txt"));
        for code in [KeyCode::F(4), KeyCode::F(2), KeyCode::Esc] {
            assert!(matches!(ui.handle_key_press(KeyEvent::from(code)), PageMessage::ResultState(_)));
        }
        assert_eq!(editor_calls.load(Ordering::Relaxed), 0);
        ui.handle_key_press(KeyEvent::from(KeyCode::F(2)));
        assert_eq!(editor_calls.load(Ordering::Relaxed), 1);
        assert!(matches!(ui.handle_key_press(KeyEvent::from(KeyCode::Esc)), PageMessage::Close));
        assert_eq!(std::fs::read_to_string(root.path().join("selected.txt")).unwrap(), "unchanged");
    }

    fn test_dir() -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("icy-board-tui-file-create-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir(&dir).unwrap();
        dir
    }

    #[test]
    fn a_missing_file_with_an_existing_parent_can_be_created() {
        let dir = test_dir();
        assert!(can_create_file(&dir.join("new.txt")));
    }

    #[test]
    fn a_file_with_a_missing_parent_cannot_be_created() {
        let dir = test_dir();
        assert!(!can_create_file(&dir.join("missing/new.txt")));
    }

    #[test]
    fn an_existing_file_is_for_editing_not_creation() {
        let file = test_dir().join("existing.txt");
        std::fs::File::create(&file).unwrap();
        assert!(!can_create_file(&file));
    }

    #[test]
    fn ansi_files_use_the_graphics_editor() {
        assert!(uses_graphics_editor(std::path::Path::new("welcome.ANS")));
        assert!(uses_graphics_editor(std::path::Path::new("menu.icy")));
        assert!(!uses_graphics_editor(std::path::Path::new("welcome.asc")));
    }
}
