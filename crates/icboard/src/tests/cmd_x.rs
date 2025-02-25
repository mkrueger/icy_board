use std::path::PathBuf;

use crate::tests::test_output;

// X
#[test]
fn test_extended_mode_toggle() {
    let output = test_output("X\nX\n".to_string(), |_| {});
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mX\n\n\u{1b}[1;37mExpert mode is now on, Sysop ...\n\n\u{1b}[33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mX\n\n\u{1b}[1;37mExpert mode is now off, Sysop ...\n\n\u{1b}[32mPress (Enter) to continue? \u{1b}[0m"
    );
}

#[test]
fn test_extended_mode_on() {
    let output = test_output("x on\n".to_string(), |_| {});
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mx on\n\n\u{1b}[1;37mExpert mode is now on, Sysop ...\n\n\u{1b}[33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0m"
    );
}

#[test]
fn test_extended_mode_invalid() {
    let output = test_output("x invalid\n".to_string(), |_| {});
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mx invalid\n\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0m"
    );
}

#[test]
fn test_extended_mode_off() {
    let output = test_output("X OFF\n".to_string(), |_| {});
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mX OFF\n\n\u{1b}[1;37mExpert mode is now off, Sysop ...\n\n\u{1b}[32mPress (Enter) to continue? \u{1b}[0m"
    );
}

#[test]
fn test_extended_mode_cmd_file() {
    let output = test_output("x on\n".to_string(), |board| {
        board.config.paths.command_display_path = PathBuf::from("src/tests/cmd_files".to_string());
    });
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mx on\nXCMDFILE\n\u{1b}[1;37mExpert mode is now on, Sysop ...\n\n\u{1b}[33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0m"
    );
}
