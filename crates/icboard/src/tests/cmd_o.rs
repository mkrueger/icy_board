use crate::tests::test_output;

#[test]
fn test_cmd_o_offers_a_comment_when_the_sysop_is_away() {
    let output = test_output("O\nN\n\n".to_string(), |board| {
        board.config.options.page_bell = false;
    });
    assert!(output.contains("Sysop is"), "{output}");
    assert!(output.contains("comment"), "{output}");
}
