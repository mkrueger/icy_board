use super::*;
use ratatui::{Terminal, backend::TestBackend};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn dialog(command: Command) -> EditCommandDialog<'static> {
    EditCommandDialog::new(Arc::new(Mutex::new(IcyBoard::default())), Arc::new(Mutex::new(Menu::default())), command, 1)
}

fn draw(dialog: &mut EditCommandDialog<'_>, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| dialog.ui(frame, frame.area())).unwrap();
    terminal.backend().buffer().content.iter().map(|c| c.symbol()).collect()
}

fn set_text(dialog: &mut EditCommandDialog<'_>, index: usize, text: &str) {
    dialog.config.get_item_mut(index).unwrap().value = ListValue::Text(28, TextFlags::None, text.into());
}

#[test]
fn autorun_choices_localize_display_but_preserve_parseable_values() {
    for (auto_run, key) in AutoRun::iter().zip([
        "mnu_editor_autorun_disabled",
        "mnu_editor_autorun_first",
        "mnu_editor_autorun_every",
        "mnu_editor_autorun_after",
        "mnu_editor_autorun_loop",
    ]) {
        let value = auto_run_value(&auto_run);
        assert_eq!(value.display, get_text(key));
        assert_eq!(AutoRun::from_str(&value.value).unwrap(), auto_run);
    }
}

#[test]
fn rendering_table_focus_and_stale_selection_never_looks_up_a_sentinel() {
    let mut d = dialog(Command::default());
    for selected in [0, POSITION, usize::MAX] {
        d.state.selected = selected;
        d.insert_table.table_state.select(Some(usize::MAX));
        d.mode = EditCommandMode::Table;
        draw(&mut d, 80, 25);
        assert!(d.state.selected < d.config.count());
        assert_eq!(d.insert_table.table_state.selected(), None);
        d.mode = EditCommandMode::Config;
        draw(&mut d, 80, 25);
    }
}

#[test]
fn small_screens_and_all_field_selections_are_safe() {
    let mut d = dialog(Command::default());
    for (w, h) in [(1, 1), (2, 2), (20, 5), (60, 15), (80, 25)] {
        for i in 0..d.config.count() {
            d.state.selected = i;
            draw(&mut d, w, h);
        }
    }
}

#[test]
fn immediate_f10_flushes_text_without_requiring_a_render() {
    let mut d = dialog(Command::default());
    d.handle_key_press(key(KeyCode::Char('X')));
    assert_eq!(d.handle_key_press(key(KeyCode::F(10))), DialogResult::Accepted);
    assert_eq!(d.command.lock().unwrap().display, "X");
}

#[test]
fn charges_reject_invalid_nonfinite_negative_and_overflow_values() {
    for bad in ["", "-1", "NaN", "inf", "-inf", "1e999", "junk", "1,5"] {
        for index in [CHARGE_USE, CHARGE_MINUTE] {
            let mut d = dialog(Command::default());
            set_text(&mut d, index, bad);
            assert_eq!(d.handle_key_press(key(KeyCode::F(10))), DialogResult::Pending, "{bad}");
            assert_eq!(d.state.selected, index);
            assert!(!d.error.is_empty());
            assert_eq!(d.command.lock().unwrap().charge_per_use, 0.0);
        }
    }
    let mut d = dialog(Command::default());
    set_text(&mut d, CHARGE_USE, "2.75");
    set_text(&mut d, CHARGE_MINUTE, "0.125");
    assert_eq!(d.handle_key_press(key(KeyCode::F(10))), DialogResult::Accepted);
    assert_eq!(d.command.lock().unwrap().charge_per_use, 2.75);
    assert_eq!(d.command.lock().unwrap().charge_per_minute, 0.125);
}

#[test]
fn imported_large_timer_is_not_truncated() {
    let mut d = dialog(Command {
        autorun_time: u64::MAX,
        ..Default::default()
    });
    assert_eq!(d.handle_key_press(key(KeyCode::F(10))), DialogResult::Accepted);
    assert_eq!(d.command.lock().unwrap().autorun_time, u64::MAX);
}

#[test]
fn combo_and_path_browser_receive_escape_before_outer_dialog() {
    let mut d = dialog(Command::default());
    d.state.selected = 4;
    d.handle_key_press(key(KeyCode::Enter));
    assert!(nested(&d.config, &d.state));
    assert_eq!(d.handle_key_press(key(KeyCode::Tab)), DialogResult::Pending);
    assert_eq!(d.mode, EditCommandMode::Config);
    assert_eq!(d.handle_key_press(key(KeyCode::Esc)), DialogResult::Pending);
    assert!(!nested(&d.config, &d.state));
    d.state.selected = 6;
    assert!(matches!(d.config.get_item(6).unwrap().value, ListValue::Path(_)));
    d.handle_key_press(key(KeyCode::F(4)));
    assert!(d.state.is_path_browser_open());
    draw(&mut d, 80, 25);
    assert_eq!(d.handle_key_press(key(KeyCode::Esc)), DialogResult::Pending);
    assert!(!d.state.is_path_browser_open());
    assert_eq!(d.handle_key_press(key(KeyCode::Esc)), DialogResult::Cancelled);
}

#[test]
fn action_drafts_cancel_commit_insert_after_selection_and_reorder() {
    let mut d = dialog(Command::default());
    d.handle_key_press(key(KeyCode::Tab));
    d.handle_key_press(key(KeyCode::Insert));
    assert!(d.command.lock().unwrap().actions.is_empty());
    d.handle_key_press(key(KeyCode::Esc));
    assert!(d.command.lock().unwrap().actions.is_empty());
    for shortcut in ['m', 'p', 'q'] {
        d.handle_key_press(key(KeyCode::Char(shortcut)));
        d.handle_key_press(key(KeyCode::F(10)));
    }
    assert_eq!(d.command.lock().unwrap().actions.len(), 3);
    assert_eq!(d.insert_table.table_state.selected(), Some(2));
    d.handle_key_press(key(KeyCode::Char('1')));
    assert_eq!(d.command.lock().unwrap().actions[1].command_type, CommandType::QuitMenu);
    d.handle_key_press(key(KeyCode::Char('2')));
    assert_eq!(d.command.lock().unwrap().actions[2].command_type, CommandType::QuitMenu);
    d.insert_table.table_state.select(Some(0));
    d.handle_key_press(key(KeyCode::Char('d')));
    d.handle_key_press(key(KeyCode::F(10)));
    assert_eq!(d.command.lock().unwrap().actions[1].command_type, CommandType::Door);
    d.handle_key_press(key(KeyCode::Enter));
    let editor = d.action_editor.as_mut().unwrap();
    editor.config.get_item_mut(1).unwrap().value = ListValue::Text(42, TextFlags::None, "cancelled".into());
    flush(&editor.config);
    d.handle_key_press(key(KeyCode::Esc));
    assert!(d.command.lock().unwrap().actions[1].parameter.is_empty());
    for _ in 0..6 {
        d.insert_table.table_state.select(Some(usize::MAX));
        d.handle_key_press(key(KeyCode::Delete));
        draw(&mut d, 80, 25);
    }
    assert_eq!(d.insert_table.content_length, 0);
    assert_eq!(d.insert_table.table_state.selected(), None);
}

#[test]
fn action_type_switch_builds_path_control_and_raw_toggle_preserves_value() {
    let mut board = IcyBoard::default();
    board.root_path = std::env::temp_dir();
    let mut editor = ActionEditor::new(CommandAction::default(), None, &board);
    editor.handle(key(KeyCode::Enter), &board);
    for c in "DisplayFile".chars() {
        editor.handle(key(KeyCode::Char(c)), &board);
    }
    editor.handle(key(KeyCode::Enter), &board);
    assert_eq!(editor.draft.lock().unwrap().command_type, CommandType::DisplayFile);
    assert_eq!(editor.state.selected, 1);
    assert!(matches!(editor.config.get_item(1).unwrap().value, ListValue::Path(_)));
    assert_eq!(editor.state.path_base.as_ref(), Some(&board.root_path));
    editor.handle(key(KeyCode::Char('a')), &board);
    editor.handle(key(KeyCode::F(2)), &board);
    assert!(matches!(editor.config.get_item(1).unwrap().value, ListValue::Text(_, _, _)));
    assert_eq!(editor.draft.lock().unwrap().parameter, "a");
    editor.handle(key(KeyCode::F(2)), &board);
    editor.handle(key(KeyCode::F(4)), &board);
    assert!(editor.state.is_path_browser_open());
    assert_eq!(editor.handle(key(KeyCode::Esc), &board), DialogResult::Pending);
    assert_eq!(editor.handle(key(KeyCode::Esc), &board), DialogResult::Cancelled);
}

#[test]
fn empty_type_search_enter_and_escape_are_safe() {
    let board = IcyBoard::default();
    let mut editor = ActionEditor::new(CommandAction::default(), None, &board);
    editor.handle(key(KeyCode::Enter), &board);
    for c in "zzzz-no-type".chars() {
        editor.handle(key(KeyCode::Char(c)), &board);
    }
    assert_eq!(editor.handle(key(KeyCode::Enter), &board), DialogResult::Pending);
    assert_eq!(editor.handle(key(KeyCode::Esc), &board), DialogResult::Pending);
    assert_eq!(editor.handle(key(KeyCode::Esc), &board), DialogResult::Cancelled);
}

#[test]
fn path_classification_follows_runtime_not_obsolete_enum_comments() {
    assert!(file_parameter(CommandType::StuffFile));
    assert!(!file_parameter(CommandType::StuffTextAndExitMenu));
    assert!(!file_parameter(CommandType::Script));
}

#[test]
fn every_advertised_type_roundtrips_even_when_engine_from_str_is_incomplete() {
    for choice in type_choices("") {
        assert!(choice_type(&choice.value).is_some(), "{}", choice.value);
    }
    assert_eq!(choice_type("FlagFiles"), Some(CommandType::FlagFiles));
}

#[test]
fn reference_choices_use_zero_based_conferences_and_one_based_local_ids() {
    use icy_board_engine::icy_board::{
        conferences::Conference,
        doors::{Door, DoorList},
        file_directory::{DirectoryList, FileDirectory},
    };
    let mut board = IcyBoard::default();
    board.conferences.clear();
    let mut dirs = DirectoryList::default();
    dirs.push(FileDirectory {
        name: "Files".into(),
        ..Default::default()
    });
    board.conferences.push(Conference {
        name: "Main".into(),
        doors: Some(Arc::new(DoorList {
            doors: vec![Door {
                name: "Game".into(),
                ..Default::default()
            }],
            ..Default::default()
        })),
        directories: Some(Arc::new(dirs)),
        ..Default::default()
    });
    assert_eq!(parameter_choices(&board, CommandType::Conference)[0].value, "0");
    assert_eq!(parameter_choices(&board, CommandType::Door)[0].value, "1");
    assert_eq!(parameter_choices(&board, CommandType::DisplayDir)[0].value, "1");
    let editor = ActionEditor::new(
        CommandAction {
            command_type: CommandType::Door,
            parameter: "Game".into(),
            ..Default::default()
        },
        None,
        &board,
    );
    assert!(matches!(editor.config.get_item(1).unwrap().value, ListValue::ComboBox(_)));
    assert_eq!(editor.draft.lock().unwrap().parameter, "Game");
    board.conferences.push(Conference {
        name: "Other".into(),
        ..Default::default()
    });
    assert!(parameter_choices(&board, CommandType::Door).is_empty());
    assert!(parameter_choices(&board, CommandType::DisplayDir).is_empty());
    assert_eq!(parameter_choices(&board, CommandType::Conference)[1].value, "1");
}

#[test]
fn executable_display_file_is_rejected_by_the_static_loader() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("menu.ppe");
    std::fs::write(&path, b"not executable in a static preview").unwrap();
    let menu = Arc::new(Mutex::new(Menu {
        display_file: path.clone(),
        ..Default::default()
    }));
    let mut d = EditCommandDialog::new(Arc::new(Mutex::new(IcyBoard::default())), menu, Command::default(), 1);
    std::fs::remove_file(path).unwrap();
    assert!(d.preview_error.contains(&get_text("mnu_preview_format")));
    d.position_edit = Some(Position::default());
    assert!(draw(&mut d, 80, 25).contains(&get_text("mnu_work_preview_error")));
    assert_eq!(d.preview.width(), 80);
    assert_eq!(d.preview.height(), 25);
}

#[test]
fn preview_identifies_index_renders_draft_and_handles_oversized_positions() {
    let board = Arc::new(Mutex::new(IcyBoard::default()));
    let original = Command {
        display: "SAME".into(),
        ..Default::default()
    };
    let other = Command {
        display: "SAME".into(),
        position: Position { x: 0, y: 2 },
        ..Default::default()
    };
    let menu = Arc::new(Mutex::new(Menu {
        commands: vec![original.clone(), other],
        ..Default::default()
    }));
    let mut d = EditCommandDialog::new(board, menu, original, 1);
    d.command.lock().unwrap().display = "DRAFT".into();
    d.command.lock().unwrap().lighbar_display = "HIGHLIGHT".into();
    d.position_edit = Some(Position { x: 2, y: 1 });
    let rendered = draw(&mut d, 80, 25);
    assert!(rendered.contains("DRAFT"));
    assert_eq!(rendered.matches("SAME").count(), 1);
    d.handle_key_press(key(KeyCode::F(6)));
    assert!(draw(&mut d, 80, 25).contains("HIGHLIGHT"));
    d.position_edit = Some(Position { x: u16::MAX, y: u16::MAX });
    for (w, h) in [(1, 1), (4, 4), (30, 10), (80, 25)] {
        draw(&mut d, w, h);
    }
    d.handle_key_press(key(KeyCode::Right));
    d.handle_key_press(key(KeyCode::Down));
    assert_eq!(d.handle_key_press(key(KeyCode::Esc)), DialogResult::Pending);
    assert!(d.command.lock().unwrap().position.is_default());
    d.position_edit = Some(Position { x: 4, y: 3 });
    d.handle_key_press(key(KeyCode::F(10)));
    assert_eq!(field_text(&d.config, POSITION), "4,3");
    assert_eq!(d.handle_key_press(key(KeyCode::F(10))), DialogResult::Accepted);
}

#[test]
fn unreadable_display_file_has_visible_warning_and_blank_preview() {
    let menu = Arc::new(Mutex::new(Menu {
        display_file: PathBuf::from("/nonexistent/mkicbmnu-test/no-file.ans"),
        ..Default::default()
    }));
    let mut d = EditCommandDialog::new(Arc::new(Mutex::new(IcyBoard::default())), menu, Command::default(), 1);
    assert!(!d.preview_error.is_empty());
    assert_eq!(d.preview.width(), 80);
    assert!(draw(&mut d, 80, 25).contains(&get_text("mnu_work_preview_error")));
}

#[test]
fn action_popup_and_choice_lists_render_at_80x25() {
    let mut d = dialog(Command::default());
    d.open_action(None, CommandAction::default());
    d.handle_key_press(key(KeyCode::Enter));
    draw(&mut d, 80, 25);
    d.handle_key_press(key(KeyCode::Esc));
    d.handle_key_press(key(KeyCode::Esc));
    d.open_action(
        None,
        CommandAction {
            command_type: CommandType::DisplayFile,
            ..Default::default()
        },
    );
    d.action_editor.as_mut().unwrap().state.selected = 1;
    d.handle_key_press(key(KeyCode::F(4)));
    draw(&mut d, 80, 25);
}

fn draw_app_area(d: &mut EditCommandDialog<'_>) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
    terminal
        .draw(|frame| {
            Line::from("APP HEADER").render(Rect::new(0, 0, 80, 1), frame.buffer_mut());
            Line::from("APP TABS").render(Rect::new(0, 1, 80, 1), frame.buffer_mut());
            d.ui(frame, Rect::new(0, 2, 80, 23));
        })
        .unwrap();
    terminal.backend().buffer().clone()
}

fn row(buffer: &Buffer, y: u16) -> String {
    (buffer.area.x..buffer.area.right()).map(|x| buffer[(x, y)].symbol()).collect()
}

fn buffer_text(buffer: &Buffer) -> String {
    (buffer.area.y..buffer.area.bottom()).map(|y| row(buffer, y)).collect::<Vec<_>>().join("\n")
}

fn display_dialog(path: PathBuf) -> EditCommandDialog<'static> {
    EditCommandDialog::new(
        Arc::new(Mutex::new(IcyBoard::default())),
        Arc::new(Mutex::new(Menu {
            display_file: path,
            ..Default::default()
        })),
        Command::default(),
        1,
    )
}

#[test]
fn display_resolution_uses_security_zero_graphics_suffix_and_explicit_extension() {
    let dir = tempfile::tempdir().unwrap();
    for (name, content) in [
        ("menu", "BARE"),
        ("menug.ans", "GRAPHICS"),
        ("menu0g.ans", "SECURITY"),
        ("menu.ans", "EXPLICIT"),
    ] {
        std::fs::write(dir.path().join(name), content).unwrap();
    }
    let mut board = IcyBoard::default();
    board.root_path = dir.path().to_path_buf();
    let board = Arc::new(Mutex::new(board));
    for (display, expected) in [("menu", "SECURITY"), ("menu.ans", "EXPLICIT")] {
        let d = EditCommandDialog::new(
            board.clone(),
            Arc::new(Mutex::new(Menu {
                display_file: display.into(),
                ..Default::default()
            })),
            Command::default(),
            1,
        );
        assert!(d.preview_error.is_empty(), "{}", d.preview_error);
        assert!(row(&d.position_canvas(Position::default()), 0).starts_with(expected));
    }
    std::fs::remove_file(dir.path().join("menu0g.ans")).unwrap();
    let d = display_dialog(dir.path().join("menu"));
    assert!(row(&d.position_canvas(Position::default()), 0).starts_with("GRAPHICS"));
}

#[test]
fn oversized_files_and_directories_fall_back_to_a_blank_80x25_canvas() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.ans");
    std::fs::File::create(&path).unwrap().set_len(1024 * 1024 + 1).unwrap();
    for (path, error) in [(path, "mnu_preview_file_limit"), (dir.path().to_path_buf(), "mnu_preview_regular_file")] {
        let d = display_dialog(path);
        assert!(d.preview_error.contains(&get_text(error)), "{}", d.preview_error);
        let canvas = d.position_canvas(Position::default());
        assert_eq!(canvas.area, Rect::new(0, 0, 80, 25));
        assert!(canvas.content.iter().all(|cell| cell.symbol() == " "));
    }
}

#[test]
fn short_background_does_not_shrink_position_bounds_and_lower_rows_pan_into_view() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("short.ans");
    std::fs::write(&path, b"SHORT").unwrap();
    let mut d = display_dialog(path);
    assert!(d.preview_error.is_empty());
    // Model a decoder that returned only one row; movement must not use it.
    d.preview = TextBuffer::new((80, 1));
    d.command.lock().unwrap().display = "BOTTOM".into();
    d.menu.lock().unwrap().commands = vec![
        Command::default(),
        Command {
            display: "NEIGHBOR".into(),
            position: Position { x: 0, y: 23 },
            ..Default::default()
        },
    ];
    d.position_edit = Some(Position { x: 70, y: 0 });
    for _ in 0..30 {
        d.handle_key_press(key(KeyCode::Down));
    }
    assert!(d.position_edit == Some(Position { x: 70, y: 24 }));
    let buffer = draw_app_area(&mut d);
    assert_eq!((d.view_x, d.view_y), (0, 4));
    assert_eq!(buffer[(70, 22)].symbol(), "B");
    assert!(row(&buffer, 21).starts_with("NEIGHBOR"));
    assert!(row(&buffer, 23).starts_with("(70,24)"));
    for key in [KeyCode::F(6), KeyCode::F(10), KeyCode::Esc] {
        assert!(row(&buffer, 24).contains(&icy_board_tui::hotkeys::key_symbol(key)), "{}", row(&buffer, 24));
    }
    assert!(row(&buffer, 0).starts_with("APP HEADER"));
    assert!(row(&buffer, 1).starts_with("APP TABS"));
    assert!(d.command.lock().unwrap().position.is_default());
    for _ in 0..20 {
        d.handle_key_press(key(KeyCode::Right));
    }
    assert!(d.position_edit == Some(Position { x: 79, y: 24 }));
    d.handle_key_press(key(KeyCode::Enter));
    assert_eq!(field_text(&d.config, POSITION), "79,24");
}

#[test]
fn position_viewport_clips_overlays_after_compositing_and_keeps_model_coordinates() {
    use ratatui::backend::Backend;
    let mut d = dialog(Command {
        display: "SELECTED".into(),
        ..Default::default()
    });
    d.menu.lock().unwrap().commands = vec![
        Command::default(),
        Command {
            display: "0123456789".into(),
            position: Position { x: 60, y: 24 },
            ..Default::default()
        },
    ];
    let pos = Position { x: 70, y: 24 };
    d.position_edit = Some(pos);
    let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
    terminal.draw(|frame| d.ui(frame, Rect::new(3, 4, 8, 6))).unwrap();
    // Origin (63,21): even the visible tail of an overlay starting left of
    // the viewport must survive, and the selected text is not repositioned.
    assert_eq!((d.view_x, d.view_y), (63, 21));
    assert_eq!(terminal.backend().buffer()[(3, 7)].symbol(), "3");
    assert_eq!(terminal.backend().buffer()[(10, 7)].symbol(), "S");
    let cursor = terminal.backend_mut().get_cursor_position().unwrap();
    assert_eq!((cursor.x, cursor.y), (10, 7));
    assert!(d.position_edit == Some(pos));
    assert!(d.command.lock().unwrap().position.is_default());
    d.handle_key_press(key(KeyCode::Enter));
    assert!(d.command.lock().unwrap().position == pos);
}

#[test]
fn out_of_canvas_positions_are_not_painted_or_silently_clamped() {
    let pos = Position { x: u16::MAX, y: u16::MAX };
    let mut d = dialog(Command {
        display: "OFFCANVAS".into(),
        position: pos,
        ..Default::default()
    });
    d.position_edit = Some(pos);
    assert!(!buffer_text(&draw_app_area(&mut d)).contains("OFFCANVAS"));
    for code in [KeyCode::Right, KeyCode::Down, KeyCode::F(6)] {
        d.handle_key_press(key(code));
    }
    assert!(d.position_edit == Some(pos));
    assert!(d.command.lock().unwrap().position == pos);
    d.handle_key_press(key(KeyCode::F(10)));
    assert_eq!(field_text(&d.config, POSITION), "65535,65535");
}

#[test]
fn position_cells_convert_cp437_and_preserve_direct_palette_bold_and_blink_colors() {
    use icy_engine::{AttributedChar, TextAttribute};
    let mut d = dialog(Command::default());
    // Preserve transparent attributes through icy_engine's layer compositor.
    d.preview.layers[0].properties.has_alpha_channel = true;
    let cases = [
        (
            AttributeColor::Rgb(12, 34, 56),
            AttributeColor::ExtendedPalette(123),
            true,
            Color::Rgb(12, 34, 56),
            Color::Indexed(123),
        ),
        (
            AttributeColor::ExtendedPalette(201),
            AttributeColor::Rgb(98, 76, 54),
            false,
            Color::Indexed(201),
            Color::Rgb(98, 76, 54),
        ),
        (
            AttributeColor::Palette(1),
            AttributeColor::Palette(2),
            true,
            palette_color(&d.preview, 9),
            palette_color(&d.preview, 2),
        ),
        (
            AttributeColor::Palette(12),
            AttributeColor::Transparent,
            true,
            palette_color(&d.preview, 12),
            Color::Reset,
        ),
        (
            AttributeColor::Transparent,
            AttributeColor::Rgb(1, 2, 3),
            false,
            Color::Reset,
            Color::Rgb(1, 2, 3),
        ),
    ];
    for (x, (fg, bg, bold, _, _)) in cases.iter().enumerate() {
        let mut attr = TextAttribute::default();
        attr.set_foreground_color(*fg);
        attr.set_background_color(*bg);
        attr.set_is_bold(*bold);
        attr.set_is_blinking(true);
        d.preview.layers[0].set_char((x as i32, 0), AttributedChar::new('\u{db}', attr));
    }
    d.position_edit = Some(Position { x: 20, y: 0 });
    let buffer = draw_app_area(&mut d);
    for (x, (_, _, _, fg, bg)) in cases.iter().enumerate() {
        let cell = &buffer[(x as u16, 2)];
        assert_eq!(cell.symbol(), "█");
        assert_eq!(cell.fg, *fg);
        assert_eq!(cell.bg, *bg);
        assert!(cell.modifier.contains(Modifier::SLOW_BLINK));
    }
}

fn palette_color(buffer: &TextBuffer, index: u32) -> Color {
    let (r, g, b) = buffer.palette.rgb(index);
    Color::Rgb(r, g, b)
}

#[test]
fn position_canvas_sanitizes_controls_but_preserves_pcb_colors_and_highlight() {
    let mut d = dialog(Command {
        display: "@X0CRED\x1b".into(),
        ..Default::default()
    });
    d.preview.buffer_type = icy_engine::BufferType::Unicode;
    d.preview.layers[0].set_char((0, 0), icy_engine::AttributedChar::from_char('\x1b'));
    let pos = Position { x: 2, y: 1 };
    d.highlight = true;
    let canvas = d.position_canvas(pos);
    assert_eq!(canvas[(0, 0)].symbol(), "�");
    assert_eq!(canvas[(2, 1)].symbol(), "R");
    assert_eq!(canvas[(5, 1)].symbol(), "�");
    assert_eq!(canvas[(2, 1)].fg, icy_board_tui::theme::DOS_LIGHT_RED);
    assert!(canvas[(2, 1)].modifier.contains(Modifier::BOLD));
    assert!(!buffer_text(&canvas).contains('\x1b'));
}

#[test]
fn actual_app_area_shows_every_field_action_rows_parameters_and_hints() {
    let mut d = dialog(Command {
        display: "NORMAL SAMPLE".into(),
        lighbar_display: "LIGHT SAMPLE".into(),
        keyword: "GO".into(),
        help: "help.txt".into(),
        position: Position { x: 12, y: 5 },
        autorun_time: 123,
        charge_per_use: 2.5,
        charge_per_minute: 0.25,
        actions: (0..6)
            .map(|i| CommandAction {
                command_type: CommandType::DisplayFile,
                parameter: format!("action-{i}.ans"),
                trigger: if i == 1 { ActionTrigger::Selection } else { ActionTrigger::Activation },
            })
            .collect(),
        ..Default::default()
    });
    for selected in 0..d.config.count() {
        d.state.selected = selected;
        let buffer = draw_app_area(&mut d);
        let text = buffer_text(&buffer);
        assert_eq!(d.state.selected, selected);
        assert_eq!(d.state.first_row, 0);
        for (index, label) in FIELD_LABELS[..10].iter().enumerate() {
            assert!(
                row(&buffer, 4 + index as u16).contains(&get_text(label)),
                "{label}, selection {selected}:\n{text}"
            );
        }
        for value in ["NORMAL SAMPLE", "LIGHT SAMPLE", "GO", "help.txt", "12,5", "123", "2.5", "0.25"] {
            assert!(text.contains(value), "{value}, selection {selected}:\n{text}");
        }
        assert!(row(&buffer, 14).contains(&get_text("mnu_work_normal")));
        assert!(row(&buffer, 15).contains(&get_text("mnu_work_highlight")));
        assert!(row(&buffer, 16).contains(&get_text("command_editor_command_type")));
        assert!(row(&buffer, 16).contains(&get_text("command_editor_header_parameter")));
        assert!(row(&buffer, 16).contains(&get_text("mnu_work_trigger")));
        for i in 0..3 {
            assert!(row(&buffer, 18 + i).contains(&format!("action-{i}.ans")), "{text}");
        }
        assert!(row(&buffer, 18).contains(&CommandType::DisplayFile.to_string()));
        assert!(row(&buffer, 19).contains(&get_text("mnu_work_selection")));
        for hint in [KeyCode::F(10), KeyCode::Esc, KeyCode::Tab] {
            assert!(row(&buffer, 23).contains(&icy_board_tui::hotkeys::key_symbol(hint)), "{text}");
        }
        assert!(row(&buffer, 0).starts_with("APP HEADER"));
        assert!(row(&buffer, 1).starts_with("APP TABS"));
    }
    d.handle_key_press(key(KeyCode::Tab));
    d.handle_key_press(key(KeyCode::End));
    let buffer = draw_app_area(&mut d);
    assert_eq!(d.insert_table.table_state.selected(), Some(5));
    assert!(row(&buffer, 20).contains("action-5.ans"));
    let hints = [row(&buffer, 21), row(&buffer, 22)].map(|s| s.trim().trim_matches('║').to_string()).join(" ");
    assert!(
        hints
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .contains(&get_text("mnu_work_action_keys").split_whitespace().collect::<Vec<_>>().join(" ")),
        "{}",
        buffer_text(&buffer)
    );
    d.handle_key_press(key(KeyCode::Tab));
    assert_eq!(d.state.selected, CHARGE_MINUTE);
}

#[test]
fn german_field_labels_fit_unabridged_for_every_selection_without_changing_global_locale() {
    // Read the real German catalog, not the process-wide locale singleton:
    // this regression runs alongside EN tests without changing their language.
    let catalog = include_str!("../../icy_board_tui/i18n/de/icy_board_tui.ftl");
    let labels: Vec<_> = FIELD_LABELS
        .iter()
        .map(|key| {
            catalog
                .lines()
                .filter_map(|line| line.split_once('='))
                .find_map(|(id, text)| (id.trim() == *key).then(|| text.trim().to_string()))
                .unwrap_or_else(|| panic!("Missing German field {key}"))
        })
        .collect();
    let mut d = dialog(Command::default());
    // Only substitute the labels of this rendering fixture. The production
    // field ordering, values and selection state are retained.
    d.config.entry = std::mem::take(&mut d.config.entry)
        .into_iter()
        .zip(&labels)
        .map(|(entry, label)| {
            let ConfigEntry::Item(item) = entry else {
                panic!("Expected flat dialog fields")
            };
            ConfigEntry::Item(ListItem::new(label.clone(), item.value).with_label_width(field_label_width()))
        })
        .collect();
    for selected in 0..d.config.count() {
        d.state.selected = selected;
        let buffer = draw_app_area(&mut d);
        for (index, label) in labels[..10].iter().enumerate() {
            assert!(
                row(&buffer, 4 + index as u16).contains(label),
                "German field truncated: {label}\n{}",
                buffer_text(&buffer)
            );
        }
        assert_eq!(d.state.selected, selected);
    }
    d.open_action(
        None,
        CommandAction {
            command_type: CommandType::DisplayFile,
            parameter: "menus/main.ans".into(),
            ..Default::default()
        },
    );
    let editor = d.action_editor.as_mut().unwrap();
    editor.config.entry = std::mem::take(&mut editor.config.entry)
        .into_iter()
        .zip(&labels[10..])
        .map(|(entry, label)| {
            let ConfigEntry::Item(item) = entry else {
                panic!("Expected flat action fields")
            };
            ConfigEntry::Item(ListItem::new(label.clone(), item.value).with_label_width(field_label_width()))
        })
        .collect();
    for selected in 0..3 {
        d.action_editor.as_mut().unwrap().state.selected = selected;
        let buffer = draw_app_area(&mut d);
        for (index, label) in labels[10..].iter().enumerate() {
            assert!(row(&buffer, 6 + index as u16).contains(label), "German action field truncated: {label}");
        }
        assert!(row(&buffer, 7).contains("menus/main.ans"));
        assert_eq!(d.action_editor.as_ref().unwrap().state.selected, selected);
    }
}

#[test]
fn action_popup_in_actual_app_area_preserves_all_controls_parameter_and_help() {
    let mut d = dialog(Command::default());
    d.open_action(
        None,
        CommandAction {
            command_type: CommandType::DisplayFile,
            parameter: "menus/main.ans".into(),
            ..Default::default()
        },
    );
    for selected in 0..3 {
        d.action_editor.as_mut().unwrap().state.selected = selected;
        let buffer = draw_app_area(&mut d);
        for (index, label) in FIELD_LABELS[10..].iter().enumerate() {
            assert!(row(&buffer, 6 + index as u16).contains(&get_text(label)), "{}", buffer_text(&buffer));
        }
        assert!(row(&buffer, 7).contains("menus/main.ans"));
        assert!(buffer_text(&buffer).contains("F10"));
        assert!(row(&buffer, 21).contains("F2"));
        let help: String = (12..21).flat_map(|y| (3..77).map(move |x| (x, y))).map(|pos| buffer[pos].symbol()).collect();
        // Wrapping may insert padding/newlines, but no words may disappear.
        let words = help.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(words.contains(&get_text("mnu_work_path_help").split_whitespace().collect::<Vec<_>>().join(" ")));
        assert!(words.contains(&get_text("mnu_work_trigger_help").split_whitespace().collect::<Vec<_>>().join(" ")));
        assert_eq!(d.action_editor.as_ref().unwrap().state.selected, selected);
    }
}

fn ppe_editor(parameter: &str, board: &IcyBoard) -> ActionEditor {
    ActionEditor::new(
        CommandAction {
            command_type: CommandType::RunPPE,
            parameter: parameter.into(),
            ..Default::default()
        },
        None,
        board,
    )
}

#[test]
fn ppe_untouched_raw_assisted_roundtrips_preserve_every_byte() {
    let board = IcyBoard::default();
    for raw in [
        "",
        "   ",
        ";",
        ";;args;",
        "  scripts//./café.ppe  first;;third; ;",
        "file.ppe;",
        "file.ppe ;",
        "file.ppe;;",
        "file.ppe\t ;a\t;",
        "\"file name.ppe\" \"not quoted\";",
    ] {
        let mut editor = ppe_editor(raw, &board);
        for _ in 0..3 {
            assert_eq!(editor.config.count(), 4);
            assert!(matches!(editor.config.get_item(1).unwrap().value, ListValue::Path(_)));
            assert!(matches!(editor.config.get_item(2).unwrap().value, ListValue::Bool(_)));
            // Rendering and moving through controls must not reserialize PPE.
            flush(&editor.config);
            editor.handle(key(KeyCode::Tab), &board);
            editor.handle(key(KeyCode::F(2)), &board);
            assert!(editor.raw);
            assert_eq!(editor.config.count(), 3);
            assert!(matches!(&editor.config.get_item(1).unwrap().value, ListValue::Text(_, _, text) if text == raw));
            editor.handle(key(KeyCode::F(2)), &board);
            assert_eq!(editor.draft.lock().unwrap().parameter, raw);
        }
        assert_eq!(editor.handle(key(KeyCode::F(10)), &board), DialogResult::Accepted, "{raw:?}");
        assert_eq!(editor.draft.lock().unwrap().parameter, raw);
    }
}

#[test]
fn ppe_actual_file_selection_preserves_exact_suffix_and_empty_runtime_arguments() {
    use icy_board_engine::tokens::tokenize;
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("new.ppe"), "fixture").unwrap();
    let mut board = IcyBoard::default();
    board.root_path = root.path().to_path_buf();
    for suffix in ["", ";", ";;", " ;", "  ", ";;arg; ;tail;", "  α;;last;", ";\"a b\";", ";; "] {
        let original = format!("  old.ppe{suffix}");
        let mut editor = ppe_editor(&original, &board);
        assert!(matches!(&editor.config.get_item(3).unwrap().value, ListValue::Text(_, _, text) if text == suffix));
        editor.state.selected = 1;
        editor.handle(key(KeyCode::F(4)), &board);
        assert!(editor.state.is_path_browser_open());
        editor.handle(key(KeyCode::End), &board);
        editor.handle(key(KeyCode::Enter), &board);
        assert!(!editor.state.is_path_browser_open());
        assert!(matches!(&editor.config.get_item(1).unwrap().value, ListValue::Path(path) if path.as_os_str() == "new.ppe"));
        assert_eq!(editor.handle(key(KeyCode::F(10)), &board), DialogResult::Accepted);
        let changed = editor.draft.lock().unwrap().parameter.clone();
        assert_eq!(changed, format!("  new.ppe{suffix}"));
        assert_eq!(&tokenize(&changed)[1..], &tokenize(&original)[1..], "{suffix:?}");
        editor.handle(key(KeyCode::F(2)), &board);
        editor.handle(key(KeyCode::F(2)), &board);
        assert_eq!(editor.draft.lock().unwrap().parameter, changed);
    }
}

#[test]
fn ppe_reselecting_same_file_or_cancelling_browser_does_not_normalize_raw() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("same.ppe"), "fixture").unwrap();
    let mut board = IcyBoard::default();
    board.root_path = root.path().to_path_buf();
    let raw = "  same.ppe ;;a; ";
    let mut editor = ppe_editor(raw, &board);
    editor.state.selected = 1;
    for close in [KeyCode::Esc, KeyCode::Enter] {
        editor.handle(key(KeyCode::F(4)), &board);
        assert!(editor.state.is_path_browser_open());
        editor.handle(key(close), &board);
        assert!(!editor.state.is_path_browser_open());
        assert_eq!(editor.draft.lock().unwrap().parameter, raw);
    }
}

#[test]
fn ppe_f10_reads_both_latest_renderless_fields_without_adding_empty_arguments() {
    use icy_board_engine::tokens::tokenize;
    let board = IcyBoard::default();
    for (arguments, expected) in [
        ("", "new.ppe"),
        ("first;;third", "new.ppe first;;third"),
        (";", "new.ppe;"),
        (";;", "new.ppe;;"),
        (";arg", "new.ppe;arg"),
        (" ;arg", "new.ppe ;arg"),
        (";;arg;", "new.ppe;;arg;"),
    ] {
        let mut editor = ppe_editor("old.ppe previous", &board);
        editor.config.get_item_mut(1).unwrap().value = ListValue::Path("new.ppe".into());
        editor.config.get_item_mut(3).unwrap().value = ListValue::Text(42, TextFlags::None, arguments.into());
        editor.config.get_item_mut(2).unwrap().value = ListValue::Bool(true);
        assert_eq!(editor.handle(key(KeyCode::F(10)), &board), DialogResult::Accepted);
        let action = editor.draft.lock().unwrap();
        assert_eq!(action.parameter, expected);
        assert_eq!(tokenize(&action.parameter), tokenize(expected));
        assert_eq!(action.trigger, ActionTrigger::Selection);
    }
    let mut editor = ppe_editor("old.ppe", &board);
    editor.state.selected = 3;
    for c in ";;arg".chars() {
        editor.handle(key(KeyCode::Char(c)), &board);
    }
    assert_eq!(editor.handle(key(KeyCode::F(10)), &board), DialogResult::Accepted);
    assert_eq!(editor.draft.lock().unwrap().parameter, "old.ppe;;arg");
}

#[test]
fn ppe_raw_f2_reads_latest_unrendered_value_and_clamps_fourth_selection() {
    let board = IcyBoard::default();
    let mut editor = ppe_editor("old.ppe", &board);
    editor.state.selected = 3;
    editor.config.get_item_mut(3).unwrap().value = ListValue::Text(42, TextFlags::None, ";;new;".into());
    editor.handle(key(KeyCode::F(2)), &board);
    assert!(editor.state.selected < editor.config.count());
    assert_eq!(editor.draft.lock().unwrap().parameter, "old.ppe;;new;");
    let raw = "  changed//./file.ppe ; ;\"a b\";;";
    editor.config.get_item_mut(1).unwrap().value = ListValue::Text(42, TextFlags::None, raw.into());
    editor.handle(key(KeyCode::F(2)), &board);
    assert_eq!(editor.draft.lock().unwrap().parameter, raw);
    editor.handle(key(KeyCode::F(2)), &board);
    assert!(matches!(&editor.config.get_item(1).unwrap().value, ListValue::Text(_, _, text) if text == raw));
    let latest = "  latest.ppe ;;";
    editor.config.get_item_mut(1).unwrap().value = ListValue::Text(42, TextFlags::None, latest.into());
    assert_eq!(editor.handle(key(KeyCode::F(10)), &board), DialogResult::Accepted);
    assert_eq!(editor.draft.lock().unwrap().parameter, latest);
}

#[test]
fn ppe_assisted_f10_rejects_latest_typed_path_and_allows_correction_without_render() {
    let board = IcyBoard::default();
    for unsafe_path in ["a b.ppe", "a;b.ppe", "\"a b.ppe\"", "a.ppe ", "a.ppe;"] {
        let mut editor = ppe_editor("old.ppe;;arg", &board);
        editor.config.get_item_mut(1).unwrap().value = ListValue::Path(unsafe_path.into());
        assert_eq!(editor.handle(key(KeyCode::F(10)), &board), DialogResult::Pending);
        assert_eq!(editor.error, get_text("mnu_work_ppe_path_error"));
        editor.config.get_item_mut(1).unwrap().value = ListValue::Path("corrected.ppe".into());
        assert_eq!(editor.handle(key(KeyCode::F(10)), &board), DialogResult::Accepted);
        assert!(editor.error.is_empty());
        assert_eq!(editor.draft.lock().unwrap().parameter, "corrected.ppe;;arg");
    }
}

#[test]
fn ppe_unsafe_selected_files_require_raw_override_with_warning_and_isolated_draft() {
    for filename in ["unsafe file.ppe", "unsafe;file.ppe"] {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join(filename), "fixture").unwrap();
        let original = CommandAction {
            command_type: CommandType::RunPPE,
            parameter: "old.ppe;;arg;".into(),
            ..Default::default()
        };
        let mut d = dialog(Command {
            actions: vec![original.clone()],
            ..Default::default()
        });
        d.board.lock().unwrap().root_path = root.path().to_path_buf();
        for cancel in [true, false] {
            d.open_action(Some(0), original.clone());
            d.action_editor.as_mut().unwrap().state.selected = 1;
            for code in [KeyCode::F(4), KeyCode::End, KeyCode::Enter, KeyCode::F(10)] {
                d.handle_key_press(key(code));
            }
            let editor = d.action_editor.as_ref().expect("unsafe assisted path must not be accepted");
            assert_eq!(editor.state.selected, 1);
            assert_eq!(editor.error, get_text("mnu_work_ppe_path_error"));
            assert_eq!(d.status(), get_text("mnu_work_ppe_path_error"));
            assert_eq!(d.command.lock().unwrap().actions[0].parameter, original.parameter);
            if cancel {
                d.handle_key_press(key(KeyCode::Esc));
                assert!(d.action_editor.is_none());
                assert_eq!(d.command.lock().unwrap().actions[0].parameter, original.parameter);
            } else {
                d.handle_key_press(key(KeyCode::F(2)));
                assert_eq!(d.status(), get_text("mnu_work_ppe_help"));
                d.handle_key_press(key(KeyCode::F(10)));
                assert!(d.action_editor.is_none());
                assert_eq!(d.command.lock().unwrap().actions[0].parameter, format!("{filename};;arg;"));
            }
        }
    }
}

#[test]
fn ppe_type_switches_keep_trigger_at_two_and_remove_only_arguments_field() {
    let board = IcyBoard::default();
    let mut editor = ppe_editor("file.ppe;;arg", &board);
    editor.config.get_item_mut(3).unwrap().value = ListValue::Text(42, TextFlags::None, ";;latest".into());
    for (kind, count) in [(CommandType::DisplayFile, 3), (CommandType::RunPPE, 4), (CommandType::PrintText, 3)] {
        let ListValue::ComboBox(c) = &mut editor.config.get_item_mut(0).unwrap().value else {
            panic!("type combobox missing")
        };
        c.cur_value = ComboBoxValue::new(kind.to_string(), format!("{kind:?}"));
        editor.handle(key(KeyCode::Tab), &board);
        assert_eq!(editor.config.count(), count);
        assert!(matches!(editor.config.get_item(2).unwrap().value, ListValue::Bool(_)));
        assert_eq!(editor.draft.lock().unwrap().parameter, "file.ppe;;latest");
    }
}

#[test]
fn ppe_popup_renders_all_four_fields_without_mutating_raw_parameter() {
    let raw = "  file.ppe;;argument;";
    let mut d = dialog(Command::default());
    d.open_action(
        None,
        CommandAction {
            command_type: CommandType::RunPPE,
            parameter: raw.into(),
            ..Default::default()
        },
    );
    for selected in 0..4 {
        d.action_editor.as_mut().unwrap().state.selected = selected;
        let buffer = draw_app_area(&mut d);
        assert!(row(&buffer, 7).contains("file.ppe"));
        assert!(!row(&buffer, 7).contains("argument"));
        assert!(row(&buffer, 8).contains(&get_text("mnu_editor_run_on_selection")));
        assert!(row(&buffer, 9).contains(&get_text("mnu_work_arguments")));
        assert!(row(&buffer, 9).contains(";;argument;"));
        let first_hint = get_text(if selected == 0 { "mnu_work_type_help" } else { "mnu_work_ppe_help" });
        assert!(row(&buffer, 14).contains(first_hint.split_whitespace().next().unwrap()));
        assert_eq!(d.action_editor.as_ref().unwrap().draft.lock().unwrap().parameter, raw);
    }
    assert!(d.command.lock().unwrap().actions.is_empty());
}
