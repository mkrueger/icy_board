use super::*;
use ratatui::{Terminal, backend::TestBackend};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn tab(commands: Vec<Command>) -> CommandsTab<'static> {
    CommandsTab::new(
        Arc::new(Mutex::new(IcyBoard::default())),
        Arc::new(Mutex::new(Menu {
            commands,
            ..Default::default()
        })),
    )
}

fn command(keyword: &str) -> Command {
    Command {
        keyword: keyword.into(),
        ..Default::default()
    }
}

fn draw(tab: &mut CommandsTab<'_>, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| tab.render(frame, frame.area())).unwrap();
    terminal.backend().buffer().content.iter().map(|c| c.symbol()).collect()
}

#[test]
fn commands_tab_uses_localized_title_and_headers() {
    let mut tab = tab(vec![]);
    assert_eq!(tab.title(), get_text("tui_tab_commands"));
    let rendered = draw(&mut tab, 80, 25);
    for id in ["command_editor_keyword", "mnu_editor_display"] {
        let text = get_text(id);
        assert_ne!(text, id);
        assert!(rendered.contains(&text), "missing {id}: {rendered}");
    }
}

#[test]
fn new_is_only_inserted_on_f10_and_cancel_leaves_no_empty_entry() {
    let mut tab = tab(vec![]);
    tab.handle_key_press(key(KeyCode::Insert));
    assert!(tab.has_control());
    assert!(tab.menu.lock().unwrap().commands.is_empty());
    tab.handle_key_press(key(KeyCode::Esc));
    assert!(!tab.has_control());
    assert!(tab.menu.lock().unwrap().commands.is_empty());
    assert!(!tab.is_dirty());
    tab.handle_key_press(key(KeyCode::Insert));
    tab.handle_key_press(key(KeyCode::F(10)));
    assert!(tab.is_dirty());
    assert_eq!(tab.menu.lock().unwrap().commands.len(), 1);
    assert_eq!(tab.insert_table.content_length, 1);
    assert_eq!(tab.insert_table.table_state.selected(), Some(0));
}

#[test]
fn existing_draft_cancel_and_commit_do_not_depend_on_parent_selection() {
    let mut tab = tab(vec![command("A"), command("B")]);
    tab.handle_key_press(key(KeyCode::Enter));
    tab.handle_key_press(key(KeyCode::Char('X')));
    tab.insert_table.table_state.select(Some(1));
    tab.handle_key_press(key(KeyCode::Esc));
    assert!(tab.menu.lock().unwrap().commands[0].display.is_empty());
    tab.insert_table.table_state.select(Some(0));
    tab.handle_key_press(key(KeyCode::Enter));
    tab.handle_key_press(key(KeyCode::Char('X')));
    tab.insert_table.table_state.select(Some(1));
    tab.handle_key_press(key(KeyCode::F(10)));
    assert_eq!(tab.menu.lock().unwrap().commands[0].display, "X");
    assert!(tab.menu.lock().unwrap().commands[1].display.is_empty());
}

#[test]
fn duplicate_is_a_draft_not_a_shared_record() {
    let mut tab = tab(vec![command("A")]);
    let duplicate = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
    tab.handle_key_press(duplicate);
    assert_eq!(tab.menu.lock().unwrap().commands.len(), 1);
    tab.handle_key_press(key(KeyCode::Esc));
    assert_eq!(tab.menu.lock().unwrap().commands.len(), 1);
    tab.handle_key_press(duplicate);
    tab.handle_key_press(key(KeyCode::Char('X')));
    tab.handle_key_press(key(KeyCode::F(10)));
    let menu = tab.menu.lock().unwrap();
    assert_eq!(menu.commands.len(), 2);
    assert!(menu.commands[0].display.is_empty());
    assert_eq!(menu.commands[1].display, "X");
}

#[test]
fn empty_lists_and_extreme_stale_indices_are_repaired_before_navigation() {
    let mut tab = tab(vec![]);
    for code in [
        KeyCode::Down,
        KeyCode::Up,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::Delete,
        KeyCode::Enter,
    ] {
        tab.insert_table.table_state.select(Some(usize::MAX));
        tab.handle_key_press(key(code));
        assert_eq!(tab.insert_table.table_state.selected(), None);
    }
    tab.menu.lock().unwrap().commands = vec![command("A"), command("B")];
    tab.refresh(None);
    tab.insert_table.table_state.select(Some(usize::MAX));
    tab.handle_key_press(key(KeyCode::PageUp));
    assert_eq!(tab.menu.lock().unwrap().commands[0].keyword, "B");
    tab.handle_key_press(key(KeyCode::Delete));
    assert_eq!(tab.insert_table.table_state.selected(), Some(0));
    tab.handle_key_press(key(KeyCode::Delete));
    assert_eq!(tab.insert_table.table_state.selected(), None);
    assert_eq!(tab.insert_table.content_length, 0);
}

#[test]
fn filtered_edits_deletes_and_escape_use_model_indices() {
    let mut tab = tab(vec![command("alpha"), command("beta"), command("gamma")]);
    tab.handle_key_press(key(KeyCode::Char('/')));
    assert!(tab.has_control());
    for c in "beta".chars() {
        tab.handle_key_press(key(KeyCode::Char(c)));
    }
    assert_eq!(tab.selected_index(), Some(1));
    tab.handle_key_press(key(KeyCode::PageUp));
    assert_eq!(tab.menu.lock().unwrap().commands[0].keyword, "alpha");
    tab.handle_key_press(key(KeyCode::Enter));
    tab.handle_key_press(key(KeyCode::Char('X')));
    tab.handle_key_press(key(KeyCode::F(10)));
    assert_eq!(tab.menu.lock().unwrap().commands[1].display, "X");
    assert!(tab.has_control());
    tab.handle_key_press(key(KeyCode::Delete));
    assert_eq!(tab.menu.lock().unwrap().commands.len(), 2);
    assert_eq!(tab.insert_table.content_length, 0);
    tab.handle_key_press(key(KeyCode::Esc));
    assert!(!tab.has_control());
    assert_eq!(tab.insert_table.content_length, 2);
}

#[test]
fn list_and_modal_fit_or_degrade_safely_on_small_screens() {
    let mut tab = tab(vec![command("A")]);
    for (w, h) in [(1, 1), (5, 5), (40, 12), (80, 25)] {
        draw(&mut tab, w, h);
    }
    tab.handle_key_press(key(KeyCode::Enter));
    assert!(tab.has_control());
    tab.handle_key_press(key(KeyCode::Tab));
    draw(&mut tab, 80, 25);
    assert!(tab.has_control());
    tab.handle_key_press(key(KeyCode::Esc));
    assert!(!tab.has_control());
}
