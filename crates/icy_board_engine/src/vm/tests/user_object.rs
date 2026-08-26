use crate::vm::tests::{compile_errors, run_ppl};

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
        "1 Called about the upload",
        run_ppl(
            r#"
PRINT Session.User.Notes.Set(0, "Called about the upload"), " "
PRINT Session.User.Notes[0]
"#,
        )
    );
}

#[test]
fn a_note_outside_the_five_slots_is_refused() {
    assert_eq!("0", run_ppl(r#"PRINT Session.User.Notes.Set(5, "nowhere")"#));
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
