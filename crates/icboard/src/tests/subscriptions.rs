use chrono::{Duration, Utc};
use icy_board_engine::icy_board::{IcyBoard, conferences::Conference, user_base::ConferenceFlags};

use crate::tests::{fixture, setup_conference, test_login_output, test_ppe_output};

fn enable_subscriptions(board: &mut IcyBoard) {
    setup_conference(board);
    board.config.subscription_info.is_enabled = true;
    board.config.subscription_info.warning_days = 10;
    board.config.paths.expired = fixture("main/blt1");
    board.config.paths.expire_warning = fixture("main/blt2");
}

#[test]
fn an_expired_subscription_uses_the_expired_security_for_the_session() {
    let output = test_ppe_output("PRINT CURSEC()", |board| {
        enable_subscriptions(board);
        board.users[0].security_level = 255;
        board.users[0].exp_security_level = 12;
        board.users[0].expiration_date = Utc::now() - Duration::days(1);
    });
    assert!(output.starts_with("12"), "{output}");
}

#[test]
fn advancing_the_date_restores_the_normal_security_without_rewriting_it() {
    let output = test_ppe_output("PRINT CURSEC()", |board| {
        enable_subscriptions(board);
        board.users[0].security_level = 255;
        board.users[0].exp_security_level = 12;
        board.users[0].expiration_date = Utc::now() + Duration::days(1);
    });
    assert!(output.starts_with("255"), "{output}");
}

#[test]
fn disabled_subscription_mode_ignores_an_expired_date() {
    let output = test_ppe_output("PRINT CURSEC()", |board| {
        enable_subscriptions(board);
        board.config.subscription_info.is_enabled = false;
        board.users[0].security_level = 255;
        board.users[0].exp_security_level = 12;
        board.users[0].expiration_date = Utc::now() - Duration::days(1);
    });
    assert!(output.starts_with("255"), "{output}");
}

#[test]
fn an_expired_login_displays_the_expired_file_and_continues() {
    let output = test_login_output("SYSOP\n\n".to_string(), |board| {
        enable_subscriptions(board);
        board.users[0].exp_security_level = 12;
        board.users[0].expiration_date = Utc::now() - Duration::days(1);
    });
    assert!(output.contains("BULLETIN1"), "{output}");
    assert!(
        output.contains("Main Board Command?"),
        "the expired caller was not allowed to continue:\n{output}"
    );
}

#[test]
fn a_login_inside_the_warning_window_displays_the_warning_file() {
    let output = test_login_output("SYSOP\n\n".to_string(), |board| {
        enable_subscriptions(board);
        board.users[0].expiration_date = Utc::now() + Duration::days(5);
    });
    assert!(output.contains("BULLETIN2"), "{output}");
    assert!(!output.contains("BULLETIN1"), "{output}");
}

#[test]
fn the_expiration_date_itself_does_not_expire_the_caller() {
    let output = test_login_output("SYSOP\n\n".to_string(), |board| {
        enable_subscriptions(board);
        board.users[0].expiration_date = Utc::now();
    });
    assert!(!output.contains("BULLETIN1"), "{output}");
}

#[test]
fn warning_files_can_show_the_days_until_expiration() {
    let output = test_login_output("SYSOP\n\n".to_string(), |board| {
        enable_subscriptions(board);
        let path = board.root_path.join("subscription-warning");
        std::fs::write(&path, "DAYS=@EXPDAYS@ DATE=@EXPDATE@").unwrap();
        board.config.paths.expire_warning = path;
        board.users[0].expiration_date = Utc::now() + Duration::days(5);
    });
    assert!(output.contains("DAYS=5"), "{output}");
    assert!(!output.contains("DATE=00-00-00"), "{output}");
}

fn setup_expired_conferences(board: &mut IcyBoard, flags: ConferenceFlags) {
    enable_subscriptions(board);
    board.conferences.push(Conference {
        name: "Expired Access".to_string(),
        is_public: true,
        ..Default::default()
    });
    board.users[0].exp_security_level = 255;
    board.users[0].expiration_date = Utc::now() - Duration::days(1);
    board.users[0].conference_flags.insert(1, flags);
}

#[test]
fn an_expired_caller_needs_the_x_flag_to_join_a_conference() {
    let output = test_login_output("SYSOP\n\nJ 1\n".to_string(), |board| {
        setup_expired_conferences(board, ConferenceFlags::Registered);
    });
    assert!(output.contains("not registered in Conference"), "{output}");
}

#[test]
fn an_expired_caller_can_join_a_conference_with_r_and_x() {
    let output = test_login_output("SYSOP\n\nJ 1\n".to_string(), |board| {
        setup_expired_conferences(board, ConferenceFlags::Registered | ConferenceFlags::Expired);
    });
    assert!(output.contains("(1) Joined"), "{output}");
    assert!(!output.contains("not registered in Conference"), "{output}");
}
