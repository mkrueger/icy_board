use crate::tests::{setup_conference, test_output};

#[test]
fn personal_mail_scan_defaults_to_quick_when_configured() {
    let output = test_output("Y\nC\n".to_string(), |board| {
        setup_conference(board);
        board.config.message.default_quick_personal_scan = true;
    });
    assert!(output.contains("Total"), "the quick scan header is missing:\n{output}");
}

#[test]
fn personal_mail_scan_defaults_to_long_when_quick_is_disabled() {
    let output = test_output("Y\nC\n".to_string(), |board| {
        setup_conference(board);
        board.config.message.default_quick_personal_scan = false;
    });
    assert!(!output.contains("Total"), "the quick scan header was shown:\n{output}");
}
