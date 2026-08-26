//! Reading messages out of an area as `MSG` values.

use std::{path::PathBuf, sync::Arc};

use crate::icy_board::{
    conferences::Conference,
    message_area::{AreaList, MessageArea},
};

use super::{compile_errors, run_ppl_on, run_ppl_with_messages, scratch_dir};

/// `(from, to, subject)`, numbered from one in order.
const MESSAGES: &[(&str, &str, &str)] = &[
    ("SYSOP", "STAN", "About PPL"),
    ("STAN", "SYSOP", "Re: About PPL"),
    ("FRED", "ALL", "Announcement"),
];

#[test]
fn a_message_reports_its_header() {
    let output = run_ppl_with_messages(
        r#"
        MSG msg = Board.Conferences[0].Areas[0].Read(1)
        PrintLn msg.Valid, " ", msg.Number
        PrintLn msg.From, " -> ", msg.To
        PrintLn msg.Subject
        PrintLn msg.IsPrivate, " ", msg.IsRead, " ", msg.IsDeleted, " ", msg.IsEcho
        "#,
        MESSAGES,
    );

    assert_eq!(output, "1 1\nSYSOP -> STAN\nAbout PPL\n0 0 0 0\n");
}

/// The body stays in the base until it is asked for, so reading it is a call.
#[test]
fn a_message_reads_its_body_on_demand() {
    let output = run_ppl_with_messages(
        r"
        PrintLn Board.Conferences[0].Areas[0].Read(2).Text()
        ",
        MESSAGES,
    );

    assert_eq!(output, "body\n");
}

/// A number the area has no message for stays readable, the way every other
/// board object does, so a walk over a base with holes cannot fall over.
#[test]
fn an_unknown_message_number_answers_an_invalid_message() {
    let output = run_ppl_with_messages(
        r#"
        MSG msg = Board.Conferences[0].Areas[0].Read(99)
        PrintLn msg.Valid, " ", msg.Number, " [", msg.Subject, "] [", msg.Text(), "] ", Error.Last().OK
        PrintLn Board.Conferences[0].Areas[0].Read(-1).Valid, " ", Error.Last().OK
        "#,
        MESSAGES,
    );

    assert_eq!(output, "0 0 [] [] 1\n0 1\n");
}

/// A number outside JAM's range is one no message has, not one that wraps.
#[test]
fn a_message_number_too_large_for_the_base_does_not_wrap() {
    let output = run_ppl_with_messages(
        r"
        LONG number = 4294967297
        PrintLn Board.Conferences[0].Areas[0].Read(number).Valid
        PrintLn Board.Conferences[0].Areas[0].Read(4294967295).Valid
        ",
        MESSAGES,
    );

    assert_eq!(output, "0\n0\n");
}

/// The walk the docs show, pinned so the types it declares keep working.
#[test]
fn the_documented_walk_reads_every_message_in_the_area() {
    let output = run_ppl_with_messages(
        r#"
        AREA area = Board.Conferences[0].Areas[0]
        LONG n

        FOR n = area.LowMsg() TO area.HighMsg()
            MSG msg = area.Read(n)
            IF !msg.Valid CONTINUE
            PrintLn msg.Number, " ", msg.From, " ", msg.Subject
        NEXT
        "#,
        MESSAGES,
    );

    assert_eq!(output, "1 SYSOP About PPL\n2 STAN Re: About PPL\n3 FRED Announcement\n");
}

#[test]
fn a_missing_message_clears_an_older_operation_error() {
    let output = run_ppl_with_messages(
        r#"
        Terminal.EndUpdate()
        PrintLn Error.Last().OK
        MSG msg = Board.Conferences[0].Areas[0].Read(99)
        PrintLn msg.Valid, " ", Error.Last().OK
        "#,
        MESSAGES,
    );

    assert_eq!(output, "0\n0 1\n");
}

/// The area is read through one open message base, so a message written past it -
/// by this PPE here, by another node in practice - still has to show up.
#[test]
fn a_message_written_after_the_base_was_opened_is_still_found() {
    let output = run_ppl_with_messages(
        r#"
        AREA area = Board.Conferences[0].Areas[0]
        PrintLn area.HighMsg(), " ", area.Read(4).Valid

        FCREATE 1, "body.txt", O_WR, S_DN
        FPUTLN 1, "the body"
        FCLOSE 1
        MESSAGE 0, "SOMEONE", "ME", "Written later", "N", 0, FALSE, FALSE, "body.txt"

        PrintLn area.HighMsg(), " ", area.Read(4).Subject
        "#,
        MESSAGES,
    );

    // `MESSAGE` prints a notice of its own between the two lines.
    assert!(output.starts_with("3 0\n"), "{output:?}");
    assert!(output.ends_with("4 Written later\n"), "{output:?}");
}

fn run_on_message_base(source: &str, path: PathBuf) -> String {
    run_ppl_on(source, |board| {
        board.conferences.push(Conference {
            name: "Main Board".to_string(),
            areas: Some(Arc::new(AreaList::new(vec![MessageArea {
                name: "General".to_string(),
                path: path.clone(),
                ..Default::default()
            }]))),
            ..Default::default()
        });
    })
}

#[test]
fn a_missing_message_base_reports_message_io_errors() {
    let path = scratch_dir("missing-message-base").join("general");
    let output = run_on_message_base(
        r#"
        AREA area = Board.Conferences[0].Areas[0]
        MSG msg = area.Read(1)
        PrintLn msg.Valid, " ", Error.Last().Kind = ErrKind.Msg, " ", Error.Last().Code = ErrCode.Io
        PrintLn area.Find(MsgField.To, "STAN").Valid, " ", Error.Last().Kind = ErrKind.Msg, " ", Error.Last().Code = ErrCode.Io
        PrintLn area.LowMsg(), " ", Error.Last().Kind = ErrKind.Msg, " ", Error.Last().Code = ErrCode.Io
        PrintLn area.HighMsg(), " ", Error.Last().Kind = ErrKind.Msg, " ", Error.Last().Code = ErrCode.Io
        "#,
        path.clone(),
    );
    let _ = std::fs::remove_dir_all(path.parent().unwrap());

    assert_eq!(output, "0 1 1\n0 1 1\n0 1 1\n0 1 1\n");
}

#[test]
fn a_message_operation_failure_enters_an_on_error_handler() {
    let path = scratch_dir("message-on-error").join("general");
    let output = run_on_message_base(
        r#"
        ON ERROR GOSUB failed
        MSG msg = Board.Conferences[0].Areas[0].Read(1)
        PrintLn "continued"
        EXIT

        :failed
        PrintLn Error.Last().Kind = ErrKind.Msg, " ", Error.Last().Code = ErrCode.Io
        RETURN
        "#,
        path.clone(),
    );
    let _ = std::fs::remove_dir_all(path.parent().unwrap());

    assert_eq!(output, "1 1\ncontinued\n");
}

#[test]
fn a_corrupt_message_index_reports_a_message_format_error() {
    let path = scratch_dir("corrupt-message-index").join("general");
    let mut base = jamjam::jam::JamMessageBase::create(&path).unwrap();
    base.write_message(&jamjam::jam::JamMessage::default().with_text(bstr::BString::from("body")))
        .unwrap();
    base.write_jhr_header().unwrap();
    drop(base);
    std::fs::write(path.with_extension("jdx"), [0]).unwrap();

    let output = run_on_message_base(
        r#"
        MSG msg = Board.Conferences[0].Areas[0].Read(1)
        PrintLn msg.Valid, " ", Error.Last().Kind = ErrKind.Msg, " ", Error.Last().Code = ErrCode.Format
        "#,
        path.clone(),
    );
    let _ = std::fs::remove_dir_all(path.parent().unwrap());

    assert_eq!(output, "0 1 1\n");
}

#[test]
fn a_corrupt_message_body_reports_a_message_format_error() {
    let path = scratch_dir("corrupt-message-body").join("general");
    let mut base = jamjam::jam::JamMessageBase::create(&path).unwrap();
    base.write_message(&jamjam::jam::JamMessage::default().with_text(bstr::BString::from("body")))
        .unwrap();
    base.write_jhr_header().unwrap();
    drop(base);
    std::fs::write(path.with_extension("jdt"), []).unwrap();

    let output = run_on_message_base(
        r#"
        MSG msg = Board.Conferences[0].Areas[0].Read(1)
        PrintLn msg.Valid, " ", Error.Last().OK
        PrintLn "[", msg.Text(), "] ", Error.Last().Kind = ErrKind.Msg, " ", Error.Last().Code = ErrCode.Format
        "#,
        path.clone(),
    );
    let _ = std::fs::remove_dir_all(path.parent().unwrap());

    assert_eq!(output, "1 1\n[] 1 1\n");
}

/// The numbers a walk runs between. A JAM base is sparse, so a PPE counts over
/// the range and asks each message whether it is there.
#[test]
fn an_area_reports_the_range_of_its_messages() {
    let output = run_ppl_with_messages(
        r#"
        AREA area = Board.Conferences[0].Areas[0]
        PrintLn area.LowMsg(), " ", area.HighMsg()

        INTEGER n
        INTEGER found
        FOR n = area.LowMsg() TO area.HighMsg()
            MSG msg = area.Read(n)
            IF msg.Valid found = found + 1
        NEXT
        PrintLn found
        "#,
        MESSAGES,
    );

    assert_eq!(output, "1 3\n3\n");
}

#[test]
fn find_reports_the_first_message_whose_field_matches() {
    let output = run_ppl_with_messages(
        r#"
        AREA area = Board.Conferences[0].Areas[0]
        PrintLn area.Find(MsgField.To, "STAN").Number
        PrintLn area.Find(MsgField.From, "FRED").Number
        PrintLn area.Find(MsgField.Subject, "About PPL").Number
        "#,
        MESSAGES,
    );

    assert_eq!(output, "1\n3\n1\n");
}

/// Matching is case-insensitive and anywhere in the field, the way `SCANMSGHDR`
/// has always matched.
#[test]
fn find_matches_part_of_a_field_whatever_its_case() {
    let output = run_ppl_with_messages(
        r#"
        PrintLn Board.Conferences[0].Areas[0].Find(MsgField.Subject, "announce").Number
        "#,
        MESSAGES,
    );

    assert_eq!(output, "3\n");
}

/// The start number is what lets a PPE walk on to the next match.
#[test]
fn find_starts_where_it_is_told_to() {
    let output = run_ppl_with_messages(
        r#"
        AREA area = Board.Conferences[0].Areas[0]
        MSG first = area.Find(MsgField.Subject, "About PPL")
        MSG later = area.Find(MsgField.Subject, "About PPL", first.Number + 1)
        PrintLn first.Number, " ", later.Number, " ", later.Subject
        "#,
        MESSAGES,
    );

    assert_eq!(output, "1 2 Re: About PPL\n");
}

#[test]
fn find_answers_an_invalid_message_when_nothing_matches() {
    let output = run_ppl_with_messages(
        r#"
        MSG msg = Board.Conferences[0].Areas[0].Find(MsgField.To, "NOBODY")
        PrintLn msg.Valid, " ", msg.Number
        "#,
        MESSAGES,
    );

    assert_eq!(output, "0 0\n");
}

/// A message that is there carries the date it was written; one that is not
/// reads as the empty date rather than as today.
#[test]
fn a_message_carries_the_date_it_was_written() {
    let output = run_ppl_with_messages(
        r#"
        AREA area = Board.Conferences[0].Areas[0]
        PrintLn area.Read(1).Date <> "00/00/00"
        PrintLn area.Read(99).Date
        "#,
        MESSAGES,
    );

    assert_eq!(output, "1\n00/00/00\n");
}

/// A message is what the area holds, not something a PPE may rewrite.
#[test]
fn a_message_member_cannot_be_assigned() {
    for write in [
        "MSG msg = Session.Area.Read(1)\nmsg.Subject = \"x\"",
        "MSG msg = Session.Area.Read(1)\nmsg.Number = 2",
        "MSG msg = Session.Area.Read(1)\nmsg.IsDeleted = TRUE",
    ] {
        let errors = compile_errors(write);
        assert!(errors.iter().any(|error| error.contains("can only be read")), "{write}: {errors:?}");
    }
}

/// `MESSAGE` is a statement from PPL 1.00 and stays one, which is why the type
/// beside it is called `MSG`.
#[test]
fn the_message_statement_still_parses_at_language_400() {
    let errors = compile_errors(
        r#"
        INTEGER conf
        STRING to
        MESSAGE conf, to, "SYSOP", "subject", "R", 0, TRUE, TRUE, "body.txt"
        "#,
    );

    assert!(errors.is_empty(), "{errors:?}");
}
