use crate::tests::{fixture, setup_conference, test_login_output, test_ppe_output};
use icy_board_engine::icy_board::icb_config::DisplayNewsBehavior;

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

#[test]
fn a_direct_ppe_has_only_its_output_and_the_completion_prompt() {
    let output = test_ppe_output("PRINT \"PPE ONLY\"", |board| {
        setup_conference(board);
        board.conferences[0].news_file = fixture("main/blt1");
        board.config.paths.welcome = fixture("main/blt2");
        board.config.switches.display_news_behavior = DisplayNewsBehavior::Always;
        board.config.switches.scan_new_blt = true;
    });

    assert_eq!(output, format!("PPE ONLY\n{}\n", icy_board_tui::get_text("run_ppe_completed")));
}
