use crate::tests::test_login_output;

fn setup_login(board: &mut icy_board_engine::icy_board::IcyBoard, allow_comment: bool) {
    board.config.paths.welcome = crate::tests::fixture("main/blt1");
    board.config.system_control.allow_password_failure_comment = allow_comment;
}

#[test]
fn a_failed_password_can_offer_a_sysop_comment() {
    let output = test_login_output("SYSOP\nWRONG\nWRONG\nWRONG\nWRONG\nN\n".to_string(), |board| {
        setup_login(board, true);
    });
    assert!(
        output.contains("leave a comment to the sysop"),
        "the password failure comment was not offered:\n{output}"
    );
}

#[test]
fn a_failed_password_does_not_offer_a_comment_when_disabled() {
    let output = test_login_output("SYSOP\nWRONG\nWRONG\nWRONG\nWRONG\n".to_string(), |board| {
        setup_login(board, false);
    });
    assert!(
        !output.contains("leave a comment to the sysop"),
        "the password failure comment was offered:\n{output}"
    );
}
