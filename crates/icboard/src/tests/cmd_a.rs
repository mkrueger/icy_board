use crate::tests::test_output;

#[test]
fn test_abandon_empty_confs() {
    let output = test_output("A\n".to_string(), |_| {});
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mA\n\n.\u{1b}[1;31mSorry, Sysop, no Conferences are presently available!\n\n\u{1b}[32mPress (Enter) to continue? \u{1b}[0m"
    );
}
