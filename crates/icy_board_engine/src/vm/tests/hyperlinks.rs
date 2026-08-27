use super::{run_ppl, run_ppl_at_boundary};

#[test]
fn a_url_macro_wraps_its_label_in_osc_8() {
    let output = run_ppl(r#"PRINT "@URL:IcyBoard docs(https://example.com/docs)@""#);

    assert_eq!(output, "\x1b]8;;https://example.com/docs\x1b\\IcyBoard docs\x1b]8;;\x1b\\");
}

#[test]
fn a_url_followed_immediately_by_a_color_macro_closes_both_cleanly() {
    let output = run_ppl(
        r#"PRINTLN "@X08Map: @URL:OpenStreetMap(https://www.openstreetmap.org/copyright)@ / @URL:OpenTopoMap(https://opentopomap.org/credits)@@X07"
           PRINT "@X0Fnext""#,
    );

    assert_eq!(
        output,
        "\x1b[1;30mMap: \x1b]8;;https://www.openstreetmap.org/copyright\x1b\\OpenStreetMap\x1b]8;;\x1b\\ / \
         \x1b]8;;https://opentopomap.org/credits\x1b\\OpenTopoMap\x1b]8;;\x1b\\\x1b[0m\n\x1b[37mnext"
    );
}

#[test]
fn a_ppe_ending_with_a_colored_url_restores_the_caller_color() {
    let output = run_ppl_at_boundary(
        r#"
        PRINTLN "@X08@URL:Foo(https://bar)@"
        EXIT
        "#,
    );

    assert!(output.ends_with("\x1b[0;1;37;44m"), "caller color was not restored: {output:?}");
}

/// A macro that never closes reaches the caller as the text it was written as.
#[test]
fn an_unclosed_url_macro_prints_itself() {
    let output = run_ppl(r#"PRINT "@URL:oops no closing at""#);

    assert_eq!(output, "@URL:oops no closing at");
}
