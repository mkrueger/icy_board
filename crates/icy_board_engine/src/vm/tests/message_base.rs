//! The message base opcodes, checked through what a PPE can observe of them.

use super::{run_ppl_on, run_ppl_with_messages};

const MESSAGES: &[(&str, &str, &str)] = &[
    ("SYSOP", "STAN", "Welcome aboard"),
    ("STAN", "SYSOP", "About PPL"),
    ("SYSOP", "ALL", "Board news"),
];

#[test]
fn test_scanmsghdr_finds_the_first_message_addressed_to_someone() {
    assert_eq!(run_ppl_with_messages(r#"PRINT SCANMSGHDR(0, 1, HDR_TO, "STAN")"#, MESSAGES), "1");
}

#[test]
fn test_scanmsghdr_starts_where_it_is_told_to() {
    // Message 1 is also from the sysop, so a scan that started at the beginning
    // would answer 1 rather than 3.
    assert_eq!(run_ppl_with_messages(r#"PRINT SCANMSGHDR(0, 3, HDR_FROM, "SYSOP")"#, MESSAGES), "3");
}

#[test]
fn test_scanmsghdr_matches_part_of_a_subject() {
    assert_eq!(run_ppl_with_messages(r#"PRINT SCANMSGHDR(0, 1, HDR_SUBJ, "PPL")"#, MESSAGES), "2");
}

#[test]
fn test_scanmsghdr_reports_zero_when_nothing_matches() {
    assert_eq!(run_ppl_with_messages(r#"PRINT SCANMSGHDR(0, 1, HDR_TO, "NOBODY")"#, MESSAGES), "0");
}

#[test]
fn test_setmsghdr_changes_a_field_that_getmsghdr_reads_back() {
    assert_eq!(
        run_ppl_with_messages(
            "INTEGER n\nn = SETMSGHDR(0, 2, HDR_SUBJ, \"Something else\")\nPRINT GETMSGHDR(0, 2, HDR_SUBJ)",
            MESSAGES
        ),
        "Something else"
    );
}

#[test]
fn test_setmsghdr_leaves_the_other_messages_alone() {
    assert_eq!(
        run_ppl_with_messages("INTEGER n\nn = SETMSGHDR(0, 2, HDR_TO, \"NOBODY\")\nPRINT GETMSGHDR(0, 1, HDR_TO)", MESSAGES),
        "STAN"
    );
}

#[test]
fn test_setmsghdr_reports_zero_for_a_message_that_is_not_there() {
    assert_eq!(run_ppl_with_messages(r#"PRINT SETMSGHDR(0, 99, HDR_SUBJ, "x")"#, MESSAGES), "0");
}

#[test]
fn test_killmsg_marks_the_message_inactive() {
    // An active message reads back as 225, a killed one as 226.
    assert_eq!(run_ppl_with_messages("KILLMSG 0, 2\nPRINT GETMSGHDR(0, 2, HDR_ACTIVE)", MESSAGES), "226");
}

#[test]
fn test_a_message_that_was_not_killed_stays_active() {
    assert_eq!(run_ppl_with_messages("KILLMSG 0, 2\nPRINT GETMSGHDR(0, 1, HDR_ACTIVE)", MESSAGES), "225");
}

#[test]
fn test_killmsg_on_a_message_that_is_not_there_does_not_stop_the_program() {
    assert_eq!(run_ppl_with_messages("KILLMSG 0, 99\nPRINT \"still running\"", MESSAGES), "still running");
}

#[test]
fn test_setlmr_moves_the_last_message_read_pointer() {
    assert_eq!(run_ppl_with_messages("SETLMR 0, 2\nPRINT \"ok\"", MESSAGES), "ok");
}

#[test]
fn test_move_msg_copies_the_message_into_the_other_conference() {
    assert_eq!(
        run_ppl_with_messages("MOVEMSG 1, 2, FALSE\nPRINT GETMSGHDR(1, 1, HDR_SUBJ)", MESSAGES),
        "About PPL"
    );
}

#[test]
fn test_a_copy_leaves_the_original_where_it_was() {
    assert_eq!(run_ppl_with_messages("MOVEMSG 1, 2, FALSE\nPRINT GETMSGHDR(0, 2, HDR_ACTIVE)", MESSAGES), "225");
}

#[test]
fn test_a_move_takes_the_original_away() {
    assert_eq!(run_ppl_with_messages("MOVEMSG 1, 2, TRUE\nPRINT GETMSGHDR(0, 2, HDR_ACTIVE)", MESSAGES), "226");
}

#[test]
fn test_opencap_reports_a_capture_it_could_not_open() {
    // A directory that does not exist cannot hold a capture file.
    assert_eq!(run_ppl_on("BOOLEAN ok\nOPENCAP \"no/such/place/CAP\", ok\nPRINT ok", |_| {}), "0");
}

#[test]
fn test_opencap_captures_what_the_caller_sees() {
    let output = run_ppl_on("BOOLEAN ok\nOPENCAP \"CAP\", ok\nPRINT \"captured\"\nCLOSECAP\nPRINTLN\nPRINT ok", |_| {});
    // The capture is a tee, so the caller still sees the text as well.
    assert_eq!(output, "captured\n1");
}

#[test]
fn test_stackabort_can_be_turned_off_and_on_again() {
    assert_eq!(run_ppl_on("STACKABORT FALSE\nSTACKABORT TRUE\nPRINT STACKERR()", |_| {}), "0");
}
