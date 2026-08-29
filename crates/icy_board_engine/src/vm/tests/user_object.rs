use crate::vm::tests::{compile_errors, run_ppl, run_ppl_on};

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
