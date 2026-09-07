use crossterm::event::{KeyCode, KeyEvent};
use icy_board_tui::{get_text, tab_page::Page};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};

use super::{
    areas::MessageAreasEditor, bullettins::BullettinsEditor, dirs::DirsEditor, door::DoorEditor, sec_editor::SecurityLevelEditor, surveys::SurveyEditor,
};

const VIEWPORT: Rect = Rect::new(0, 1, 80, 23);

#[test]
fn nested_path_browse_hint_is_visible_before_opening_and_does_not_cover_modal() {
    let directory = tempfile::tempdir().unwrap();
    let pages: Vec<(Box<dyn Page>, usize, usize, u16)> = vec![
        (Box::new(DirsEditor::new(&directory.path().join("dirs.toml")).unwrap()), 1, 2, 19),
        (Box::new(BullettinsEditor::new(&directory.path().join("bulletins.toml")).unwrap()), 0, 1, 14),
        (Box::new(SurveyEditor::new(&directory.path().join("surveys.toml")).unwrap()), 0, 2, 14),
        (Box::new(MessageAreasEditor::new(&directory.path().join("areas.toml")).unwrap()), 4, 1, 19),
    ];
    let hint = get_text("path_browser_shortcut");
    assert!(hint.contains("F4"));
    for (mut page, path_index, next_non_path, border_y) in pages {
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        page.handle_key_press(KeyEvent::from(KeyCode::Insert));
        page.handle_key_press(KeyEvent::from(KeyCode::Enter));
        for _ in 0..path_index {
            page.handle_key_press(KeyEvent::from(KeyCode::Down));
        }
        terminal.draw(|frame| page.render(frame, VIEWPORT)).unwrap();
        let border: String = (4..76).map(|x| terminal.backend().buffer()[(x, border_y)].symbol()).collect();
        assert!(border.contains(&hint), "missing browse hint before F4: {border}");

        page.handle_key_press(KeyEvent::from(KeyCode::F(4)));
        terminal.draw(|frame| page.render(frame, VIEWPORT)).unwrap();
        let rows: Vec<String> = (0..25)
            .map(|y| (0..80).map(|x| terminal.backend().buffer()[(x, y)].symbol()).collect())
            .collect();
        assert!(
            rows.iter().any(|row| row.contains(&get_text("path_browser_title"))),
            "browser title was obscured"
        );
        assert!(rows.iter().all(|row| !row.contains(&hint)), "inactive form hint must not cover the browser");

        page.handle_key_press(KeyEvent::from(KeyCode::Esc));
        terminal.draw(|frame| page.render(frame, VIEWPORT)).unwrap();
        let border: String = (4..76).map(|x| terminal.backend().buffer()[(x, border_y)].symbol()).collect();
        assert!(border.contains(&hint), "hint must return after cancelling the browser");
        for _ in 0..next_non_path {
            page.handle_key_press(KeyEvent::from(KeyCode::Down));
        }
        terminal.draw(|frame| page.render(frame, VIEWPORT)).unwrap();
        let border: String = (4..76).map(|x| terminal.backend().buffer()[(x, border_y)].symbol()).collect();
        assert!(!border.contains(&hint), "non-path field advertises browsing: {border}");
    }
}

fn assert_popup_labels_fit(mut page: impl Page, popup: Rect, keys: &[&str]) {
    let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
    for _ in 0..=keys.len() {
        terminal.draw(|frame| page.render(frame, VIEWPORT)).unwrap();
        let buffer = terminal.backend().buffer();
        let rows: Vec<String> = (popup.top() + 1..popup.bottom() - 1)
            .map(|y| (popup.left() + 1..popup.right() - 1).map(|x| buffer[(x, y)].symbol()).collect())
            .collect();
        for key in keys {
            let label = get_text(key);
            assert!(
                rows.iter()
                    .any(|row| row.split_once(&label).is_some_and(|(_, rest)| rest.trim_start().starts_with(':'))),
                "clipped or scrolled-off label: {label}\n{}",
                rows.join("\n")
            );
        }
        assert!(rows.iter().all(|row| !row.contains(['▲', '▼'])), "unnecessary scrollbar: {}", rows.join("\n"));
        for y in popup.top() + 1..popup.bottom() - 1 {
            assert_eq!(buffer[(popup.left(), y)].symbol(), "║");
            assert_eq!(buffer[(popup.right() - 1, y)].symbol(), "║");
        }
        page.handle_key_press(KeyEvent::from(KeyCode::Down));
    }
}

#[test]
fn door_popup_labels_fit_at_80_columns() {
    let directory = tempfile::tempdir().unwrap();
    let mut page = DoorEditor::new(&directory.path().join("doors.toml")).unwrap();
    page.handle_key_press(KeyEvent::from(KeyCode::F(2)));
    page.handle_key_press(KeyEvent::from(KeyCode::Enter));
    assert_popup_labels_fit(
        page,
        Rect::new(3, 5, 74, 15),
        &[
            "door_editor_name",
            "door_editor_description",
            "door_editor_password",
            "door_editor_path",
            "door_editor_security",
            "door_editor_door_type",
            "door_editor_use_shell_execute",
            "door_editor_drop_file",
            "door_editor_dos_command",
            "door_editor_dos_memory",
            "door_editor_dos_max_seconds",
        ],
    );
}

#[test]
fn bbslink_labels_fit_at_80_columns() {
    let directory = tempfile::tempdir().unwrap();
    let mut page = DoorEditor::new(&directory.path().join("doors.toml")).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
    terminal.draw(|frame| page.render(frame, VIEWPORT)).unwrap();
    let buffer = terminal.backend().buffer();
    let rows: Vec<String> = (2..7).map(|y| (1..79).map(|x| buffer[(x, y)].symbol()).collect()).collect();
    for key in [
        "doors_editor_bbslink_credentials",
        "doors_editor_system_code",
        "doors_editor_auth_code",
        "doors_editor_scheme_code",
    ] {
        let label = get_text(key);
        assert!(rows.iter().any(|row| row.contains(&label)), "clipped label: {label}");
    }

    let title = get_text("doors_editor_bbslink_credentials");
    let title_row = rows.iter().position(|row| row.contains(&title)).unwrap();
    assert_eq!(rows[title_row + 1].matches('═').count(), title.chars().count());
    let title_x = rows[title_row].find(&title).unwrap() as u16 + 1;
    let title_style = buffer[(title_x, title_row as u16 + 2)].style();
    let underline_style = buffer[(title_x, title_row as u16 + 3)].style();
    assert_eq!(title_style, underline_style, "title and underline must share the common config-title style");
}

#[test]
fn directory_popup_shows_all_fields_without_scrolling() {
    let directory = tempfile::tempdir().unwrap();
    let mut page = DirsEditor::new(&directory.path().join("dirs.toml")).unwrap();
    page.handle_key_press(KeyEvent::from(KeyCode::Insert));
    page.handle_key_press(KeyEvent::from(KeyCode::Enter));
    assert_popup_labels_fit(
        page,
        Rect::new(4, 5, 72, 15),
        &[
            "dirs_edit_name",
            "dirs_edit_path",
            "dirs_metadata_path",
            "dirs_edit_password",
            "dirs_edit_fido_tag",
            "dirs_edit_sort",
            "dirs_edit_sort_asc",
            "dirs_edit_has_new_files",
            "dirs_edit_is_free",
            "dirs_edit_list_sec",
            "dirs_download_sec",
        ],
    );
}

#[test]
fn area_popup_labels_fit_at_80_columns() {
    let directory = tempfile::tempdir().unwrap();
    let mut page = MessageAreasEditor::new(&directory.path().join("areas.toml")).unwrap();
    page.handle_key_press(KeyEvent::from(KeyCode::Insert));
    page.handle_key_press(KeyEvent::from(KeyCode::Enter));
    assert_popup_labels_fit(
        page,
        Rect::new(4, 5, 72, 15),
        &[
            "area_editor_name",
            "area_editor_qwk_name",
            "area_editor_fido_tag",
            "area_editor_fido_origin",
            "area_editor_file",
            "area_editor_is_readonly",
            "area_editor_allow_aliases",
            "area_editor_list_sec",
            "area_editor_enter_sec",
            "area_editor_attach_sec",
            "area_editor_qwk_number",
        ],
    );
}

#[test]
fn security_popup_labels_fit_at_80_columns() {
    let directory = tempfile::tempdir().unwrap();
    let mut page = SecurityLevelEditor::new(&directory.path().join("security.toml")).unwrap();
    page.handle_key_press(KeyEvent::from(KeyCode::Enter));
    assert_popup_labels_fit(
        page,
        Rect::new(4, 4, 72, 18),
        &[
            "sec_level_editor_security",
            "sec_level_editor_description",
            "sec_level_editor_password",
            "sec_level_editor_time_per_day",
            "sec_level_editor_daily_bytes",
            "sec_level_editor_file_ratio",
            "sec_level_editor_byte_ratio",
            "sec_level_editor_file_limit",
            "sec_level_editor_kb_limit",
            "sec_level_editor_file_credit",
            "sec_level_editor_kb_credit",
            "sec_level_editor_enforce_time",
            "sec_level_editor_allow_alias",
            "sec_level_force_read_mail",
            "sec_level_demo_acc",
            "sec_level_enable_acc",
        ],
    );
}

mod common_regressions {
    use std::{
        fs,
        path::PathBuf,
        sync::{Arc, Mutex},
    };

    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
    use icy_board_engine::icy_board::{
        IcyBoard, IcyBoardSerializer,
        bulletins::{Bullettin, BullettinList},
        doors::{BBSLink, DEFAULT_DOS_MAX_RUNTIME_SECONDS, DoorList, DoorServerAccount, DoorType, DropFile},
        language::{Language, SupportedLanguages},
        sec_levels::SecurityLevelDefinitions,
        surveys::{Survey, SurveyList},
        xfer_protocols::SupportedProtocols,
    };
    use icy_board_tui::{
        config_menu::EditMessage,
        get_text,
        tab_page::{InfoState, Page, PageMessage, TabPage},
    };
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    use super::VIEWPORT;
    use crate::editors::{
        areas::MessageAreasEditor, bullettins::BullettinsEditor, command::CommandsEditor, dirs::DirsEditor, door::DoorEditor, languages::LanguageListEditor,
        protocols::ProtocolEditor, sec_editor::SecurityLevelEditor, surveys::SurveyEditor,
    };

    #[derive(Clone, Copy, Debug)]
    enum FileEditor {
        Protocols,
        Languages,
        Commands,
        Security,
        Dirs,
        Bullettins,
        Surveys,
        Areas,
        Doors,
    }

    impl FileEditor {
        fn open(self, path: &PathBuf) -> Box<dyn Page> {
            match self {
                Self::Protocols => Box::new(ProtocolEditor::new(path).unwrap()),
                Self::Languages => Box::new(LanguageListEditor::new(path).unwrap()),
                Self::Commands => Box::new(CommandsEditor::new(path).unwrap()),
                Self::Security => Box::new(SecurityLevelEditor::new(path).unwrap()),
                Self::Dirs => Box::new(DirsEditor::new(path).unwrap()),
                Self::Bullettins => Box::new(BullettinsEditor::new(path).unwrap()),
                Self::Surveys => Box::new(SurveyEditor::new(path).unwrap()),
                Self::Areas => Box::new(MessageAreasEditor::new(path).unwrap()),
                Self::Doors => Box::new(DoorEditor::new(path).unwrap()),
            }
        }

        fn insert_key(self) -> KeyCode {
            if matches!(self, Self::Doors) { KeyCode::F(2) } else { KeyCode::Insert }
        }

        fn help(self, initial: bool) -> &'static str {
            match self {
                Self::Doors if initial => "doors_editor_key_help",
                Self::Doors => "doors_editor_key_help_door",
                Self::Areas => "area_editor_key_help",
                _ => "icb_setup_key_conf_list_help",
            }
        }
    }

    fn render(page: &mut dyn Page, terminal: &mut Terminal<TestBackend>) -> Buffer {
        terminal.draw(|frame| page.render(frame, VIEWPORT)).unwrap();
        terminal.backend().buffer().clone()
    }

    fn rows(buffer: &Buffer) -> Vec<String> {
        (0..25).map(|y| (0..80).map(|x| buffer[(x, y)].symbol()).collect()).collect()
    }

    fn assert_text(buffer: &Buffer, key: &str, visible: bool) {
        let text = get_text(key);
        assert!(!text.is_empty() && text != key, "missing translation: {key}");
        let rows = rows(buffer);
        assert_eq!(
            rows.iter().any(|row| row.contains(&text)),
            visible,
            "{key}, visible={visible}\n{}",
            rows.join("\n")
        );
    }

    fn assert_parent_help(buffer: &Buffer, key: &str, visible: bool) {
        assert_text(buffer, key, visible);
        let footer = VIEWPORT.bottom() - 1;
        if visible {
            assert!(rows(buffer)[usize::from(footer)].contains(&get_text(key)), "help must be on the parent border");
        } else {
            // Also catch partially clipped/stale help, not just a complete translated string.
            for x in VIEWPORT.left() + 1..VIEWPORT.right() - 1 {
                assert_eq!(
                    buffer[(x, footer)].symbol(),
                    icy_board_tui::BORDER_SET.horizontal_bottom,
                    "parent footer at x={x}"
                );
            }
        }
    }

    fn assert_detail(buffer: &Buffer, visible: bool) {
        // Parent frames use mixed corners (╓/╖); only the nested form uses double corners.
        let rows = rows(buffer);
        assert_eq!(
            rows.iter().any(|row| row.contains('╔')),
            visible,
            "detail visible={visible}\n{}",
            rows.join("\n")
        );
        assert_eq!(rows.iter().any(|row| row.contains('╝')), visible, "detail bottom border visible={visible}");
    }

    fn assert_stays(message: PageMessage) {
        match message {
            PageMessage::None => {}
            PageMessage::ResultState(state) => assert!(matches!(state.edit_msg, EditMessage::None), "unexpected form action"),
            PageMessage::InfoBox(_, text) => panic!("unexpected info box: {text}"),
            _ => panic!("key unexpectedly closed the page or opened another page"),
        }
    }

    fn press(page: &mut dyn Page, code: KeyCode) {
        assert_stays(page.handle_key_press(KeyEvent::from(code)));
    }

    fn assert_close(page: &mut dyn Page, code: KeyCode) {
        assert!(
            matches!(page.handle_key_press(KeyEvent::from(code)), PageMessage::Close),
            "expected PageMessage::Close for {code:?}"
        );
    }

    fn assert_ignored_release(page: &mut dyn Page, code: KeyCode) {
        assert_stays(page.handle_key_press(KeyEvent::new_with_kind(code, KeyModifiers::NONE, KeyEventKind::Release)));
    }

    fn file_editor_modal_lifecycle(editor: FileEditor) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("nested/list.toml");
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();

        // Missing files (including security/door editor defaults) are not dirty just by opening them.
        let mut pristine = editor.open(&path);
        let buffer = render(pristine.as_mut(), &mut terminal);
        assert_parent_help(&buffer, editor.help(true), true);
        assert_detail(&buffer, false);
        assert_close(pristine.as_mut(), KeyCode::Esc);
        drop(pristine);
        assert!(!path.parent().unwrap().exists(), "{editor:?}: opening/closing created storage");

        let mut page = editor.open(&path);
        press(page.as_mut(), editor.insert_key());
        let buffer = render(page.as_mut(), &mut terminal);
        assert_parent_help(&buffer, editor.help(false), true);
        press(page.as_mut(), KeyCode::Enter);
        let buffer = render(page.as_mut(), &mut terminal);
        assert_detail(&buffer, true);
        assert_parent_help(&buffer, editor.help(false), false);
        assert_text(&buffer, "icbtext_save_changes", false);

        assert_ignored_release(page.as_mut(), KeyCode::Esc);
        assert_detail(&render(page.as_mut(), &mut terminal), true);
        assert!(matches!(page.handle_key_press(KeyEvent::from(KeyCode::Esc)), PageMessage::None));
        let buffer = render(page.as_mut(), &mut terminal);
        assert_detail(&buffer, false);
        assert_parent_help(&buffer, editor.help(false), true);
        assert_text(&buffer, "icbtext_save_changes", false);

        press(page.as_mut(), KeyCode::Esc);
        let buffer = render(page.as_mut(), &mut terminal);
        assert_parent_help(&buffer, editor.help(false), false);
        assert_text(&buffer, "icbtext_save_changes", true);
        assert_detail(&buffer, false);
        press(page.as_mut(), KeyCode::Right); // Selecting Yes is not confirmation.
        assert_ignored_release(page.as_mut(), KeyCode::Enter);
        assert_text(&render(page.as_mut(), &mut terminal), "icbtext_save_changes", true);
        assert!(!path.parent().unwrap().exists(), "{editor:?}: wrote before confirming Yes");

        press(page.as_mut(), KeyCode::Esc); // Cancel, not discard and not close.
        let buffer = render(page.as_mut(), &mut terminal);
        assert_parent_help(&buffer, editor.help(false), true);
        assert_text(&buffer, "icbtext_save_changes", false);
        press(page.as_mut(), KeyCode::Enter);
        assert_detail(&render(page.as_mut(), &mut terminal), true);
        press(page.as_mut(), KeyCode::Esc);
        press(page.as_mut(), KeyCode::Esc);
        assert_text(&render(page.as_mut(), &mut terminal), "icbtext_save_changes", true);
        assert_close(page.as_mut(), KeyCode::Enter); // A fresh prompt defaults to No, not the cancelled Yes.
        drop(page);
        assert!(!path.parent().unwrap().exists(), "{editor:?}: cancelling/discarding created storage");
    }

    macro_rules! file_editor_cases {
        ($($name:ident: $editor:ident),+ $(,)?) => {
            $(#[test]
            fn $name() {
                file_editor_modal_lifecycle(FileEditor::$editor);
            })+
        };
    }

    file_editor_cases! {
        protocols_modal_lifecycle: Protocols,
        languages_modal_lifecycle: Languages,
        commands_modal_lifecycle: Commands,
        security_modal_lifecycle: Security,
        dirs_modal_lifecycle: Dirs,
        bullettins_modal_lifecycle: Bullettins,
        surveys_modal_lifecycle: Surveys,
        areas_modal_lifecycle: Areas,
        doors_modal_lifecycle: Doors,
    }

    fn edit_first_row(page: &mut dyn Page, terminal: &mut Terminal<TestBackend>, field: usize, text: &str) {
        press(page, KeyCode::Enter);
        assert_detail(&render(page, terminal), true);
        for _ in 0..field {
            press(page, KeyCode::Down);
            // Like the app event loop, render the newly focused field before input.
            // TextfieldState gets its viewport only when its editor is rendered.
            assert_detail(&render(page, terminal), true);
        }
        press(page, KeyCode::Home);
        for _ in 0..64 {
            press(page, KeyCode::Delete);
        }
        for ch in text.chars() {
            press(page, KeyCode::Char(ch));
        }
        let buffer = render(page, terminal);
        let screen = rows(&buffer);
        assert!(
            screen.iter().any(|row| row.contains(text)),
            "edited value not visible: {text}\n{}",
            screen.join("\n")
        );
        press(page, KeyCode::Esc);
    }

    fn persisted_save_discard_cancel<M: IcyBoardSerializer + PartialEq>(editor: FileEditor, initial: M, expected: M, field: usize) {
        assert!(initial != expected, "fixture must represent a real change");
        for save in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("list.toml");
            initial.save(&path).unwrap();
            let original_bytes = fs::read(&path).unwrap();
            assert!(M::load(&path).unwrap() == initial, "fixture must round-trip");
            let unchanged = || assert_eq!(fs::read(&path).unwrap(), original_bytes, "{editor:?}: premature disk write");
            let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
            let mut page = editor.open(&path);

            edit_first_row(page.as_mut(), &mut terminal, field, "draft");
            unchanged();
            press(page.as_mut(), KeyCode::Esc);
            let buffer = render(page.as_mut(), &mut terminal);
            assert_parent_help(&buffer, editor.help(false), false);
            assert_text(&buffer, "icbtext_save_changes", true);
            // Modal keys must not insert/delete rows in the underlying list.
            press(page.as_mut(), KeyCode::Insert);
            press(page.as_mut(), KeyCode::Delete);
            press(page.as_mut(), KeyCode::Right);
            assert_ignored_release(page.as_mut(), KeyCode::Enter);
            unchanged();
            press(page.as_mut(), KeyCode::Esc);
            let buffer = render(page.as_mut(), &mut terminal);
            assert_parent_help(&buffer, editor.help(false), true);
            assert!(rows(&buffer).iter().any(|row| row.contains("draft")), "cancel lost the working copy");
            unchanged();

            // A second edit after cancel must be saved, not a snapshot taken at the first prompt.
            edit_first_row(page.as_mut(), &mut terminal, field, "kept");
            unchanged();
            press(page.as_mut(), KeyCode::Esc);
            assert_text(&render(page.as_mut(), &mut terminal), "icbtext_save_changes", true);
            if save {
                press(page.as_mut(), KeyCode::Right);
                unchanged();
            }
            assert_close(page.as_mut(), KeyCode::Enter);
            drop(page);
            if save {
                assert!(
                    M::load(&path).unwrap() == expected,
                    "{editor:?}: latest edit or unrelated model fields were lost"
                );
                assert_ne!(fs::read(&path).unwrap(), original_bytes);
            } else {
                unchanged();
                assert!(M::load(&path).unwrap() == initial, "{editor:?}: No must discard all edits");
            }

            let after_close = fs::read(&path).unwrap();
            let mut reopened = editor.open(&path);
            assert_close(reopened.as_mut(), KeyCode::Esc);
            drop(reopened);
            assert_eq!(fs::read(&path).unwrap(), after_close, "reopening a saved/discarded file must be clean");
        }
    }

    #[test]
    fn protocols_save_discard_cancel_preserves_other_protocols() {
        let initial = SupportedProtocols::generate_pcboard_defaults();
        assert!(initial.len() > 1);
        // Diagnose model serialization separately from editor navigation. Keep all
        // defaults, including None: normalizing/filtering them would hide data loss.
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("protocols.toml");
        initial.save(&path).unwrap();
        let loaded = SupportedProtocols::load(&path).unwrap();
        assert_eq!(loaded.len(), initial.len(), "protocol fixture length must round-trip");
        for (before, after) in initial.iter().zip(loaded.iter()) {
            assert_eq!(
                (&after.send_command, &after.recv_command),
                (&before.send_command, &before.recv_command),
                "protocol {}: model serde changed transfer commands before opening the editor",
                before.char_code
            );
        }
        let mut expected = initial.clone();
        expected[0].description = "kept".to_string();
        persisted_save_discard_cancel(FileEditor::Protocols, initial, expected, 1);
    }

    #[test]
    fn languages_save_discard_cancel_preserves_date_formats_and_characters() {
        let mut initial = SupportedLanguages::default();
        initial.date_formats.push(("custom".to_string(), "%Y-%j".to_string()));
        initial.push(Language {
            description: "original".to_string(),
            locale: "de_DE".to_string(),
            extension: "de".to_string(),
            yes_char: 'J',
            no_char: 'N',
        });
        let mut expected = initial.clone();
        expected[0].description = "kept".to_string();
        persisted_save_discard_cancel(FileEditor::Languages, initial, expected, 0);
    }

    #[test]
    fn bullettins_save_discard_cancel_uses_latest_working_vector() {
        let initial = BullettinList {
            bullettins: vec![
                Bullettin::new(&PathBuf::from("original")),
                Bullettin {
                    path: PathBuf::from("unchanged/bulletin"),
                    required_security: "10".parse().unwrap(),
                },
            ],
        };
        let mut expected = initial.clone();
        expected[0].path = PathBuf::from("kept");
        persisted_save_discard_cancel(FileEditor::Bullettins, initial, expected, 0);
    }

    #[test]
    fn surveys_save_discard_cancel_preserves_answer_file_and_security() {
        let initial = SurveyList {
            surveys: vec![Survey {
                survey_file: PathBuf::from("original"),
                answer_file: PathBuf::from("answers/survey.log"),
                required_security: "20".parse().unwrap(),
            }],
        };
        let mut expected = initial.clone();
        expected[0].survey_file = PathBuf::from("kept");
        persisted_save_discard_cancel(FileEditor::Surveys, initial, expected, 0);
    }

    #[test]
    fn new_door_is_written_only_on_yes_with_account_and_runtime_defaults() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("nested/doors.toml");
        let mut page = FileEditor::Doors.open(&path);
        press(page.as_mut(), KeyCode::F(2));
        press(page.as_mut(), KeyCode::Esc);
        press(page.as_mut(), KeyCode::Right);
        assert!(!path.parent().unwrap().exists());
        assert_close(page.as_mut(), KeyCode::Enter);
        let saved = DoorList::load(&path).unwrap();
        assert!(saved.accounts == vec![DoorServerAccount::BBSLink(BBSLink::default())]);
        assert_eq!(saved.len(), 1);
        let door = &saved[0];
        assert_eq!(door.name, "door1");
        assert!(door.door_type == DoorType::Local);
        assert_eq!(door.drop_file, DropFile::None);
        assert!(!door.use_shell_execute);
        assert!(door.path.is_empty() && door.dos_command.is_empty());
        assert_eq!(door.dos_memory_mb, 64);
        assert_eq!(door.dos_max_runtime_seconds, DEFAULT_DOS_MAX_RUNTIME_SECONDS);
    }

    #[test]
    fn new_security_list_keeps_built_in_levels_when_saved() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("nested/security.toml");
        let mut page = FileEditor::Security.open(&path);
        press(page.as_mut(), KeyCode::Insert);
        press(page.as_mut(), KeyCode::Esc);
        press(page.as_mut(), KeyCode::Right);
        assert!(!path.parent().unwrap().exists());
        assert_close(page.as_mut(), KeyCode::Enter);
        let saved = SecurityLevelDefinitions::load(&path).unwrap();
        assert_eq!(saved.iter().map(|level| level.security).collect::<Vec<_>>(), vec![0, 10, 100, 0]);
        assert!(saved.iter().all(|level| level.is_enabled && level.enforce_time_limit));
        assert_eq!(saved.iter().map(|level| level.allow_alias).collect::<Vec<_>>(), vec![false, true, true, false]);
    }

    #[test]
    fn failed_save_returns_to_list_and_allows_retry_without_losing_edits() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("protocols.toml");
        let mut page = FileEditor::Protocols.open(&path);
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        press(page.as_mut(), KeyCode::Insert);
        edit_first_row(page.as_mut(), &mut terminal, 1, "retry");
        // A directory at the file path fails deterministically, even when tests run as root.
        fs::create_dir(&path).unwrap();
        press(page.as_mut(), KeyCode::Esc);
        press(page.as_mut(), KeyCode::Right);
        assert!(matches!(
            page.handle_key_press(KeyEvent::from(KeyCode::Enter)),
            PageMessage::InfoBox(InfoState::Error, _)
        ));
        let buffer = render(page.as_mut(), &mut terminal);
        assert_parent_help(&buffer, "icb_setup_key_conf_list_help", true);
        assert_text(&buffer, "icbtext_save_changes", false);
        assert!(path.is_dir());
        fs::remove_dir(&path).unwrap();
        press(page.as_mut(), KeyCode::Esc);
        press(page.as_mut(), KeyCode::Right);
        assert_close(page.as_mut(), KeyCode::Enter);
        let saved = SupportedProtocols::load(&path).unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].description, "retry");
        assert!(saved[0].is_enabled);
    }

    fn subpage(message: PageMessage) -> Box<dyn Page> {
        match message {
            PageMessage::OpenSubPage(page) => page,
            _ => panic!("menu did not open the expected subpage"),
        }
    }

    #[test]
    fn ftn_list_details_restore_help_and_leave_saving_to_the_parent() {
        for (shortcut, title, insert_opens_detail) in [
            ('C', "fido_node_title", false),
            ('D', "fido_address_title", true),
            ('F', "fido_route_title", false),
            ('G', "fido_freq_path_title", false),
            ('I', "fido_freq_magic_title", false),
            ('J', "fido_freq_deny_title", false),
        ] {
            let directory = tempfile::tempdir().unwrap();
            let board = Arc::new(Mutex::new(IcyBoard {
                file_name: directory.path().join("icboard.toml"),
                root_path: directory.path().to_path_buf(),
                ..Default::default()
            }));
            // Traverse public menus: FTN implementation modules deliberately remain private.
            let mut general = crate::tabs::GeneralTab::new(board.clone());
            general.handle_key_press(KeyEvent::from(KeyCode::Char('K')));
            let mut networking = general.page.sub_pages.pop().expect("message networking menu");
            let mut ftn = subpage(networking.handle_key_press(KeyEvent::from(KeyCode::Char('B'))));
            let mut page = subpage(ftn.handle_key_press(KeyEvent::from(KeyCode::Char(shortcut))));
            let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
            let buffer = render(page.as_mut(), &mut terminal);
            assert_text(&buffer, title, true);
            assert_parent_help(&buffer, "icb_setup_key_conf_list_help", true);
            press(page.as_mut(), KeyCode::Insert);
            if !insert_opens_detail {
                press(page.as_mut(), KeyCode::Enter);
            }
            let buffer = render(page.as_mut(), &mut terminal);
            assert_detail(&buffer, true);
            assert_parent_help(&buffer, "icb_setup_key_conf_list_help", false);
            assert_ignored_release(page.as_mut(), KeyCode::Esc);
            assert_detail(&render(page.as_mut(), &mut terminal), true);
            press(page.as_mut(), KeyCode::Esc);
            let buffer = render(page.as_mut(), &mut terminal);
            assert_detail(&buffer, false);
            assert_parent_help(&buffer, "icb_setup_key_conf_list_help", true);
            assert_text(&buffer, "icbtext_save_changes", false);
            assert_close(page.as_mut(), KeyCode::Esc);
            let board = board.lock().unwrap();
            match shortcut {
                'C' => assert_eq!(board.ftn.links.len(), 1),
                'D' => assert!(board.ftn.akas.is_empty(), "invalid newly inserted AKA must be rolled back"),
                'F' => assert_eq!(board.ftn.routes.len(), 1),
                'G' => assert_eq!(board.ftn.freq.paths.len(), 1),
                'I' => assert_eq!(board.ftn.freq.magic.len(), 1),
                'J' => assert_eq!(board.ftn.freq.deny.len(), 1),
                _ => unreachable!(),
            }
            assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0, "FTN subpage must not save independently");
        }
    }
}
