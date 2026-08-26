use super::run_ppl;

#[test]
fn tolong_is_64_bit_in_language_400() {
    let output = run_ppl(
        r#"
        ;$LANGVERSION 400
        LONG value = ToLong(4294967295)
        PrintLn value, " ", value + 1, " ", value > 10000
        "#,
    );

    assert_eq!(output, "4294967295 4294967296 1\n");
}

#[test]
fn tolong_keeps_its_legacy_width_before_language_400() {
    let output = run_ppl(
        r"
        ;$LANGVERSION 340
        PrintLn ToLong(4294967295)
        ",
    );

    assert_eq!(output, "-1\n");
}

#[test]
fn toulong_converts_the_full_unsigned_range() {
    let output = run_ppl(
        r#"
        ;$LANGVERSION 400
        ULONG value = ToULong("18446744073709551615")
        PrintLn value
        "#,
    );

    assert_eq!(output, "18446744073709551615\n");
}
