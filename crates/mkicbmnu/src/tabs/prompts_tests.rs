use super::*;
use icy_board_engine::icy_board::language::Language;
use ratatui::{Terminal, backend::TestBackend};

fn board() -> Arc<Mutex<IcyBoard>> {
    let mut board = IcyBoard::default();
    *board.languages = vec![
        Language {
            description: "Deutsch".into(),
            extension: "deu".into(),
            locale: "de_DE".into(),
            yes_char: 'J',
            no_char: 'N',
        },
        Language {
            description: "Español".into(),
            extension: "spa".into(),
            locale: "es_ES".into(),
            yes_char: 'S',
            no_char: 'N',
        },
    ];
    Arc::new(Mutex::new(board))
}

fn setup() -> (Arc<Mutex<Menu>>, PromptsTab<'static>) {
    let mut menu = Menu::default();
    menu.title = "Unchanged".into();
    menu.prompt = "@X07 Command: ".into();
    menu.commands.push(Default::default());
    menu.prompts = vec![(".DEU".into(), "@X07 Auswahl: ".into()), (".SPA".into(), "Opción: ".into())];
    let menu = Arc::new(Mutex::new(menu));
    (menu.clone(), PromptsTab::new(board(), menu))
}

fn draw(tab: &mut PromptsTab<'_>, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| tab.render(frame, frame.area())).unwrap();
    terminal.backend().buffer().content.iter().map(|cell| cell.symbol()).collect()
}

fn apply(tab: &mut PromptsTab<'_>) {
    tab.handle_key_press(KeyCode::F(2).into());
}

#[test]
fn the_default_prompt_is_shown_so_suffixes_have_a_visible_meaning() {
    let (_, mut tab) = setup();
    let rendered = draw(&mut tab, 80, 25);
    assert!(rendered.contains(&get_text("mnu_prompts_default")), "{rendered}");
    assert!(rendered.contains("Command:"), "the suffix-less prompt itself: {rendered}");
    let empty = Arc::new(Mutex::new(Menu::default()));
    let mut tab = PromptsTab::new(board(), empty);
    let rendered = draw(&mut tab, 80, 25);
    assert!(rendered.contains(&get_text("mnu_prompts_default_empty")), "{rendered}");
    assert!(rendered.contains(&get_text("mnu_prompts_empty")), "{rendered}");
}

#[test]
fn known_suffixes_are_listed_as_board_languages_and_unknown_ones_stay_verbatim() {
    let (menu, mut tab) = setup();
    menu.lock().unwrap().prompts.push((".XYZ".into(), "Raw".into()));
    let rendered = draw(&mut tab, 80, 25);
    assert!(rendered.contains("Deutsch (.deu)"), "{rendered}");
    assert!(rendered.contains("Español (.spa)"), "{rendered}");
    assert!(rendered.contains(".XYZ"), "unknown suffixes are not invented away: {rendered}");
}

#[test]
fn a_draft_offers_the_board_languages_and_keeps_the_stored_suffix() {
    let (menu, mut tab) = setup();
    tab.handle_key_press(KeyCode::Enter.into());
    let draft = tab.draft.as_ref().unwrap();
    let ListValue::ComboBox(combo) = &draft.config.get_item(0).unwrap().value else {
        panic!("expected a language list");
    };
    assert_eq!(combo.cur_value.value, ".DEU", "the imported spelling is kept");
    assert_eq!(combo.cur_value.display, "Deutsch (.deu)", "but it is named like the board language");
    assert!(combo.values.iter().any(|choice| choice.value == "spa"));
    draw(&mut tab, 80, 25);
    let rendered = draw(&mut tab, 80, 25);
    assert!(
        rendered.contains(&get_text("mnu_prompts_language")) && rendered.contains(&get_text("mnu_prompts_text")),
        "{rendered}"
    );
    apply(&mut tab);
    assert!(!tab.has_control());
    // Applying without editing must not rewrite the imported ".DEU" key.
    assert_eq!(menu.lock().unwrap().prompts[0].0, ".DEU");
}

#[test]
fn free_text_entry_is_available_for_suffixes_outside_the_board_languages() {
    let (menu, mut tab) = setup();
    tab.handle_key_press(KeyCode::Insert.into());
    tab.handle_key_press(KeyCode::F(3).into());
    assert!(matches!(
        tab.draft.as_ref().unwrap().config.get_item(0).unwrap().value,
        ListValue::Text(_, _, _)
    ));
    for ch in ".FRE".chars() {
        tab.handle_key_press(KeyCode::Char(ch).into());
    }
    apply(&mut tab);
    assert_eq!(menu.lock().unwrap().prompts[2].0, ".FRE");
    assert!(!tab.has_control());
}

#[test]
fn duplicates_ignore_a_leading_dot_and_letter_case_like_the_language_list() {
    let (menu, mut tab) = setup();
    tab.handle_key_press(KeyCode::Insert.into());
    tab.handle_key_press(KeyCode::F(3).into());
    for ch in "deu".chars() {
        tab.handle_key_press(KeyCode::Char(ch).into());
    }
    apply(&mut tab);
    assert_eq!(tab.request_status().status_line, get_text("mnu_prompts_duplicate"));
    assert_eq!(menu.lock().unwrap().prompts.len(), 2);
    assert!(tab.has_control());
}

#[test]
fn empty_and_separator_suffixes_are_rejected_without_inserting_placeholders() {
    for text in ["", ".", "de u", "de,u"] {
        let (menu, mut tab) = setup();
        tab.handle_key_press(KeyCode::Insert.into());
        tab.handle_key_press(KeyCode::F(3).into());
        for ch in text.chars() {
            tab.handle_key_press(KeyCode::Char(ch).into());
        }
        apply(&mut tab);
        assert_eq!(tab.request_status().status_line, get_text("mnu_prompts_invalid_extension"), "{text}");
        assert_eq!(menu.lock().unwrap().prompts.len(), 2, "{text}");
    }
}

#[test]
fn terminal_reserved_control_enter_no_longer_applies_a_draft() {
    let (menu, mut tab) = setup();
    let original = menu.lock().unwrap().clone();
    tab.handle_key_press(KeyCode::Enter.into());
    tab.handle_key_press(KeyCode::Down.into());
    tab.handle_key_press(KeyCode::End.into());
    tab.handle_key_press(KeyCode::Char('!').into());
    tab.handle_key_press(KeyEvent::new(KeyCode::Enter, crossterm::event::KeyModifiers::CONTROL));
    assert!(tab.has_control());
    assert!(*menu.lock().unwrap() == original);
    tab.handle_key_press(KeyCode::F(10).into());
    assert!(!tab.has_control());
    assert!(menu.lock().unwrap().prompts[0].1.ends_with('!'));
}

#[test]
fn escape_closes_the_choice_list_before_the_draft() {
    let (menu, mut tab) = setup();
    let original = menu.lock().unwrap().clone();
    tab.handle_key_press(KeyCode::Enter.into());
    tab.handle_key_press(KeyCode::Enter.into());
    assert!(tab.combo_open());
    tab.handle_key_press(KeyCode::Esc.into());
    assert!(!tab.combo_open() && tab.has_control());
    tab.handle_key_press(KeyCode::Esc.into());
    assert!(!tab.has_control());
    assert!(*menu.lock().unwrap() == original);
}

#[test]
fn delete_asks_first_and_refuses_stale_confirmations() {
    let (menu, mut tab) = setup();
    tab.handle_key_press(KeyCode::Delete.into());
    assert!(tab.has_control());
    assert_eq!(menu.lock().unwrap().prompts.len(), 2);
    menu.lock().unwrap().prompts.push((".FRE".into(), "Choix: ".into()));
    tab.handle_key_press(KeyCode::Enter.into());
    assert_eq!(menu.lock().unwrap().prompts.len(), 3);
    assert_eq!(tab.request_status().status_line, get_text("mnu_prompts_conflict"));
    tab.handle_key_press(KeyCode::Delete.into());
    tab.handle_key_press(KeyCode::Enter.into());
    assert_eq!(menu.lock().unwrap().prompts.len(), 2);
    assert!(!tab.has_control());
}

#[test]
fn drafts_refuse_to_overwrite_prompts_changed_elsewhere() {
    let (menu, mut tab) = setup();
    tab.handle_key_press(KeyCode::Enter.into());
    menu.lock().unwrap().prompts.remove(1);
    apply(&mut tab);
    assert_eq!(tab.request_status().status_line, get_text("mnu_prompts_conflict"));
    assert!(tab.has_control());
    assert_eq!(menu.lock().unwrap().prompts.len(), 1);
}

#[test]
fn undo_style_replacement_of_the_menu_is_reflected_without_losing_the_selection() {
    let (menu, mut tab) = setup();
    tab.handle_key_press(KeyCode::Down.into());
    assert_eq!(tab.table.table_state.selected(), Some(1));
    menu.lock().unwrap().prompts.truncate(1);
    tab.refresh_from_menu();
    assert_eq!(tab.table.table_state.selected(), Some(0));
    menu.lock().unwrap().prompts.clear();
    tab.refresh_from_menu();
    assert_eq!(tab.table.table_state.selected(), None);
    assert!(!tab.is_dirty() == false);
}

#[test]
fn list_and_modals_render_at_small_sizes_and_80x25_without_mutation() {
    let (menu, mut tab) = setup();
    let original = menu.lock().unwrap().clone();
    for (width, height) in [(0, 0), (1, 1), (12, 4), (30, 8), (80, 25)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| tab.render(frame, frame.area())).unwrap();
        if width == 80 {
            let rendered = terminal.backend().buffer().content.iter().map(|cell| cell.symbol()).collect::<String>();
            assert!(rendered.contains("Deutsch") && rendered.contains("Auswahl"));
        }
        tab.handle_key_press(KeyCode::Enter.into());
        terminal.draw(|frame| tab.render(frame, frame.area())).unwrap();
        tab.handle_key_press(KeyCode::Esc.into());
        tab.handle_key_press(KeyCode::Delete.into());
        terminal.draw(|frame| tab.render(frame, frame.area())).unwrap();
        tab.handle_key_press(KeyCode::Esc.into());
    }
    assert!(*menu.lock().unwrap() == original);
}

#[test]
fn the_draft_advertises_function_keys_and_never_control_enter() {
    let (_, mut tab) = setup();
    tab.handle_key_press(KeyCode::Enter.into());
    let rendered = draw(&mut tab, 80, 25);
    assert!(rendered.contains("F2"), "{rendered}");
    assert!(!rendered.contains("Ctrl+"), "{rendered}");
}
