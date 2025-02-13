use crate::tests::{setup_conference, test_output};

// B
#[test]
fn test_blt_empty() {
    let output = test_output("B\n".to_string(), |_| {});
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mB\n\n.\u{1b}[1;31mSorry, Sysop, no Bulletins are presently available.\n\n\u{1b}[32mPress (Enter) to continue? \u{1b}[0m");
}

#[test]
fn test_blt() {
    let output = test_output("B\n".to_string(), |board| {
        setup_conference(board);
    });
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mB\n1) BLT1 2) BLT2\n\u{1b}[1;33m(H)elp, (1-2), Bulletin List Command? \u{1b}[0m");
}

#[test]
fn test_blt_show1() {
    let output = test_output("B\n1\n".to_string(), |board| {
        setup_conference(board);
    });
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mB\n1) BLT1 2) BLT2\n\u{1b}[1;33m(H)elp, (1-2), Bulletin List Command? \u{1b}[0m1\nBULLETIN1\n\u{1b}[1;33m(H)elp, (1-2), Bulletin List Command? \u{1b}[0m");
}

#[test]
fn test_blt_show2() {
    let output = test_output("B 2\n".to_string(), |board| {
        setup_conference(board);
    });
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mB 2\nBULLETIN2\n\n\u{1b}[1;33m(H)elp, (1-2), Bulletin List Command? \u{1b}[0m");
}

#[test]
fn test_blt_show12() {
    let output = test_output("B 1 2\n".to_string(), |board| {
        setup_conference(board);
    });
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mB 1 2\nBULLETIN1BULLETIN2\n\n\u{1b}[1;33m(H)elp, (1-2), Bulletin List Command? \u{1b}[0m");
}

#[test]
fn test_blt_show_invalid() {
    let output = test_output("B 42\n".to_string(), |board| {
        setup_conference(board);
    });
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mB 42\n\n\u{1b}[1;31mSorry, Sysop, you entered an invalid Bulletin #!\n\n\n\u{1b}[33m(H)elp, (1-2), Bulletin List Command? \u{1b}[0m");
}

#[test]
fn test_blt_a_subcommand() {
    let output = test_output("B A\n".to_string(), |board| {
        setup_conference(board);
    });
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mB A\nBULLETIN1BULLETIN2\n\n\u{1b}[1;33m(H)elp, (1-2), Bulletin List Command? \u{1b}[0m");
}
