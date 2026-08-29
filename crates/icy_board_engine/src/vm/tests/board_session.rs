use crate::icy_board::{
    conferences::Conference,
    user_base::{User, UserContact},
};

use super::{compile_errors_with_runtime, run_ppl, run_ppl_on};

fn seed_board(board: &mut crate::icy_board::IcyBoard) {
    board.config.board.name = "Icy Board".to_string();
    board.config.board.location = "Somewhere".to_string();
    board.config.board.operator = "The Operator".to_string();
    board.config.board.num_nodes = 4;
    board.config.sysop.name = "The Sysop".to_string();
    board.conferences.clear();
    board.conferences.push(Conference {
        name: "Main Board".to_string(),
        ..Default::default()
    });
    board.conferences.push(Conference {
        name: "Second".to_string(),
        ..Default::default()
    });
}

#[test]
fn board_and_session_require_runtime_400() {
    for runtime in [330, 340] {
        let errors = compile_errors_with_runtime("PRINTLN Board.Name", runtime);
        assert!(
            errors.iter().any(|error| error.contains("Board needs runtime 400")),
            "runtime {runtime}: {errors:?}"
        );

        let errors = compile_errors_with_runtime("PRINTLN Session.Node", runtime);
        assert!(
            errors.iter().any(|error| error.contains("Session needs runtime 400")),
            "runtime {runtime}: {errors:?}"
        );
    }
    assert!(compile_errors_with_runtime("PRINTLN Board.Name, Session.Node", 400).is_empty());
}

#[test]
fn board_reports_what_the_board_is_configured_to_be() {
    let output = run_ppl_on(
        r#"
        PrintLn Board.Name
        PrintLn Board.Location
        PrintLn Board.Operator
        PrintLn Board.SysopName
        PrintLn Board.NodeCount, " ", Board.Conferences.Len()
        "#,
        seed_board,
    );

    assert_eq!(output, "Icy Board\nSomewhere\nThe Operator\nThe Sysop\n4 2\n");
}

/// The count and the accessor together are what lets a PPE walk the board
/// without `HIGHCONFNUM()`.
#[test]
fn every_conference_can_be_reached_by_number() {
    let output = run_ppl_on(
        r#"
        INTEGER i
        FOR i = 0 TO Board.Conferences.Len() - 1
            CONFERENCE conf = Board.Conferences[i]
            PrintLn i, ": ", conf.Name, " ", conf.HasAccess()
        NEXT
        "#,
        seed_board,
    );

    assert_eq!(output, "0: Main Board 1\n1: Second 1\n");
}

#[test]
fn board_snapshot_collections_are_typed_arrays() {
    let output = run_ppl_on(
        r#"
        CONFERENCE conferences[]
        USER users[]
        AREA areas[]
        DIRECTORY directories[]
        DOOR doors[]
        conferences = Board.Conferences
        users = Board.Users
        areas = conferences[0].Areas
        directories = conferences[0].Directories
        doors = conferences[0].Doors
        PrintLn conferences.Len(), " ", users.Len(), " ", areas.Len(), " ", directories.Len(), " ", doors.Len()
        "#,
        |board| {
            seed_board(board);
            board.users.clear();
            board.users.new_user(User::default());
        },
    );

    assert_eq!(output, "2 1 1 0 0\n");
}

#[test]
fn every_user_can_be_read_as_an_independent_snapshot() {
    let output = run_ppl_on(
        r#"
        PrintLn Board.Users.Len()
        PrintLn Board.Users[0].Name, " ", Board.Users[0].City
        PrintLn Board.Users[1].Name, " ", Board.Users[1].Notes[0]
        PrintLn Board.Users[1].Contacts[0].Service, " ", Board.Users[1].Contacts[0].Account
        PrintLn Board.Users[99].Valid, " [", Board.Users[99].Name, "]"
        Board.Users[0].City = "Changed"
        PrintLn Error.Last().OK, " ", Board.Users[0].City
        "#,
        |board| {
            board.users.clear();
            let mut first = User {
                name: "Alice".to_string(),
                city_or_state: "Berlin".to_string(),
                ..Default::default()
            };
            first.custom_comment1 = "First note".to_string();
            board.users.new_user(first);

            let mut second = User {
                name: "Bob".to_string(),
                ..Default::default()
            };
            second.custom_comment1 = "Second note".to_string();
            second.contacts.push(UserContact {
                service: "matrix".to_string(),
                account: "@bob:example.org".to_string(),
            });
            board.users.new_user(second);
        },
    );

    assert_eq!(output, "2\nAlice Berlin\nBob Second note\nmatrix @bob:example.org\n0 []\n0 Berlin\n");
}

/// A bad number stays readable but cannot be mistaken for conference zero.
#[test]
fn an_unknown_conference_number_answers_an_empty_conference() {
    let output = run_ppl_on(
        r#"
        PrintLn "[", Board.Conferences[99].Name, "] ", Board.Conferences[99].Valid
        PrintLn "[", Board.Conferences[-1].Name, "] ", Board.Conferences[-1].Valid
        "#,
        seed_board,
    );

    assert_eq!(output, "[] 0\n[] 0\n");
}

#[test]
fn a_board_value_can_be_kept_in_a_variable() {
    let output = run_ppl_on(
        r#"
        BOARD board = Board()
        PrintLn board.Name, " ", board.Conferences.Len()
        "#,
        seed_board,
    );

    assert_eq!(output, "Icy Board 2\n");
}

#[test]
fn session_reports_the_call_it_is_running_in() {
    let output = run_ppl(
        r#"
        PrintLn Session.Node
        PrintLn Session.Conference.Number, " ", Session.Area.Number, " ", Session.Directory.Number
        PrintLn Session.SecurityLevel, " ", Session.PageLength
        PrintLn Session.IsLocal, " ", Session.IsSysop
        PrintLn "[", Session.UserName, "] [", Session.AliasName, "] [", Session.Language, "]"
        "#,
    );

    assert_eq!(output, "1\n0 0 0\n0 24\n0 0\n[] [] []\n");
}

/// The current conference is an object, so it reads like any other one.
#[test]
fn session_hands_out_the_conference_the_caller_is_in() {
    let output = run_ppl(
        r#"
        PrintLn "[", Session.Conference.Name, "] ", Session.Conference.Valid
        "#,
    );

    assert_eq!(output, "[] 0\n");
}

/// The current area and directory are objects too. The scratch session has not
/// joined a conference, so both are the empty object rather than a seeded one.
#[test]
fn session_hands_out_the_current_area_and_directory() {
    let output = run_ppl(
        r#"
        PrintLn "[", Session.Area.Name, "] ", Session.Area.Number, " ", Session.Area.Valid
        PrintLn "[", Session.Directory.Name, "] ", Session.Directory.Number, " ", Session.Directory.Valid
        "#,
    );

    assert_eq!(output, "[] 0 0\n[] 0 0\n");
}

/// Every board object reports where it sits, so a listing can name the number
/// a caller has to type.
#[test]
fn board_objects_know_their_own_number() {
    let output = run_ppl_on(
        r#"
        CONFERENCE conf = Board.Conferences[1]
        PrintLn conf.Number, " ", conf.Name, " ", conf.Valid
        PrintLn conf.Areas[1].Number, " ", conf.Areas[1].Name, " ", conf.Areas[1].Valid
        "#,
        |board| {
            seed_board(board);
            board.conferences[1].areas = Some(std::sync::Arc::new(crate::icy_board::message_area::AreaList::new(vec![
                crate::icy_board::message_area::MessageArea {
                    name: "First".to_string(),
                    ..Default::default()
                },
                crate::icy_board::message_area::MessageArea {
                    name: "Second".to_string(),
                    ..Default::default()
                },
            ])));
        },
    );

    assert_eq!(output, "1 Second 1\n1 Second 1\n");
}

/// `Session` is read live rather than snapshotted, so a value kept in a
/// variable still answers with what the call became.
#[test]
fn session_is_read_live() {
    let output = run_ppl(
        r"
        SESSION session = Session()
        INTEGER before = session.MinutesLeft
        ADJTIME 5
        PrintLn session.MinutesLeft - before
        ",
    );

    assert_eq!(output, "5\n");
}
