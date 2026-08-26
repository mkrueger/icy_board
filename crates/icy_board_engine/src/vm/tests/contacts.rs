use crate::{
    icy_board::user_base::UserContact,
    vm::tests::{compile_errors, compile_errors_with_runtime, run_ppl, run_ppl_on},
};

#[test]
fn the_user_object_requires_runtime_400() {
    let errors = compile_errors_with_runtime("PRINT Session.User.Name", 340);

    assert!(errors.iter().any(|error| error.contains("Session needs runtime 400")), "{errors:?}");
}

/// `U_CONTACT` is gone; contacts belong to the user rather than to a predefined array.
#[test]
fn the_retired_contact_variable_is_no_longer_defined() {
    let errors = compile_errors("PRINT U_CONTACT[0].Service");

    assert!(errors.iter().any(|error| error.contains("U_CONTACT")), "{errors:?}");
}

#[test]
fn a_contact_can_be_written_and_read_back() {
    assert_eq!(
        "1 matrix:@sysop:example.org",
        run_ppl(
            r#"
PRINT Session.User.Contacts.Put("Matrix", "@sysop:example.org"), " "
PRINT Session.User.Contacts[0].Service, ":", Session.User.Contacts[0].Account
"#,
        )
    );
}

#[test]
fn contacts_can_be_walked() {
    assert_eq!(
        "github matrix ",
        run_ppl(
            r#"
Session.User.Contacts.Put("github", "sysop")
Session.User.Contacts.Put("matrix", "@sysop:example.org")
CONTACT entry
FOREACH entry IN Session.User.Contacts
    PRINT entry.Service, " "
ENDFOREACH
"#,
        )
    );
}

#[test]
fn notes_can_be_walked() {
    assert_eq!(
        "5 first  third   ",
        run_ppl(
            r#"
Session.User.Notes.Set(0, "first")
Session.User.Notes.Set(2, "third")
STRING note
PRINT Session.User.Notes.Count, " "
FOREACH note IN Session.User.Notes
    PRINT note, " "
ENDFOREACH
"#,
        )
    );
}

/// The object is read live, so a contact the PPE adds is there without a
/// `GETUSER`/`PUTUSER` round trip.
#[test]
fn contacts_are_seen_and_changed_without_getuser() {
    assert_eq!(
        "1 github:sysop|2 matrix:@sysop:example.org",
        run_ppl_on(
            r#"
PRINT Session.User.Contacts.Count, " ", Session.User.Contacts[0].Service, ":", Session.User.Contacts[0].Account, "|"
Session.User.Contacts.Put("MATRIX", "@sysop:example.org")
PRINT Session.User.Contacts.Count, " ", Session.User.Contacts[1].Service, ":", Session.User.Contacts[1].Account
"#,
            |board| {
                board.users[0].contacts.push(UserContact {
                    service: "github".to_string(),
                    account: "sysop".to_string(),
                });
            },
        )
    );
}

/// Setting a service that is already there replaces its account rather than
/// adding a second entry that means the same thing.
#[test]
fn setting_a_known_service_replaces_its_account() {
    assert_eq!(
        "1 github:someone-else",
        run_ppl_on(
            r#"
Session.User.Contacts.Put("GitHub", "someone-else")
PRINT Session.User.Contacts.Count, " ", Session.User.Contacts[0].Service, ":", Session.User.Contacts[0].Account
"#,
            |board| {
                board.users[0].contacts.push(UserContact {
                    service: "github".to_string(),
                    account: "sysop".to_string(),
                });
            },
        )
    );
}

#[test]
fn a_contact_can_be_deleted() {
    assert_eq!(
        "1 0 0",
        run_ppl_on(
            r#"PRINT Session.User.Contacts.Delete("GitHub"), " ", Session.User.Contacts.Count, " ", Session.User.Contacts.Delete("github")"#,
            |board| {
                board.users[0].contacts.push(UserContact {
                    service: "github".to_string(),
                    account: "sysop".to_string(),
                });
            },
        )
    );
}

/// A blank service or account is not a contact, so it is refused rather than stored.
#[test]
fn an_empty_contact_is_refused() {
    assert_eq!(
        "0 0 0",
        run_ppl(r#"PRINT Session.User.Contacts.Put("", "sysop"), " ", Session.User.Contacts.Put("matrix", " "), " ", Session.User.Contacts.Count"#)
    );
}

#[test]
fn an_unknown_contact_index_answers_an_empty_contact() {
    assert_eq!(
        "[][]",
        run_ppl(r#"PRINT "[", Session.User.Contacts[99].Service, "][", Session.User.Contacts[-1].Account, "]""#)
    );
}

/// The user's own details read the same way the `U_*` variables report them.
#[test]
fn the_user_reports_its_own_details() {
    assert_eq!(
        "SYSOP|255|1|0",
        run_ppl(r#"PRINT Session.User.Name, "|", Session.User.SecurityLevel, "|", Session.User.Notes.Count > 0, "|", Session.User.Uploads"#)
    );
}
