use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    cfg_entry_bool, cfg_entry_path, cfg_entry_u16,
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, EditMessage, ListValue, ResultState},
    get_text,
    tab_page::{InfoState, Page, PageMessage},
    theme::get_tui_theme,
};
use ratatui::{
    layout::{Alignment, Margin},
    text::Span,
    widgets::{Block, Borders, Widget},
};

pub struct EventSetup {
    menu: ConfigMenu<Arc<Mutex<IcyBoard>>>,
    state: ConfigMenuState,
}

impl EventSetup {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let icy_board2 = icy_board.clone();
            let lock: std::sync::MutexGuard<'_, IcyBoard> = icy_board.lock().unwrap();
            let label_width = 37;
            let sysop_info: Vec<ConfigEntry<Arc<Mutex<IcyBoard>>>> = vec![
                ConfigEntry::Separator,
                cfg_entry_bool!("event_enabled", label_width, event, enabled, lock),
                cfg_entry_path!(
                    "event_file",
                    label_width,
                    event,
                    event_file,
                    Box::new(crate::editors::events::edit_events),
                    lock
                ),
                ConfigEntry::Separator,
                ConfigEntry::Label(get_text("event_enabled_for_expedited_label")),
                ConfigEntry::Separator,
                cfg_entry_u16!("event_suspend_minutes", label_width, 0, 99, event, suspend_minutes, lock),
                cfg_entry_bool!("event_disallow_uploads", label_width, event, disallow_uploads, lock),
                cfg_entry_u16!("event_minutes_uploads_disallowed", label_width, 0, 99, event, minutes_uploads_disallowed, lock),
            ];

            ConfigMenu {
                obj: icy_board2,
                entry: sysop_info,
            }
        };

        let mut state = ConfigMenuState::default();
        state.path_base = Some(icy_board.lock().unwrap().root_path.clone());
        Self { menu, state }
    }
}

impl Page for EventSetup {
    fn render(&mut self, frame: &mut ratatui::Frame, disp_area: ratatui::prelude::Rect) {
        let area = disp_area.inner(Margin { horizontal: 1, vertical: 1 });
        let hint = if !self.state.is_path_browser_open() && self.state.selected == 1 {
            get_text("event_editor_setup_keys")
        } else {
            get_text("icb_setup_key_menu_help")
        };
        Block::new()
            .title_alignment(Alignment::Center)
            .title(get_text("event_setup_title"))
            .title_bottom(Span::styled(hint, get_tui_theme().key_binding))
            .style(get_tui_theme().background)
            .borders(Borders::ALL)
            .border_set(icy_board_tui::BORDER_SET)
            .border_style(get_tui_theme().menu_box)
            .render(area, frame.buffer_mut());
        let content = area.inner(Margin { horizontal: 2, vertical: 1 });
        if content.width > 1 && content.height > 0 {
            self.menu.render(content, frame, &mut self.state);
        }
    }
    fn request_status(&self) -> ResultState {
        ResultState::status_line(if self.state.selected == 1 {
            get_text("event_editor_file_status")
        } else {
            self.menu.current_status_line(&self.state)
        })
    }
    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if key.kind == KeyEventKind::Release {
            return PageMessage::None;
        }
        // Browsing owns every key, including F2 and Esc, until its modal closes.
        if !self.state.is_path_browser_open()
            && key.code == KeyCode::F(2)
            && let Some(item) = self.menu.get_item(self.state.selected)
            && item.editable()
            && let ListValue::Path(path) = &item.value
        {
            if path.as_os_str().is_empty() {
                return PageMessage::InfoBox(InfoState::Warning, get_text("no_file_name_given"));
            }
            // Use the text currently in the form, not a render-dependent config copy.
            if let Some(update) = &item.update_value {
                update(&self.menu.obj, &item.value);
            }
            let resolved = self.menu.obj.lock().unwrap().resolve_file(path);
            // Unlike the generic file editor, a missing list opens in memory and is
            // created only on Save. A malformed/unreadable file must never become empty.
            if let Some(editor) = &item.path_editor {
                return editor(self.menu.obj.clone(), resolved);
            }
        }
        let result = self.menu.handle_key_press(key, &mut self.state);
        if result.edit_msg == EditMessage::Close {
            PageMessage::Close
        } else {
            PageMessage::ResultState(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use icy_board_engine::icy_board::{IcyBoardSerializer, events::EventList};
    use ratatui::{Terminal, backend::TestBackend, layout::Rect};

    fn setup(root: &std::path::Path, path: &str) -> EventSetup {
        let mut board = IcyBoard {
            root_path: root.to_path_buf(),
            ..Default::default()
        };
        board.config.event.event_file = path.into();
        EventSetup::new(Arc::new(Mutex::new(board)))
    }

    fn press(page: &mut impl Page, code: KeyCode) -> PageMessage {
        page.handle_key_press(KeyEvent::from(code))
    }

    #[test]
    fn event_setup_f2_opens_missing_list_at_board_root_without_creating_it() {
        let root = tempfile::tempdir().unwrap();
        let mut page = setup(root.path(), "new/events.toml");
        // F2 is a path-field shortcut, not a global editor key.
        assert!(!matches!(press(&mut page, KeyCode::F(2)), PageMessage::OpenSubPage(_)));
        press(&mut page, KeyCode::Down);
        let PageMessage::OpenSubPage(mut editor) = press(&mut page, KeyCode::F(2)) else {
            panic!("expected event editor")
        };
        let path = root.path().join("new/events.toml");
        assert!(!path.parent().unwrap().exists());
        assert!(matches!(editor.handle_key_press(KeyEvent::from(KeyCode::Esc)), PageMessage::Close));
        assert!(!path.exists());
        let PageMessage::OpenSubPage(mut editor) = press(&mut page, KeyCode::F(2)) else {
            panic!("expected event editor")
        };
        editor.handle_key_press(KeyEvent::from(KeyCode::Insert));
        // Esc leaves the list and asks whether to save.
        editor.handle_key_press(KeyEvent::from(KeyCode::Esc));
        editor.handle_key_press(KeyEvent::from(KeyCode::Right));
        assert!(matches!(editor.handle_key_press(KeyEvent::from(KeyCode::Enter)), PageMessage::Close));
        assert_eq!(EventList::load(&path).unwrap().len(), 1);
    }

    #[test]
    fn event_setup_f2_uses_current_path_before_render() {
        let root = tempfile::tempdir().unwrap();
        let mut page = setup(root.path(), "");
        press(&mut page, KeyCode::Down);
        for ch in "typed.toml".chars() {
            press(&mut page, KeyCode::Char(ch));
        }
        let PageMessage::OpenSubPage(mut editor) = press(&mut page, KeyCode::F(2)) else {
            panic!("expected event editor")
        };
        assert_eq!(page.menu.obj.lock().unwrap().config.event.event_file, std::path::PathBuf::from("typed.toml"));
        editor.handle_key_press(KeyEvent::from(KeyCode::Insert));
        editor.handle_key_press(KeyEvent::from(KeyCode::Esc));
        editor.handle_key_press(KeyEvent::from(KeyCode::Right));
        assert!(matches!(editor.handle_key_press(KeyEvent::from(KeyCode::Enter)), PageMessage::Close));
        assert!(root.path().join("typed.toml").is_file());
    }

    #[test]
    fn event_setup_empty_malformed_and_directory_paths_report_errors() {
        let root = tempfile::tempdir().unwrap();
        let mut page = setup(root.path(), "");
        page.state.selected = 1;
        assert!(matches!(press(&mut page, KeyCode::F(2)), PageMessage::InfoBox(InfoState::Warning, _)));
        std::fs::write(root.path().join("bad.toml"), "broken = [").unwrap();
        for path in ["bad.toml", "."] {
            let mut page = setup(root.path(), path);
            page.state.selected = 1;
            assert!(matches!(press(&mut page, KeyCode::F(2)), PageMessage::InfoBox(InfoState::Error, _)));
        }
        assert_eq!(std::fs::read_to_string(root.path().join("bad.toml")).unwrap(), "broken = [");
    }

    #[test]
    fn event_setup_path_browser_uses_board_root_and_guards_f2_f3_and_escape() {
        let root = tempfile::tempdir().unwrap();
        let selected = root.path().join("selected.toml");
        EventList::default().save(&selected).unwrap();
        let original = std::fs::read(&selected).unwrap();
        let mut page = setup(root.path(), "missing.toml");
        page.state.selected = 1;
        assert_eq!(page.state.path_base.as_deref(), Some(root.path()));
        press(&mut page, KeyCode::F(4));
        assert!(page.state.is_path_browser_open());
        for code in [KeyCode::F(2), KeyCode::F(3)] {
            assert!(!matches!(press(&mut page, code), PageMessage::OpenSubPage(_) | PageMessage::Close));
            assert!(page.state.is_path_browser_open());
        }
        assert!(!matches!(press(&mut page, KeyCode::Esc), PageMessage::Close));
        assert!(!page.state.is_path_browser_open());
        assert_eq!(page.menu.obj.lock().unwrap().config.event.event_file, std::path::PathBuf::from("missing.toml"));
        press(&mut page, KeyCode::F(4));
        press(&mut page, KeyCode::End);
        press(&mut page, KeyCode::Enter);
        assert!(!page.state.is_path_browser_open());
        assert_eq!(page.menu.obj.lock().unwrap().config.event.event_file, std::path::PathBuf::from("selected.toml"));
        assert!(matches!(press(&mut page, KeyCode::F(2)), PageMessage::OpenSubPage(_)));
        assert!(!root.path().join("missing.toml").exists());
        assert_eq!(std::fs::read(selected).unwrap(), original);
    }

    #[test]
    fn event_setup_all_global_fields_and_path_shortcuts_fit_at_80x25() {
        let root = tempfile::tempdir().unwrap();
        let mut page = setup(root.path(), "events.toml");
        page.state.selected = 1;
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| page.render(frame, Rect::new(0, 1, 80, 23))).unwrap();
        let buffer = terminal.backend().buffer();
        let rows: Vec<String> = (0..25).map(|y| (0..80).map(|x| buffer[(x, y)].symbol()).collect()).collect();
        for key in [
            "event_enabled",
            "event_file",
            "event_suspend_minutes",
            "event_disallow_uploads",
            "event_minutes_uploads_disallowed",
            "event_editor_setup_keys",
        ] {
            assert!(rows.iter().any(|row| row.contains(&get_text(key))), "missing {key}: {rows:?}");
        }
        assert!(page.request_status().status_line.contains("F2"));
        assert!(page.request_status().status_line.contains("F4"));
    }
}
