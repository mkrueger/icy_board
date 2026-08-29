use super::run_ppl;

#[test]
fn ppl400_substring_members_are_zero_based_and_unbounded() {
    // Mid is zero-based, unlike the one-based classic MID; Left/Right mirror the classics.
    let output = run_ppl(
        r#";$LANGVERSION 400
STRING text = "abcdef"
PRINTLN "mid=[", text.Mid(1, 3), "]"
PRINTLN "classic=[", MID(text, 1, 3), "]"
PRINTLN "left=[", text.Left(3), "]"
PRINTLN "right=[", text.Right(2), "]"
STRING long = STRING.Repeat("x", 300)
PRINTLN "midlen=", long.Mid(0, 300).Len()
"#,
    );

    assert_eq!(output, "mid=[bcd]\nclassic=[abc]\nleft=[abc]\nright=[ef]\nmidlen=300\n");
}

#[test]
fn legacy_string_storage_matches_pcboard_256_character_capacity() {
    let output = run_ppl(
        r#";$LANGVERSION 340
        STRING s, t
        BIGSTR b
        s = SPACE(254) + "A"
        PRINTLN "assign255=", LEN(s), ":[", RIGHT(s, 3), "]"
        s = SPACE(254) + "AB"
        PRINTLN "assign256=", LEN(s), ":[", RIGHT(s, 3), "]"
        s = SPACE(299) + "A"
        PRINTLN "assign300=", LEN(s), ":[", RIGHT(s, 3), "]"
        s = SPACE(250)
        s = s + "12345"
        PRINTLN "concat255=", LEN(s), ":[", RIGHT(s, 7), "]"
        s = s + "67890"
        PRINTLN "concat260=", LEN(s), ":[", RIGHT(s, 7), "]"
        b = SPACE(299) + "B"
        PRINTLN "bigstr300=", LEN(b), ":[", RIGHT(b, 3), "]"
        s = b
        PRINTLN "big_to_string=", LEN(s), ":[", RIGHT(s, 3), "]"
        t = s
        PRINTLN "string_copy=", LEN(t), ":[", RIGHT(t, 3), "]"
        "#,
    );

    assert_eq!(
        output,
        concat!(
            "assign255=255:[  A]\n",
            "assign256=256:[ AB]\n",
            "assign300=256:[   ]\n",
            "concat255=255:[  12345]\n",
            "concat260=256:[ 123456]\n",
            "bigstr300=300:[  B]\n",
            "big_to_string=256:[   ]\n",
            "string_copy=256:[   ]\n",
        )
    );
}

#[test]
fn legacy_string_arrays_parameters_and_returns_match_pcboard_capacity() {
    let output = run_ppl(
        r#";$LANGVERSION 340
        DECLARE FUNCTION ECHO(STRING value) STRING
        DECLARE FUNCTION GROW(STRING value) STRING
        STRING values(1), s
        BIGSTR b
        values(0) = SPACE(254) + "AB"
        PRINTLN "array256=", LEN(values(0)), ":[", RIGHT(values(0), 3), "]"
        b = SPACE(299) + "B"
        s = ECHO(b)
        PRINTLN "param_return300=", LEN(s), ":[", RIGHT(s, 3), "]"
        s = GROW(SPACE(250))
        PRINTLN "return260=", LEN(s), ":[", RIGHT(s, 7), "]"

        FUNCTION ECHO(STRING value) STRING
            PRINTLN "param300=", LEN(value), ":[", RIGHT(value, 3), "]"
            ECHO = value
        ENDFUNC

        FUNCTION GROW(STRING value) STRING
            value = value + "1234567890"
            PRINTLN "local260=", LEN(value), ":[", RIGHT(value, 7), "]"
            GROW = value
        ENDFUNC
        "#,
    );

    assert_eq!(
        output,
        concat!(
            "array256=256:[ AB]\n",
            "param300=256:[   ]\n",
            "param_return300=256:[   ]\n",
            "local260=256:[ 123456]\n",
            "return260=256:[ 123456]\n",
        )
    );
}
