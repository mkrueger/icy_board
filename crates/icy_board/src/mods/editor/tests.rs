use crate::mods::editor::EditUpdate;

use super::EditState;

fn create_state(text: &str) -> EditState {
    let mut state = EditState::default();
    state.max_line_length = 79;
    for (i, line) in text.lines().enumerate() {
        let mut line = line.to_string();
        if line.contains('|') {
            let pos = line.chars().position(|c| c == '|').unwrap();
            state.cursor = (pos, i).into();
            line = line.replace("|", "");
        }
        state.msg.push(line);
    }
    state
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
    assert_eq!("Foo", state.msg[0]);
    assert_eq!("Bar", state.msg[1]);
}
