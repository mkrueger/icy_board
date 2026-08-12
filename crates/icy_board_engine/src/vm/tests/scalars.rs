//! The loose scalar functions: DDATE conversion, the event flag, the keyboard
//! script flag and free disk space.

use super::{run_ppl, run_ppl_on};

#[test]
fn test_toddate_reads_a_ccyymmdd_string() {
    assert_eq!(run_ppl("PRINT TODDATE(\"19940527\")"), "19940527");
}

/// PCBACCSTAT field 0 answers 0 when accounting is off and 2 when it is on;
/// icy_board has no separate tracking mode, so an enabled system is fully on.
#[test]
fn test_pcbaccstat_reports_the_accounting_status() {
    assert_eq!(run_ppl("PRINT PCBACCSTAT(0)"), "0");
    let enabled = run_ppl_on("PRINT PCBACCSTAT(0)", |board| {
        board.config.accounting.enabled = true;
    });
    assert_eq!(enabled, "2");
}

/// A request that cannot be made answers empty and lets the program carry on,
/// the way the runtime treats every other call it cannot carry out. The url is
/// malformed, so it fails before anything is sent.
#[test]
fn test_a_web_request_that_fails_does_not_stop_the_program() {
    assert_eq!(run_ppl("PRINT \"[\", WEBREQUEST(\"not a url\"), \"]\""), "[]");
}

/// The same for the statement form, which writes no file and keeps going.
#[test]
fn test_a_failed_web_request_statement_does_not_stop_the_program() {
    assert_eq!(
        run_ppl("WEBREQUEST \"not a url\", \"out.txt\"\nPRINT \"still here\""),
        "still here"
    );
}

/// PCBoard kept a name and a city per node in USERNET, so what WRUNET writes is
/// what UN_NAME and UN_CITY read back.
#[test]
fn test_wrunet_keeps_the_name_and_city_a_ppe_wrote() {
    let output = run_ppl(
        r#"
        WRUNET PCBNODE(), "", "FAKE CALLER", "FAKE CITY", "doing things", ""
        RDUNET PCBNODE()
        PRINTLN "name=", UN_NAME(), " city=", UN_CITY(), " oper=", UN_OPER()
    "#,
    );
    assert_eq!(output, "name=FAKE CALLER city=FAKE CITY oper=doing things\n");
}

/// A DDATE holds the julian date a DATE holds; only its text form is CCYYMMDD.
/// Verified against PCBoard 15.4/M.
#[test]
fn test_a_ddate_holds_the_julian_date_behind_its_ccyymmdd_text() {
    assert_eq!(run_ppl("INTEGER i\ni = TODDATE(\"19940527\")\nPRINT i"), "34480");
    assert_eq!(run_ppl("DDATE d\nd = TODDATE(\"19940527\")\nPRINT d"), "19940527");
}

/// An EDATE holds that same julian and shows itself as YYMM.DD.
#[test]
fn test_an_edate_shows_the_date_as_yymm_dd() {
    assert_eq!(run_ppl("EDATE e\ne = MKDATE(1996, 3, 15)\nPRINT e"), "9603.15");
    assert_eq!(run_ppl("EDATE e\ne = MKDATE(1996, 3, 15)\nPRINT TOINTEGER(e)"), "35138");
}

/// PCBoard does not read a date out of a string for an EDATE, it answers 0.
#[test]
fn test_an_edate_does_not_read_a_date_out_of_a_string() {
    assert_eq!(run_ppl("PRINT TOEDATE(\"03-15-96\")"), "0000.00");
}

#[test]
fn test_toddate_converts_a_date() {
    assert_eq!(run_ppl("PRINT TODDATE(MKDATE(1994, 5, 27))"), "19940527");
}

#[test]
fn test_a_date_survives_the_trip_through_ddate_and_back() {
    assert_eq!(run_ppl("PRINT TODATE(TODDATE(MKDATE(1994, 5, 27)))"), "05/27/94");
}

#[test]
fn test_a_ddate_variable_takes_a_date_by_assignment() {
    assert_eq!(run_ppl("DDATE d\nd = MKDATE(2001, 12, 31)\nPRINT d"), "20011231");
}

#[test]
fn test_no_event_has_taken_time_away() {
    assert_eq!(run_ppl("PRINT EVTTIMEADJ()"), "0");
}

#[test]
fn test_adjtime_adds_time_while_no_event_is_pending() {
    assert_eq!(run_ppl("ADJTIME 10\nPRINT MINLEFT()"), "1010");
}

#[test]
fn test_no_keyboard_script_is_running_to_start_with() {
    assert_eq!(run_ppl("PRINT KBDFILUSED()"), "0");
}

#[test]
fn test_kbdstuff_is_not_a_keyboard_script() {
    assert_eq!(run_ppl("KBDSTUFF \"X\"\nPRINT KBDFILUSED()"), "0");
}

#[test]
fn test_kbdfile_is_a_keyboard_script() {
    assert_eq!(
        run_ppl("FCREATE 1, \"S.KBD\", O_WR, S_DN\nFPUTLN 1, \"HELLO\"\nFCLOSE 1\nKBDFILE \"S.KBD\"\nPRINT KBDFILUSED()"),
        "1"
    );
}

#[test]
fn test_drivespace_reports_room_on_the_drive_the_board_is_on() {
    assert_eq!(run_ppl("PRINT DRIVESPACE(\"C:\\\\\") > 0"), "1");
}

#[test]
fn test_drivespace_reports_nothing_for_a_path_that_is_not_there() {
    assert_eq!(run_ppl("PRINT DRIVESPACE(\"NOSUCHDIR\")"), "0");
}
