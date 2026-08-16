use crate::tests::{fixture, setup_conference, test_login_output, test_ppe_output, test_ppe_output_with_input};
use icy_board_engine::icy_board::icb_config::DisplayNewsBehavior;
use icy_engine::{TextPane, TextScreen};
use icy_parser_core::{AnsiParser, CommandParser};

fn setup_login(board: &mut icy_board_engine::icy_board::IcyBoard, allow_comment: bool) {
    board.config.paths.welcome = crate::tests::fixture("main/blt1");
    board.config.system_control.allow_password_failure_comment = allow_comment;
}

#[test]
fn a_failed_password_can_offer_a_sysop_comment() {
    let output = test_login_output("SYSOP\nWRONG\nWRONG\nWRONG\nWRONG\nN\n".to_string(), |board| {
        setup_login(board, true);
    });
    assert!(
        output.contains("leave a comment to the sysop"),
        "the password failure comment was not offered:\n{output}"
    );
}

#[test]
fn a_failed_password_does_not_offer_a_comment_when_disabled() {
    let output = test_login_output("SYSOP\nWRONG\nWRONG\nWRONG\nWRONG\n".to_string(), |board| {
        setup_login(board, false);
    });
    assert!(
        !output.contains("leave a comment to the sysop"),
        "the password failure comment was offered:\n{output}"
    );
}

#[test]
fn a_direct_ppe_has_only_its_output_and_the_completion_prompt() {
    let output = test_ppe_output("PRINT \"PPE ONLY\"", |board| {
        setup_conference(board);
        board.conferences[0].news_file = fixture("main/blt1");
        board.config.paths.welcome = fixture("main/blt2");
        board.config.switches.display_news_behavior = DisplayNewsBehavior::Always;
        board.config.switches.scan_new_blt = true;
    });

    assert_eq!(output, format!("PPE ONLY\n{}\n", icy_board_tui::get_text("run_ppe_completed")));
}

/// The lines of an 80x25 screen after the board's own ANSI has been replayed into it.
fn rendered_lines(output: &str) -> Vec<String> {
    let mut screen = TextScreen::new((80, 25));
    let mut parser = AnsiParser::default();
    parser.parse(output.as_bytes(), &mut icy_engine::ScreenSink::new(&mut screen));
    (0..25).map(|y| (0..80).map(|x| screen.char_at((x, y).into()).ch).collect::<String>()).collect()
}

#[test]
fn a_long_input_field_keeps_the_cursor_on_its_prompt_line() {
    let output = test_ppe_output_with_input("STRING name\nINPUT \"What is your name? \", name", "test\r", |_| {});

    let lines = rendered_lines(&output);
    assert!(lines[0].contains("What is your name? ? (test"), "{:?}", lines[0]);
    assert!(!lines[1].contains("test"), "{:?}", lines[1]);
}

#[test]
fn a_default_answer_stays_inside_a_clamped_field() {
    // A prompt of this width keeps the field delimiters but leaves fewer than the
    // sixty columns the field asks for, so the field itself is clamped.
    let prompt = "x".repeat(30);
    let default = "y".repeat(60);
    let output = test_ppe_output_with_input(&format!("STRING name\nname = \"{default}\"\nINPUT \"{prompt}\", name"), "\r", |_| {});

    let lines = rendered_lines(&output);
    assert!(lines[0].contains(')'), "the field delimiters were dropped: {:?}", lines[0]);
    assert!(!lines[1].contains('y'), "the default answer wrapped onto the next line: {:?}", lines[1]);
}
