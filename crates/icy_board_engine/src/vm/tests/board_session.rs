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

/// A password reads as a fixed mask everywhere, so a PPE can check one but
/// never learn it, not even its length.
#[test]
fn a_password_reads_as_its_mask_and_never_as_the_secret() {
    let seed = |board: &mut crate::icy_board::IcyBoard| {
        seed_board(board);
        board.conferences[0].password = crate::icy_board::user_base::Password::PlainText("secret".to_string());
    };
    let output = run_ppl_on(
        r#"
        CONFERENCE guarded = Board.Conferences[0]
        CONFERENCE open = Board.Conferences[1]
        PrintLn guarded.Password
        PrintLn "[", guarded.Password, "]"
        PrintLn guarded.Password.Len(), " ", LEN(guarded.Password)
        PrintLn guarded.Password = "secret", " ", guarded.Password = "guess"
        PrintLn open.Password, " ", open.Password.Len()
        "#,
        seed,
    );

    assert_eq!(
        output, "******\n[******]\n6 6\n1 0\n****** 6\n",
        "the mask must not vary with the stored password"
    );
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

mod password_recovery {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::icy_board::{
        IcyBoard, IcyBoardSerializer,
        icb_config::PasswordStorageMethod,
        password_recovery::{MailSender, PasswordRecoveryConfig, RecoveryService},
        user_base::{Password, UserBase},
    };

    #[derive(Default)]
    struct Mail {
        recipients: Mutex<Vec<String>>,
        fail: bool,
    }

    #[async_trait::async_trait]
    impl MailSender for Mail {
        async fn send(&self, _: &PasswordRecoveryConfig, to: &str, _: &str, _: String) -> Result<(), ()> {
            self.recipients.lock().unwrap().push(to.to_string());
            if self.fail { Err(()) } else { Ok(()) }
        }
    }

    fn seed(board: &mut IcyBoard, mail: Arc<Mail>) {
        board.config.system_control.password_storage_method = PasswordStorageMethod::Argon2;
        board.config.password_recovery = PasswordRecoveryConfig {
            enabled: true,
            smtp_host: "smtp.example.invalid".into(),
            sender: "board@example.invalid".into(),
            ..Default::default()
        };
        board.password_recovery_service = Arc::new(RecoveryService::new(mail));
        board.users[0].security_level = 0;
        let mut user = User {
            name: "Target User".into(),
            alias: "TargetAlias".into(),
            email: "target@example.invalid".into(),
            ..Default::default()
        };
        user.password.password = Password::new_argon2("old-password");
        board.users.new_user(user);
    }

    #[test]
    fn request_password_recovery_requires_runtime_400_and_a_name() {
        let source = "PRINTLN Session.RequestPasswordRecovery(\"Target User\")";
        for runtime in [330, 340] {
            assert!(!compile_errors_with_runtime(source, runtime).is_empty());
        }
        assert!(compile_errors_with_runtime(source, 400).is_empty());
        assert!(!compile_errors_with_runtime("Session.RequestPasswordRecovery()", 400).is_empty());
        assert!(!compile_errors_with_runtime("Session.RequestPasswordRecovery(\"a\", \"b\")", 400).is_empty());
    }

    #[test]
    fn request_password_recovery_sends_persists_and_keeps_the_caller() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("users.toml");
        let mail = Arc::new(Mail::default());
        let output = run_ppl_on(
            r#"
            PRINTLN Session.IsSysop, "|", Session.SecurityLevel
            REGEX bad = REGEX.Compile("[")
            PRINTLN Session.RequestPasswordRecovery("  targetalias  ")
            PRINTLN Error.Last().OK
            PRINTLN Session.User.Name, "|", Session.User.RecordNumber
            Session.User.City = "Caller City"
            PRINTLN "still running"
            "#,
            |board| {
                seed(board, mail.clone());
                board.config.paths.user_file = file.clone();
            },
        );
        assert_eq!(output, "0|0\n1\n1\nSYSOP|1\nstill running\n");
        assert_eq!(*mail.recipients.lock().unwrap(), ["target@example.invalid"]);
        let users = UserBase::load(&file).unwrap();
        assert!(users[1].recovery.is_some());
        assert!(users[1].password.password.is_valid("old-password"));
        assert!(users[0].recovery.is_none());
        assert_eq!(users[0].city_or_state, "Caller City");
    }

    #[test]
    fn request_password_recovery_disabled_returns_false_without_sending() {
        let mail = Arc::new(Mail::default());
        let output = run_ppl_on(
            r#"
            REGEX bad = REGEX.Compile("[")
            PRINTLN Session.RequestPasswordRecovery("Target User")
            PRINTLN Error.Last().OK
            "#,
            |board| {
                seed(board, mail.clone());
                board.config.password_recovery.enabled = false;
            },
        );
        assert_eq!(output, "0\n1\n");
        assert!(mail.recipients.lock().unwrap().is_empty());
    }

    #[test]
    fn request_password_recovery_hides_unknown_excluded_and_failed_requests() {
        for case in ["unknown", "empty", "sysop", "disabled", "deleted", "email", "plaintext", "smtp", "save"] {
            let mail = Arc::new(Mail {
                fail: case == "smtp",
                ..Default::default()
            });
            let name = match case {
                "unknown" => "Nobody",
                "empty" => " ",
                "sysop" => "SYSOP",
                _ => "Target User",
            };
            let output = run_ppl_on(
                &format!(
                    "REGEX bad = REGEX.Compile(\"[\")\nON ERROR GOTO Failed\nPRINTLN Session.RequestPasswordRecovery(\"{name}\")\nPRINTLN Error.Last().OK\nEXIT\n:Failed\nPRINTLN \"unexpected handler\""
                ),
                |board| {
                    seed(board, mail.clone());
                    match case {
                        "disabled" => board.users[1].flags.disabled_flag = true,
                        "deleted" => board.users[1].flags.delete_flag = true,
                        "email" => board.users[1].email.clear(),
                        "plaintext" => board.users[1].password.password = Password::PlainText("old-password".into()),
                        "save" => board.config.paths.user_file = board.root_path.clone(),
                        _ => {}
                    }
                },
            );
            assert_eq!(output, "1\n1\n", "{case}");
            assert_eq!(mail.recipients.lock().unwrap().len(), usize::from(case == "smtp"), "{case}");
        }
    }

    #[test]
    fn request_password_recovery_reuses_limits_not_a_per_connection_cap() {
        for board_limit in [1, 50] {
            let mail = Arc::new(Mail::default());
            let output = run_ppl_on(
                r#"
                PRINTLN Session.RequestPasswordRecovery("Target User")
                PRINTLN Session.RequestPasswordRecovery("Target User")
                PRINTLN Session.RequestPasswordRecovery("Other User")
                PRINTLN Error.Last().OK
                "#,
                |board| {
                    seed(board, mail.clone());
                    board.config.password_recovery.board_per_hour = board_limit;
                    let mut other = board.users[1].clone();
                    other.name = "Other User".into();
                    other.alias.clear();
                    other.email = "other@example.invalid".into();
                    board.users.new_user(other);
                },
            );
            assert_eq!(output, "1\n1\n1\n1\n");
            let expected = if board_limit == 1 {
                vec!["target@example.invalid"]
            } else {
                vec!["target@example.invalid", "other@example.invalid"]
            };
            assert_eq!(*mail.recipients.lock().unwrap(), expected);
        }
    }
}
