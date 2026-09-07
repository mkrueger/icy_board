use crate::icy_board::state::user_commands::mods::editor::EditUpdate;

use super::EditState;

fn create_state(text: &str) -> EditState {
    let mut state = EditState {
        insert_mode: true,
        max_line_length: 79,
        max_lines: 100,
        ..Default::default()
    };
    for (i, line) in text.lines().enumerate() {
        let mut line = line.to_string();
        if line.contains('|') {
            let pos = line.chars().position(|c| c == '|').unwrap();
            state.cursor = (pos, i).into();
            line = line.replace('|', "");
        }
        state.msg.push(line);
    }
    state
}

#[test]
fn full_screen_viewport_reserves_the_pcboard_footer() {
    assert_eq!(20, EditState::visible_line_count(0));
    assert_eq!(23, EditState::footer_row(0));
    assert_eq!(20, EditState::visible_line_count(24));
    assert_eq!(23, EditState::footer_row(24));
}

#[test]
fn test_fse_enter_eol() {
    let mut state = create_state("FooBar|");
    let update = state.press_enter();
    assert_eq!(EditUpdate::UpdateLinesFrom(1), update);
    assert_eq!(2, state.msg.len());
    assert_eq!(0, state.msg[1].len());
    assert_eq!(1, state.cursor.y);
}

#[test]
fn enter_does_not_grow_past_the_message_line_limit() {
    let mut state = create_state("One|\nTwo");
    state.max_lines = 2;
    let update = state.press_enter();
    assert_eq!(EditUpdate::None, update);
    assert_eq!(2, state.msg.len());
}

#[test]
fn test_fse_enter_mid_line() {
    let mut state = create_state("Foo|Bar");
    let update = state.press_enter();
    assert_eq!(EditUpdate::UpdateLinesFrom(0), update);
    assert_eq!(2, state.msg.len());
    assert_eq!("Foo", state.msg[0]);
    assert_eq!("Bar", state.msg[1]);
    assert_eq!(1, state.cursor.y);
}

#[test]
fn test_fse_enter_mid_line2() {
    let mut state = create_state("FooBar| 3");
    let update = state.press_enter();
    assert_eq!(EditUpdate::UpdateLinesFrom(0), update);
    assert_eq!(2, state.msg.len());
    assert_eq!("FooBar", state.msg[0]);
    assert_eq!(" 3", state.msg[1]);
    assert_eq!(1, state.cursor.y);
}

#[test]
fn test_fse_enter_after_eol() {
    let mut state = create_state("FooBar|");
    state.cursor.x += 5;
    let update = state.press_enter();
    assert_eq!(EditUpdate::UpdateLinesFrom(1), update);
    assert_eq!(2, state.msg.len());
    assert_eq!("FooBar", state.msg[0]);
    assert_eq!("", state.msg[1]);
    assert_eq!(1, state.cursor.y);
}

#[test]
fn overwrite_enter_advances_without_inserting_a_line() {
    let mut state = create_state("One  |\nTwo");
    state.insert_mode = false;

    let update = state.press_enter();

    assert_eq!(EditUpdate::UpdateLinesFrom(1), update);
    assert_eq!(vec!["One", "Two"], state.msg);
    assert_eq!(0, state.cursor.x);
    assert_eq!(1, state.cursor.y);
}

#[test]
fn forced_newline_splits_even_in_overwrite_mode() {
    let mut state = create_state("One|Two");
    state.insert_mode = false;

    let update = state.force_new_line();

    assert_eq!(EditUpdate::UpdateLinesFrom(0), update);
    assert_eq!(vec!["One", "Two"], state.msg);
    assert_eq!(0, state.cursor.x);
    assert_eq!(1, state.cursor.y);
}

#[test]
fn test_del_end_of_text() {
    let mut state = create_state("FooBar|");
    let pos = state.cursor;
    let update = state.delete_char();
    assert_eq!(EditUpdate::UpdateLinesFrom(0), update);
    assert_eq!(1, state.msg.len());
    assert_eq!("FooBar", state.msg[0]);
    assert_eq!(pos, state.cursor);
}

#[test]
fn test_del_mid_text() {
    let mut state = create_state("Foo|Bar");
    let pos = state.cursor;
    let update = state.delete_char();
    assert_eq!(EditUpdate::UpdateCurrentLineFrom(pos.x as usize), update);
    assert_eq!(1, state.msg.len());
    assert_eq!("Fooar", state.msg[0]);
    assert_eq!(pos, state.cursor);
}

#[test]
fn test_del_line_merge_text() {
    let mut state = create_state("FooBar|\nBaz Bar");
    let pos = state.cursor;
    let update = state.delete_char();
    assert_eq!(EditUpdate::UpdateLinesFrom(pos.y as usize), update);
    assert_eq!(1, state.msg.len());
    assert_eq!("FooBarBaz Bar", state.msg[0]);
    assert_eq!(pos, state.cursor);
}

#[test]
fn test_del_max_length_no_merge() {
    let mut state = create_state("FooBar|\nBaz Bar");
    state.max_line_length = 6;
    let pos = state.cursor;
    let update = state.delete_char();
    assert_eq!(EditUpdate::UpdateLinesFrom(pos.y as usize), update);
    assert_eq!(2, state.msg.len());
    assert_eq!("FooBar", state.msg[0]);
    assert_eq!("Baz Bar", state.msg[1]);
    assert_eq!(pos, state.cursor);
}

#[test]
fn test_del_max_length_merge() {
    let mut state = create_state("FooBar|\nBaz Bar");
    state.max_line_length = 9;
    let pos = state.cursor;
    let update = state.delete_char();
    assert_eq!(EditUpdate::UpdateLinesFrom(pos.y as usize), update);
    assert_eq!(2, state.msg.len());
    assert_eq!("FooBarBaz", state.msg[0]);
    assert_eq!("Bar", state.msg[1]);
    assert_eq!(pos, state.cursor);
}

#[test]
fn test_del_bug() {
    let mut state = create_state("FooBar|\n1 2");
    let pos = state.cursor;
    let update = state.delete_char();
    assert_eq!(EditUpdate::UpdateLinesFrom(pos.y as usize), update);
    assert_eq!(1, state.msg.len());
    assert_eq!("FooBar1 2", state.msg[0]);
    assert_eq!(pos, state.cursor);
}

#[test]
fn test_bs_eol() {
    let mut state = create_state("FooBar|");
    let pos = state.cursor;
    let update = state.backspace();
    assert_eq!(EditUpdate::UpdateCurrentLineFrom(pos.x as usize - 1), update);
    assert_eq!(1, state.msg.len());
    assert_eq!("FooBa", state.msg[0]);
    assert_eq!(pos.x - 1, state.cursor.x);
}

#[test]
fn test_mid_line_eol() {
    let mut state = create_state("Foo|Bar");
    let pos = state.cursor;
    let update = state.backspace();
    assert_eq!(EditUpdate::UpdateCurrentLineFrom(pos.x as usize - 1), update);
    assert_eq!(1, state.msg.len());
    assert_eq!("FoBar", state.msg[0]);
    assert_eq!(pos.x - 1, state.cursor.x);
}

#[test]
fn test_start_line() {
    let mut state = create_state("Foo\n|Bar");
    let pos = state.cursor;
    let update = state.backspace();
    assert_eq!(EditUpdate::UpdateLinesFrom(pos.y as usize - 1), update);
    assert_eq!(1, state.msg.len());
    assert_eq!("FooBar", state.msg[0]);
    assert_eq!(3, state.cursor.x);
}

#[test]
fn test_start_word_wrap() {
    let mut state = create_state("FooBar\n|Baz");
    state.max_line_length = 6;
    let pos = state.cursor;
    let update = state.backspace();
    assert_eq!(EditUpdate::UpdateLinesFrom(pos.y as usize - 1), update);
    assert_eq!(2, state.msg.len());
    assert_eq!("FooBar", state.msg[0]);
    assert_eq!("Baz", state.msg[1]);
    assert_eq!(6, state.cursor.x);
}

#[test]
fn test_left_justify() {
    let mut state = create_state("   |Baz");
    state.max_line_length = 6;
    let update = state.left_justify();
    assert_eq!(EditUpdate::UpdateCurrentLineFrom(0), update);
    assert_eq!(1, state.msg.len());
    assert_eq!("Baz", state.msg[0]);
    assert_eq!(3, state.cursor.x);
}

#[test]
fn test_center() {
    let mut state = create_state("|Baz");
    state.max_line_length = 7;
    let update = state.center();
    assert_eq!(EditUpdate::UpdateCurrentLineFrom(0), update);
    assert_eq!(1, state.msg.len());
    assert_eq!("  Baz", state.msg[0]);
    assert_eq!(5, state.cursor.x);
}

#[test]
fn test_delete_word() {
    let mut state = create_state("Foo |Bar Baz");
    let pos = state.cursor;
    let update = state.delete_word();
    assert_eq!(EditUpdate::UpdateCurrentLineFrom(pos.x as usize), update);
    assert_eq!(1, state.msg.len());
    assert_eq!("Foo  Baz", state.msg[0]);
    assert_eq!(pos.x, state.cursor.x);
}

#[test]
fn test_delete_to_eol() {
    let mut state = create_state("Foo| Bar");
    let pos = state.cursor;
    let update = state.delete_to_eol();
    assert_eq!(EditUpdate::UpdateCurrentLineFrom(pos.x as usize), update);
    assert_eq!(1, state.msg.len());
    assert_eq!("Foo", state.msg[0]);
    assert_eq!(pos.x, state.cursor.x);
}

#[test]
fn test_break_line() {
    let mut state: EditState = create_state("Foo Bar");
    state.max_line_length = 5;
    let _update = state.break_line(0);

    assert_eq!(2, state.msg.len());
    assert_eq!("Foo", state.msg[0]);
    assert_eq!("Bar", state.msg[1]);
}

#[test]
fn test_break_full_line() {
    let mut state: EditState = create_state("FooBar");
    state.max_line_length = 5;
    let _update = state.break_line(0);

    assert_eq!(2, state.msg.len());
    assert_eq!("FooBa", state.msg[0]);
    assert_eq!("r", state.msg[1]);
}

#[test]
fn stacked_editor_commands_keep_argument_order() {
    for command in ["D", "E", "I", "L"] {
        assert_eq!((command.to_string(), vec!["2".into()]), EditState::parse_command(&format!(" {command}  2 ")));
    }
    assert_eq!(("E".into(), vec!["2".into(), "é;ß".into()]), EditState::parse_command("e 2 é;ß"));
    assert_eq!((String::new(), Vec::new()), EditState::parse_command(" \t"));
}

#[test]
fn save_actions_are_distinct_and_empty_messages_abort() {
    use super::EditResult;
    let mut state = create_state("body");
    for (command, expected) in [
        ("S", EditResult::SendMessage),
        ("SC", EditResult::CarbonCopy),
        ("SA", EditResult::AttachFile),
        ("SN", EditResult::SendNext),
        ("SK", EditResult::SendKill),
    ] {
        assert_eq!(expected, state.save_result(command));
        let empty = create_state(" \n\t");
        assert_eq!(EditResult::Abort, empty.save_result(command));
    }
    state.quote_text = vec!["only a reference, not a draft".into()];
    state.msg.clear();
    assert_eq!(EditResult::Abort, state.save_result("S"));
}

#[test]
fn substitution_is_first_match_case_sensitive_and_unicode_bounded() {
    let mut state = create_state("  café café");
    assert!(state.substitute(0, "café;Grüße"));
    assert_eq!(vec!["  Grüße café"], state.msg);
    assert!(!state.substitute(0, "CAFÉ;x"));
    assert!(!state.substitute(0, ""));
    assert!(!state.substitute(0, ";insert"));
    assert!(!state.substitute(99, "x;y"));
    assert!(state.substitute(0, "Grüße"));
    assert_eq!(vec!["   café"], state.msg);
    state.max_line_length = 6;
    assert!(state.substitute(0, "café;ééééééé"));
    assert_eq!(vec!["   ééé"], state.msg);
}

#[test]
fn wrapping_keeps_leading_indent_and_never_splits_utf8() {
    assert_eq!(("  café".into(), "été".into()), EditState::wrap_once("  café été", 7));
    assert_eq!(("é╬ß".into(), "ø界".into()), EditState::wrap_once("é╬ßø界", 3));
    assert_eq!(("   ".into(), " text".into()), EditState::wrap_once("    text", 3));
    assert_eq!(("abc".into(), String::new()), EditState::wrap_once("abc ", 3));
}

#[test]
fn unicode_insert_overwrite_delete_and_split_use_columns() {
    let mut state = create_state("é|╬界");
    state.type_char('ß');
    assert_eq!(vec!["éß╬界"], state.msg);
    assert_eq!(2, state.cursor.x);
    state.insert_mode = false;
    state.type_char('ø');
    assert_eq!(vec!["éßø界"], state.msg);
    state.backspace();
    assert_eq!(vec!["éß界"], state.msg);
    state.delete_char();
    assert_eq!(vec!["éß"], state.msg);
    state.cursor.x = 1;
    state.force_new_line();
    assert_eq!(vec!["é", "ß"], state.msg);
}

#[test]
fn unicode_word_movement_progresses_at_word_and_space_boundaries() {
    let text = "éé  ╬ß zz";
    assert_eq!(0, EditState::word_left(text, 0));
    assert_eq!(0, EditState::word_left(text, 4));
    assert_eq!(4, EditState::word_left(text, 6));
    assert_eq!(7, EditState::word_left(text, 999));
    assert_eq!(4, EditState::word_right(text, 0));
    assert_eq!(4, EditState::word_right(text, 2));
    assert_eq!(7, EditState::word_right(text, 4));
    assert_eq!(9, EditState::word_right(text, 999));
    assert_eq!(0, EditState::word_right("", 999));
}

#[test]
fn unicode_word_and_tail_deletion_and_center_are_safe() {
    let mut state = create_state("é |╬ß ø");
    state.delete_word();
    assert_eq!(vec!["é  ø"], state.msg);
    state.delete_to_eol();
    assert_eq!(vec!["é "], state.msg);
    state.max_line_length = 1;
    state.center(); // A pre-existing overlong line must not underflow.
    assert_eq!(vec!["é "], state.msg);
    state.cursor.x = 20;
    state.backspace();
    assert_eq!(19, state.cursor.x);
    assert_eq!(vec!["é "], state.msg);
}

#[test]
fn typing_wraps_without_dropping_words_or_existing_following_lines() {
    let mut state = create_state("éé ╬|\nafter");
    state.max_line_length = 5;
    state.type_char('ß');
    state.type_char('ø');
    assert_eq!(vec!["éé", "╬ßø", "after"], state.msg);
    assert_eq!((3, 1), (state.cursor.x, state.cursor.y));
    let mut hard = create_state("é╬ß|");
    hard.max_line_length = 3;
    hard.type_char('ø');
    assert_eq!(vec!["é╬ß", "ø"], hard.msg);
    assert_eq!((1, 1), (hard.cursor.x, hard.cursor.y));
}

#[test]
fn insertion_near_start_of_wrapping_line_does_not_move_cursor_to_overflow() {
    let mut state = create_state("|abc def");
    state.max_line_length = 7;
    state.type_char('é');
    assert_eq!(vec!["éabc", "def"], state.msg);
    assert_eq!((1, 0), (state.cursor.x, state.cursor.y));
}

#[test]
fn full_message_rejects_growth_atomically_but_allows_overwrite() {
    let mut state = create_state("éé|");
    state.max_lines = 1;
    state.max_line_length = 2;
    assert_eq!(EditUpdate::None, state.type_char('ß'));
    assert_eq!(vec!["éé"], state.msg);
    assert_eq!(2, state.cursor.x);
    state.cursor.x = 0;
    state.insert_mode = false;
    state.type_char('╬');
    assert_eq!(vec!["╬é"], state.msg);
    assert_eq!(EditUpdate::None, state.force_new_line());
    assert_eq!(EditUpdate::None, state.press_enter());
}

#[test]
fn whitespace_at_full_width_does_not_leave_an_overlong_line() {
    let mut state = create_state("abc|");
    state.max_line_length = 3;
    state.type_char(' ');
    assert_eq!(vec!["abc"], state.msg);
    assert!(state.cursor.x <= 3);
}

#[test]
fn tab_inserts_or_moves_without_erasing_and_respects_narrow_last_stop() {
    let mut state = create_state("a|b");
    state.tab();
    assert_eq!(vec!["a       b"], state.msg);
    assert_eq!(8, state.cursor.x);
    let mut state = create_state("a|bcdefghij");
    state.insert_mode = false;
    state.tab();
    assert_eq!(vec!["abcdefghij"], state.msg);
    assert_eq!(8, state.cursor.x);
    state.max_line_length = 72;
    state.cursor.x = 63;
    state.tab();
    assert_eq!((0, 1), (state.cursor.x, state.cursor.y));
}

#[test]
fn tab_that_cannot_fit_does_not_partially_edit() {
    let mut state = create_state("a|bcdefgh");
    state.max_line_length = 9;
    state.max_lines = 1;
    assert_eq!(EditUpdate::None, state.tab());
    assert_eq!(vec!["abcdefgh"], state.msg);
    assert_eq!(1, state.cursor.x);
}

#[test]
fn join_uses_whole_unicode_words_without_exceeding_margin() {
    let mut state = create_state("éé|\nßß øø");
    state.max_line_length = 4;
    state.merge_line(0);
    assert_eq!(vec!["ééßß", "øø"], state.msg);
    state.merge_line(0);
    assert_eq!(vec!["ééßß", "øø"], state.msg);
}

#[test]
fn width_switch_is_reversible_and_rejects_long_existing_lines() {
    let mut state = create_state(&"é".repeat(72));
    assert!(state.toggle_width());
    assert_eq!(72, state.max_line_length);
    assert_eq!(5, state.left_margin());
    assert!(state.toggle_width());
    assert_eq!(79, state.max_line_length);
    state.msg[0].push('é');
    assert!(!state.toggle_width());
    assert_eq!(79, state.max_line_length);
}

#[test]
fn narrow_and_wide_rendering_clip_without_modifying_long_drafts() {
    let mut state = create_state(&"é".repeat(90));
    assert_eq!(79, state.screen_line(0).chars().count());
    state.max_line_length = 72;
    assert_eq!(format!("  1: {}", "é".repeat(72)), state.screen_line(0));
    assert_eq!("  2: ", state.screen_line(1));
    assert_eq!("", state.screen_line(state.max_lines));
    assert_eq!(90, state.msg[0].chars().count());
}

#[test]
fn pages_relocate_cursor_and_stay_bounded_even_for_tiny_pages() {
    let mut state = create_state("line");
    state.move_page(true, 24);
    assert_eq!((18, 20), (state.top_line, state.cursor.y));
    state.move_page(false, 24);
    assert_eq!((0, 17), (state.top_line, state.cursor.y));
    for page in [0, 1, 2, 3, 10, 24, u16::MAX] {
        state.cursor = (-10, -10).into();
        state.top_line = usize::MAX;
        for _ in 0..150 {
            state.move_page(true, page);
            assert!(state.cursor.y >= 0 && state.cursor.y < state.max_lines as i32);
            assert!(state.cursor.y as usize >= state.top_line);
            assert!((state.cursor.y as usize) < state.top_line + EditState::visible_line_count(page));
        }
        for _ in 0..150 {
            state.move_page(false, page);
        }
        assert_eq!(0, state.top_line);
    }
    // Navigation alone must not allocate blank lines up to the cursor.
    assert_eq!(vec!["line"], state.msg);
}

#[test]
fn quoting_prefixes_every_wrapped_line_and_skips_fido_control_lines() {
    let mut state = create_state("draft|");
    state.max_line_length = 9;
    state.quote_text = vec!["\x01MSGID: hidden".into(), "éé ╬╬ ßß".into(), "".into(), "tail".into()];
    assert!(state.insert_quote(1, 2));
    assert_eq!(vec!["draft", "-> éé ╬╬", "-> ßß", "-> "], state.msg);
    assert!(state.msg.iter().all(|line| line.chars().count() <= 9));
    assert_eq!(4, state.quote_text.len());
}

#[test]
fn invalid_empty_or_oversized_quote_leaves_draft_and_cursor_unchanged() {
    let mut state = create_state("draft|");
    let original = state.msg.clone();
    let cursor = state.cursor;
    assert!(!state.insert_quote(1, 1));
    state.quote_text = vec!["source".into()];
    for (start, end) in [(0, 1), (2, 1), (1, 2), (usize::MAX, usize::MAX)] {
        assert!(!state.insert_quote(start, end));
    }
    state.max_lines = 1;
    assert!(!state.insert_quote(1, 1));
    assert_eq!(original, state.msg);
    assert_eq!(cursor, state.cursor);
}

#[test]
fn reflow_preserves_paragraph_indent_and_blank_boundaries() {
    let mut state = create_state("  one\ntwo |three\n\nleave alone");
    state.max_line_length = 10;
    state.reformat();
    assert_eq!(vec!["  one two", "three", "", "leave alone"], state.msg);
}

#[test]
fn upload_decodes_utf8_and_cp437_expands_tabs_and_is_atomic() {
    let mut state = create_state("draft|");
    state.max_line_length = 12;
    assert!(state.append_uploaded_text("\u{feff}  café\r\nα\tβ\r\n".as_bytes()));
    assert_eq!(vec!["draft", "  café", "α       β"], state.msg);
    assert!(state.append_uploaded_text(&[0x82, b'\r', b'\n', 0x1a]));
    assert_eq!(Some("é"), state.msg.last().map(String::as_str));
    let before = state.msg.clone();
    state.max_lines = state.msg.len();
    assert!(!state.append_uploaded_text(b"no room"));
    assert!(!state.append_uploaded_text(b""));
    assert!(!state.append_uploaded_text(&vec![b'x'; EditState::MAX_UPLOAD_BYTES + 1]));
    assert_eq!(before, state.msg);
}

#[test]
fn upload_wraps_long_words_and_does_not_store_terminal_controls() {
    let mut state = create_state("");
    state.max_line_length = 3;
    assert!(state.append_uploaded_text("éééé\x1b\0\r\n".as_bytes()));
    assert_eq!(vec!["ééé", "é"], state.msg);
}

#[test]
fn zero_capacity_operations_do_not_allocate_or_panic() {
    let mut state = EditState::default();
    assert_eq!(EditUpdate::None, state.type_char('a'));
    assert_eq!(EditUpdate::None, state.tab());
    assert_eq!(EditUpdate::None, state.press_enter());
    assert_eq!(EditUpdate::None, state.force_new_line());
    assert_eq!(EditUpdate::None, state.delete_char());
    assert_eq!(EditUpdate::None, state.backspace());
    assert_eq!(EditUpdate::None, state.center());
    assert_eq!(EditUpdate::None, state.delete_word());
    assert_eq!(EditUpdate::None, state.delete_to_eol());
    assert_eq!(EditUpdate::None, state.reformat());
    assert!(!state.append_uploaded_text(b"x"));
    assert!(state.msg.is_empty());
}

async fn input_state(input: &str) -> (super::IcyBoardState, icy_net::channel::ChannelConnection) {
    use crate::icy_board::{
        IcyBoard,
        bbs::BBS,
        state::{KeyChar, KeySource},
        user_base::User,
    };
    use icy_net::{ConnectionType, channel::ChannelConnection};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let bbs = Arc::new(Mutex::new(BBS::new(1)));
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let (peer, connection) = ChannelConnection::create_pair();
    let mut board = IcyBoard::new();
    board.users.new_user(User {
        name: "EDITOR TEST".into(),
        security_level: 255,
        ..Default::default()
    });
    let caller = board.users[0].clone();
    let mut state = super::IcyBoardState::new(bbs, Arc::new(Mutex::new(board)), nodes, node, Box::new(connection)).await;
    state.session.current_user = Some(caller);
    state.session.cur_user_id = 0;
    state.session.page_len = 0;
    state.char_buffer.extend(input.chars().map(|ch| KeyChar::new(KeySource::User, ch)));
    (state, peer)
}

#[tokio::test]
async fn real_editor_dispatch_executes_stacked_delete_edit_insert_and_list() {
    let (mut board, _peer) = input_state("").await;
    let yes = board.session.yes_char;
    let input = format!("\rD 2\r{yes}\rE 1\ré;ß\r\rI 2\rinsert\r\rL 2\rSN\r");
    board.char_buffer.extend(
        input
            .chars()
            .map(|ch| crate::icy_board::state::KeyChar::new(crate::icy_board::state::KeySource::User, ch)),
    );
    let mut editor = create_state("é one\ntwo\nthree");
    let result = tokio::time::timeout(std::time::Duration::from_secs(3), editor.edit_message(&mut board))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(super::EditResult::SendNext, result);
    assert_eq!(vec!["ß one", "insert", "three"], editor.msg);
}

#[tokio::test]
async fn line_input_keeps_indentation_and_wraps_unicode_at_the_margin() {
    let (mut board, _peer) = input_state("  éé ╬ß\r\rS\r").await;
    let mut editor = create_state("");
    editor.max_line_length = 6;
    let result = tokio::time::timeout(std::time::Duration::from_secs(3), editor.edit_message(&mut board))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(super::EditResult::SendMessage, result);
    assert_eq!(vec!["  éé", "╬ß"], editor.msg);
}

#[tokio::test]
async fn full_screen_ctrl_j_joins_ctrl_u_exits_and_tab_does_not_center() {
    let (mut board, _peer) = input_state("\n\x15SK\r").await;
    let mut editor = create_state("one|\ntwo");
    editor.use_fse = true;
    let result = tokio::time::timeout(std::time::Duration::from_secs(3), editor.edit_message(&mut board))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(super::EditResult::SendKill, result);
    assert_eq!(vec!["onetwo"], editor.msg);

    let (mut board, _peer) = input_state("a\tb\x15SA\r").await;
    let mut editor = create_state("");
    editor.use_fse = true;
    let result = tokio::time::timeout(std::time::Duration::from_secs(3), editor.edit_message(&mut board))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(super::EditResult::AttachFile, result);
    assert_eq!(vec!["a       b"], editor.msg);
}

#[tokio::test]
async fn quote_command_and_control_o_use_the_caller_supplied_source() {
    for (fse, input) in [(false, "\rQ 1 1\rS\r"), (true, "\x0f1 1\r\x15S\r")] {
        let (mut board, _peer) = input_state(input).await;
        let mut editor = create_state("");
        editor.use_fse = fse;
        editor.quote_text = vec!["original é".into()];
        let result = tokio::time::timeout(std::time::Duration::from_secs(3), editor.edit_message(&mut board))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(super::EditResult::SendMessage, result);
        assert_eq!(vec!["-> original é"], editor.msg);
    }
}

#[tokio::test]
async fn cancelling_quote_at_either_prompt_preserves_the_draft() {
    for input in ["Q\r", "1\rQ\r"] {
        let (mut board, _peer) = input_state(input).await;
        let mut editor = create_state("draft|");
        editor.quote_text = vec!["source".into()];
        let cursor = editor.cursor;
        tokio::time::timeout(std::time::Duration::from_secs(3), editor.quote(&mut board))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(vec!["draft"], editor.msg);
        assert_eq!(cursor, editor.cursor);
    }
}

#[tokio::test]
async fn disconnect_guards_still_abort_both_editors_without_consuming_input() {
    for fse in [false, true] {
        let (mut board, _peer) = input_state("S\r").await;
        board.session.request_logoff = true;
        let mut editor = create_state("draft");
        editor.use_fse = fse;
        assert_eq!(super::EditResult::Abort, editor.edit_message(&mut board).await.unwrap());
        assert_eq!(vec!["draft"], editor.msg);
        assert_eq!(2, board.char_buffer.len());
    }
}

#[tokio::test]
async fn expert_commands_accept_visible_input_and_substitution_enter_cancels() {
    let (mut board, _peer) = input_state("\rE 1\r\rSC\r").await;
    board.session.current_user.as_mut().unwrap().flags.expert_mode = true;
    let mut editor = create_state("café");
    let result = tokio::time::timeout(std::time::Duration::from_secs(3), editor.edit_message(&mut board))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(super::EditResult::CarbonCopy, result);
    assert_eq!(vec!["café"], editor.msg);
}

#[tokio::test]
async fn upload_protocol_cancel_leaves_the_draft_untouched() {
    let (mut board, _peer) = input_state("N\r").await;
    let mut editor = create_state("draft|");
    tokio::time::timeout(std::time::Duration::from_secs(3), editor.upload_text(&mut board))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(vec!["draft"], editor.msg);
    assert_eq!(5, editor.cursor.x);
}
