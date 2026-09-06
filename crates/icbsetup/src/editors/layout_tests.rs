use crossterm::event::{KeyCode, KeyEvent};
use icy_board_tui::{get_text, tab_page::Page};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};

use super::{areas::MessageAreasEditor, dirs::DirsEditor, door::DoorEditor, sec_editor::SecurityLevelEditor};

const VIEWPORT: Rect = Rect::new(0, 1, 80, 23);

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
    let rows: Vec<String> = (2..6).map(|y| (1..79).map(|x| buffer[(x, y)].symbol()).collect()).collect();
    for key in [
        "doors_editor_bbslink_credentials",
        "doors_editor_system_code",
        "doors_editor_auth_code",
        "doors_editor_scheme_code",
    ] {
        let label = get_text(key);
        assert!(rows.iter().any(|row| row.contains(&label)), "clipped label: {label}");
    }
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
