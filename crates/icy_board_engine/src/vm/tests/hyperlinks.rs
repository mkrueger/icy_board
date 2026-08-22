use super::run_ppl;

#[test]
fn a_url_macro_wraps_its_label_in_osc_8() {
    let output = run_ppl(r#"PRINT "@URL:IcyBoard docs(https://example.com/docs)@""#);

    assert_eq!(output, "\x1b]8;;https://example.com/docs\x1b\\IcyBoard docs\x1b]8;;\x1b\\");
}

/// A macro that never closes reaches the caller as the text it was written as.
#[test]
fn an_unclosed_url_macro_prints_itself() {
    let output = run_ppl(r#"PRINT "@URL:oops no closing at""#);

    assert_eq!(output, "@URL:oops no closing at");
}
