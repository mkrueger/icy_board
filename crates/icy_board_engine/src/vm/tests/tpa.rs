//! The third party application store, checked through what a PPE can observe.

use super::run_ppl;

#[test]
fn test_a_keyword_nobody_wrote_reads_back_empty() {
    assert_eq!(run_ppl("STRING s\nTPAGET \"NOSUCH\", s\nPRINT \"[\", s, \"]\""), "[]");
}

#[test]
fn test_tpaget_reads_back_what_tpaput_stored() {
    assert_eq!(run_ppl("STRING s\nTPAPUT \"GAME\", \"level 5\"\nTPAGET \"GAME\", s\nPRINT s"), "level 5");
}

#[test]
fn test_a_keyword_is_matched_without_regard_to_case() {
    assert_eq!(run_ppl("STRING s\nTPAPUT \"Game\", \"hello\"\nTPAGET \"GAME\", s\nPRINT s"), "hello");
}

#[test]
fn test_two_keywords_do_not_share_a_record() {
    assert_eq!(
        run_ppl("STRING s\nTPAPUT \"ONE\", \"first\"\nTPAPUT \"TWO\", \"second\"\nTPAGET \"ONE\", s\nPRINT s"),
        "first"
    );
}

#[test]
fn test_writing_a_keyword_twice_replaces_the_record() {
    assert_eq!(
        run_ppl("STRING s\nTPAPUT \"GAME\", \"old\"\nTPAPUT \"GAME\", \"new\"\nTPAGET \"GAME\", s\nPRINT s"),
        "new"
    );
}

#[test]
fn test_tparead_gives_back_the_type_the_variable_was_declared_with() {
    assert_eq!(
        run_ppl("INTEGER score\nTPAWRITE \"SCORE\", 50000\nTPAREAD \"SCORE\", score\nPRINT score * 2"),
        "100000"
    );
}

#[test]
fn test_a_conference_record_is_separate_from_the_static_one() {
    assert_eq!(
        run_ppl("STRING s\nTPAPUT \"GAME\", \"static\"\nTPACPUT \"GAME\", \"in conference\", 3\nTPAGET \"GAME\", s\nPRINT s"),
        "static"
    );
}

#[test]
fn test_each_conference_keeps_its_own_record() {
    assert_eq!(
        run_ppl("STRING s\nTPACPUT \"GAME\", \"three\", 3\nTPACPUT \"GAME\", \"four\", 4\nTPACGET \"GAME\", s, 3\nPRINT s"),
        "three"
    );
}

#[test]
fn test_a_conference_nobody_wrote_for_reads_back_empty() {
    assert_eq!(
        run_ppl("STRING s\nTPACPUT \"GAME\", \"three\", 3\nTPACGET \"GAME\", s, 9\nPRINT \"[\", s, \"]\""),
        "[]"
    );
}

#[test]
fn test_tpacread_gives_back_the_type_the_variable_was_declared_with() {
    assert_eq!(
        run_ppl("INTEGER level\nTPACWRITE \"LEVEL\", 7, 2\nTPACREAD \"LEVEL\", level, 2\nPRINT level + 1"),
        "8"
    );
}
