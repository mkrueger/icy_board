//! The loose scalar functions: DDATE conversion, the event flag, the keyboard
//! script flag and free disk space.

use super::run_ppl;

#[test]
fn test_toddate_reads_a_ccyymmdd_string() {
    assert_eq!(run_ppl("PRINT TODDATE(\"19940527\")"), "19940527");
}

#[test]
fn test_a_ddate_holds_the_date_as_the_number_ccyymmdd() {
    assert_eq!(run_ppl("INTEGER i\ni = TODDATE(\"19940527\")\nPRINT i"), "19940527");
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
