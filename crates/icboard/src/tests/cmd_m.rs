use crate::tests::test_output;

#[test]
fn test_cmd_m() {
    let output = test_output("M\n".to_string(), |_| {});
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mM\n\nGraphics mode is now off, Sysop ...\n\nPress (Enter) to continue? "
    );
}

#[test]
fn test_cmd_m_toggle() {
    let output = test_output("M\n\nM\n".to_string(), |_| {});
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mM\n\nGraphics mode is now off, Sysop ...\n\nPress (Enter) to continue? \r\u{1b}[K(1000 min. left) Main Board Command? M\n\n\u{1b}[1;37mGraphics mode is now on, Sysop ...\n\n\u{1b}[32mPress (Enter) to continue? \u{1b}[0m");
}

#[test]
fn test_cmd_m_token() {
    let output = test_output("M ANSI\n".to_string(), |_| {});
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mM ANSI\n\nANSI mode is now on, Sysop ...\n\nPress (Enter) to continue? "
    );
}
