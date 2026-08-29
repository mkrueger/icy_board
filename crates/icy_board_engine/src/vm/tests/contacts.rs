use crate::{
    icy_board::user_base::UserContact,
    vm::tests::{compile_errors, compile_errors_with_runtime, run_ppl, run_ppl_on},
};

#[test]
fn contact_record_fields_use_dynamic_string_storage() {
    let value = crate::executable::create_record_value(crate::parser::CONTACT_ID as u8, &[]).unwrap();
    let crate::executable::GenericVariableData::Record(fields) = value.generic_data else {
        panic!("contact record fields were not initialized");
    };
    assert!(fields.iter().all(|field| field.vtype == crate::executable::VariableType::BigStr));
}

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
fn the_contacts_wrapper_type_is_no_longer_defined() {
    let errors = compile_errors("CONTACTS contacts");
    assert!(!errors.is_empty(), "CONTACTS should not remain a PPL 400 type");
}

#[test]
fn a_contact_can_be_written_and_read_back() {
    assert_eq!(
        "1 matrix:@sysop:example.org",
        run_ppl(
            r#"
PRINT Session.User.AddContact("Matrix", "@sysop:example.org"), " "
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
Session.User.AddContact("github", "sysop")
Session.User.AddContact("matrix", "@sysop:example.org")
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
Session.User.SetNote(0, "first")
Session.User.SetNote(2, "third")
STRING note
PRINT Session.User.Notes.Len(), " "
FOREACH note IN Session.User.Notes
    PRINT note, " "
ENDFOREACH
"#,
        )
    );
}

/// Reading Contacts again returns the current list without a GETUSER/PUTUSER round trip.
#[test]
fn contacts_are_seen_and_changed_without_getuser() {
    assert_eq!(
        "1 github:sysop|2 matrix:@sysop:example.org",
        run_ppl_on(
            r#"
PRINT Session.User.Contacts.Len(), " ", Session.User.Contacts[0].Service, ":", Session.User.Contacts[0].Account, "|"
Session.User.AddContact("MATRIX", "@sysop:example.org")
PRINT Session.User.Contacts.Len(), " ", Session.User.Contacts[1].Service, ":", Session.User.Contacts[1].Account
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

/// Contacts are a list, so duplicate services remain separate entries.
#[test]
fn adding_a_known_service_appends_another_contact() {
    assert_eq!(
        "2 github:sysop github:someone-else",
        run_ppl_on(
            r#"
Session.User.AddContact("GitHub", "someone-else")
PRINT Session.User.Contacts.Len(), " ", Session.User.Contacts[0].Service, ":", Session.User.Contacts[0].Account, " ", Session.User.Contacts[1].Service, ":", Session.User.Contacts[1].Account
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
        "1 1 matrix 0",
        run_ppl_on(
            r#"
Session.User.AddContact("matrix", "account")
PRINT Session.User.RemoveContact(0), " ", Session.User.Contacts.Len(), " ", Session.User.Contacts[0].Service, " ", Session.User.RemoveContact(99)
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

/// A blank service or account is not a contact, so it is refused rather than stored.
#[test]
fn an_empty_contact_is_refused() {
    assert_eq!(
        "0 0 0",
        run_ppl(r#"PRINT Session.User.AddContact("", "sysop"), " ", Session.User.AddContact("matrix", " "), " ", Session.User.Contacts.Len()"#)
    );
}

#[test]
fn a_contacts_array_is_a_snapshot() {
    assert_eq!(
        "1 2",
        run_ppl_on(
            r#"
CONTACT contacts[]
contacts = Session.User.Contacts
Session.User.AddContact("matrix", "account")
PRINT contacts.Len(), " ", Session.User.Contacts.Len()
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
        run_ppl(r#"PRINT Session.User.Name, "|", Session.User.SecurityLevel, "|", Session.User.Notes.Len() > 0, "|", Session.User.Uploads"#)
    );
}

/// The record number is the 1-based position matching PCBoard's user file.
#[test]
fn the_user_exposes_its_one_based_record_number() {
    assert_eq!("1 1", run_ppl(r#"PRINT Session.User.RecordNumber, " ", Board.Users[0].RecordNumber"#));
}

/// A user record holds at most 100 contacts, and the overflow reports ErrCode.Limit.
#[test]
fn contacts_are_capped_at_one_hundred() {
    assert_eq!(
        "100 0 1",
        run_ppl(
            r#"
INTEGER i
FOR i = 1 TO 100
    Session.User.AddContact("svc" + STRING(i), "acc")
NEXT
BOOLEAN over = Session.User.AddContact("over", "acc")
PRINT Session.User.Contacts.Len(), " ", over, " ", Error.Last().Code = ErrCode.Limit
"#,
        )
    );
}

/// Byte totals are 64-bit, so a value above 4 GiB is not truncated.
#[test]
fn download_bytes_use_64_bit_storage() {
    assert_eq!(
        "5000000000",
        run_ppl_on(r#"PRINT Session.User.DownloadBytes"#, |board| {
            board.users[0].stats.total_dnld_bytes = 5_000_000_000;
        })
    );
}
