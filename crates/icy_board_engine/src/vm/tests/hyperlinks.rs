use super::run_ppl;

#[test]
fn url_macros_wrap_link_text_in_osc_8() {
    let output = run_ppl(r#"PRINT "@URL:https://example.com/docs@IcyBoard docs@URL@""#);

    assert_eq!(output, "\x1b]8;;https://example.com/docs\x1b\\IcyBoard docs\x1b]8;;\x1b\\");
}
