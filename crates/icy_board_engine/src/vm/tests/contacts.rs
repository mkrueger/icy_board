use crate::{
    icy_board::user_base::UserContact,
    vm::tests::{compile_errors_with_runtime, run_ppl, run_ppl_on},
};

#[test]
fn u_contact_requires_runtime_400() {
    let errors = compile_errors_with_runtime("PRINT U_CONTACT[0].Service", 340);

    assert!(!errors.is_empty(), "U_CONTACT unexpectedly compiled for runtime 340");
}

#[test]
fn a_predefined_contact_can_be_written_and_read() {
    assert_eq!(
        "matrix:@sysop:example.org",
        run_ppl(
            r#"
U_CONTACT[0].Service = "matrix"
U_CONTACT[0].Account = "@sysop:example.org"
PRINT U_CONTACT[0].Service, ":", U_CONTACT[0].Account
"#,
        )
    );
}

#[test]
fn getuser_and_putuser_round_trip_contacts() {
    assert_eq!(
        "GitHub:sysop|matrix:@sysop:example.org",
        run_ppl_on(
            r#"
GETUSER
PRINT U_CONTACT[0].Service, ":", U_CONTACT[0].Account, "|"
U_CONTACT[0].Service = "MATRIX"
U_CONTACT[0].Account = "@sysop:example.org"
PUTUSER
GETUSER
PRINT U_CONTACT[0].Service, ":", U_CONTACT[0].Account
"#,
            |board| {
                board.users[0].contacts.push(UserContact {
                    service: "GitHub".to_string(),
                    account: "sysop".to_string(),
                });
            },
        )
    );
}

#[test]
fn encoding_and_hash_functions_match_standard_vectors() {
    assert_eq!(
        "R3LDvMOfZQ==|Grüße|ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        run_ppl(
            r#"
    PRINT BASE64ENC("Grüße"), "|"
    PRINT BASE64DEC("R3LD!!vMOfZQ=="), "|"
PRINT SHA256("abc")
"#,
        )
    );
}
