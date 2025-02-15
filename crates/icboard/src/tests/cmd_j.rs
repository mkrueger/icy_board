use crate::tests::{setup_conference, test_output};

#[test]
fn test_cmd_j_empty_confs() {
    let output = test_output("J 1\n".to_string(), |_| {});
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mJ 1\n\n.\u{1b}[1;31mSorry, Sysop, no Conferences are presently available!\n\n\u{1b}[32mPress (Enter) to continue? \u{1b}[0m");
}

#[test]
fn test_cmd_j_join() {
    let output = test_output("J 1\n".to_string(), |board| {
        setup_conference(board);
    });
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mJ 1\n\n\u{1b}[1;32mTESTCONF (1) Joined\n\nPress (Enter) to continue? \u{1b}[0m");
}

#[test]
fn test_cmd_j_abandon() {
    let output = test_output("J 1\n\nJ 0\n".to_string(), |board| {
        setup_conference(board);
    });
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mJ 1\n\n\u{1b}[1;32mTESTCONF (1) Joined\n\nPress (Enter) to continue? \u{1b}[0m\r\u{1b}[K\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) TESTCONF (1) Conference Command? \u{1b}[0mJ 0\n\n\u{1b}[1;36mTESTCONF (1) Abandoned\n\n\u{1b}[32mPress (Enter) to continue? \u{1b}[0m");
}
