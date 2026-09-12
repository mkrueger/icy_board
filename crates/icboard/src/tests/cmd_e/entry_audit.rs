//! Persistence checks, rather than just checking that the editor printed Save.
use std::{path::PathBuf, sync::Mutex};

use bstr::BString;
use icy_board_engine::icy_board::{
    IcyBoard,
    security_expr::SecurityExpression,
    user_base::{FSEMode, User},
};
use jamjam::jam::{
    JamMessage, JamMessageBase, attributes,
    msg_header::{MessageSubfield, SubfieldType},
};

use crate::tests::{setup_conference, test_output};

fn session(input: &str, init: impl Fn(&mut IcyBoard)) -> (String, PathBuf) {
    let path = Mutex::new(PathBuf::new());
    let output = test_output(input.to_string(), |board| {
        setup_conference(board);
        board.users[0].flags.fse_mode = FSEMode::No;
        board.config.message.validate_to_name = false;
        board.config.message.allow_carbon_copy = true;
        *path.lock().unwrap() = board.conferences[0].areas.as_ref().unwrap()[0].path.clone();
        init(board);
    });
    (output, path.into_inner().unwrap())
}

fn seed(board: &IcyBoard, message: JamMessage) {
    let mut base = JamMessageBase::create(&board.conferences[0].areas.as_ref().unwrap()[0].path).unwrap();
    base.write_message(&message).unwrap();
    base.write_jhr_header().unwrap();
}

fn original(from: &str, to: &str) -> JamMessage {
    JamMessage::default()
        .with_from(BString::from(from))
        .with_to(BString::from(to))
        .with_subject(BString::from("Original subject"))
        .with_date_time(chrono::Utc::now())
        .with_msg_id(BString::from("test-original-id"))
        .with_text(BString::from("first original line\r\nsecond original line"))
}

#[test]
fn sc_persists_independent_copies_with_recipient_addresses_and_attributes() {
    let (output, path) = session(
        "E\nfirst@example.org\nSubject\nR\nY\nY\nfirst@example.org\nBody\n\nSC\nsecond@example.org\nTHIRD\n\n",
        |board| {
            board.conferences[0].echo_mail_in_conference = true;
            board.conferences[0].prompt_for_routing = true;
        },
    );
    let mut base = JamMessageBase::open(path).unwrap();
    assert_eq!(base.active_messages(), 3, "{output}");
    let expected = [
        ("first@example.org", Some("first@example.org")),
        ("second@example.org", Some("second@example.org")),
        ("THIRD", None),
    ];
    let mut ids = Vec::new();
    let mut offsets = Vec::new();
    for (index, (recipient, address)) in expected.into_iter().enumerate() {
        let header = base.read_header(index as u32 + 1).unwrap();
        assert_eq!(header.to().unwrap().to_string(), recipient);
        assert_eq!(base.read_message_text(&header).unwrap().to_string().trim(), "Body");
        assert_eq!(
            header.attributes & (attributes::MSG_LOCAL | attributes::MSG_TYPEECHO | attributes::MSG_PRIVATE | attributes::MSG_RECEIPTREQ),
            attributes::MSG_LOCAL | attributes::MSG_TYPEECHO | attributes::MSG_PRIVATE | attributes::MSG_RECEIPTREQ
        );
        assert_eq!(header.attributes & (attributes::MSG_SENT | attributes::MSG_READ | attributes::MSG_DELETED), 0);
        assert_eq!(
            header
                .sub_fields
                .iter()
                .find(|field| field.field_type() == SubfieldType::AddressD)
                .map(|field| field.content().to_string()),
            address.map(str::to_string)
        );
        ids.push(header.msgid_crc);
        offsets.push(header.offset);
    }
    assert!(ids[0] != ids[1] && ids[1] != ids[2] && ids[0] != ids[2]);
    assert!(offsets.windows(2).all(|pair| pair[0] < pair[1]));
    base.delete_message(2).unwrap();
    assert!(!base.read_header(1).unwrap().is_deleted());
    assert!(base.read_header(2).is_err());
    assert!(!base.read_header(3).unwrap().is_deleted());
}

#[test]
fn disabled_sc_still_saves_exactly_one_message() {
    let (output, path) = session("E\nALL\nSubject\nN\nBody\n\nSC\n", |board| board.config.message.allow_carbon_copy = false);
    let base = JamMessageBase::open(path).unwrap();
    assert_eq!(base.active_messages(), 1, "{output}");
    assert!(!output.contains("Carbon Copy To"));
    assert_eq!(base.read_message(1).unwrap().text().to_string().trim(), "Body");
}

#[test]
fn comment_sc_never_prompts_for_or_persists_carbon_copies() {
    let (output, path) = session("C\nY\nN\nPrivate comment\n\nSC\n", |board| {
        board.config.message.force_comments_to_main = true;
    });
    let base = JamMessageBase::open(path).unwrap();
    assert_eq!(base.active_messages(), 1, "{output}");
    assert!(!output.contains("Carbon Copy To"));
    assert!(base.read_header(1).unwrap().is_private());
    assert!(base.read_message(1).unwrap().text().to_string().contains("Private comment"));
}

#[test]
fn carbon_list_collects_before_subject_and_persists_private_recipients_not_sentinel() {
    let (output, path) = session("E\n@LIST@\nALICE\nBOB\nSubject\nN\nBody\n\nS\n", |board| {
        board.conferences[0].carbon_list_limit = 2;
        // PCBoard's list permission is independent of sequential SC copies.
        board.config.message.allow_carbon_copy = false;
    });
    let base = JamMessageBase::open(path).unwrap();
    assert_eq!(base.active_messages(), 2, "{output}");
    for (number, to) in [(1, "ALICE"), (2, "BOB")] {
        let header = base.read_header(number).unwrap();
        assert_eq!(header.to().unwrap().to_string(), to);
        assert!(header.is_private());
        assert!(!header.sub_fields.iter().any(|field| field.content().to_string().contains("ICYBOARD-CARBON-TO")));
        assert_eq!(base.read_message_text(&header).unwrap().to_string().trim(), "Body");
    }
}

#[test]
fn disabled_carbon_list_falls_back_to_all_without_collecting_names() {
    let (output, path) = session("E\n@LIST@\nSubject\nN\nBody\n\nS\n", |board| {
        board.conferences[0].carbon_list_limit = 1;
    });
    let base = JamMessageBase::open(path).unwrap();
    assert_eq!(base.active_messages(), 1, "{output}");
    assert_eq!(base.read_header(1).unwrap().to().unwrap().to_string(), "ALL");
}

#[test]
fn entry_no_private_conference_skips_security_and_receipt_prompts() {
    let (output, path) = session("E\nTEST USER\nSubject\nBody\n\nS\n", |board| {
        board.conferences[0].disallow_private_msgs = true;
    });
    let header = JamMessageBase::open(path).unwrap().read_header(1).unwrap();
    assert!(!header.is_private(), "{output}");
    assert!(!header.is_receipt_req());
    assert!(!output.contains("Message Security"));
    assert!(!output.contains("Require Return Receipt"));
}

#[test]
fn echo_no_is_persisted_as_local_only() {
    let (output, path) = session("E\nALL\nSubject\nN\nN\nBody\n\nS\n", |board| {
        board.conferences[0].echo_mail_in_conference = true;
    });
    let header = JamMessageBase::open(path).unwrap().read_header(1).unwrap();
    assert_ne!(header.attributes & attributes::MSG_TYPELOCAL, 0, "{output}");
    assert_eq!(header.attributes & attributes::MSG_TYPEECHO, 0);
    assert_ne!(header.attributes & attributes::MSG_LOCAL, 0);
}

#[test]
fn sender_password_is_distinct_from_group_read_password() {
    let (_, path) = session("E\nALL\nSubject\nS\nSECRET\nBody\n\nS\n", |_| {});
    let header = JamMessageBase::open(path).unwrap().read_header(1).unwrap();
    assert!(header.is_password_valid("SECRET"));
    assert!(!header.is_private());
    assert!(
        header
            .sub_fields
            .iter()
            .any(|field| field.field_type() == SubfieldType::FTSKludge && field.content() == "ICYBOARD-SECURITY: S")
    );
}

#[test]
fn keepmsg_level_prevents_packout_date_prompt() {
    let (output, path) = session("E\nALL\nSubject\nD\nBody\n\nS\n", |board| {
        board.users[0].security_level = 10;
        board.config.sysop_command_level.set_pack_out_date_on_messages = SecurityExpression::from_req_security(255);
    });
    assert!(!output.contains("Pack-Out"), "{output}");
    let header = JamMessageBase::open(path).unwrap().read_header(1).unwrap();
    assert!(!header.sub_fields.iter().any(|field| field.field_type() == SubfieldType::PackoutDate));
}

#[test]
fn long_to_allows_empty_subject_and_preserves_full_address() {
    let to = "a.long.remote.recipient@example.org";
    let (output, path) = session(&format!("E\n{to}\n\nN\nBody\n\nSC\n"), |board| board.conferences[0].long_to_names = true);
    let header = JamMessageBase::open(path).unwrap().read_header(1).unwrap();
    assert_eq!(header.to().unwrap().to_string(), to, "{output}");
    assert_eq!(header.subject().unwrap().to_string(), "");
    assert!(!output.contains("Carbon Copy To"));
}

#[test]
fn soundex_can_choose_an_existing_user() {
    let (output, path) = session("E\nRUPERT SMYTH\nS\nU\nSubject\nN\nBody\n\nS\n", |board| {
        board.config.message.validate_to_name = true;
        board.users.new_user(User {
            name: "ROBERT SMITH".to_string(),
            ..Default::default()
        });
    });
    let header = JamMessageBase::open(path).unwrap().read_header(1).unwrap();
    assert_eq!(header.to().unwrap().to_string(), "ROBERT SMITH", "{output}");
}

#[test]
fn reply_to_own_message_targets_original_recipient_and_keeps_numeric_thread() {
    let (output, path) = session("REPLY 1\n\nN\nReply body\n\nS\n", |board| {
        seed(
            board,
            original("SYSOP", "ALICE").with_sub_field(MessageSubfield::new(SubfieldType::AddressD, BString::from("alice@example.org"))),
        );
    });
    let base = JamMessageBase::open(path).unwrap();
    let reply = base.read_header(2).unwrap();
    assert_eq!(reply.to().unwrap().to_string(), "ALICE", "{output}");
    assert_eq!(reply.reply_to, 1);
    assert_eq!(reply.reply_crc, JamMessageBase::crc(&BString::from("test-original-id")));
    assert!(
        reply
            .sub_fields
            .iter()
            .any(|field| field.field_type() == SubfieldType::AddressD && field.content() == "alice@example.org")
    );
    let parent = base.read_header(1).unwrap();
    assert_eq!(parent.date_received, 0);
    assert!(
        parent
            .sub_fields
            .iter()
            .any(|field| field.content().to_string().starts_with("ICYBOARD-REPLY-DATE: "))
    );
    assert!(!output.contains("Require Return Receipt"));
}

#[test]
fn reply_quote_uses_original_body_not_draft_and_save_next_persists() {
    let (output, path) = session("REPLY 1\n\nN\nMy reply\n\nQ\n1 2\nSN\n", |board| seed(board, original("ALICE", "ALL")));
    let base = JamMessageBase::open(path).unwrap();
    let body = base.read_message(2).unwrap().text().to_string();
    assert!(body.contains("My reply"), "{output}");
    assert!(body.contains("-> first original line"));
    assert!(body.contains("-> second original line"));
    assert!(!base.read_header(1).unwrap().is_deleted());
}

#[test]
fn reply_denies_unrelated_private_messages_before_subject_or_body() {
    let (output, path) = session("REPLY 1\n", |board| {
        board.users[0].security_level = 10;
        board.config.sysop_command_level.read_all_mail = SecurityExpression::from_req_security(255);
        seed(board, original("ALICE", "BOB").with_attributes(attributes::MSG_PRIVATE));
    });
    assert_eq!(JamMessageBase::open(path).unwrap().active_messages(), 1);
    assert!(!output.contains("Original subject"), "{output}");
    assert!(!output.contains("first original line"));
}

#[test]
fn reply_obeys_conference_write_level() {
    let (output, path) = session("REPLY 1\n", |board| {
        board.users[0].security_level = 10;
        board.conferences[0].sec_write_message = SecurityExpression::from_req_security(255);
        seed(board, original("ALICE", "ALL"));
    });
    assert_eq!(JamMessageBase::open(path).unwrap().active_messages(), 1);
    assert!(!output.contains("Original subject"), "{output}");
}

#[test]
fn reply_group_password_is_verified_and_retained_without_turning_private() {
    let (output, path) = session("REPLY 1\nSECRET\n\nReply body\n\nS\n", |board| {
        board.users[0].security_level = 10;
        board.config.sysop_command_level.read_all_mail = SecurityExpression::from_req_security(255);
        seed(board, original("ALICE", "ALL").with_password(&BString::from("SECRET")));
    });
    let base = JamMessageBase::open(path).unwrap();
    assert_eq!(base.active_messages(), 2, "{output}");
    let header = base.read_header(2).unwrap();
    assert!(header.is_password_valid("SECRET"));
    assert!(!header.is_private());
    assert_eq!(header.reply_to, 1);
}

#[test]
fn reply_private_receipt_is_not_offered_below_receipt_level() {
    let (output, path) = session("REPLY 1\n\nReply body\n\nSK\n", |board| {
        board.users[0].security_level = 10;
        board.conferences[0].sec_request_rr = SecurityExpression::from_req_security(255);
        seed(board, original("ALICE", "SYSOP").with_attributes(attributes::MSG_PRIVATE));
    });
    let base = JamMessageBase::open(path).unwrap();
    let header = base.read_header(2).unwrap();
    assert!(header.is_private(), "{output}");
    assert!(!header.is_receipt_req());
    // PCBoard's standalone REPLY also honors SK after a successful save.
    assert!(base.read_header(1).is_err());
}

#[test]
fn reply_does_not_expose_group_message_after_failed_password() {
    let (output, path) = session("REPLY 1\nWRONG\nWRONG\nWRONG\nN\n", |board| {
        board.users[0].security_level = 10;
        board.config.sysop_command_level.read_all_mail = SecurityExpression::from_req_security(255);
        seed(board, original("ALICE", "ALL").with_password(&BString::from("SECRET")));
    });
    assert_eq!(JamMessageBase::open(path).unwrap().active_messages(), 1);
    assert!(!output.contains("Original subject"), "{output}");
    assert!(!output.contains("first original line"));
}

#[test]
fn carbon_list_security_denial_falls_back_to_public_all() {
    let (output, path) = session("E\n@LIST@\nSubject\nN\nBody\n\nS\n", |board| {
        board.users[0].security_level = 10;
        board.conferences[0].carbon_list_limit = 2;
        board.conferences[0].sec_carbon_copy = SecurityExpression::from_req_security(255);
    });
    let header = JamMessageBase::open(path).unwrap().read_header(1).unwrap();
    assert_eq!(header.to().unwrap().to_string(), "ALL", "{output}");
    assert!(!header.is_private());
}
