use super::*;
use ratatui::{Terminal, backend::TestBackend};

const VIEWPORT: Rect = Rect::new(0, 1, 80, 23);

fn press(editor: &mut impl Page, code: KeyCode) -> PageMessage {
    editor.handle_key_press(KeyEvent::from(code))
}

/// Esc closes the form and stores the record, so it also reports validation errors.
fn close_form(editor: &mut EventListEditor<'_>) -> PageMessage {
    press(editor, KeyCode::Esc)
}

/// Esc leaves the list; a changed list asks whether to save first.
fn save_list(editor: &mut EventListEditor<'_>) -> PageMessage {
    let left = press(editor, KeyCode::Esc);
    if !editor.save_changes.is_open() {
        return left;
    }
    press(editor, KeyCode::Right);
    press(editor, KeyCode::Enter)
}

fn sample(description: &str) -> BoardEvent {
    BoardEvent {
        description: description.into(),
        enabled: true,
        time: IcbTime::new(23, 45, 12),
        days: IcbDoW::from("NYYYYYN".to_string()),
        mode: EventMode::Slide,
        command: "./maintenance --report 'nightly log'".into(),
        ..BoardEvent::default()
    }
}

fn existing(path: &Path) -> EventListEditor<'static> {
    let mut events = EventList::default();
    events.push(sample("Nightly maintenance"));
    events.save(&path).unwrap();
    EventListEditor::new(path, path.parent().unwrap()).unwrap()
}

fn replace_text(editor: &mut EventListEditor<'_>, index: usize, text: &str) {
    editor.detail.state.selected = index;
    let old_len = match &editor.detail.menu.as_ref().unwrap().get_item(index).unwrap().value {
        ListValue::Text(_, _, text) => text.chars().count(),
        _ => panic!("not a text field"),
    };
    press(editor, KeyCode::Home);
    for _ in 0..old_len {
        press(editor, KeyCode::Delete);
    }
    for ch in text.chars() {
        press(editor, KeyCode::Char(ch));
    }
}

fn screen(page: &mut impl Page) -> Vec<String> {
    let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
    terminal.draw(|frame| page.render(frame, VIEWPORT)).unwrap();
    let buffer = terminal.backend().buffer();
    (0..25).map(|y| (0..80).map(|x| buffer[(x, y)].symbol()).collect()).collect()
}

#[test]
fn event_editor_validates_time_without_silent_midnight_or_truncation() {
    for (text, expected) in [
        ("00:00", IcbTime::new(0, 0, 0)),
        ("23:59:59", IcbTime::new(23, 59, 59)),
        ("01:02:03", IcbTime::new(1, 2, 3)),
    ] {
        assert_eq!(parse_time(text), Some(expected));
    }
    for text in [
        "",
        "1:02",
        "12:3",
        "24:00",
        "00:60",
        "23:59:60",
        "256:00:00",
        "-1:00:00",
        "01:02:03:04",
        "ab:cd",
        " 01:02",
        "１２:00",
    ] {
        assert_eq!(parse_time(text), None, "{text}");
    }
}

#[test]
fn event_editor_validates_exact_sunday_first_day_mask() {
    for mask in ["YYYYYYY", "NYYYYYN", "NNNNNNN", "YNNNNNN", "NNNNNNY"] {
        assert_eq!(parse_days(mask).unwrap().to_string(), mask);
        assert_eq!(parse_days(&mask.to_lowercase()).unwrap().to_string(), mask);
    }
    for mask in ["", "Y", "YYYYYYYY", "SMTWTFS", "JJJJJJJ", "YYYYYY?", "YYYYYYé"] {
        assert!(parse_days(mask).is_none(), "{mask}");
    }
    assert!(parse_days("NNNNNNN").unwrap().is_empty());
}

#[test]
fn event_editor_empty_navigation_and_deleting_last_record_are_safe() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new/events.toml");
    let mut editor = EventListEditor::new(&path, dir.path()).unwrap();
    for code in [
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::Delete,
        KeyCode::F(5),
        KeyCode::F(6),
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::Enter,
    ] {
        press(&mut editor, code);
        assert_eq!(editor.table.table_state.selected(), None);
    }
    let rows = screen(&mut editor);
    let empty_text = get_text("event_editor_empty");
    let empty_row = rows.iter().find(|row| row.contains(&empty_text)).unwrap_or_else(|| panic!("{rows:?}"));
    let leading = empty_row.len() - empty_row.trim_start().len();
    let trailing = empty_row.len() - empty_row.trim_end().len();
    assert!(leading.abs_diff(trailing) <= 1, "the empty notice is not centered: {empty_row:?}");
    press(&mut editor, KeyCode::Insert);
    assert!(!editor.events.lock().unwrap()[0].enabled);
    assert_eq!(editor.table.table_state.selected(), Some(0));
    press(&mut editor, KeyCode::Delete);
    assert_eq!(editor.table.content_length, 0);
    assert_eq!(editor.table.table_state.selected(), None);
    assert!(matches!(press(&mut editor, KeyCode::Esc), PageMessage::Close));
    assert!(!path.parent().unwrap().exists());
}

#[test]
fn event_editor_insert_repeat_reorder_and_delete_preserve_selection() {
    let dir = tempfile::tempdir().unwrap();
    let mut editor = existing(&dir.path().join("events.toml"));
    press(&mut editor, KeyCode::F(5));
    assert_eq!(editor.table.table_state.selected(), Some(1));
    {
        let events = editor.events.lock().unwrap();
        assert_ne!(events[0].id, events[1].id);
        let mut copy = events[1].clone();
        copy.id = events[0].id.clone();
        assert_eq!(events[0], copy);
    }
    editor.events.lock().unwrap()[1].description = "Copy".into();
    press(&mut editor, KeyCode::PageUp);
    assert_eq!(editor.table.table_state.selected(), Some(0));
    assert_eq!(editor.events.lock().unwrap()[0].description, "Copy");
    press(&mut editor, KeyCode::PageUp);
    assert_eq!(editor.table.table_state.selected(), Some(0));
    press(&mut editor, KeyCode::PageDown);
    press(&mut editor, KeyCode::PageDown);
    assert_eq!(editor.table.table_state.selected(), Some(1));
    press(&mut editor, KeyCode::Insert);
    assert_eq!(editor.table.table_state.selected(), Some(2));
    press(&mut editor, KeyCode::Delete);
    assert_eq!(editor.table.table_state.selected(), Some(1));
    press(&mut editor, KeyCode::Home);
    press(&mut editor, KeyCode::Delete);
    assert_eq!(editor.table.content_length, 1);
    assert_eq!(editor.table.table_state.selected(), Some(0));
    assert_eq!(editor.events.lock().unwrap()[0].description, "Copy");
}

#[test]
fn event_editor_applies_every_field_without_a_render_and_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.toml");
    let mut editor = existing(&path);
    let before = std::fs::read(&path).unwrap();
    press(&mut editor, KeyCode::Enter);
    let description = "Überlange Beschreibung für die tägliche Wartung ".repeat(4);
    let command = "./scripts/maintenance --label 'Grüße 世界' --output '/tmp/nightly report'".repeat(3);
    replace_text(&mut editor, 0, &description);
    editor.detail.state.selected = 1;
    press(&mut editor, KeyCode::Char(' '));
    replace_text(&mut editor, 2, "05:12:34");
    replace_text(&mut editor, 3, "ynnnnny");
    editor.detail.state.selected = 4;
    // Open the list, pick Idle and confirm; no update callback or render required.
    press(&mut editor, KeyCode::Enter);
    press(&mut editor, KeyCode::Down);
    press(&mut editor, KeyCode::Enter);
    replace_text(&mut editor, 5, &command);
    replace_text(&mut editor, 6, "06:12:34");
    replace_text(&mut editor, 7, "15");
    replace_text(&mut editor, 8, "60");
    editor.detail.state.selected = 9;
    press(&mut editor, KeyCode::Enter);
    press(&mut editor, KeyCode::Down);
    press(&mut editor, KeyCode::Enter);
    close_form(&mut editor);
    assert!(!editor.detail.is_open());
    let event = editor.events.lock().unwrap()[0].clone();
    assert_eq!(event.description, description);
    assert!(!event.enabled);
    assert_eq!(event.time, IcbTime::new(5, 12, 34));
    assert_eq!(event.days.to_string(), "YNNNNNY");
    assert_eq!(event.mode, EventMode::Idle);
    assert_eq!(event.command, command);
    assert_eq!(event.id, editor.original[0].id);
    assert_eq!(event.end_time, Some(IcbTime::new(6, 12, 34)));
    assert_eq!(event.interval_minutes, Some(15));
    assert_eq!(event.warning_minutes, Some(60));
    assert_eq!(event.execution, EventExecution::Online);
    assert_eq!(std::fs::read(&path).unwrap(), before, "applying a record must not save the list");
    assert!(matches!(save_list(&mut editor), PageMessage::Close));
    assert_eq!(EventList::load(&path).unwrap()[0], event);
}

#[test]
fn event_editor_form_cancel_and_unchanged_apply_do_not_dirty_the_list() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.toml");
    let mut editor = existing(&path);
    let before = std::fs::read(&path).unwrap();
    press(&mut editor, KeyCode::Enter);
    screen(&mut editor);
    close_form(&mut editor);
    assert_eq!(*editor.events.lock().unwrap(), editor.original);
    assert!(matches!(press(&mut editor, KeyCode::Esc), PageMessage::Close));
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn event_editor_invalid_form_keeps_draft_and_original_for_correction() {
    let dir = tempfile::tempdir().unwrap();
    let mut editor = existing(&dir.path().join("events.toml"));
    press(&mut editor, KeyCode::Enter);
    replace_text(&mut editor, 0, "Keep this draft");
    replace_text(&mut editor, 2, "25:00");
    assert!(matches!(close_form(&mut editor), PageMessage::InfoBox(InfoState::Warning, text) if text == get_text("event_editor_invalid_time")));
    assert!(editor.detail.is_open());
    assert_eq!(editor.detail.state.selected, 2);
    assert_eq!(*editor.events.lock().unwrap(), editor.original);
    replace_text(&mut editor, 2, "00:00");
    replace_text(&mut editor, 3, "YYYYYYYY");
    assert!(matches!(close_form(&mut editor), PageMessage::InfoBox(InfoState::Warning, text) if text == get_text("event_editor_invalid_days")));
    assert!(editor.detail.is_open());
    assert_eq!(editor.detail.state.selected, 3);
    replace_text(&mut editor, 3, "NNNNNNN");
    close_form(&mut editor);
    assert_eq!(editor.events.lock().unwrap()[0].description, "Keep this draft");
    assert!(editor.events.lock().unwrap()[0].days.is_empty());
}

#[test]
fn event_editor_combobox_escape_closes_dropdown_before_validating_and_applying_form() {
    for index in [4, 9] {
        for invalid in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("events.toml");
            let mut editor = existing(&path);
            let before = std::fs::read(&path).unwrap();
            press(&mut editor, KeyCode::Enter);
            replace_text(&mut editor, 0, "Keep dropdown draft");
            if invalid {
                replace_text(&mut editor, 2, "25:00");
            }
            editor.detail.state.selected = index;
            press(&mut editor, KeyCode::Enter);
            press(&mut editor, KeyCode::Down);
            screen(&mut editor);
            let ListValue::ComboBox(combo) = &editor.detail.menu.as_ref().unwrap().get_item(index).unwrap().value else {
                panic!("not a combo box");
            };
            assert!(combo.is_edit_open);
            assert_ne!(combo.values[combo.selected_item].value, combo.cur_value.value);
            let original_value = combo.cur_value.value.clone();

            assert!(matches!(press(&mut editor, KeyCode::Esc), PageMessage::ResultState(result) if result.edit_msg == EditMessage::None));
            assert!(editor.detail.is_open());
            assert_eq!(editor.detail.state.selected, index);
            let menu = editor.detail.menu.as_ref().unwrap();
            let ListValue::ComboBox(combo) = &menu.get_item(index).unwrap().value else {
                panic!("not a combo box");
            };
            assert!(!combo.is_edit_open);
            assert_eq!(combo.cur_value.value, original_value, "Escape must not commit the highlighted choice");
            assert!(matches!(&menu.get_item(0).unwrap().value, ListValue::Text(_, _, text) if text == "Keep dropdown draft"));
            assert_eq!(*editor.events.lock().unwrap(), editor.original);
            assert!(!editor.save_changes.is_open());
            screen(&mut editor);

            let result = press(&mut editor, KeyCode::Esc);
            if invalid {
                assert!(matches!(result, PageMessage::InfoBox(InfoState::Warning, text) if text == get_text("event_editor_invalid_time")));
                assert!(editor.detail.is_open());
                assert_eq!(editor.detail.state.selected, 2);
                assert_eq!(*editor.events.lock().unwrap(), editor.original);
                replace_text(&mut editor, 2, "23:45:12");
                assert!(matches!(close_form(&mut editor), PageMessage::None));
            } else {
                assert!(matches!(result, PageMessage::None));
            }
            assert!(!editor.detail.is_open());
            let mut expected = editor.original.clone();
            expected[0].description = "Keep dropdown draft".into();
            assert_eq!(*editor.events.lock().unwrap(), expected);
            assert_eq!(std::fs::read(&path).unwrap(), before);
        }
    }
}

#[test]
fn event_editor_reopening_detail_resets_selection_but_preserves_board_root() {
    let dir = tempfile::tempdir().unwrap();
    let mut editor = existing(&dir.path().join("events.toml"));
    let root = dir.path().join("board-root");
    editor.detail.state.path_base = Some(root.clone());
    press(&mut editor, KeyCode::Enter);
    editor.detail.state.selected = 9;
    assert!(matches!(close_form(&mut editor), PageMessage::None));
    assert!(!editor.detail.is_open());
    press(&mut editor, KeyCode::Enter);
    assert!(editor.detail.is_open());
    assert_eq!(editor.detail.state.selected, 0);
    assert_eq!(editor.detail.state.path_base, Some(root));
    assert!(!editor.detail.state.is_path_browser_open());
    assert_eq!(*editor.events.lock().unwrap(), editor.original);
}

#[test]
fn event_editor_save_prompt_cancel_discard_and_save_have_distinct_effects() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.toml");
    let mut editor = existing(&path);
    let before = std::fs::read(&path).unwrap();
    press(&mut editor, KeyCode::Delete);
    press(&mut editor, KeyCode::Esc);
    assert!(editor.save_changes.is_open());
    screen(&mut editor);
    press(&mut editor, KeyCode::Esc); // Cancel the prompt, not the edits.
    assert!(!editor.save_changes.is_open());
    assert!(editor.events.lock().unwrap().is_empty());
    press(&mut editor, KeyCode::Esc);
    assert!(matches!(press(&mut editor, KeyCode::Enter), PageMessage::Close)); // Defaults to No.
    assert!(!editor.save_changes.is_open());
    assert_eq!(std::fs::read(&path).unwrap(), before);

    let mut editor = EventListEditor::new(&path, dir.path()).unwrap();
    press(&mut editor, KeyCode::F(5));
    press(&mut editor, KeyCode::Esc);
    press(&mut editor, KeyCode::Right); // Yes.
    assert!(matches!(press(&mut editor, KeyCode::Enter), PageMessage::Close));
    assert!(!editor.save_changes.is_open());
    assert_eq!(EventList::load(&path).unwrap().len(), 2);
}

#[test]
fn event_editor_creates_missing_parents_only_on_save() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new/nested/events.toml");
    let mut editor = EventListEditor::new(&path, dir.path()).unwrap();
    press(&mut editor, KeyCode::Insert);
    assert!(!path.parent().unwrap().exists());
    assert!(matches!(save_list(&mut editor), PageMessage::Close));
    let saved = EventList::load(&path).unwrap();
    assert_eq!(saved.len(), 1);
    assert!(!saved[0].enabled);
}

#[test]
fn event_editor_save_failure_retains_working_list_and_allows_retry() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("blocked/events.toml");
    let mut editor = EventListEditor::new(&path, dir.path()).unwrap();
    press(&mut editor, KeyCode::Insert);
    std::fs::write(path.parent().unwrap(), "do not overwrite").unwrap();
    press(&mut editor, KeyCode::Esc);
    press(&mut editor, KeyCode::Right);
    assert!(matches!(press(&mut editor, KeyCode::Enter), PageMessage::InfoBox(InfoState::Error, _)));
    assert!(!editor.save_changes.is_open());
    assert_eq!(editor.events.lock().unwrap().len(), 1);
    assert_eq!(std::fs::read_to_string(path.parent().unwrap()).unwrap(), "do not overwrite");
    std::fs::remove_file(path.parent().unwrap()).unwrap();
    assert!(matches!(save_list(&mut editor), PageMessage::Close));
    assert_eq!(EventList::load(&path).unwrap().len(), 1);
}

#[test]
fn event_editor_malformed_files_and_directories_are_reported_not_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.toml");
    std::fs::write(&path, "not valid TOML = [").unwrap();
    let board = Arc::new(Mutex::new(IcyBoard::default()));
    assert!(matches!(edit_events(board.clone(), path.clone()), PageMessage::InfoBox(InfoState::Error, text) if text.contains("events.toml")));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "not valid TOML = [");
    assert!(matches!(
        edit_events(board, dir.path().to_path_buf()),
        PageMessage::InfoBox(InfoState::Error, _)
    ));
    assert!(EventListEditor::new(Path::new(""), dir.path()).is_err());
}

#[test]
fn the_mode_legend_is_readable_instead_of_inheriting_the_frame_colour() {
    let dir = tempfile::tempdir().unwrap();
    let mut editor = existing(&dir.path().join("events.toml"));
    let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
    terminal.draw(|frame| editor.render(frame, VIEWPORT)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let legend = get_text("event_editor_mode_legend");
    let row = (0..25)
        .find(|y| (0..80).map(|x| buffer[(x, *y)].symbol()).collect::<String>().contains(&legend))
        .expect("mode legend");
    let theme = get_tui_theme();
    assert_ne!(theme.config_title.fg, theme.dialog_box.fg, "the legend colour must differ from the frame");
    let cells: Vec<_> = (1..79)
        .map(|x| buffer[(x, row)].clone())
        .filter(|cell| !cell.symbol().trim().is_empty())
        .collect();
    assert_eq!(cells.len(), legend.chars().filter(|ch| !ch.is_whitespace()).count());
    for cell in cells {
        assert_eq!(Some(cell.fg), theme.config_title.fg, "unreadable legend cell {:?}", cell.symbol());
        assert_eq!(Some(cell.bg), theme.config_title.bg);
    }
}

#[test]
fn event_editor_table_and_all_detail_labels_fit_at_80x25() {
    let dir = tempfile::tempdir().unwrap();
    let mut editor = existing(&dir.path().join("events.toml"));
    let rows = screen(&mut editor);
    for key in [
        "event_editor_title",
        "event_editor_keys",
        "event_editor_keys_more",
        "event_editor_mode_legend",
        "event_editor_header_enabled",
        "event_editor_header_mode",
        "event_editor_header_time",
        "event_editor_header_days",
        "event_editor_header_description",
        "event_editor_header_command",
    ] {
        assert!(rows.iter().any(|row| row.contains(&get_text(key))), "missing {key}: {rows:?}");
    }
    // Both legend rows stay inside the frame, directly below the mode legend.
    let legend = rows.iter().position(|row| row.contains(&get_text("event_editor_mode_legend"))).unwrap();
    assert!(rows[legend + 1].contains(&get_text("event_editor_keys")));
    assert!(rows[legend + 2].contains(&get_text("event_editor_keys_more")));
    assert!(rows[23].trim_matches(|ch: char| !ch.is_alphanumeric()).is_empty());
    // A modal owns the keyboard, so the list legend must step aside.
    for open in [KeyCode::Enter, KeyCode::F(6), KeyCode::Esc] {
        if open == KeyCode::Esc {
            press(&mut editor, KeyCode::F(5)); // Dirty the list to open the save prompt.
        }
        press(&mut editor, open);
        if open == KeyCode::Esc {
            assert!(editor.save_changes.is_open());
        }
        let covered = screen(&mut editor);
        for key in ["event_editor_mode_legend", "event_editor_keys", "event_editor_keys_more"] {
            assert!(!covered.iter().any(|row| row.contains(&get_text(key))), "{key} stays visible: {covered:?}");
        }
        press(&mut editor, KeyCode::Esc);
        assert!(!editor.save_changes.is_open());
        assert!(screen(&mut editor).iter().any(|row| row.contains(&get_text("event_editor_keys"))));
    }
    // Active marker, mode letter, full time and day mask stay in separate columns.
    let record = rows.iter().find(|row| row.contains("1)")).unwrap_or_else(|| panic!("{rows:?}"));
    let letter = get_text("event_editor_mode_slide_letter");
    assert!(
        record.contains(&format!("✓    {letter}    23:45:12  NYYYYYN  Nightly maintenance  .")),
        "columns collide or the time is truncated: {record:?}"
    );
    assert!(record.contains("./maintenance"));
    press(&mut editor, KeyCode::Enter);
    for index in 0..10 {
        editor.detail.state.selected = index;
        let rows = screen(&mut editor);
        for key in [
            "event_editor_description",
            "event_editor_enabled",
            "event_editor_time",
            "event_editor_days",
            "event_editor_mode",
            "event_editor_command",
            "event_editor_end_time",
            "event_editor_interval",
            "event_editor_warning",
            "event_editor_execution",
            "event_editor_detail_keys",
        ] {
            assert!(rows.iter().any(|row| row.contains(&get_text(key))), "missing {key}: {rows:?}");
        }
        for row in &rows[6..18] {
            assert_eq!(row.chars().nth(4), Some('║'), "{row}");
            assert_eq!(row.chars().nth(75), Some('║'), "{row}");
        }
        assert!(rows[6..18].iter().all(|row| !row.contains(['▲', '▼'])));
    }
}

#[test]
fn event_editor_parent_table_geometry_stays_stable_under_every_modal() {
    let dir = tempfile::tempdir().unwrap();
    let mut editor = existing(&dir.path().join("events.toml"));
    for _ in 1..30 {
        press(&mut editor, KeyCode::F(5));
    }
    assert_eq!(editor.table.table_state.selected(), Some(29));
    let rows = screen(&mut editor);
    let offset = editor.table.table_state.offset();
    assert!(offset > 0, "the fixture must scroll to expose viewport-height changes");
    assert!(rows.iter().any(|row| row.contains("30)")));
    let staged = editor.events.lock().unwrap().clone();

    for open in [KeyCode::Enter, KeyCode::F(6), KeyCode::Esc] {
        press(&mut editor, open);
        match open {
            KeyCode::Enter => assert!(editor.detail.is_open()),
            KeyCode::F(6) => assert!(editor.history.is_some()),
            KeyCode::Esc => assert!(editor.save_changes.is_open()),
            _ => unreachable!(),
        }
        // Recompute visibility from the same offset: a larger modal-era table
        // would fit extra rows and scroll the selected record to a different row.
        *editor.table.table_state.offset_mut() = 0;
        let covered = screen(&mut editor);
        assert_eq!(editor.table.table_state.offset(), offset, "parent table resized under {open:?}");
        assert_eq!(editor.table.table_state.selected(), Some(29));
        for key in ["event_editor_mode_legend", "event_editor_keys", "event_editor_keys_more"] {
            assert!(!covered.iter().any(|row| row.contains(&get_text(key))), "{key} stays visible under {open:?}");
        }
        assert!(matches!(press(&mut editor, KeyCode::Esc), PageMessage::None));
        assert!(!editor.detail.is_open());
        assert!(editor.history.is_none());
        assert!(!editor.save_changes.is_open());
        assert_eq!(screen(&mut editor), rows, "parent table changed after {open:?}");
        assert_eq!(editor.table.table_state.offset(), offset);
        assert_eq!(*editor.events.lock().unwrap(), staged);
    }
}

#[test]
fn event_editor_help_and_command_path_guard_are_wired() {
    let dir = tempfile::tempdir().unwrap();
    let mut editor = existing(&dir.path().join("events.toml"));
    assert!(
        matches!(press(&mut editor, KeyCode::F(1)), PageMessage::ResultState(result) if result.edit_msg == EditMessage::DisplayHelp(get_text("event_editor_help")))
    );
    press(&mut editor, KeyCode::Enter);
    for index in 0..10 {
        editor.detail.state.selected = index;
        assert!(
            matches!(press(&mut editor, KeyCode::F(1)), PageMessage::ResultState(result) if matches!(result.edit_msg, EditMessage::DisplayHelp(ref text) if !text.is_empty()))
        );
    }
    let command = editor.events.lock().unwrap()[0].command.clone();
    editor.detail.state.selected = 5;
    press(&mut editor, KeyCode::F(4));
    assert!(!editor.detail.state.is_path_browser_open());
    close_form(&mut editor);
    assert_eq!(editor.events.lock().unwrap()[0].command, command);
}

#[test]
fn event_editor_optional_minutes_reject_zero_signs_fractions_and_overflow() {
    for text in ["", "   "] {
        assert_eq!(parse_minutes(text), Some(None));
    }
    assert_eq!(parse_minutes("1"), Some(Some(1)));
    assert_eq!(parse_minutes("4294967295"), Some(Some(u32::MAX)));
    for text in ["0", "000", "-1", "+1", "1.5", "4294967296", "abc", "１２", " 1"] {
        assert_eq!(parse_minutes(text), None, "{text}");
    }
}

#[test]
fn event_editor_latest_start_is_inclusive_same_day_and_blank_disables_options() {
    let dir = tempfile::tempdir().unwrap();
    let mut editor = existing(&dir.path().join("events.toml"));
    press(&mut editor, KeyCode::Enter);
    for text in ["23:45:11", "00:01", "24:00", "bad"] {
        replace_text(&mut editor, 6, text);
        assert!(matches!(close_form(&mut editor), PageMessage::InfoBox(InfoState::Warning, message) if message == get_text("event_editor_invalid_end_time")));
        assert!(editor.detail.is_open());
        assert_eq!(editor.detail.state.selected, 6);
        assert_eq!(*editor.events.lock().unwrap(), editor.original);
    }
    replace_text(&mut editor, 6, "23:45:12");
    replace_text(&mut editor, 7, "1");
    replace_text(&mut editor, 8, "4294967295");
    close_form(&mut editor);
    let event = editor.events.lock().unwrap()[0].clone();
    assert_eq!(event.end_time, Some(event.time));
    assert_eq!(event.interval_minutes, Some(1));
    assert_eq!(event.warning_minutes, Some(u32::MAX));
    press(&mut editor, KeyCode::Enter);
    for index in [6, 7, 8] {
        replace_text(&mut editor, index, "");
    }
    close_form(&mut editor);
    let event = editor.events.lock().unwrap()[0].clone();
    assert_eq!(event.end_time, None);
    assert_eq!(event.interval_minutes, None);
    assert_eq!(event.warning_minutes, None);
    assert_eq!(event.execution, EventExecution::Maintenance);
}

#[test]
fn event_editor_invalid_minutes_focus_field_and_preserve_draft() {
    let dir = tempfile::tempdir().unwrap();
    let mut editor = existing(&dir.path().join("events.toml"));
    press(&mut editor, KeyCode::Enter);
    replace_text(&mut editor, 0, "Draft");
    for index in [7, 8] {
        for text in ["0", "-1", "1.5", "4294967296"] {
            replace_text(&mut editor, index, text);
            assert!(
                matches!(close_form(&mut editor), PageMessage::InfoBox(InfoState::Warning, message) if message == get_text("event_editor_invalid_minutes"))
            );
            assert!(editor.detail.is_open());
            assert_eq!(editor.detail.state.selected, index);
            assert_eq!(*editor.events.lock().unwrap(), editor.original);
        }
        replace_text(&mut editor, index, "");
    }
    close_form(&mut editor);
    assert_eq!(editor.events.lock().unwrap()[0].description, "Draft");
}

#[test]
fn event_editor_new_and_duplicate_ids_are_distinct_and_persisted() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.toml");
    let mut editor = existing(&path);
    let original_id = editor.original[0].id.clone();
    press(&mut editor, KeyCode::F(5));
    press(&mut editor, KeyCode::Insert);
    press(&mut editor, KeyCode::Insert);
    let staged = editor.events.lock().unwrap().clone();
    let ids: std::collections::HashSet<_> = staged.iter().map(|event| event.id.clone()).collect();
    assert_eq!(ids.len(), 4);
    assert_eq!(staged[0].id, original_id);
    assert!(staged.iter().all(|event| !event.id.is_empty()));
    assert!(matches!(save_list(&mut editor), PageMessage::Close));
    assert_eq!(EventList::load(&path).unwrap(), staged);
}

#[test]
fn event_editor_online_warning_help_and_aligned_values_fit_at_80x25() {
    let dir = tempfile::tempdir().unwrap();
    let mut editor = existing(&dir.path().join("events.toml"));
    press(&mut editor, KeyCode::Enter);
    editor.detail.state.selected = 9;
    press(&mut editor, KeyCode::Enter);
    press(&mut editor, KeyCode::Down);
    press(&mut editor, KeyCode::Enter);
    let rows = screen(&mut editor);
    let displayed = rows.join("\n");
    assert!(displayed.contains("ONLINE:"));
    assert!(displayed.contains("LIVE"));
    for key in [
        "event_editor_execution",
        "event_editor_warning",
        "event_editor_interval",
        "event_editor_end_time",
        "event_editor_detail_keys",
    ] {
        assert!(rows.iter().any(|row| row.contains(&get_text(key))), "{key}: {rows:?}");
    }
    // Every single-column label uses the same delimiter cell, even in German.
    let columns: Vec<_> = rows
        .iter()
        .filter(|row| row.contains(&get_text("event_editor_time")) || row.contains(&get_text("event_editor_end_time")))
        .map(|row| row.chars().position(|ch| ch == ':').unwrap())
        .collect();
    assert_eq!(columns.len(), 2);
    assert_eq!(columns[0], columns[1]);
    assert!(
        matches!(press(&mut editor, KeyCode::F(1)), PageMessage::ResultState(result) if result.edit_msg == EditMessage::DisplayHelp(get_text("event_editor_execution-help")))
    );
    assert!(get_text("event_editor_execution-help").contains("icbmailer"));
    close_form(&mut editor);
    assert_eq!(editor.events.lock().unwrap()[0].execution, EventExecution::Online);
}

#[test]
fn event_editor_missing_history_is_empty_read_only_and_does_not_create_root() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("missing-board");
    let path = dir.path().join("events.toml");
    let mut editor = existing(&path);
    editor.root = root.clone();
    let before = std::fs::read(&path).unwrap();
    press(&mut editor, KeyCode::F(6));
    assert!(editor.history.as_ref().unwrap().entries.is_empty());
    assert!(screen(&mut editor).iter().any(|row| row.contains(&get_text("event_editor_history_empty"))));
    for key in [
        KeyCode::F(2),
        KeyCode::F(5),
        KeyCode::Insert,
        KeyCode::Delete,
        KeyCode::Enter,
        KeyCode::Up,
        KeyCode::Down,
    ] {
        assert!(matches!(press(&mut editor, key), PageMessage::None));
    }
    assert!(editor.history.is_some());
    assert_eq!(*editor.events.lock().unwrap(), editor.original);
    press(&mut editor, KeyCode::Esc);
    assert!(editor.history.is_none());
    assert!(matches!(press(&mut editor, KeyCode::Esc), PageMessage::Close));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(!root.exists());
}

#[test]
fn event_editor_history_reads_while_scheduler_owns_lease_without_recovering_pending() {
    use icy_board_engine::icy_board::events::event_history::HISTORY_FILE;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.toml");
    let mut editor = existing(&path);
    let event = editor.original[0].clone();
    let now = "2026-09-07T03:00:00Z".parse::<chrono::DateTime<chrono::Utc>>().unwrap();
    // Simulate a live scheduler: the editor must succeed while this lease is held.
    let mut journal = EventHistory::open(dir.path(), now).unwrap();
    let old = journal.claim(&event, now, false).unwrap().unwrap();
    let log = journal.start(&old.key, now).unwrap();
    std::fs::create_dir_all(log.parent().unwrap()).unwrap();
    std::fs::write(&log, "fixture output").unwrap();
    journal
        .finish(&old.key, now, EventResult::NonzeroExit, Some(7), Some("fixture detail".into()))
        .unwrap();
    let later = now + chrono::Duration::minutes(1);
    journal.claim(&event, later, false).unwrap().unwrap();
    journal.claim(&sample("Other event"), later, false).unwrap().unwrap();
    let before = std::fs::read(dir.path().join(HISTORY_FILE)).unwrap();
    press(&mut editor, KeyCode::F(6));
    let history = editor.history.as_ref().unwrap();
    assert_eq!(history.entries.len(), 2);
    assert_eq!(history.entries[0].result, EventResult::Pending);
    assert_eq!(history.entries[0].scheduled_for, later);
    press(&mut editor, KeyCode::Down);
    let rows = screen(&mut editor);
    for key in [
        "event_editor_history_title",
        "event_editor_history_keys",
        "event_editor_history_latest",
        "event_editor_history_exit",
        "event_editor_history_log_time",
        "event_editor_result_nonzero_exit",
    ] {
        assert!(rows.iter().any(|row| row.contains(&get_text(key))), "{key}: {rows:?}");
    }
    let display = rows.join("\n");
    assert!(display.contains("2026-09-07T03:00:00+00:00"));
    assert!(display.contains(&format!("{}: 7", get_text("event_editor_history_exit"))));
    assert!(display.contains(&format!("{}:", get_text("event_editor_history_log"))));
    assert!(display.contains("event_logs/"));
    assert!(!display.contains(&get_text("event_editor_history_unavailable")));
    assert!(
        matches!(press(&mut editor, KeyCode::F(1)), PageMessage::ResultState(result) if result.edit_msg == EditMessage::DisplayHelp(get_text("event_editor_history_help")))
    );
    press(&mut editor, KeyCode::PageDown);
    assert_eq!(editor.history.as_ref().unwrap().scroll, 5);
    press(&mut editor, KeyCode::PageUp);
    assert_eq!(editor.history.as_ref().unwrap().scroll, 0);
    press(&mut editor, KeyCode::F(6));
    assert_eq!(editor.history.as_ref().unwrap().selected, 0);
    assert_eq!(std::fs::read(dir.path().join(HISTORY_FILE)).unwrap(), before);
    assert_eq!(std::fs::read_to_string(&log).unwrap(), "fixture output");
    press(&mut editor, KeyCode::Esc);
    press(&mut editor, KeyCode::F(5));
    press(&mut editor, KeyCode::F(6));
    assert!(
        editor.history.as_ref().unwrap().entries.is_empty(),
        "copies must not inherit the source ID's history"
    );
    assert_eq!(std::fs::read(dir.path().join(HISTORY_FILE)).unwrap(), before);
}

#[test]
fn event_editor_history_result_labels_are_localized() {
    for result in [
        EventResult::Pending,
        EventResult::Success,
        EventResult::NonzeroExit,
        EventResult::SpawnError,
        EventResult::WaitError,
        EventResult::Interrupted,
        EventResult::SkippedBusy,
        EventResult::Expired,
        EventResult::Superseded,
    ] {
        let label = result_label(&result);
        assert!(!label.is_empty());
        assert!(!label.contains("event_editor_"), "untranslated result: {label}");
    }
}

#[test]
fn event_editor_history_uses_board_root_not_arbitrary_event_file_parent() {
    use icy_board_engine::icy_board::events::event_history::HISTORY_FILE;
    let root = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let path = other.path().join("events.toml");
    let editor = existing(&path);
    let now = "2026-09-07T03:00:00Z".parse::<chrono::DateTime<chrono::Utc>>().unwrap();
    let mut journal = EventHistory::open(root.path(), now).unwrap();
    journal.claim(&editor.original[0], now, false).unwrap();
    std::fs::write(other.path().join(HISTORY_FILE), "invalid foreign journal = [").unwrap();
    let board = IcyBoard {
        root_path: root.path().to_path_buf(),
        ..Default::default()
    };
    let PageMessage::OpenSubPage(mut page) = edit_events(Arc::new(Mutex::new(board)), path) else {
        panic!("editor")
    };
    assert!(matches!(page.handle_key_press(KeyEvent::from(KeyCode::F(6))), PageMessage::None));
    let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
    terminal.draw(|frame| page.render(frame, VIEWPORT)).unwrap();
    let buffer = terminal.backend().buffer();
    let text = (0..25)
        .map(|y| (0..80).map(|x| buffer[(x, y)].symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains(&get_text("event_editor_result_pending")));
    assert!(!text.contains(&get_text("event_editor_history_empty")));
    assert_eq!(std::fs::read_to_string(other.path().join(HISTORY_FILE)).unwrap(), "invalid foreign journal = [");
}

#[test]
fn event_editor_history_errors_are_reported_without_replacement_and_do_not_block_editing() {
    use icy_board_engine::icy_board::events::event_history::HISTORY_FILE;
    let dir = tempfile::tempdir().unwrap();
    let mut editor = existing(&dir.path().join("events.toml"));
    let history = dir.path().join(HISTORY_FILE);
    for broken in ["invalid = [", "version = 999\nentries = []"] {
        std::fs::write(&history, broken).unwrap();
        assert!(matches!(press(&mut editor, KeyCode::F(6)), PageMessage::InfoBox(InfoState::Error, _)));
        assert!(editor.history.is_none());
        assert_eq!(std::fs::read_to_string(&history).unwrap(), broken);
        assert!(!dir.path().join(".event_history.lock").exists());
    }
    std::fs::remove_file(&history).unwrap();
    std::fs::create_dir(&history).unwrap();
    assert!(matches!(press(&mut editor, KeyCode::F(6)), PageMessage::InfoBox(InfoState::Error, _)));
    assert!(history.is_dir());
    press(&mut editor, KeyCode::Enter);
    replace_text(&mut editor, 0, "Still editable");
    close_form(&mut editor);
    assert_eq!(editor.events.lock().unwrap()[0].description, "Still editable");
    assert!(matches!(save_list(&mut editor), PageMessage::Close));
    assert!(history.is_dir());
}
