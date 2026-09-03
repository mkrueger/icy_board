use super::run_ppl;

#[test]
fn ppl400_substring_members_are_zero_based_and_unbounded() {
    // Substring is zero-based, unlike the one-based classic MID; Left/Right mirror the classics.
    let output = run_ppl(
        r#";$LANGVERSION 400
STRING text = "abcdef"
PRINTLN "substring=[", text.Substring(1, 3), "]"
PRINTLN "classic=[", MID(text, 1, 3), "]"
PRINTLN "left=[", text.Left(3), "]"
PRINTLN "right=[", text.Right(2), "]"
STRING long = STRING.Repeat("x", 300)
PRINTLN "substring_len=", long.Substring(0, 300).Len()
"#,
    );

    assert_eq!(output, "substring=[bcd]\nclassic=[abc]\nleft=[abc]\nright=[ef]\nsubstring_len=300\n");
}

#[test]
fn legacy_bigstr_is_limited_to_2048_unicode_characters() {
    let output = run_ppl(
        r#";$LANGVERSION 400
BIGSTR text
text = STRING.Repeat("ä", 3000)
PRINTLN LEN(text)
PRINTLN text = STRING.Repeat("ä", 2048)
"#,
    );

    assert_eq!(output, "2048\n1\n");
}

#[test]
fn ppl400_strings_keep_their_length_through_classic_functions_and_output() {
    // Classic functions and statements still declare the legacy string types, so a
    // 4.00 STRING has to survive a round trip through them untruncated.
    let output = run_ppl(
        r#";$LANGVERSION 400
DECLARE FUNCTION Echo(STRING value) STRING
STRING long = STRING.Repeat("x", 70000)
PRINTLN "upper=", LEN(UPPER(long))
PRINTLN "classic_mid=", LEN(MID(long, 1, 70000))
PRINTLN "concat=", LEN(long + long)
PRINTLN "roundtrip=", LEN(Echo(long))
STRING printed = STRING.Repeat("y", 5000)
PRINTLN LEN(printed)
FUNCTION Echo(STRING value) STRING
    RETURN value
ENDFUNC
"#,
    );

    assert_eq!(output, "upper=70000\nclassic_mid=70000\nconcat=140000\nroundtrip=70000\n5000\n");
}

#[test]
fn ppl400_string_arrays_and_records_keep_long_values() {
    let output = run_ppl(
        r#";$LANGVERSION 400
TYPE Item
    STRING Text
ENDTYPE
STRING values(1)
values(0) = STRING.Repeat("x", 70000)
PRINTLN "array=", LEN(values(0))
Item value
value.Text = values(0)
PRINTLN "record=", LEN(value.Text)
STRING parts[]
parts = STRING.Split(STRING.Repeat("x", 70000) + "," + "b", ",")
PRINTLN "split=", LEN(parts[0])
"#,
    );

    assert_eq!(output, "array=70000\nrecord=70000\nsplit=70000\n");
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
