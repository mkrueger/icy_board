use crate::tests::{setup_conference, test_output};
use icy_engine::{TextPane, TextScreen};
use icy_parser_core::{AnsiParser, CommandParser};

/// The lines of an 80x25 screen, so the colour codes do not shift the columns.
fn rendered_lines(output: &str) -> Vec<String> {
    let mut screen = TextScreen::new((80, 25));
    let mut parser = AnsiParser::default();
    parser.parse(output.as_bytes(), &mut icy_engine::ScreenSink::new(&mut screen));
    (0..25).map(|y| (0..80).map(|x| screen.char_at((x, y).into()).ch).collect::<String>()).collect()
}

fn node_line(output: &str) -> String {
    rendered_lines(output)
        .into_iter()
        .find(|line| line.trim_start().starts_with("1   "))
        .unwrap_or_default()
}

/// The oracle printed `   1   Available for CHAT      ORACLE TESTER (TESTCITY)`,
/// so the status column carries the node's state, not what it is busy with.
#[test]
fn who_shows_the_node_status_in_the_status_column() {
    let output = test_output("WHO\n".to_string(), setup_conference);
    let line = node_line(&output);

    assert!(line.contains("Available for CHAT"), "the status column does not name the state:\n{line:?}");
}

#[test]
fn who_lines_the_columns_up_under_its_header() {
    let output = test_output("WHO\n".to_string(), setup_conference);
    let line = node_line(&output);

    assert_eq!(line.find("Available"), Some(7), "the status column moved:\n{line:?}");
    assert_eq!(line.find("SYSOP"), Some(31), "the user column moved:\n{line:?}");
}

/// The city follows the name in brackets while the option is on.
#[test]
fn who_shows_the_city_with_the_name() {
    let output = test_output("WHO\n".to_string(), |board| {
        setup_conference(board);
        board.config.board.who_include_city = true;
        board.users[0].city_or_state = "TESTCITY".to_string();
    });

    assert!(output.contains("(TESTCITY)"), "the city is missing:\n{output}");
}

#[test]
fn who_leaves_the_city_out_when_the_option_is_off() {
    let output = test_output("WHO\n".to_string(), |board| {
        setup_conference(board);
        board.config.board.who_include_city = false;
        board.users[0].city_or_state = "TESTCITY".to_string();
    });

    assert!(!output.contains("(TESTCITY)"), "the city was shown although the option is off:\n{output}");
}
