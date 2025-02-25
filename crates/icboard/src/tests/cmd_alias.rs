use crate::tests::test_output;

#[test]
fn test_alias_toggle() {
    let output = test_output("ALIAS\n\nALIAS\n".to_string(), |_| {});
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mALIAS\n\n\u{1b}[1;37mHiding identity change, please wait...\n\n\u{1b}[31mChanged name to SYSOP.\nYour true identity is protected while you remain in this conference.\n\n\n\u{1b}[32mPress (Enter) to continue? \u{1b}[0m\r\u{1b}[K\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mALIAS\n\n\u{1b}[1;37mHiding identity change, please wait...\n\n\u{1b}[31mChanged name to SYSOP.\n\n\u{1b}[32mPress (Enter) to continue? \u{1b}[0m"
    );
}

#[test]
fn test_alias_on() {
    let output = test_output("ALIAS ON\n".to_string(), |_| {});
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mALIAS ON\n\n\u{1b}[1;37mHiding identity change, please wait...\n\n\u{1b}[31mChanged name to SYSOP.\nYour true identity is protected while you remain in this conference.\n\n\n\u{1b}[32mPress (Enter) to continue? \u{1b}[0m"
    );
}

#[test]
fn test_alias_off() {
    let output = test_output("ALIAS OFF\n".to_string(), |_| {});
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mALIAS OFF\n\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0m"
    );
}
