//! The loose scalar functions: DDATE conversion, the event flag, the keyboard
//! script flag and free disk space.

use super::{compile_errors, run_ppl, run_ppl_on};

#[test]
fn test_toddate_reads_a_ccyymmdd_string() {
    assert_eq!(run_ppl("PRINT TODDATE(\"19940527\")"), "19940527");
}

/// `ABS` answers in the type it was given rather than truncating to a whole number,
/// the way `cVARVAL::abs` does; only the unsigned and date family folds to an integer.
#[test]
fn test_abs_keeps_the_type_of_its_argument() {
    let output = run_ppl(
        r#"
        DOUBLE d
        SWORD w
        d = 0.0 - 2.5
        w = 0 - 300
        PRINTLN Abs(d)
        PRINTLN Abs(0.0 - 1.75)
        PRINTLN Abs(0 - 3)
        PRINTLN Abs(w)
        PRINTLN Abs(3)
        "#,
    );

    assert_eq!(output, "2.5\n1.75\n3\n300\n3\n");
}

/// Arithmetic is evaluated without the async walk, so an expression that mixes it with a
/// call has to fall back and still answer the same. `&` and `|` evaluate both sides either
/// way, which a counting function makes visible.
#[test]
fn test_expressions_mixing_arithmetic_and_calls_evaluate_the_same() {
    let output = run_ppl(
        r#"
        INTEGER calls, values(3)
        values[0] = 10
        values[1] = 20
        values[2] = 30
        values[3] = 40
        PRINTLN 2 * 3 + 4
        PRINTLN values[1 + 1] + 5
        PRINTLN Bump(1) * 2 + values[Bump(0) - 1]
        PRINTLN calls
        calls = 0
        PRINTLN (Bump(1) > 0) | (Bump(1) > 0)
        PRINTLN calls
        EXIT

        FUNCTION Bump(INTEGER add) INTEGER
            calls = calls + 1
            Bump = add + 1
        ENDFUNC
        "#,
    );

    assert_eq!(output, "10\n35\n14\n2\n1\n2\n");
}

#[test]
fn test_mixed_strings_and_numbers_promote_to_integer() {
    let output = run_ppl(
        r#"
STRING value
value = "2"
PRINTLN value + 1
PRINTLN 1 + value
PRINTLN value + value
PRINTLN value - 1
PRINTLN value * 3
PRINTLN value / 2
PRINTLN value % 2
PRINTLN value = 2
PRINTLN value < 10
INC value
PRINTLN value
DEC value
PRINTLN value
PRINTLN " 7x" - "2x"
PRINTLN " 7x" / "2x"
PRINTLN " 7x" % "2x"
PRINTLN -" 7x"
PRINTLN ABS(" -7x")
"#,
    );

    assert_eq!(output, "3\n3\n22\n1\n6\n1\n0\n1\n1\n3\n2\n5\n3\n1\n-7\n7\n");
}

#[test]
fn test_division_and_modulo_by_zero_answer_zero() {
    assert_eq!(run_ppl("PRINTLN 7 / 0\nPRINTLN 7 % 0\nPRINTLN \"7\" / \"0\""), "0\n0\n0\n");
}

#[test]
fn test_signed_small_integers_compare_as_signed() {
    let output = run_ppl(
        r#"
SBYTE byte_value
SWORD word_value
byte_value = -1
word_value = -1
PRINTLN byte_value < 1
PRINTLN word_value < 1
"#,
    );

    assert_eq!(output, "1\n1\n");
}

#[test]
fn test_function_name_assignment_returns_a_value_in_classic_languages() {
    for language_version in [300, 310, 320, 330, 340] {
        let source = format!(
            ";$LANGVERSION {language_version}\n\
             DECLARE FUNCTION LegacyAddOne(INTEGER value) INTEGER\n\
             PRINT LegacyAddOne(41)\n\
             END\n\
             FUNCTION LegacyAddOne(INTEGER value) INTEGER\n\
               LegacyAddOne = value + 1\n\
             ENDFUNC\n"
        );

        assert_eq!(run_ppl(&source), "42", "language version {language_version}");
    }
}

#[test]
fn test_a_function_can_read_and_rewrite_its_return_value() {
    let source = r#"
DECLARE FUNCTION NumberResult(INTEGER value) INTEGER
DECLARE FUNCTION StringResult(STRING value) STRING
PRINTLN NumberResult(1)
PRINTLN StringResult("ABCDEFGHIJKLMNOPQRST")
EXIT

FUNCTION NumberResult(INTEGER value) INTEGER
    NumberResult = value
    IF (NumberResult = 1) NumberResult = NumberResult + 4
ENDFUNC

FUNCTION StringResult(STRING value) STRING
    StringResult = value
    IF (LEN(StringResult) > 17) StringResult = LEFT(StringResult, 17)
ENDFUNC
"#;

    assert_eq!(run_ppl(source), "5\nABCDEFGHIJKLMNOPQ\n");
}

/// PCBACCSTAT field 0 answers 0 when accounting is off and 2 when it is on;
/// `icy_board` has no separate tracking mode, so an enabled system is fully on.
#[test]
fn test_pcbaccstat_reports_the_accounting_status() {
    assert_eq!(run_ppl("PRINT PCBACCSTAT(0)"), "0");
    let enabled = run_ppl_on("PRINT PCBACCSTAT(0)", |board| {
        board.config.accounting.enabled = true;
    });
    assert_eq!(enabled, "2");
}

#[test]
fn test_a_door_password_can_be_compared_but_not_printed() {
    let output = run_ppl_on(
        r#"
        CONFERENCE conf
        DOOR item
        conf = Board.Conferences[0]
        item = conf.Doors[0]
        PRINT "[", item.Password, "] ", item.Password = "SeCrEt", " ", item.Password = "wrong"
    "#,
        |board| {
            board.conferences.clear();
            board.conferences.push(crate::icy_board::conferences::Conference {
                doors: Some(std::sync::Arc::new(crate::icy_board::doors::DoorList {
                    doors: vec![crate::icy_board::doors::Door {
                        password: "secret".to_string(),
                        ..Default::default()
                    }],
                    ..Default::default()
                })),
                ..Default::default()
            });
        },
    );
    assert_eq!(output, "[******] 1 0");
}

/// Reading the password must not hash it: a key derivation costs milliseconds and
/// megabytes a call, which a loop over the doors would spend for nothing.
#[test]
fn test_reading_a_door_password_stays_cheap() {
    let start = std::time::Instant::now();
    let output = run_ppl_on(
        r#"
        CONFERENCE conf
        DOOR item
        INTEGER i, hits
        conf = Board.Conferences[0]
        item = conf.Doors[0]
        FOR i = 1 TO 200
            IF item.Password = "secret" hits = hits + 1
        NEXT
        PRINT hits
    "#,
        |board| {
            board.conferences.clear();
            board.conferences.push(crate::icy_board::conferences::Conference {
                doors: Some(std::sync::Arc::new(crate::icy_board::doors::DoorList {
                    doors: vec![crate::icy_board::doors::Door {
                        password: "secret".to_string(),
                        ..Default::default()
                    }],
                    ..Default::default()
                })),
                ..Default::default()
            });
        },
    );
    assert_eq!(output, "200");
    // Argon2 would need seconds for this, comparing the secret needs microseconds.
    assert!(start.elapsed() < std::time::Duration::from_secs(2), "took {:?}", start.elapsed());
}

#[test]
fn test_conference_properties_report_configuration_and_counts() {
    let output = run_ppl_on(
        r#"
        CONFERENCE conf
        conf = Board.Conferences[0]
        PRINT conf.IsPublic, " ", conf.HasAccess(), " ", conf.Directories.Count, " ", conf.Areas.Count, " ", conf.Doors.Count
    "#,
        |board| {
            board.conferences.clear();
            board.conferences.push(crate::icy_board::conferences::Conference {
                is_public: false,
                areas: Some(std::sync::Arc::new(crate::icy_board::message_area::AreaList::new(vec![
                    crate::icy_board::message_area::MessageArea::default(),
                    crate::icy_board::message_area::MessageArea::default(),
                ]))),
                doors: Some(std::sync::Arc::new(crate::icy_board::doors::DoorList {
                    doors: vec![crate::icy_board::doors::Door::default(), crate::icy_board::doors::Door::default()],
                    ..Default::default()
                })),
                ..Default::default()
            });
        },
    );
    assert_eq!(output, "0 1 0 2 2");
}

#[test]
fn test_an_invalid_conference_number_still_returns_a_conference() {
    let output = run_ppl(
        r#"
        CONFERENCE conf
        conf = Board.Conferences[999]
        PRINT "[", conf.Name, "] ", conf.IsPublic, " ", conf.Directories.Count, " ", conf.Areas.Count, " ", conf.Doors.Count
    "#,
    );
    assert_eq!(output, "[] 0 0 0 0");
}

#[test]
fn test_len_dimension_returns_each_dimension_element_count() {
    let output = run_ppl(
        r#"
        INTEGER one(10)
        INTEGER two(2, 3)
        INTEGER three(1, 2, 3)
        PRINT one.Len(), " ", LEN(one, 0), " ", LEN(two, 0), " ", LEN(two, 1), " ", LEN(three, 0), " ", LEN(three, 1), " ", LEN(three, 2)
    "#,
    );
    assert_eq!(output, "11 11 3 4 2 3 4");
}

#[test]
fn test_ppl400_string_member_positions_are_zero_based() {
    let output = run_ppl(
        r#"
        BIGSTR text = "ä two two"
        PRINTLN text.Find("ä"), " ", text.Find("two"), " ", text.Find("two", 3)
        PRINTLN text.FindLast("two"), " ", text.FindLast("two", 4)
        PRINTLN text.Find("missing"), " ", text.FindLast("missing")
        PRINTLN INSTR(text, "two"), " ", INSTRR(text, "two")
        "#,
    );
    assert_eq!(output, "0 2 6\n6 2\n-1 -1\n3 7\n");
}

#[test]
fn test_ppl400_scalar_strings_support_zero_based_character_indices() {
    let output = run_ppl(
        r#"
        BIGSTR text = "Aäß"
        PRINTLN "[", text[0], "][", text[1], "][", text[2], "]"
        PRINTLN "[", text[-1], "][", text[3], "]"
        PRINTLN "xy"[1], " ", " z ".Trim()[0]
        STRING words[0]
        words[0] = "whole"
        PRINTLN words[0], " ", words[0][0], words[0][4]
        "#,
    );
    assert_eq!(output, "[A][ä][ß]\n[][]\ny z\nwhole we\n");

    let errors = compile_errors(";$LANGVERSION 340\nSTRING text = \"abc\"\nPRINTLN text[0]");
    assert!(!errors.is_empty(), "scalar string indexing should require language 400");
}

#[test]
fn test_ppl400_string_comparison_controls_search_and_equality() {
    let output = run_ppl(
        r#"
        BIGSTR text = "Ä One ONE"
        PRINTLN text.Find("one"), " ", text.Find("one", 0, StringComparison.OrdinalIgnoreCase)
        PRINTLN text.FindLast("one", 8, StringComparison.OrdinalIgnoreCase), " ", text.FindLast("one", 5, StringComparison.OrdinalIgnoreCase)
        PRINTLN text.Contains("one"), " ", text.Contains("one", StringComparison.OrdinalIgnoreCase)
        PRINTLN text.StartsWith("ä", StringComparison.OrdinalIgnoreCase), " ", text.EndsWith("one", StringComparison.OrdinalIgnoreCase)
        PRINTLN text.Count("one"), " ", text.Count("one", StringComparison.OrdinalIgnoreCase)
        PRINTLN "Äpfel".Equals("äPFEL"), " ", "Äpfel".Equals("äPFEL", StringComparison.OrdinalIgnoreCase)
        "#,
    );
    assert_eq!(output, "-1 2\n6 2\n0 1\n1 1\n0 2\n0 1\n");

    let errors = compile_errors("PRINTLN \"text\".Contains(\"x\", 1)");
    assert!(errors.iter().any(|error| error.contains("StringComparison")), "{errors:?}");
}

#[test]
fn test_ppl400_string_split_returns_a_dynamic_bigstr_array() {
    let output = run_ppl(
        r#"
        BIGSTR text = "one,,two,three"
        BIGSTR parts[]
        parts = text.Split(",")
        PRINTLN parts.Len(), " ", parts[0], "[", parts[1], "]", parts[2], " ", parts[3]
        PRINTLN text.Split(",", 3).Len(), " ", text.Split(",", 3)[2]
        BIGSTR part
        FOREACH part IN STRING.Split("a:b:c", ":")
            PRINT part
        ENDFOREACH
        BIGSTR invalid[]
        invalid = text.Split("")
        PRINTLN " ", invalid.Len(), " ", Error.Last().Kind = ErrKind.String, " ", Error.Last().Code = ErrCode.Invalid
        "#,
    );
    assert_eq!(output, "4 one[]two three\n3 two,three\nabc 0 1 1\n");

    let errors = compile_errors("STRING parts[]\n\"a,b\".Split(\",\", parts)");
    assert!(!errors.is_empty(), "the removed output-array Split signature should not compile");
}

/// An object is held by the values that name it rather than by a table that only
/// ever grows, so a loop can keep asking for one and reading it back.
#[test]
fn test_a_loop_can_keep_asking_for_objects() {
    let output = run_ppl_on(
        r#"
        CONFERENCE conf
        AREA first
        INTEGER i, seen
        FOR i = 1 TO 500
            conf = Board.Conferences[0]
            first = conf.Areas[0]
            IF first.Name = "General" seen = seen + 1
        NEXT
        PRINT seen, " ", conf.Name
    "#,
        |board| {
            board.conferences.clear();
            board.conferences.push(crate::icy_board::conferences::Conference {
                name: "Main".to_string(),
                areas: Some(std::sync::Arc::new(crate::icy_board::message_area::AreaList::new(vec![
                    crate::icy_board::message_area::MessageArea {
                        name: "General".to_string(),
                        ..Default::default()
                    },
                ]))),
                ..Default::default()
            });
        },
    );
    assert_eq!(output, "500 Main");
}

/// `PCBoard` kept a name and a city per node in USERNET, so what WRUNET writes is
/// what `UN_NAME` and `UN_CITY` read back.
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
/// Verified against `PCBoard` 15.4/M.
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

/// `PCBoard` does not read a date out of a string for an EDATE, it answers 0.
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
