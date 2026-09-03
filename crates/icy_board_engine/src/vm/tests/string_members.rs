use super::run_ppl;

#[test]
fn ppl400_string_pad_left_and_right_pad_with_space_or_a_given_character() {
    let output = run_ppl(
        r#";$LANGVERSION 400
STRING text = "ab"
PRINTLN "[", text.PadLeft(5), "]"
PRINTLN "[", text.PadLeft(5, "*"), "]"
PRINTLN "[", text.PadRight(5), "]"
PRINTLN "[", text.PadRight(5, "*"), "]"
PRINTLN "[", text.PadLeft(1), "]"
"#,
    );
    assert_eq!(output, "[   ab]\n[***ab]\n[ab   ]\n[ab***]\n[ab]\n");
}

#[test]
fn ppl400_string_remove_deletes_a_zero_based_span() {
    let output = run_ppl(
        r#";$LANGVERSION 400
STRING text = "abcdef"
PRINTLN text.Remove(2, 3)
PRINTLN text.Remove(0, 100)
PRINTLN text.Remove(10, 2)
"#,
    );
    assert_eq!(output, "abf\n\nabcdef\n");
}

#[test]
fn ppl400_string_insert_places_a_value_at_a_zero_based_index() {
    let output = run_ppl(
        r#";$LANGVERSION 400
STRING text = "abcdef"
PRINTLN text.Insert(2, "XYZ")
PRINTLN text.Insert(0, "XYZ")
PRINTLN text.Insert(100, "XYZ")
"#,
    );
    assert_eq!(output, "abXYZcdef\nXYZabcdef\nabcdefXYZ\n");
}

#[test]
fn ppl400_string_reverse_reverses_unicode_characters() {
    let output = run_ppl(
        r#";$LANGVERSION 400
STRING text = "abcdef"
PRINTLN text.Reverse()
PRINTLN "abc".Reverse()
"#,
    );
    assert_eq!(output, "fedcba\ncba\n");
}

#[test]
fn ppl400_string_to_int_parses_with_a_default_or_explicit_base() {
    let output = run_ppl(
        r#";$LANGVERSION 400
PRINTLN "42".ToInt()
PRINTLN "ff".ToInt(16)
PRINTLN "".ToInt()
"#,
    );
    assert_eq!(output, "42\n255\n0\n");
}

#[test]
fn ppl400_string_to_mixed_case_title_cases_words() {
    let output = run_ppl(
        r#";$LANGVERSION 400
PRINTLN "hello world".ToMixedCase()
"#,
    );
    assert_eq!(output, "Hello World\n");
}

#[test]
fn ppl400_string_stripatx_removes_at_x_codes() {
    let output = run_ppl(
        r#";$LANGVERSION 400
STRING text = "@X0Fhello@X07 world"
PRINTLN text.StripATX()
PRINTLN STRIPATX(text)
"#,
    );
    assert_eq!(output, "hello world\nhello world\n");
}
