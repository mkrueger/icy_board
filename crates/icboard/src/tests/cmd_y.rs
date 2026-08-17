use crate::tests::{setup_conference, setup_conference_with_messages, test_output};
use icy_board_engine::icy_board::IcyBoard;
use icy_engine::{TextPane, TextScreen};
use icy_parser_core::{AnsiParser, CommandParser};

/// The lines of an 80x25 screen, so the colour codes between the columns do not
/// get in the way of reading them.
fn rendered_lines(output: &str) -> Vec<String> {
    let mut screen = TextScreen::new((80, 25));
    let mut parser = AnsiParser::default();
    parser.parse(output.as_bytes(), &mut icy_engine::ScreenSink::new(&mut screen));
    (0..25).map(|y| (0..80).map(|x| screen.char_at((x, y).into()).ch).collect::<String>()).collect()
}

#[test]
fn personal_mail_scan_defaults_to_quick_when_configured() {
    let output = test_output("Y\nC\n".to_string(), |board| {
        setup_conference(board);
        board.config.message.default_quick_personal_scan = true;
    });
    assert!(output.contains("Total"), "the quick scan header is missing:\n{output}");
}

#[test]
fn personal_mail_scan_defaults_to_long_when_quick_is_disabled() {
    let output = test_output("Y\nC\n".to_string(), |board| {
        setup_conference(board);
        board.config.message.default_quick_personal_scan = false;
    });
    assert!(!output.contains("Total"), "the quick scan header was shown:\n{output}");
}

/// An empty answer aborts before anything is scanned.
#[test]
fn personal_mail_scan_aborts_on_an_empty_answer() {
    let output = test_output("Y\n\n".to_string(), setup_conference_with_messages);
    assert!(!output.contains("Aborts"), "the scan ran despite an empty answer:\n{output}");
}

/// The long form lists the message numbers themselves, which the quick one only counts.
#[test]
fn personal_mail_scan_long_lists_the_message_numbers() {
    let output = test_output("Y\nL C\n".to_string(), setup_conference_with_messages);

    assert!(output.contains("Msgs For You"), "the to-you list is missing:\n{output}");
    assert!(output.contains("Msgs From You"), "the from-you list is missing:\n{output}");
    assert!(output.contains("# Msgs Found"), "the total is missing:\n{output}");

    let from = output.lines().find(|line| line.contains("Msgs From You")).unwrap_or_default();
    for number in ['1', '2', '3'] {
        assert!(from.contains(number), "message {number} is missing from the list:\n{output}");
    }
    let to = output.lines().find(|line| line.contains("Msgs For You")).unwrap_or_default();
    assert!(to.contains("None"), "mail addressed to nobody was counted as the caller's:\n{output}");
}

/// The quick columns are "To You" and "Total Found", not to and from.
#[test]
fn personal_mail_scan_quick_counts_everything_found() {
    let output = test_output("Y\nQ C\n".to_string(), setup_conference_with_messages);
    let lines = rendered_lines(&output);
    let line = lines.iter().find(|line| line.contains("Main Board .")).cloned().unwrap_or_default();

    assert!(line.trim_end().ends_with("0     3"), "the columns are not (to you, total found):\n{line:?}");
}

fn seed_email(board: &mut IcyBoard) {
    setup_conference_with_messages(board);
    let path = board.resolve_file(&board.config.paths.email_msgbase.clone());
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut base = jamjam::jam::JamMessageBase::create(&path).unwrap();
    base.write_message(
        &jamjam::jam::JamMessage::default()
            .with_from(bstr::BString::from("ALICE"))
            .with_to(bstr::BString::from("SYSOP"))
            .with_subject(bstr::BString::from("Private note"))
            .with_date_time(chrono::Utc::now())
            .with_attributes(jamjam::jam::attributes::MSG_PRIVATE)
            .with_text(bstr::BString::from("hello")),
    )
    .unwrap();
    base.write_jhr_header().unwrap();
}

/// PCBoard had no private mail base; icy_board's gets a line of its own.
#[test]
fn personal_mail_scan_reports_waiting_email() {
    let output = test_output("Y\nQ C\n".to_string(), seed_email);
    let lines = rendered_lines(&output);
    let line = lines.iter().find(|line| line.contains("E-Mail")).cloned().unwrap_or_default();

    assert!(!line.is_empty(), "the e-mail base was not scanned:\n{output}");
    assert!(line.trim_end().ends_with("1     1"), "the waiting e-mail was not counted:\n{line:?}");
}

#[test]
fn personal_mail_scan_lists_email_in_the_long_form() {
    let output = test_output("Y\nL C\n".to_string(), seed_email);
    assert!(output.contains("E-Mail"), "the e-mail base was not scanned:\n{output}");
}
