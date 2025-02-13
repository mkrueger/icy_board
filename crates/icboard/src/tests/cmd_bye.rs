use std::path::PathBuf;

use crate::tests::test_output;

#[test]
fn test_cmd_bye() {
    let output = test_output("BYE\n".to_string(), |_| {});
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mBYE\n\n\u{1b}[1;36mMinutes Used: 0\n\n\u{1b}[32mThanks for calling, Sysop!\n\u{1b}[0m");
}

#[test]
fn test_cmd_bye_cmdfile() {
    let output = test_output("BYE\n".to_string(), |board| {
        board.config.paths.command_display_path = PathBuf::from("src/tests/cmd_files".to_string());
    });
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mBYE\nBYECMDFILE\n\u{1b}[1;36mMinutes Used: 0\n\n\u{1b}[32mThanks for calling, Sysop!\n\u{1b}[0m");
}
