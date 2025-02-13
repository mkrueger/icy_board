use crate::tests::test_output;

#[test]
fn test_cmd_abort() {
    let output = test_output("C\n\n".to_string(), |_| {});
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mC\n\n\u{1b}[1;37mLeave a comment for the sysop (Enter)=no? (\u{1b}[1C)\u{1b}[2D\u{1b}[0mN\u{1b}[1D\n\n\u{1b}[1;32mPress (Enter) to continue? \u{1b}[0m");
}
