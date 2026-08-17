use crate::tests::{setup_conference, test_output};
use icy_board_engine::icy_board::IcyBoard;
use icy_board_engine::icy_board::user_base::{ConferenceFlags, User};

/// A second caller so the list has something besides the sysop record.
fn add_callers(board: &mut IcyBoard) {
    setup_conference(board);
    for (name, city) in [("ALICE EXAMPLE", "BERLIN"), ("BOB SAMPLE", "HAMBURG")] {
        let mut user = User::default();
        user.set_name(name.to_string());
        user.city_or_state = city.to_string();
        user.security_level = 10;
        user.conference_flags.insert(0, ConferenceFlags::Registered);
        board.users.push(user);
    }
}

#[test]
fn the_user_list_shows_the_callers() {
    let output = test_output("USERS\n\n".to_string(), add_callers);

    assert!(output.contains("ALICE EXAMPLE"), "a registered caller is missing:\n{output}");
    assert!(output.contains("BOB SAMPLE"), "a registered caller is missing:\n{output}");
}

/// PCBoard lines the name and the location up under its header.
#[test]
fn the_user_list_lines_up_with_its_header() {
    let output = test_output("USERS\n\n".to_string(), add_callers);
    let line = output.lines().find(|line| line.contains("ALICE EXAMPLE")).unwrap_or_default();

    assert!(line.starts_with("ALICE EXAMPLE"), "the name column is indented:\n{line:?}");
    assert_eq!(line.find("BERLIN"), Some(26), "the location does not start in its column:\n{line:?}");
}

/// The search covers the location as well, not just the name.
#[test]
fn the_user_list_searches_the_location_too() {
    let output = test_output("USERS HAMBURG\n".to_string(), add_callers);

    assert!(output.contains("BOB SAMPLE"), "the location was not searched:\n{output}");
    assert!(!output.contains("ALICE EXAMPLE"), "the search did not narrow the list:\n{output}");
}

/// Every token belongs to the search text; only the first was taken before.
#[test]
fn the_user_list_takes_every_token_of_the_search() {
    let output = test_output("USERS ALICE EXAMPLE\n".to_string(), add_callers);

    assert!(output.contains("ALICE EXAMPLE"), "the search text was cut short:\n{output}");
    assert!(!output.contains("BOB SAMPLE"), "the search did not narrow the list:\n{output}");
}

/// A locked out record carries no security level and is not a caller.
#[test]
fn the_user_list_leaves_out_records_without_a_security_level() {
    let output = test_output("USERS\n\n".to_string(), |board| {
        add_callers(board);
        board.users.last_mut().unwrap().security_level = 0;
    });

    assert!(output.contains("ALICE EXAMPLE"), "a registered caller is missing:\n{output}");
    assert!(!output.contains("BOB SAMPLE"), "a record with no security level was listed:\n{output}");
}

#[test]
fn the_user_list_leaves_out_the_fido_account() {
    let output = test_output("USERS\n\n".to_string(), |board| {
        add_callers(board);
        let mut fido = User::default();
        fido.set_name("~FIDO~".to_string());
        fido.security_level = 10;
        fido.conference_flags.insert(0, ConferenceFlags::Registered);
        board.users.push(fido);
    });

    assert!(!output.contains("~FIDO~"), "the FidoNet placeholder was listed as a caller:\n{output}");
}
