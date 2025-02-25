use crate::tests::test_output;

#[test]
fn test_cmd_p() {
    let output = test_output("P\n\n".to_string(), |_| {});
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mP\n\n\u{1b}[1;32mPage Length is currently set to 0\n\u{1b}[33mEnter new length (0)=continuous, (Enter)=no change? (\u{1b}[2C)\u{1b}[3D\u{1b}[0m0\u{1b}[1D\n\n\u{1b}[1;32mPress (Enter) to continue? \u{1b}[0m"
    );
}

#[test]
fn test_cmd_p_token() {
    let output = test_output("P 23\n".to_string(), |_| {});
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mP 23\n\u{1b}[1;32mPage Length now set to 23.\n\nPress (Enter) to continue? \u{1b}[0m"
    );
}
