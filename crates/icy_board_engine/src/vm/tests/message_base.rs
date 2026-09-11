//! The message base opcodes, checked through what a PPE can observe of them.

use super::{run_ppl_on, run_ppl_with_messages};

const MESSAGES: &[(&str, &str, &str)] = &[
    ("SYSOP", "STAN", "Welcome aboard"),
    ("STAN", "SYSOP", "About PPL"),
    ("SYSOP", "ALL", "Board news"),
];

#[test]
fn message_header_is_an_independent_writable_value() {
    assert_eq!(
        super::run_ppl(
            r#"
MSGHEADER original
original.From = "SYSOP"
original.To = "ALL"
original.Subject = "Original"
MSGHEADER changed = original
changed.Subject = "Changed"
changed.IsPrivate = TRUE
PRINT original.Subject, ":", original.IsPrivate, "|", changed.Subject, ":", changed.IsPrivate
"#,
        ),
        "Original:0|Changed:1"
    );
}

#[test]
fn message_api_invalid_values_report_errors_without_posting() {
    assert_eq!(
        super::run_ppl(
            r#"
AREA area
MSG original
MSG posted = Session.PostMessage(area)
PRINT posted.Valid, ":", Error.Last().Kind = ErrKind.Msg, ":", Error.Last().Code = ErrCode.Invalid, "|"
MSG reply = Session.ReplyMessage(original)
PRINT reply.Valid, ":", Error.Last().Code = ErrCode.Invalid, "|"
MSG edited = Session.EditMessage(original)
PRINT edited.Valid, ":", Error.Last().Code = ErrCode.Invalid, "|"
MSGHEADER header = Session.ReplyHeader(original)
PRINT header.Subject, ":", Error.Last().Code = ErrCode.Invalid
"#
        ),
        "0:1:1|0:1|0:1|:1"
    );
}

#[test]
fn test_scanmsghdr_finds_the_first_message_addressed_to_someone() {
    assert_eq!(run_ppl_with_messages(r#"PRINT SCANMSGHDR(0, 1, HDR_TO, "STAN")"#, MESSAGES), "1");
}

#[test]
fn message_header_snapshot_does_not_write_through() {
    assert_eq!(
        run_ppl_with_messages(
            r#"
MSG message = Board.Conferences[0].Areas[0].Read(1)
MSGHEADER header = message.Header
header.Subject = "Changed"
header.IsPrivate = TRUE
MSG fresh = Board.Conferences[0].Areas[0].Read(1)
PRINT message.Subject, "|", fresh.Subject, "|", header.Subject, ":", header.IsPrivate
"#,
            MESSAGES,
        ),
        "Welcome aboard|Welcome aboard|Changed:1"
    );
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

/// The security argument of MESSAGE asks for a message only its receiver may read,
/// which is the private flag GETMSGHDR reports as '*'.
#[test]
fn test_message_marks_a_receiver_only_message_private() {
    let output = run_ppl_with_messages(
        r#"
        FCREATE 1, "body.txt", O_WR, S_DN
        FPUTLN 1, "the body"
        FCLOSE 1
        MESSAGE 0, "SOMEONE", "ME", "Private one", "R", 0, FALSE, FALSE, "body.txt"
        PRINT "[", GETMSGHDR(0, 4, HDR_STATUS), "]"
    "#,
        MESSAGES,
    );
    assert!(output.ends_with("[*]"), "unexpected output: {output:?}");
}

/// Without it the message is public, which reads back as a blank.
#[test]
fn test_message_leaves_a_public_message_public() {
    let output = run_ppl_with_messages(
        r#"
        FCREATE 1, "body.txt", O_WR, S_DN
        FPUTLN 1, "the body"
        FCLOSE 1
        MESSAGE 0, "SOMEONE", "ME", "Public one", "N", 0, FALSE, FALSE, "body.txt"
        PRINT "[", GETMSGHDR(0, 4, HDR_STATUS), "]"
    "#,
        MESSAGES,
    );
    assert!(output.ends_with("[ ]"), "unexpected output: {output:?}");
}

/// An echoed message carries the flag `HDR_ECHO` reports.
#[test]
fn test_message_marks_an_echoed_message() {
    let output = run_ppl_with_messages(
        r#"
        FCREATE 1, "body.txt", O_WR, S_DN
        FPUTLN 1, "the body"
        FCLOSE 1
        MESSAGE 0, "SOMEONE", "ME", "Echoed one", "N", 0, FALSE, TRUE, "body.txt"
        PRINT "[", GETMSGHDR(0, 4, HDR_ECHO), "]"
    "#,
        MESSAGES,
    );
    assert!(output.ends_with("[E]"), "unexpected output: {output:?}");
}

/// `PCBoard`'s `entermessagefromfile()` returns quietly when the body file is not
/// there, so the program carries on rather than being stopped.
#[test]
fn test_message_with_a_missing_file_does_not_stop_the_program() {
    let output = run_ppl_with_messages(
        r#"
        MESSAGE 0, "SOMEONE", "ME", "No body", "N", 0, FALSE, FALSE, "does_not_exist.txt"
        PRINT "still here"
    "#,
        MESSAGES,
    );
    assert!(output.ends_with("still here"), "unexpected output: {output:?}");
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

/// `PCBoard` 15.4/M numbers the writable fields 1..5, so 3 is the subject rather
/// than the reference message `HDR_MSGREF` names.
#[test]
fn test_setmsghdr_takes_the_documented_field_numbers() {
    assert_eq!(
        run_ppl_with_messages("INTEGER n\nn = SETMSGHDR(0, 2, 3, \"By number\")\nPRINT GETMSGHDR(0, 2, HDR_SUBJ)", MESSAGES),
        "By number"
    );
    assert_eq!(
        run_ppl_with_messages("INTEGER n\nn = SETMSGHDR(0, 2, 1, \"NOBODY\")\nPRINT GETMSGHDR(0, 2, HDR_TO)", MESSAGES),
        "NOBODY"
    );
}

/// The status is the character `PCBoard` kept in the header: a public message
/// nobody has read yet is a blank.
#[test]
fn test_getmsghdr_reads_the_status_character() {
    assert_eq!(run_ppl_with_messages(r#"PRINT "[", GETMSGHDR(0, 1, HDR_STATUS), "]""#, MESSAGES), "[ ]");
}

/// The header held its date and time as text, so a PPE reads MM-DD-YY and HH:MM.
#[test]
fn test_getmsghdr_reads_the_date_and_time_as_text() {
    let output = run_ppl_with_messages(r#"PRINT LEN(GETMSGHDR(0, 1, HDR_DATE)), " ", LEN(GETMSGHDR(0, 1, HDR_TIME))"#, MESSAGES);
    assert_eq!(output, "8 5");
}

/// Nothing has replied to these messages, and nothing is echoed.
#[test]
fn test_getmsghdr_leaves_the_reply_and_echo_fields_empty() {
    let output = run_ppl_with_messages(
        r#"PRINT "[", GETMSGHDR(0, 1, HDR_REPLY), "][", GETMSGHDR(0, 1, HDR_ECHO), "][", GETMSGHDR(0, 1, HDR_RPLYTIME), "]", GETMSGHDR(0, 1, HDR_RPLYDATE)"#,
        MESSAGES,
    );
    assert_eq!(output, "[][][]0");
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

/// Dumps a message to a file and reads it back, so the test can see what a PPE
/// would parse out of it.
const DUMP_AND_READ: &str = r#"
    STRING s
    MSGTOFILE 0, 3, "out.txt"
    FOPEN 1, "out.txt", O_RD, S_DN
    FGET 1, s
    WHILE (!FERR(1)) DO
        PRINTLN s
        FGET 1, s
    ENDWHILE
    FCLOSE 1
"#;

/// The dump carries the header fields `PCBoard` wrote: the real status character,
/// the number, the parties, an active flag of 225 and the body.
#[test]
fn test_msgtofile_writes_the_header_and_body() {
    let output = run_ppl_with_messages(DUMP_AND_READ, MESSAGES);
    assert!(output.contains("          Status:  \n"), "status: {output:?}");
    assert!(output.contains("  Message Number: 3\n"), "number: {output:?}");
    assert!(output.contains("Reference Number: 0\n"), "reference: {output:?}");
    assert!(output.contains("Number of blocks: 2\n"), "blocks: {output:?}");
    assert!(output.contains("              To: ALL\n"), "to: {output:?}");
    assert!(output.contains("            From: SYSOP\n"), "from: {output:?}");
    assert!(output.contains("         Subject: Board news\n"), "subject: {output:?}");
    assert!(output.contains("          Active: 225\n"), "active: {output:?}");
    assert!(output.contains("Message Body:\n"), "body label: {output:?}");
}

/// `PCBoard` overwrites its two "Reply" lines before writing them, so only the
/// "Time of reply" line survives.
#[test]
fn test_msgtofile_writes_only_the_time_of_reply_line() {
    let output = run_ppl_with_messages(DUMP_AND_READ, MESSAGES);
    assert!(output.contains("   Time of reply: \n"), "time of reply: {output:?}");
    assert!(!output.contains("           Reply:"), "stray reply line: {output:?}");
}

/// With every field short enough to fit, `PCBoard` writes no extended-header
/// section at all rather than a "0" count.
#[test]
fn test_msgtofile_omits_the_extended_headers_when_there_are_none() {
    let output = run_ppl_with_messages(DUMP_AND_READ, MESSAGES);
    assert!(!output.contains("Extended headers"), "unexpected ext headers: {output:?}");
}

/// A recipient longer than the 25-character fixed field moves into an extended
/// TO header and blanks the fixed line, the way `PCBoard` stores it.
#[test]
fn test_msgtofile_spills_a_long_recipient_into_an_extended_header() {
    let messages: &[(&str, &str, &str)] = &[("SYSOP", "A VERY LONG RECIPIENT NAME INDEED", "Hi")];
    let source = r#"
        STRING s
        MSGTOFILE 0, 1, "out.txt"
        FOPEN 1, "out.txt", O_RD, S_DN
        FGET 1, s
        WHILE (!FERR(1)) DO
            PRINTLN s
            FGET 1, s
        ENDWHILE
        FCLOSE 1
    "#;
    let output = run_ppl_with_messages(source, messages);
    assert!(output.contains("              To: \n"), "fixed to not blanked: {output:?}");
    assert!(output.contains("Extended headers: 1\n"), "ext count: {output:?}");
    let ext_to = format!("TO     :{:<60}N\n", "A VERY LONG RECIPIENT NAME INDEED");
    assert!(output.contains(&ext_to), "ext to: {output:?}");
}

#[test]
fn test_msgtofile_appends_to_an_existing_file() {
    let source = r#"
        FCREATE 1, "out.txt", O_WR, S_DN
        FPUTLN 1, "before"
        FCLOSE 1
        MSGTOFILE 0, 1, "out.txt"
        STRING s
        FOPEN 1, "out.txt", O_RD, S_DN
        FGET 1, s
        PRINT s
        FCLOSE 1
    "#;
    assert_eq!(run_ppl_with_messages(source, MESSAGES), "before");
}

#[test]
fn test_msgtofile_writes_receipt_and_packout_extended_headers() {
    let source = r#"
        FCREATE 1, "body.txt", O_WR, S_DN
        FPUTLN 1, "body"
        FCLOSE 1
        MESSAGE 0, "STAN", "SYSOP", "Expiring", "R", MKDATE(2026, 8, 15), TRUE, FALSE, "body.txt"
        MSGTOFILE 0, 4, "out.txt"
        STRING s
        FOPEN 1, "out.txt", O_RD, S_DN
        FGET 1, s
        WHILE (!FERR(1)) DO
            PRINTLN s
            FGET 1, s
        ENDWHILE
        FCLOSE 1
    "#;
    let output = run_ppl_with_messages(source, MESSAGES);
    assert!(output.contains("Extended headers: 2\n"), "ext count: {output:?}");
    assert!(output.contains(&format!("PACKOUT:{:<60}N\n", "08-15-26")), "packout: {output:?}");
    assert!(
        output.contains(&format!("REQRR  :{:<60}N\n", "Caller has requested a Return Receipt")),
        "receipt: {output:?}"
    );
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

/// A PPE writes its body with FPUTLN, which starts a fresh file with a UTF-8
/// BOM. That marker belongs to the file, not to the message.
#[test]
fn a_message_body_does_not_start_with_the_byte_order_mark() {
    let output = run_ppl_with_messages(
        r#"
        FCREATE 1, "body.txt", O_WR, S_DN
        FPUTLN 1, "first line"
        FPUTLN 1, "second line"
        FCLOSE 1
        MESSAGE 0, "SOMEONE", "ME", "Body check", "N", 0, FALSE, FALSE, "body.txt"
        STRING body = Board.Conferences[0].Areas[0].Read(4).Text()
        PRINT "[", body.Left(5), "]", TOBYTES(body.Left(1)).ToHex()
    "#,
        MESSAGES,
    );
    assert!(output.ends_with("[first]66"), "the body still carries a BOM: {output:?}");
}

/// SETLMR stores the pointer, so U_LMR has to answer with it rather than with
/// whatever the interactive reader happened to leave in the session.
#[test]
fn the_last_read_pointer_survives_being_written_and_read_back() {
    let output = run_ppl_with_messages(
        r#"
        PRINT "[", U_LMR(0), "|"
        SETLMR 0, 2
        PRINT U_LMR(0), "|"
        SETLMR 0, 1
        PRINT U_LMR(0), "]"
    "#,
        MESSAGES,
    );
    assert!(output.ends_with("[0|2|1]"), "unexpected pointer sequence: {output:?}");
}

/// A number past the end clamps, and the clamped value is what comes back.
#[test]
fn the_last_read_pointer_clamps_to_the_highest_message() {
    let output = run_ppl_with_messages(
        r#"
        SETLMR 0, 9999
        PRINT "[", U_LMR(0), "]"
    "#,
        MESSAGES,
    );
    assert!(output.ends_with("[3]"), "unexpected clamped pointer: {output:?}");
}
