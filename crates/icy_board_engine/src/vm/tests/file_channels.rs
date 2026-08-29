//! What a PPE gets back from a file channel that is not open.
//!
//! `PCBoard`'s channel routines set an error flag and returned - openChan, closeChan,
//! getChan and their neighbours in SCREXEC.CPP never ended a PPE. Boards are full of
//! PPEs that rewind a channel they just closed or open a file that is not there.

use super::{run_ppl, run_ppl_with_files};

const CONTENT: &[u8] = b"first line\r\nsecond line\r\n";

#[test]
fn a_file_that_is_not_there_reports_through_ferr() {
    let output = run_ppl(
        r#"
        FOPEN 6, "missing.pag", 0, 0
        PRINTLN "err=", FERR(6)
        PRINTLN "still running"
    "#,
    );
    assert_eq!(output, "err=1\nstill running\n");
}

/// `PCBoard` scans fileArr for the first channel that is not in use and answers -1
/// when all eight are busy. Verified against `PCBoard` 15.4/M.
#[test]
fn fnext_answers_the_first_free_channel_and_minus_one_when_full() {
    let output = run_ppl_with_files(
        r#"
        INTEGER i
        PRINTLN "start=", FNEXT()
        FOR i = 0 TO 7
          FOPEN i, "data.pag", 0, 0
        NEXT
        PRINTLN "full=", FNEXT()
        FCLOSE 3
        PRINTLN "after_close=", FNEXT()
        FCLOSEALL
    "#,
        &[("data.pag", CONTENT)],
    );
    assert_eq!(output, "start=0\nfull=-1\nafter_close=3\n");
}

/// A channel nothing ever touched has no error flag set (`PCBoard`'s fileArr starts zeroed).
#[test]
fn ferr_on_a_channel_nothing_touched_is_false() {
    let output = run_ppl(r#"PRINTLN "err=", FERR(5)"#);
    assert_eq!(output, "err=0\n");
}

/// `PCBoard` cleared errStat when FERR was read (EVALP.CPP), so a second FERR is false
/// until another failing op sets the flag again.
#[test]
fn ferr_clears_the_error_flag_when_it_is_read() {
    let output = run_ppl(
        r#"
        STRING line
        FOPEN 6, "missing.pag", 0, 0
        PRINTLN "first=", FERR(6)
        PRINTLN "second=", FERR(6)
        FGET 6, line
        PRINTLN "after_use=", FERR(6)
    "#,
    );
    assert_eq!(output, "first=1\nsecond=0\nafter_use=1\n");
}

/// The stats PPE of a real board closes a channel and rewinds it right after.
#[test]
fn rewinding_a_closed_channel_does_not_end_the_ppe() {
    let output = run_ppl_with_files(
        r#"
        STRING line
        FOPEN 6, "data.pag", 0, 0
        FGET 6, line
        FCLOSE 6
        FREWIND 6
        PRINTLN "line=", line
        PRINTLN "still running"
    "#,
        &[("data.pag", CONTENT)],
    );
    assert_eq!(output, "line=first line\nstill running\n");
}

#[test]
fn reading_from_a_channel_that_was_never_opened_gives_nothing() {
    let output = run_ppl(
        r#"
        STRING line
        FGET 6, line
        PRINTLN "[", line, "] err=", FERR(6)
    "#,
    );
    assert_eq!(output, "[] err=1\n");
}

/// A channel is free again once it is closed, so the same one can be used twice.
#[test]
fn a_channel_can_be_opened_again_after_it_was_closed() {
    let output = run_ppl_with_files(
        r#"
        STRING line
        FOPEN 6, "data.pag", 0, 0
        FGET 6, line
        FCLOSE 6
        FOPEN 6, "data.pag", 0, 0
        FGET 6, line
        PRINTLN "line=", line, " err=", FERR(6)
        FCLOSE 6
    "#,
        &[("data.pag", CONTENT)],
    );
    assert_eq!(output, "line=first line err=0\n");
}

#[test]
fn text_and_binary_reads_truncate_legacy_strings_but_not_bigstr() {
    let mut content = vec![b' '; 299];
    content.extend_from_slice(b"B\r\n");
    let output = run_ppl_with_files(
        r#";$LANGVERSION 340
        STRING s
        BIGSTR b
        FOPEN 1, "data.pag", O_RD, S_DN
        FGET 1, s
        FCLOSE 1
        PRINTLN "fget300=", LEN(s), ":[", RIGHT(s, 3), "]"
        FOPEN 1, "data.pag", O_RD, S_DN
        FREAD 1, s, 300
        FCLOSE 1
        PRINTLN "fread300=", LEN(s), ":[", RIGHT(s, 3), "]"
        FOPEN 1, "data.pag", O_RD, S_DN
        FREAD 1, b, 300
        FCLOSE 1
        PRINTLN "fread_big300=", LEN(b), ":[", RIGHT(b, 3), "]"
        "#,
        &[("data.pag", &content)],
    );

    assert_eq!(output, "fget300=256:[   ]\nfread300=256:[   ]\nfread_big300=300:[  B]\n");
}

/// FREAD past the end of a file used to index into an empty buffer and take the
/// whole board down with it. `PCBoard` set the error flag and carried on.
#[test]
fn reading_a_byte_past_the_end_sets_the_error_flag() {
    let output = run_ppl_with_files(
        r#"
        BYTE b
        FOPEN 6, "data.pag", 0, 0
        FREAD 6, b, 1
        PRINTLN "first=", b, " err=", FERR(6)
        FREAD 6, b, 1
        PRINTLN "second=", b, " err=", FERR(6), " ok=", Error.Last().OK
        FCLOSE 6
    "#,
        &[("data.pag", b"A")],
    );
    assert_eq!(output, "first=65 err=0\nsecond=0 err=1 ok=1\n");
}

/// The same for a value that needs more bytes than the file has left.
#[test]
fn reading_a_word_from_a_file_with_one_byte_left_does_not_end_the_ppe() {
    let output = run_ppl_with_files(
        r#"
        WORD w
        FOPEN 6, "data.pag", 0, 0
        FREAD 6, w, 2
        PRINTLN "w=", w, " err=", FERR(6)
        PRINTLN "still running"
        FCLOSE 6
    "#,
        &[("data.pag", b"A")],
    );
    assert_eq!(output, "w=0 err=1\nstill running\n");
}
