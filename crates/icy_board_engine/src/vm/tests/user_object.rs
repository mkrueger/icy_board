use crate::vm::tests::{compile_errors, run_ppl, run_ppl_on};

#[test]
fn api_review_user_mutations_roll_back_when_persistence_fails() {
    use crate::icy_board::user_base::{Password, UserContact};

    for operation in [
        "BOOLEAN changed = Session.User.SetNote(0, \"changed\")\nPRINTLN changed",
        "BOOLEAN changed = Session.User.SetPassword(\"changed\")\nPRINTLN changed",
        "BOOLEAN changed = Session.User.AddContact(\"new\", \"changed\")\nPRINTLN changed",
        "BOOLEAN changed = Session.User.RemoveContact(0)\nPRINTLN changed",
        "Session.User.City = \"changed\"\nPRINTLN FALSE",
    ] {
        let output = run_ppl_on(
            &format!(
                r#"
Error.Clear()
{operation}
PRINTLN Error.Last().Kind = ErrKind.User, "|", Error.Last().Code = ErrCode.Io
PRINTLN Session.User.Notes[0], "|", Session.User.City, "|", Session.User.Contacts.Len(), "|", Session.User.Contacts[0].Account
PRINTLN Board.Users[0].Notes[0], "|", Board.Users[0].City, "|", Board.Users[0].Contacts.Len(), "|", Board.Users[0].Contacts[0].Account
GETUSER
PRINTLN U_PWD = "original"
"#,
            ),
            |board| {
                board.config.paths.user_file = board.root_path.clone();
                board.users[0].custom_comment1 = "original".to_string();
                board.users[0].city_or_state = "original".to_string();
                board.users[0].password.password = Password::PlainText("original".to_string());
                board.users[0].contacts.push(UserContact {
                    service: "original".to_string(),
                    account: "original".to_string(),
                });
            },
        );
        assert_eq!(output, "0\n1|1\noriginal|original|1|original\noriginal|original|1|original\n1\n", "{operation}");
    }
}

#[test]
fn api_review_user_mutation_errors_and_successes_publish_their_result() {
    for operation in [
        "Session.User.SetNote(-1, \"bad\")",
        "Session.User.SetNote(5, \"bad\")",
        "Session.User.SetPassword(\"\")",
        "Session.User.AddContact(\" \", \"value\")",
        "Session.User.AddContact(\"service\", \" \")",
        "Session.User.RemoveContact(-1)",
        "Session.User.RemoveContact(0)",
    ] {
        let output = run_ppl(&format!(
            r#"
Error.Clear()
BOOLEAN changed = {operation}
PRINTLN changed, "|", Error.Last().Kind = ErrKind.User, "|", Error.Last().Code = ErrCode.Invalid
"#,
        ));
        assert_eq!(output, "0|1|1\n", "{operation}");
    }
    for operation in [
        "Session.User.SetNote(0, \"ok\")",
        "Session.User.SetPassword(\"ok\")",
        "Session.User.AddContact(\"service\", \"ok\")",
        "Session.User.AddContact(\"service\", \"ok\")\nREGEX badAgain = REGEX.Compile(\"[\")\nSession.User.RemoveContact(0)",
        "Session.User.City = \"ok\"",
    ] {
        let output = run_ppl_on(
            &format!(
                r#"
REGEX bad = REGEX.Compile("[")
{operation}
PRINTLN Error.Last().OK
"#,
            ),
            |board| board.config.paths.user_file = board.root_path.join("users.toml"),
        );
        assert_eq!(output, "1\n", "{operation}");
    }
}

#[test]
fn api_review_user_persistence_failure_enters_the_error_handler() {
    let output = run_ppl_on(
        r#"
ON ERROR GOTO Failed
Session.User.City = "not saved"
PRINTLN "not reached"
EXIT
:Failed
PRINTLN Error.Last().Kind = ErrKind.User, "|", Error.Last().Code = ErrCode.Io
"#,
        |board| board.config.paths.user_file = board.root_path.clone(),
    );
    assert_eq!(output, "1|1\n");
}

#[test]
fn cumulative_statistics_keep_their_full_unsigned_width() {
    let output = run_ppl_on(
        r#"
PRINT Session.User.TimesOn, " ", Session.User.MessagesRead, " ", Session.User.MessagesLeft, " ", Session.User.Uploads, " ", Session.User.Downloads
"#,
        |board| {
            let stats = &mut board.users[0].stats;
            stats.num_times_on = 4_294_967_296;
            stats.messages_read = 4_294_967_297;
            stats.messages_left = 4_294_967_298;
            stats.num_uploads = 4_294_967_299;
            stats.num_downloads = 4_294_967_300;
        },
    );

    assert_eq!(output, "4294967296 4294967297 4294967298 4294967299 4294967300");
}

#[test]
fn api_review_user_mutations_are_persisted_without_changing_other_users() {
    use crate::icy_board::{
        IcyBoardSerializer,
        user_base::{User, UserBase},
    };

    let directory = tempfile::tempdir().unwrap();
    let user_file = directory.path().join("users.toml");
    let output = run_ppl_on(
        r#"
Session.User.City = "Berlin"
Session.User.SetNote(0, "saved")
Session.User.AddContact("Matrix", "@sysop:example.org")
Session.User.SetPassword("saved-password")
PRINTLN Error.Last().OK
"#,
        |board| {
            board.config.paths.user_file = user_file.clone();
            board.users.new_user(User {
                name: "OTHER".to_string(),
                city_or_state: "unchanged".to_string(),
                ..Default::default()
            });
        },
    );
    assert_eq!(output, "1\n");
    let saved = UserBase::load(&user_file).unwrap();
    assert_eq!(saved[0].city_or_state, "Berlin");
    assert_eq!(saved[0].custom_comment1, "saved");
    assert_eq!(saved[0].contacts[0].service, "matrix");
    assert_eq!(saved[0].contacts[0].account, "@sysop:example.org");
    assert!(saved[0].password.password.is_valid("saved-password"));
    assert_eq!(saved[1].name, "OTHER");
    assert_eq!(saved[1].city_or_state, "unchanged");
}

#[test]
fn api_review_user_success_preserves_an_error_from_the_same_statement() {
    let output = run_ppl(
        r#"
PRINTLN Session.User.SetNote(5, "bad"), "|", Session.User.SetNote(0, "ok"), "|", Error.Last().Kind = ErrKind.User
PRINTLN Session.User.Notes[0]
"#,
    );
    assert_eq!(output, "0|1|1\nok\n");
}

/// What `PUTUSER` used to write is writable on the object, and it lands right away
/// instead of waiting for a round trip.
#[test]
fn a_written_property_is_read_back_without_putuser() {
    assert_eq!(
        "Sysop|Berlin|42",
        run_ppl(
            r#"
Session.User.Alias = "Sysop"
Session.User.City = "Berlin"
Session.User.SecurityLevel = 42
PRINT Session.User.Alias, "|", Session.User.City, "|", Session.User.SecurityLevel
"#,
        )
    );
}

#[test]
fn a_written_property_is_seen_by_getuser() {
    assert_eq!(
        "Sysop 42",
        run_ppl(
            r#"
Session.User.Alias = "Sysop"
Session.User.SecurityLevel = 42
GETUSER
PRINT U_ALIAS, " ", U_SEC
"#,
        )
    );
}

#[test]
fn bounded_user_fields_reject_values_that_do_not_fit() {
    assert_eq!(
        "42 24 80 1 1\n1 1 1 1",
        run_ppl(
            r#"
Session.User.SecurityLevel = 42
Session.User.ExpiredSecurityLevel = 24
Session.User.PageLength = 80
Session.User.SecurityLevel = 300
ERROR securityError = Error.Last()
Error.Clear()
Session.User.ExpiredSecurityLevel = -1
ERROR expiredError = Error.Last()
Error.Clear()
Session.User.PageLength = -1
ERROR pageError = Error.Last()
PRINTLN Session.User.SecurityLevel, " ", Session.User.ExpiredSecurityLevel, " ", Session.User.PageLength, " ", securityError.Kind = ErrKind.User, " ", securityError.Code = ErrCode.Invalid
PRINT expiredError.Kind = ErrKind.User, " ", expiredError.Code = ErrCode.Invalid, " ", pageError.Kind = ErrKind.User, " ", pageError.Code = ErrCode.Invalid
"#,
        )
    );
}

#[test]
fn the_editor_mode_replaces_the_two_editor_flags() {
    assert_eq!(
        "2",
        run_ppl(
            r"
Session.User.EditorMode = EditorMode.Ask
PRINT Session.User.EditorMode
",
        )
    );
}

/// The board keeps its own tally, so a PPE cannot rewrite what the caller did.
#[test]
fn the_board_s_own_accounting_stays_read_only() {
    let errors = compile_errors("Session.User.TimesOn = 1");

    assert!(errors.iter().any(|error| error.contains("TimesOn")), "{errors:?}");
}

#[test]
fn the_callers_name_stays_read_only() {
    let errors = compile_errors("Session.User.Name = \"Someone Else\"");

    assert!(errors.iter().any(|error| error.contains("Name")), "{errors:?}");
}

#[test]
fn a_note_can_be_written_and_read_back() {
    assert_eq!(
        "Called about the upload",
        run_ppl(
            r#"
Session.User.SetNote(0, "Called about the upload")
PRINT Session.User.Notes[0]
"#,
        )
    );
}

/// An index no note has is refused rather than failing, and leaves the rest alone.
#[test]
fn a_note_outside_the_five_slots_is_refused() {
    assert_eq!(
        "1 0 kept",
        run_ppl(
            r#"
PRINT Session.User.SetNote(0, "kept"), " ", Session.User.SetNote(5, "nowhere"), " ", Session.User.Notes[0]
"#,
        )
    );
}

/// Notes are snapshots, so mutation is explicit and an existing array stays unchanged.
#[test]
fn notes_are_array_snapshots() {
    assert_eq!(
        "one|one and two",
        run_ppl(
            r#"
Session.User.SetNote(1, "one")
STRING notes[]
notes = Session.User.Notes
Session.User.SetNote(1, "one and two")
PRINT notes[1], "|", Session.User.Notes[1]
"#,
        )
    );
}

#[test]
fn the_notes_wrapper_type_is_no_longer_defined() {
    let errors = compile_errors("NOTES notes");
    assert!(!errors.is_empty(), "NOTES should not remain a PPL 400 type");
}

#[test]
fn a_snapshot_user_cannot_set_notes() {
    assert_eq!("0", run_ppl("PRINT Board.Users[0].SetNote(0, \"changed\")"));
}

/// The board hashes the password, so what the PPE handed over is not what is stored.
#[test]
fn a_password_is_hashed_rather_than_stored_as_given() {
    assert_eq!(
        "1 1 0",
        run_ppl(
            r#"
PRINT Session.User.SetPassword("secret"), " "
GETUSER
PRINT U_PWD = "secret", " "
PRINT Session.User.SetPassword("")
"#,
        )
    );
}
