use crate::icy_board::conferences::Conference;

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
        PrintLn Board.Nodes, " ", Board.Conferences
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
        FOR i = 0 TO Board.Conferences - 1
            CONFERENCE conf = Board.GetConference(i)
            PrintLn i, ": ", conf.Name, " ", conf.HasAccess()
        NEXT
        "#,
        seed_board,
    );

    assert_eq!(output, "0: Main Board 1\n1: Second 1\n");
}

/// The same empty answer `ConfInfo()` gives, so a bad number stays readable.
#[test]
fn an_unknown_conference_number_answers_an_empty_conference() {
    let output = run_ppl_on(
        r#"
        PrintLn "[", Board.GetConference(99).Name, "]"
        PrintLn "[", Board.GetConference(-1).Name, "]"
        "#,
        seed_board,
    );

    assert_eq!(output, "[]\n[]\n");
}

#[test]
fn a_board_value_can_be_kept_in_a_variable() {
    let output = run_ppl_on(
        r#"
        BOARD board = Board()
        PrintLn board.Name, " ", board.Conferences
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
        PrintLn Session.ConferenceNumber, " ", Session.MessageArea, " ", Session.FileDirectory
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
        PrintLn "[", Session.Conference.Name, "] ", Session.Conference.HasAccess()
        "#,
    );

    assert_eq!(output, "[] 1\n");
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
