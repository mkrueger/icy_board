use super::*;
use ratatui::{Terminal, backend::TestBackend};
use std::{collections::BTreeMap, path::PathBuf};

fn board() -> Board {
    let mut board = IcyBoard::default();
    board.config.paths.zconnect_file.clear();
    Arc::new(Mutex::new(board))
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn render(page: &mut dyn Page) -> Vec<String> {
    let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
    terminal.draw(|frame| page.render(frame, Rect::new(0, 1, 80, 23))).unwrap();
    let buffer = terminal.backend().buffer();
    (0..25).map(|y| (0..80).map(|x| buffer[(x, y)].symbol()).collect()).collect()
}

fn press(page: &mut dyn Page, code: KeyCode) -> PageMessage {
    let message = page.handle_key_press(key(code));
    render(page); // ConfigMenu applies non-text updates during rendering.
    message
}

fn subpage(message: PageMessage) -> Box<dyn Page> {
    let PageMessage::OpenSubPage(page) = message else {
        panic!("expected a subpage")
    };
    page
}

fn set_text(form: &mut ZconnectForm, index: usize, value: &str) {
    form.state.selected = index;
    render(form);
    let length = match &form.menu.get_item(index).unwrap().value {
        ListValue::Text(_, _, text) => text.chars().count(),
        ListValue::Path(path) => path.to_string_lossy().chars().count(),
        _ => panic!("not a text field"),
    };
    press(form, KeyCode::End);
    for _ in 0..length {
        press(form, KeyCode::Backspace);
    }
    for ch in value.chars() {
        press(form, KeyCode::Char(ch));
    }
}

fn update(form: &ZconnectForm, index: usize, value: ListValue) {
    form.menu.get_item(index).unwrap().update_value.as_ref().unwrap()(&form.menu.obj, &value);
}

#[test]
fn zconnect_browser_routes_before_text_and_f2_and_selection_updates_without_rendering() {
    let root = tempfile::tempdir().unwrap();
    let selected = root.path().join("chosen.dat");
    std::fs::write(&selected, b"chosen").unwrap();
    let board = board();
    {
        let mut b = board.lock().unwrap();
        b.root_path = root.path().to_path_buf();
        b.file_name = root.path().join("configuration/icyboard.toml");
        b.zconnect.inbound.clear();
    }
    let mut form = ZconnectForm::general(board.clone());
    form.state.selected = 4;
    // Exercise the common form's F2 interception as well as its path editor.
    form.link = Some(0);
    assert_eq!(form.state.path_base.as_deref(), Some(root.path()));
    render(&mut form);
    form.handle_key_press(key(KeyCode::F(4)));
    assert!(form.state.is_path_browser_open());
    for code in [KeyCode::Char('x'), KeyCode::Delete, KeyCode::F(2)] {
        assert!(matches!(form.handle_key_press(key(code)), PageMessage::ResultState(_)));
        assert!(form.state.is_path_browser_open());
        assert!(board.lock().unwrap().zconnect.inbound.as_os_str().is_empty());
    }

    // Compare with the shared modal's render: the local text repaint must not
    // erase its cells or relocate its cursor to the underlying path field.
    let mut actual = Terminal::new(TestBackend::new(80, 25)).unwrap();
    actual.draw(|frame| form.render(frame, Rect::new(0, 1, 80, 23))).unwrap();
    let mut expected = Terminal::new(TestBackend::new(80, 25)).unwrap();
    expected
        .draw(|frame| {
            let area = panel(frame, Rect::new(0, 1, 80, 23), form.title, "zconnect_link_keys");
            form.menu.render(area, frame, &mut form.state);
        })
        .unwrap();
    assert_eq!(actual.backend().buffer(), expected.backend().buffer());
    assert_eq!(actual.get_cursor_position().unwrap(), expected.get_cursor_position().unwrap());

    form.handle_key_press(key(KeyCode::End));
    form.handle_key_press(key(KeyCode::Enter));
    assert!(!form.state.is_path_browser_open());
    let value = board.lock().unwrap().zconnect.inbound.clone();
    assert_eq!(root.path().join(&value), selected);
    assert!(matches!(&form.menu.get_item(4).unwrap().value, ListValue::Path(path) if path == &value));
    assert_eq!(board.lock().unwrap().config.paths.zconnect_file, PathBuf::from("zconnect.toml"));

    form.handle_key_press(key(KeyCode::F(4)));
    assert!(matches!(form.handle_key_press(key(KeyCode::Esc)), PageMessage::ResultState(_)));
    assert!(!form.state.is_path_browser_open());
    assert_eq!(board.lock().unwrap().zconnect.inbound, value);
    assert!(matches!(form.handle_key_press(key(KeyCode::F(2))), PageMessage::OpenSubPage(_)));
    assert!(matches!(form.handle_key_press(key(KeyCode::Esc)), PageMessage::Close));
}

#[test]
fn zconnect_has_a_separate_menu_and_viewing_does_not_enable_saving() {
    let board = board();
    let initial = board.lock().unwrap().zconnect.clone();
    let mut menu = super::super::MsgNetworking::new(board.clone());
    let rows = render(&mut menu).join("\n");
    for id in ["msg_networking_qwk", "msg_networking_ftn", "msg_networking_zconnect"] {
        assert!(rows.contains(&get_text(id)), "missing menu: {id}");
    }
    let mut zconnect = subpage(menu.handle_key_press(key(KeyCode::Char('c'))));
    let rows = render(zconnect.as_mut()).join("\n");
    assert!(rows.contains(&get_text("zconnect_general")));
    assert!(rows.contains(&get_text("zconnect_links")));
    let mut general = subpage(zconnect.handle_key_press(key(KeyCode::Char('a'))));
    render(general.as_mut());
    let mut links = subpage(zconnect.handle_key_press(key(KeyCode::Char('b'))));
    render(links.as_mut());
    let board = board.lock().unwrap();
    assert_eq!(board.zconnect, initial);
    assert!(board.config.paths.zconnect_file.as_os_str().is_empty());
}

#[test]
fn zconnect_general_edits_persist_in_memory_and_choose_a_save_path() {
    let board = board();
    let mut form = ZconnectForm::general(board.clone());
    render(&mut form);
    press(&mut form, KeyCode::Right);
    assert!(board.lock().unwrap().zconnect.enabled);
    assert_eq!(board.lock().unwrap().config.paths.zconnect_file, PathBuf::from("zconnect.toml"));
    set_text(&mut form, 1, "bbs.example.org");
    set_text(&mut form, 2, "local-sysop");
    set_text(&mut form, 3, "net/zconnect.toml");
    set_text(&mut form, 4, "net/inbound");
    set_text(&mut form, 5, "net/outbound");
    assert!(matches!(form.handle_key_press(key(KeyCode::Esc)), PageMessage::Close));
    {
        let b = board.lock().unwrap();
        assert_eq!(b.zconnect.local_system, "bbs.example.org");
        assert_eq!(b.zconnect.local_user, "local-sysop");
        assert_eq!(b.zconnect.inbound, PathBuf::from("net/inbound"));
        assert_eq!(b.zconnect.outbound, PathBuf::from("net/outbound"));
        assert_eq!(b.config.paths.zconnect_file, PathBuf::from("net/zconnect.toml"));
    }
    let mut reopened = ZconnectForm::general(board.clone());
    assert!(matches!(&reopened.menu.get_item(1).unwrap().value, ListValue::Text(_, _, v) if v == "bbs.example.org"));
    set_text(&mut reopened, 3, "");
    assert_eq!(board.lock().unwrap().config.paths.zconnect_file, PathBuf::from("zconnect.toml"));
}

#[test]
fn zconnect_link_and_area_crud_persists_every_field() {
    let board = board();
    let mut links = ZconnectList::new(board.clone(), ListKind::Links);
    let mut editor = subpage(links.handle_key_press(key(KeyCode::Insert)));
    render(editor.as_mut());
    assert_eq!(board.lock().unwrap().config.paths.zconnect_file, PathBuf::from("zconnect.toml"));
    {
        let b = board.lock().unwrap();
        assert_eq!(b.zconnect.links[0].port, 23);
        assert_eq!(b.zconnect.links[0].timeout_secs, 60);
        assert_eq!(b.zconnect.links[0].login, "zconnect");
        assert!(b.zconnect.links[0].remote_system.is_empty());
    }
    let mut form = ZconnectForm::link(board.clone(), 0).unwrap();
    for (index, value) in [
        (0, "peer"),
        (1, "remote.example.org"),
        (3, "remote-sysop"),
        (4, "secret-pass"),
        (7, "The Remote BBS"),
    ] {
        set_text(&mut form, index, value);
    }
    update(&form, 2, ListValue::U32(2323, 1, 65535));
    update(&form, 6, ListValue::U32(120, 1, 3600));
    form.state.selected = 5;
    render(&mut form);
    press(&mut form, KeyCode::Enter);
    press(&mut form, KeyCode::Down);
    press(&mut form, KeyCode::Enter);
    assert_eq!(board.lock().unwrap().zconnect.links[0].login, "janus");
    let mut areas = subpage(form.handle_key_press(key(KeyCode::F(2))));
    let mut mapping = subpage(areas.handle_key_press(key(KeyCode::Insert)));
    render(mapping.as_mut());
    let mut area = ZconnectForm::area(board.clone(), 0, 0).unwrap();
    set_text(&mut area, 0, "GENERAL");
    set_text(&mut area, 1, "messages/general");
    area.state.selected = 2;
    press(&mut area, KeyCode::Right);
    {
        let b = board.lock().unwrap();
        let l = &b.zconnect.links[0];
        assert_eq!((&*l.id, &*l.host, l.port), ("peer", "remote.example.org", 2323));
        assert_eq!((&*l.username, &*l.password, l.timeout_secs), ("remote-sysop", "secret-pass", 120));
        assert_eq!(l.remote_system, "The Remote BBS");
        assert_eq!(l.areas[0].remote_board, "GENERAL");
        assert_eq!(l.areas[0].local_area, PathBuf::from("messages/general"));
        assert!(l.areas[0].read_only);
    }
    let mut reopened = ZconnectForm::area(board.clone(), 0, 0).unwrap();
    assert!(render(&mut reopened).join("\n").contains("messages/general"));
    areas.handle_key_press(key(KeyCode::Delete));
    assert!(board.lock().unwrap().zconnect.links[0].areas.is_empty());
    links.handle_key_press(key(KeyCode::Delete));
    assert!(board.lock().unwrap().zconnect.links.is_empty());
    assert_eq!(links.table.table_state.selected(), None);
}

#[test]
fn zconnect_invalid_values_and_stale_or_empty_selections_do_not_panic() {
    let board = board();
    board.lock().unwrap().zconnect.links.push(ZconnectLink::default());
    let form = ZconnectForm::link(board.clone(), 0).unwrap();
    for value in [0, 65536, u32::MAX] {
        update(&form, 2, ListValue::U32(value, 0, u32::MAX));
    }
    update(&form, 2, text("not a number"));
    for value in [0, 3601, u32::MAX] {
        update(&form, 6, ListValue::U32(value, 0, u32::MAX));
    }
    update(
        &form,
        5,
        ListValue::ComboBox(ComboBox {
            is_edit_open: false,
            values: vec![],
            cur_value: ComboBoxValue::new("invalid", "invalid"),
            selected_item: 0,
            first_item: 0,
        }),
    );
    {
        let b = board.lock().unwrap();
        assert_eq!(b.zconnect.links[0].port, 23);
        assert_eq!(b.zconnect.links[0].timeout_secs, 60);
        assert_eq!(b.zconnect.links[0].login, "zconnect");
        assert!(b.config.paths.zconnect_file.as_os_str().is_empty());
    }
    assert!(ZconnectForm::link(board.clone(), usize::MAX).is_none());
    assert!(ZconnectForm::area(board.clone(), 0, usize::MAX).is_none());
    board.lock().unwrap().zconnect.links.clear();
    update(&form, 0, text("stale editor"));
    for kind in [ListKind::Links, ListKind::Areas(usize::MAX)] {
        let mut list = ZconnectList::new(board.clone(), kind);
        for code in [
            KeyCode::Delete,
            KeyCode::Enter,
            KeyCode::F(2),
            KeyCode::End,
            KeyCode::PageDown,
            KeyCode::Up,
            KeyCode::Delete,
        ] {
            list.handle_key_press(key(code));
            render(&mut list);
        }
    }
    assert!(board.lock().unwrap().config.paths.zconnect_file.as_os_str().is_empty());
}

#[test]
fn zconnect_numeric_keyboard_input_is_bounded_and_does_not_panic() {
    let board = board();
    board.lock().unwrap().zconnect.links.push(ZconnectLink::default());
    let mut form = ZconnectForm::link(board.clone(), 0).unwrap();
    for index in [2, 6] {
        form.state.selected = index;
        render(&mut form);
        press(&mut form, KeyCode::End);
        for _ in 0..20 {
            press(&mut form, KeyCode::Char('9'));
        }
        for ch in ['x', '-', 'é'] {
            press(&mut form, KeyCode::Char(ch));
        }
        press(&mut form, KeyCode::Home);
        press(&mut form, KeyCode::Delete);
    }
    let b = board.lock().unwrap();
    assert!(b.zconnect.links[0].port > 0);
    assert!((1..=3600).contains(&b.zconnect.links[0].timeout_secs));
}

#[test]
fn zconnect_expected_sys_can_be_cleared_and_timeout_accepts_endpoints() {
    let board = board();
    board.lock().unwrap().zconnect.links.push(ZconnectLink::default());
    let mut form = ZconnectForm::link(board.clone(), 0).unwrap();
    assert!(matches!(form.menu.get_item(6).unwrap().value, ListValue::U32(_, 1, 3600)));
    for seconds in [1, 3600] {
        update(&form, 6, ListValue::U32(seconds, 1, 3600));
        assert_eq!(board.lock().unwrap().zconnect.links[0].timeout_secs, seconds);
    }
    set_text(&mut form, 7, "The Remote BBS");
    let mut reopened = ZconnectForm::link(board.clone(), 0).unwrap();
    assert!(matches!(&reopened.menu.get_item(7).unwrap().value, ListValue::Text(255, _, v) if v == "The Remote BBS"));
    set_text(&mut reopened, 7, "");
    assert!(board.lock().unwrap().zconnect.links[0].remote_system.is_empty());
}

#[test]
fn zconnect_unicode_edits_and_password_rendering_are_safe() {
    let board = board();
    board.lock().unwrap().zconnect.links.push(ZconnectLink::default());
    let mut form = ZconnectForm::link(board.clone(), 0).unwrap();
    set_text(&mut form, 4, "sëcret🔑");
    let rows = render(&mut form).join("\n");
    assert!(!rows.contains("sëcret"));
    assert!(!rows.contains('🔑'));
    assert!(rows.contains("*******"));
    press(&mut form, KeyCode::Left);
    press(&mut form, KeyCode::Delete);
    press(&mut form, KeyCode::Home);
    press(&mut form, KeyCode::Right);
    press(&mut form, KeyCode::Delete);
    assert_eq!(board.lock().unwrap().zconnect.links[0].password, "scret");
    form.state.selected = 0;
    assert!(!render(&mut form).join("\n").contains("scret"));

    let mut general = ZconnectForm::general(board.clone());
    set_text(&mut general, 4, "nachrichten/ä/日本");
    press(&mut general, KeyCode::End);
    press(&mut general, KeyCode::Backspace);
    assert_eq!(board.lock().unwrap().zconnect.inbound, PathBuf::from("nachrichten/ä/日"));
    press(&mut general, KeyCode::Char('\0'));
    assert_eq!(board.lock().unwrap().zconnect.inbound, PathBuf::from("nachrichten/ä/日"));
}

#[test]
fn zconnect_viewing_login_profile_does_not_dirty_the_board() {
    let board = board();
    board.lock().unwrap().zconnect.links.push(ZconnectLink::default());
    let mut form = ZconnectForm::link(board.clone(), 0).unwrap();
    form.state.selected = 5;
    render(&mut form);
    press(&mut form, KeyCode::Enter);
    press(&mut form, KeyCode::Down);
    press(&mut form, KeyCode::Esc);
    let b = board.lock().unwrap();
    assert!(b.config.paths.zconnect_file.as_os_str().is_empty());
    assert_eq!(b.zconnect.links[0].login, "zconnect");
}

#[test]
fn zconnect_forms_fit_80x25_in_the_selected_locale() {
    // Run this test once with LANG=en_US.UTF-8 and once with LANG=de_DE.UTF-8.
    // Do not mutate the global language loader while other UI tests are running.
    let board = board();
    board.lock().unwrap().zconnect.links.push(ZconnectLink {
        id: "peer".into(),
        password: "secret-password".into(),
        areas: vec![ZconnectArea::default()],
        ..Default::default()
    });
    let pages = [
        (
            ZconnectForm::general(board.clone()),
            vec![
                "zconnect_enabled",
                "zconnect_local_system",
                "zconnect_local_user",
                "zconnect_config_file",
                "zconnect_inbound",
                "zconnect_outbound",
            ],
        ),
        (
            ZconnectForm::link(board.clone(), 0).unwrap(),
            vec![
                "zconnect_id",
                "zconnect_host",
                "zconnect_port",
                "zconnect_username",
                "zconnect_password",
                "zconnect_login",
                "zconnect_timeout",
                "zconnect_remote_system",
            ],
        ),
        (
            ZconnectForm::area(board.clone(), 0, 0).unwrap(),
            vec!["zconnect_remote_board", "zconnect_local_area", "zconnect_read_only"],
        ),
    ];
    for (mut page, labels) in pages {
        for index in 0..labels.len() {
            page.state.selected = index;
            let rows = render(&mut page);
            for label in &labels {
                let label = get_text(label);
                let row = rows.iter().find(|row| row.contains(&label)).unwrap_or_else(|| panic!("clipped label: {label}"));
                assert!(row.split_once(&label).unwrap().1.trim_start().starts_with(':'));
            }
            for row in rows.iter().take(23).skip(2) {
                assert!(row.ends_with(icy_board_tui::BORDER_SET.vertical_right), "overwritten border: {row}");
            }
            assert!(!rows.join("\n").contains("secret-password"));
        }
    }
    for kind in [ListKind::Links, ListKind::Areas(0)] {
        let mut page = ZconnectList::new(board.clone(), kind);
        let rows = render(&mut page).join("\n");
        let labels = match kind {
            ListKind::Links => ["zconnect_id", "zconnect_host", "zconnect_areas", "zconnect_links_keys"],
            ListKind::Areas(_) => ["zconnect_remote_board", "zconnect_local_area", "zconnect_read_only", "zconnect_areas_keys"],
        };
        for label in labels {
            assert!(rows.contains(&crate::editors::hint_text(label)), "clipped list label: {label}");
        }
    }
}

#[test]
fn zconnect_both_catalogs_have_complete_help_and_compact_navigation() {
    let catalogs = [
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../icy_board_tui/i18n/en/icy_board_tui.ftl")),
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../icy_board_tui/i18n/de/icy_board_tui.ftl")),
    ];
    let maps: Vec<BTreeMap<&str, &str>> = catalogs
        .iter()
        .map(|source| {
            let mut map = BTreeMap::new();
            for line in source.lines() {
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    if key.starts_with("zconnect_") || key == "msg_networking_zconnect" {
                        assert!(map.insert(key, value.trim()).is_none(), "duplicate {key}");
                        assert!(!value.trim().is_empty());
                        if key.ends_with("_keys") {
                            assert!(Line::raw(value.trim()).width() <= 76, "navigation too long: {key}");
                        }
                    }
                }
            }
            map
        })
        .collect();
    assert_eq!(maps[0].keys().collect::<Vec<_>>(), maps[1].keys().collect::<Vec<_>>());
    for map in maps {
        for key in [
            "enabled",
            "local_system",
            "local_user",
            "config_file",
            "inbound",
            "outbound",
            "id",
            "host",
            "port",
            "username",
            "password",
            "login",
            "timeout",
            "remote_system",
            "remote_board",
            "local_area",
            "read_only",
        ] {
            let key = format!("zconnect_{key}");
            assert!(Line::raw(map[key.as_str()]).width() <= 22, "label too wide: {key}");
            assert!(map.contains_key(format!("{key}-status").as_str()));
            assert!(map.contains_key(format!("{key}-help").as_str()));
        }
    }
}
