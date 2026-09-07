use crate::tests::test_output;

/// TS with no text behind it asks for the text. A PPE stuffing `R TS` relies on
/// that question being there.
#[test]
fn test_cmd_r_ts_asks_for_the_search_text() {
    let output = test_output("R\nTS\n\n\n".to_string(), crate::tests::setup_conference);
    assert!(output.contains("Text to Scan for"), "the text search prompt is missing:\n{output}");
}

/// TS with the text on the same line must not ask again.
#[test]
fn test_cmd_r_ts_with_text_does_not_ask() {
    let output = test_output("R\nTS HELLO\n\n\n".to_string(), crate::tests::setup_conference);
    assert!(!output.contains("Text to Scan for"), "the text was already given:\n{output}");
}

/// FROM and TO each ask for their own name when none was given.
#[test]
fn test_cmd_r_from_asks_for_a_name() {
    let output = test_output("R\nFROM\n\n\n".to_string(), crate::tests::setup_conference);
    assert!(output.contains("Read messages FROM"), "the sender search prompt is missing:\n{output}");
}

/// N outside the read loop scans for new messages and asks which date to start
/// from.
#[test]
fn test_cmd_r_new_asks_for_a_date() {
    let output = test_output("R\nN\n\n\n".to_string(), crate::tests::setup_conference);
    assert!(output.contains("Date"), "the date prompt is missing:\n{output}");
}

/// A word the parser does not know is a search term rather than a command, so
/// the next question is where to start searching.
#[test]
fn test_cmd_r_unknown_word_becomes_a_search_term() {
    let output = test_output("R\nZZZZ\n\n\n".to_string(), crate::tests::setup_conference);
    assert!(output.contains("to Begin Search from"), "an unknown word must become search text:\n{output}");
}

/// Reading a range must show every message in it, not just the first.
#[test]
fn test_cmd_r_range_shows_every_message() {
    let output = test_output("R\n1+\n\n\n\n\n\n".to_string(), crate::tests::setup_conference_with_messages);
    for i in 1..=3 {
        assert!(output.contains(&format!("Body of message {i}")), "message {i} was not shown:\n{output}");
    }
}

/// K in the read loop kills the message on screen without asking for a number.
#[test]
fn test_cmd_r_kill_takes_the_current_message() {
    let output = test_output("R\n1\nK\n\n\n\n".to_string(), crate::tests::setup_conference_with_messages);
    assert!(!output.contains("to Kill"), "K in the read loop must not ask for a number:\n{output}");
    assert!(output.contains("Message Killed"), "the message was not killed:\n{output}");
}

/// MOVE asks which conference to move to when the command line did not say.
#[test]
fn test_cmd_r_move_asks_for_the_conference() {
    let output = test_output("R\n1\nMOVE\n1\n\n\n\n".to_string(), crate::tests::setup_conference_with_messages);
    assert!(output.contains("what Conference"), "the conference prompt is missing:\n{output}");
    assert!(output.contains("Message Moved"), "the message was not moved:\n{output}");
}

/// MOVE with the conference already on the line must not ask again.
#[test]
fn test_cmd_r_move_with_a_conference_does_not_ask() {
    let output = test_output("R\n1\nMOVE 1\n\n\n\n".to_string(), crate::tests::setup_conference_with_messages);
    assert!(!output.contains("what Conference"), "the conference was already given:\n{output}");
}

/// SET asks where to put the last-read pointer, unless the number is already
/// there.
#[test]
fn test_cmd_r_set_asks_for_the_pointer() {
    let output = test_output("R\nSET\n2\n\n\n".to_string(), crate::tests::setup_conference_with_messages);
    assert!(output.contains("Set your Last Message Read to"), "the pointer prompt is missing:\n{output}");
}

#[test]
fn test_cmd_r_set_with_a_number_does_not_ask() {
    let output = test_output("R\nSET 2\n\n\n".to_string(), crate::tests::setup_conference_with_messages);
    assert!(!output.contains("Set your Last Message Read to"), "the number was already given:\n{output}");
    assert!(output.contains("Last Message Read now set to 2"), "the pointer was not moved:\n{output}");
}

/// A text search only shows the messages that carry the text; the rest are
/// skipped without a prompt.
#[test]
fn test_cmd_r_text_search_skips_the_other_messages() {
    let output = test_output("R\nTS BANANA 1+\n\n\n\n".to_string(), crate::tests::setup_conference_with_messages);
    assert!(output.contains("Subject 2"), "the matching message is missing:\n{output}");
    assert!(!output.contains("Subject 1"), "a message without the text was shown:\n{output}");
    assert!(!output.contains("Subject 3"), "a message without the text was shown:\n{output}");
}

/// Packing may renumber a base so it no longer starts at one. Jumping to a
/// number from inside the read loop has to clamp against the numbers the base
/// holds rather than against how many it has.
#[test]
fn test_cmd_r_jumps_within_a_renumbered_base() {
    let output = test_output("R\n500\n502\n\n\n".to_string(), |board| {
        crate::tests::setup_conference_with_messages(board);
        let path = board.conferences[0].areas.as_ref().unwrap()[0].path.clone();
        let mut base = jamjam::jam::JamMessageBase::open(path).unwrap();
        base.pack(&jamjam::jam::pack::PackOptions::default().with_renumber_from(500)).unwrap();
    });

    assert!(output.contains("Subject 3"), "the jump to 502 did not arrive:\n{output}");
}

/// Y reads only what is addressed to you. The test messages go to ALL, so there
/// is nothing to read.
#[test]
fn test_cmd_r_your_messages_skips_the_ones_to_all() {
    let output = test_output("R\nY 1+\n\n\n\n".to_string(), crate::tests::setup_conference_with_messages);
    assert!(!output.contains("Subject 1"), "a message to ALL is not addressed to you:\n{output}");
}

/// YA is Y plus the messages addressed to ALL, so the same messages come back.
#[test]
fn test_cmd_r_ya_takes_the_messages_to_all() {
    let output = test_output("R\nYA 1+\n\n\n\n".to_string(), crate::tests::setup_conference_with_messages);
    assert!(output.contains("Subject 1"), "YA must include the messages to ALL:\n{output}");
}

/// FROM narrows the read down to one sender.
#[test]
fn test_cmd_r_from_a_stranger_finds_nothing() {
    let output = test_output("R\nFROM NOBODY\n1+\n\n\n\n".to_string(), crate::tests::setup_conference_with_messages);
    assert!(!output.contains("Subject 1"), "the sender does not match:\n{output}");
}

/// The same read with the real sender shows the messages.
#[test]
fn test_cmd_r_from_the_sender_finds_the_messages() {
    let output = test_output("R\nFROM SYSOP\n1+\n\n\n\n".to_string(), crate::tests::setup_conference_with_messages);
    assert!(output.contains("Subject 1"), "the sender matches:\n{output}");
}

/// A conference with a single message area is shaped the way PCBoard expects,
/// so a move asks for the conference and nothing else.
#[test]
fn test_cmd_r_move_does_not_ask_for_an_area_when_there_is_one() {
    let output = test_output("R\n1\nMOVE 1\n\n\n\n".to_string(), crate::tests::setup_conference_with_messages);
    assert!(!output.contains("Area # to enter"), "one area needs no question:\n{output}");
    assert!(output.contains("Message Moved"), "the move did not happen:\n{output}");
}

/// Message areas are an icy_board addition. When the target conference has more
/// than one, the reader has to say which.
#[test]
fn test_cmd_r_move_asks_for_the_area_when_there_are_several() {
    let output = test_output("R\n1\nMOVE 1\n2\n\n\n\n".to_string(), crate::tests::setup_conference_with_two_areas);
    assert!(output.contains("Area # to enter"), "the area question is missing:\n{output}");
    assert!(output.contains("Message Moved"), "the move did not happen:\n{output}");
}

/// S reads from the last-read pointer forward, not from the bottom of the base.
#[test]
fn test_cmd_r_since_starts_after_the_last_read_pointer() {
    let output = test_output("R\nSET 2\nS\n\n\n\n".to_string(), crate::tests::setup_conference_with_messages);
    assert!(output.contains("Subject 3"), "S must read what comes after the pointer:\n{output}");
    assert!(!output.contains("Subject 1"), "S must not go back before the pointer:\n{output}");
}

/// E inside the read loop asks which field to edit and then for the new value.
#[test]
fn test_cmd_r_edit_header_asks_for_the_field_and_the_value() {
    let output = test_output("R\n1\nE\nS\nA New Subject\n\n\n\n".to_string(), crate::tests::setup_conference_with_messages);
    assert!(output.contains("(S)ubject"), "the field question is missing:\n{output}");
    assert!(output.contains("New Info"), "the value question is missing:\n{output}");
    // Once as the echo of what was typed, once in the header that is shown again.
    assert!(
        output.matches("A New Subject").count() >= 2,
        "the header still carries the old subject:\n{output}"
    );
}

/// A message longer than a page stops at the MORE prompt. The reader prints the
/// body a line at a time, which is what PCBoard counted towards that prompt.
#[test]
fn test_cmd_r_long_message_stops_at_the_more_prompt() {
    let output = test_output("R\n1\n\n\n\n\n".to_string(), crate::tests::setup_conference_with_a_long_message);
    assert!(output.contains("More"), "a message longer than a page must pause:\n{output}");
}

/// A PPE stuffs its commands in whatever case it likes, and PCBoard uppercased a
/// stuffed line before tokenizing it. So `r a wait` is ALL plus the WAIT option and
/// not a text to search for, which would ask where to begin the search.
#[test]
fn test_cmd_r_lower_case_options_stay_options() {
    let output = test_output("r a wait\n\n\n\n".to_string(), crate::tests::setup_conference_with_messages);
    assert!(!output.contains("Begin Search"), "the options were taken as search text:\n{output}");
}

/// PCBoard asked whether to resume an (A)ll scan only when an earlier one had stopped
/// part way - getallresumestatus() looks at Status.StartConf. Without one there is
/// nothing to resume, so the question does not come up.
#[test]
fn test_cmd_r_all_does_not_ask_to_resume_a_scan_that_never_stopped() {
    let output = test_output("R A\n\n\n\n".to_string(), crate::tests::setup_conference_with_messages);
    assert!(!output.contains("Continue with scan"), "there is no scan to resume:\n{output}");
}

/// The original walks read prompt, message, end of message prompt, read prompt
/// again - verified against PCBoard 15.4.
#[test]
fn test_cmd_r_walks_like_the_original() {
    let output = test_output("R\n1\n\n\n".to_string(), crate::tests::setup_conference_with_messages);
    assert_eq!(output.matches("Message Read Command?").count(), 2, "{output}");
    assert_eq!(output.matches("End of Message Command?").count(), 1, "{output}");
    for field in ["To: ", "From: ", "Subj: "] {
        assert!(output.contains(field), "the header is missing {field}:\n{output}");
    }
}

/// WHO inside the read loop runs the node list instead of being swallowed, and
/// waits before the message is drawn over the top of it.
#[test]
fn test_cmd_r_who_runs_inside_the_read_loop() {
    let output = test_output("R\n1\nWHO\n\n\n\n".to_string(), crate::tests::setup_conference_with_messages);
    assert!(output.contains("Handling Mail"), "WHO did not run:\n{output}");
    let after_who = output.split("WHO").nth(1).unwrap_or_default();
    assert!(
        after_who.contains("Press (Enter) to continue"),
        "the node list was not held on screen:\n{output}"
    );
}

/// SKIP leaves the read loop rather than asking for another message command.
#[test]
fn test_cmd_r_skip_leaves_the_read_loop() {
    let output = test_output("R\n1\nSKIP\n\n\n".to_string(), crate::tests::setup_conference_with_messages);
    let after_skip = output.split("SKIP").nth(1).unwrap_or_default();
    assert!(!after_skip.contains("End of Message"), "SKIP stayed in the read loop:\n{output}");
    assert!(!after_skip.contains("Invalid Entry"), "SKIP was not handled:\n{output}");
}

/// X uses the transfer UI. Cancelling its protocol prompt must return to the
/// reader, not reject the implemented command or start a transfer.
#[test]
fn test_cmd_r_export_protocol_prompt_can_be_cancelled() {
    let (output, base) = persisted_read_with_session("R\nO 1\nX\nN\n\n\n", false, |board| {
        // No usable default: exercise the export protocol prompt, not an
        // actual transfer (which would need a remote protocol peer).
        board.users[0].protocol = "?".to_string();
        board.config.sysop_command_level.read_all_mail = icy_board_engine::icy_board::security_expr::SecurityExpression::from_req_security(0);
    });
    assert!(output.contains("Protocol Type for Transfer"), "the export prompt is missing:\n{output}");
    assert!(!output.contains("Invalid Entry"), "X was rejected:\n{output}");
    assert!(!output.contains("Sending File(s)"), "cancel started a transfer:\n{output}");
    assert_eq!(output.matches("Body of message 1").count(), 2, "cancel must redisplay the message:\n{output}");
    assert_eq!(output.matches("End of Message Command?").count(), 2, "cancel must return to the reader:\n{output}");
    assert!(base.read_last_read_file().unwrap().is_empty());
}

/// Keep the base path, not the running session, so these assert disk state after
/// the reader has returned to the main command prompt.
fn persisted_read(input: &str, setup: impl Fn(&mut icy_board_engine::icy_board::IcyBoard)) -> (String, jamjam::jam::JamMessageBase) {
    persisted_read_with_session(input, true, setup)
}

fn persisted_read_with_session(input: &str, local: bool, setup: impl Fn(&mut icy_board_engine::icy_board::IcyBoard)) -> (String, jamjam::jam::JamMessageBase) {
    let path = std::sync::Mutex::new(None);
    let init = |board: &mut icy_board_engine::icy_board::IcyBoard| {
        crate::tests::setup_conference_with_messages(board);
        board.config.message.update_last_read_pointer = true;
        setup(board);
        *path.lock().unwrap() = Some(board.conferences[0].areas.as_ref().unwrap()[0].path.clone());
    };
    let output = if local {
        test_output(input.to_string(), init)
    } else {
        crate::tests::test_remote_output(input.to_string(), init)
    };
    (output, jamjam::jam::JamMessageBase::open(path.into_inner().unwrap().unwrap()).unwrap())
}

fn last_pointer(base: &jamjam::jam::JamMessageBase) -> u32 {
    base.read_last_read_file().unwrap().into_iter().map(|last| last.last_read_msg).max().unwrap_or(0)
}

#[test]
fn test_cmd_r_successful_range_advances_pointer_but_backwards_never_lowers_it() {
    let (output, base) = persisted_read("R\n1+\n\n\n\n3-1\n\n\n\n\n", |_| {});
    assert!(output.contains("Body of message 3"), "{output}");
    assert_eq!(last_pointer(&base), 3);
}

#[test]
fn test_cmd_r_no_matches_and_disjoint_ranges_do_not_create_read_records() {
    let (output, base) = persisted_read("R\nTS XYZZY 1+\n99\n\n", |_| {});
    assert!(!output.contains("Body of message"), "{output}");
    assert!(base.read_last_read_file().unwrap().is_empty());
}

#[test]
fn test_cmd_r_o_sticks_across_inner_and_outer_commands() {
    let (output, base) = persisted_read("R\nO 1\n2\n\n3\n\n\n", |_| {});
    assert!(output.contains("Body of message 3"), "{output}");
    assert!(base.read_last_read_file().unwrap().is_empty());
}

#[test]
fn test_cmd_r_config_can_disable_pointer_updates() {
    let (output, base) = persisted_read("R\n1+\n\n\n\n\n", |board| board.config.message.update_last_read_pointer = false);
    assert!(output.contains("Body of message 3"), "{output}");
    assert!(base.read_last_read_file().unwrap().is_empty());
}

fn address_first_to_sysop(board: &mut icy_board_engine::icy_board::IcyBoard) {
    let mut base = jamjam::jam::JamMessageBase::open(&board.conferences[0].areas.as_ref().unwrap()[0].path).unwrap();
    let mut header = base.read_header(1).unwrap();
    header.set_from(bstr::BString::from("TEST USER"));
    header.set_to(bstr::BString::from("SYSOP"));
    header.attributes |= jamjam::jam::attributes::MSG_PRIVATE | jamjam::jam::attributes::MSG_RECEIPTREQ;
    jamjam::jam::raw::update_header(&mut base, 1, &header).unwrap();
}

#[test]
fn test_cmd_r_recipient_read_sets_date_and_delivers_one_receipt_on_redisplay() {
    let (output, base) = persisted_read("R\n1\n/\n\n\n", address_first_to_sysop);
    assert!(output.contains("Body of message 1"), "{output}");
    let original = base.read_header(1).unwrap();
    assert!(original.is_read());
    assert_ne!(original.date_received, 0);
    assert!(!original.is_receipt_req());
    assert_eq!(base.highest_message_number(), 4);
    assert_eq!(base.read_header(4).unwrap().reply_to, 1);
}

#[test]
fn test_cmd_r_privileged_o_preserves_status_and_receipt_request() {
    let (_, base) = persisted_read("R\nO 1\n\n\n", |board| {
        address_first_to_sysop(board);
        board.config.sysop_command_level.not_update_msg_read = icy_board_engine::icy_board::security_expr::SecurityExpression::from_req_security(0);
    });
    let header = base.read_header(1).unwrap();
    assert!(!header.is_read());
    assert_eq!(header.date_received, 0);
    assert!(header.is_receipt_req());
    assert_eq!(base.highest_message_number(), 3);
    assert_eq!(last_pointer(&base), 0);
}

#[test]
fn test_cmd_r_unread_filter_uses_header_status_even_below_last_read() {
    let (output, _) = persisted_read("R\nSET 3\nU 1+\n\n\n\n", |board| {
        let mut base = jamjam::jam::JamMessageBase::open(&board.conferences[0].areas.as_ref().unwrap()[0].path).unwrap();
        jamjam::jam::raw::set_attributes(&mut base, 2, jamjam::jam::attributes::MSG_READ, 0).unwrap();
    });
    assert!(output.contains("Body of message 1"), "{output}");
    assert!(!output.contains("Body of message 2"), "{output}");
    assert!(output.contains("Body of message 3"), "{output}");
}

#[test]
fn test_cmd_r_inner_search_rebuilds_filter_and_supports_multiple_ranges() {
    let (output, _) = persisted_read("R\n1\nTS BANANA 2 3\n\n\n", |_| {});
    assert!(output.contains("Body of message 2"), "{output}");
    assert!(!output.contains("Body of message 3"), "{output}");
    let (output, _) = persisted_read("R\n1\n2 3\n\n\n\n", |_| {});
    assert!(output.contains("Body of message 2"), "{output}");
    assert!(output.contains("Body of message 3"), "{output}");
}

#[test]
fn test_cmd_r_thread_uses_displayed_subject_and_skips_unrelated_messages() {
    let (output, _) = persisted_read("R\n1\nT+\n\n\n", |board| {
        let mut base = jamjam::jam::JamMessageBase::open(&board.conferences[0].areas.as_ref().unwrap()[0].path).unwrap();
        let mut header = base.read_header(3).unwrap();
        header.set_subject(bstr::BString::from("Re: Subject 1"));
        jamjam::jam::raw::update_header(&mut base, 3, &header).unwrap();
    });
    assert!(output.contains("Body of message 3"), "{output}");
    assert!(!output.contains("Body of message 2"), "{output}");
}

fn second_conference_message(board: &mut icy_board_engine::icy_board::IcyBoard) {
    let mut base = jamjam::jam::JamMessageBase::create(&board.conferences[1].areas.as_ref().unwrap()[0].path).unwrap();
    base.write_message(&jamjam::jam::JamMessage::default().with_from(bstr::BString::from("SYSOP"))
        .with_to(bstr::BString::from("ALL")).with_subject(bstr::BString::from("Other conference"))
        .with_text(bstr::BString::from("OTHER-CONFERENCE-BODY"))).unwrap();
    base.write_jhr_header().unwrap();
}

#[test]
fn test_cmd_r_all_traverses_and_restores_original_conference() {
    let (output, base) = persisted_read("R\nALL 1\n\n\n1\n\n\n", second_conference_message);
    assert!(output.contains("OTHER-CONFERENCE-BODY"), "{output}");
    assert_eq!(output.matches("Body of message 1").count(), 2, "{output}");
    assert_eq!(last_pointer(&base), 1);
}

#[test]
fn test_cmd_r_a_only_visits_selected_conferences_and_wait_only_mail_waiting() {
    use icy_board_engine::icy_board::user_base::ConferenceFlags;
    let (output, _) = persisted_read("R\nA 1\n\n\n", second_conference_message);
    assert!(!output.contains("OTHER-CONFERENCE-BODY"), "{output}");
    let (output, _) = persisted_read("R\nA 1\n\n\n\n", |board| {
        second_conference_message(board);
        board.users[0].conference_flags.insert(1, ConferenceFlags::Selected);
    });
    assert!(output.contains("OTHER-CONFERENCE-BODY"), "{output}");
    let (output, _) = persisted_read("R\nWAIT 1\n\n\n", |board| {
        second_conference_message(board);
        board.users[0].conference_flags.insert(1, ConferenceFlags::MailWaiting);
    });
    assert!(output.contains("OTHER-CONFERENCE-BODY"), "{output}");
    assert!(!output.contains("Body of message 1"), "{output}");
}

#[test]
fn test_cmd_r_n_stops_remaining_conferences_and_restores_context() {
    let (output, _) = persisted_read("R\nALL 1\nN\nR\n1\n\n\n", second_conference_message);
    assert!(!output.contains("OTHER-CONFERENCE-BODY"), "{output}");
    assert_eq!(output.matches("Body of message 1").count(), 2, "{output}");
}

#[test]
fn test_cmd_r_body_read_failure_does_not_advance_pointer_or_mark_read() {
    let (output, base) = persisted_read("R\n1\n\n", |board| {
        address_first_to_sysop(board);
        std::fs::write(board.conferences[0].areas.as_ref().unwrap()[0].path.with_extension("jdt"), b"").unwrap();
    });
    assert!(!output.contains("Body of message"), "{output}");
    assert_eq!(last_pointer(&base), 0);
    assert!(!base.read_header(1).unwrap().is_read());
    assert!(base.read_header(1).unwrap().is_receipt_req());
}

fn password_first(board: &mut icy_board_engine::icy_board::IcyBoard) {
    address_first_to_sysop(board);
    let mut base = jamjam::jam::JamMessageBase::open(&board.conferences[0].areas.as_ref().unwrap()[0].path).unwrap();
    let mut header = base.read_header(1).unwrap();
    header.password_crc = jamjam::jam::JamMessageBase::crc(&bstr::BString::from("SECRET"));
    jamjam::jam::raw::update_header(&mut base, 1, &header).unwrap();
}

#[test]
fn test_cmd_r_failed_group_password_has_no_read_side_effects() {
    use icy_board_engine::icy_board::security_expr::{SecurityExpression, Value};
    let (output, base) = persisted_read("R\n1\nWRONG\nWRONG\nWRONG\n\n", |board| {
        password_first(board);
        board.config.sysop_command_level.read_all_mail = SecurityExpression::Constant(Value::Bool(false));
    });
    assert!(!output.contains("Body of message 1"), "{output}");
    assert_eq!(last_pointer(&base), 0);
    assert!(!base.read_header(1).unwrap().is_read());
    assert_eq!(base.read_header(1).unwrap().date_received, 0);
    assert!(base.read_header(1).unwrap().is_receipt_req());
    assert_eq!(base.highest_message_number(), 3);
}

#[test]
fn test_cmd_r_group_password_text_search_reveals_no_hit_and_preserves_pointer() {
    use icy_board_engine::icy_board::security_expr::{SecurityExpression, Value};
    // Body hit, no hit, and header hit must all skip the protected message
    // without a password prompt that would reveal whether the text matched.
    for term in ["BODY", "XYZZY", "SUBJECT"] {
        let (output, base) = persisted_read(&format!("R\nSET 2\nTS {term} 1\n\n"), |board| {
            password_first(board);
            board.config.sysop_command_level.read_all_mail = SecurityExpression::Constant(Value::Bool(false));
        });
        assert!(output.contains("no mail found to read"), "protected search reported a hit:\n{output}");
        assert!(!output.contains("Password to Read"), "search leaked a hit through authorization:\n{output}");
        assert!(!output.contains("Subject 1"), "search disclosed the matching header:\n{output}");
        assert!(!output.contains("Body of message"), "search disclosed a body:\n{output}");
        assert!(!output.contains("End of Message Command?"), "search entered a protected message:\n{output}");
        let pointers = base.read_last_read_file().unwrap();
        assert_eq!(pointers.len(), 1);
        assert_eq!(pointers[0].last_read_msg, 2);
        // SET changes last_read_msg only; the search must not raise high_read_msg.
        assert_eq!(pointers[0].high_read_msg, 0);
        let header = base.read_header(1).unwrap();
        assert!(!header.is_read());
        assert_eq!(header.date_received, 0);
        assert!(header.is_receipt_req());
        assert_eq!(base.highest_message_number(), 3);
    }
}

#[test]
fn test_cmd_r_read_all_mail_bypasses_group_password() {
    let (output, base) = persisted_read("R\n1\n\n\n", |board| {
        password_first(board);
        board.config.sysop_command_level.read_all_mail = icy_board_engine::icy_board::security_expr::SecurityExpression::from_req_security(0);
    });
    assert!(output.contains("Body of message 1"), "{output}");
    assert!(!output.contains("Password to Read"), "{output}");
    assert_eq!(last_pointer(&base), 1);
    assert!(base.read_header(1).unwrap().is_read());
}

#[test]
fn test_cmd_r_o_without_status_privilege_still_marks_recipient_mail_read() {
    use icy_board_engine::icy_board::security_expr::{SecurityExpression, Value};
    let (_, base) = persisted_read("R\nO 1\n\n\n", |board| {
        address_first_to_sysop(board);
        board.config.sysop_command_level.not_update_msg_read = SecurityExpression::Constant(Value::Bool(false));
    });
    assert_eq!(last_pointer(&base), 0);
    assert!(base.read_header(1).unwrap().is_read());
    assert_eq!(base.highest_message_number(), 4);
}

#[test]
fn test_cmd_r_all_skips_denied_conferences_and_areas_even_for_sysop() {
    use icy_board_engine::icy_board::security_expr::{SecurityExpression, Value};
    for deny_conference in [true, false] {
        let (output, _) = persisted_read("R\nALL 1\n\n\n", |board| {
            second_conference_message(board);
            if deny_conference {
                board.conferences[1].required_security = SecurityExpression::Constant(Value::Bool(false));
            } else {
                std::sync::Arc::make_mut(board.conferences[1].areas.as_mut().unwrap())[0].req_level_to_list = SecurityExpression::Constant(Value::Bool(false));
            }
        });
        assert!(!output.contains("OTHER-CONFERENCE-BODY"), "{output}");
    }
}

#[test]
fn test_cmd_r_short_long_and_help_execute_inside_reader() {
    let (output, _) = persisted_read("R\nSHORT 1\nLONG\nH\n\n\n", |board| {
        let path = board.root_path.join("reader-help");
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("hlpendr"), b"READER-HELP-MARKER\r\n").unwrap();
        board.config.paths.help_path = path;
    });
    assert!(output.contains("READER-HELP-MARKER"), "{output}");
    let (short, long) = output.split_once("LONG").unwrap();
    assert!(!short.contains("Status:"), "{output}");
    assert!(long.contains("Status:"), "{output}");
}

#[test]
fn test_cmd_r_capture_can_be_cancelled_without_body_or_read_effects() {
    for option in ["C", "D", "Z", "QWK"] {
        // Protocol cancellation requires a remote session; local capture uses
        // a host directory picker and never asks for a transfer protocol.
        let (output, base) = persisted_read_with_session(&format!("R\n1 {option}\nN\n\n"), false, address_first_to_sysop);
        assert!(output.contains("Total Messages Captured for Download"), "capture did not run:\n{output}");
        assert!(!output.contains("Invalid Entry"), "capture was rejected:\n{output}");
        if option == "C" {
            assert!(output.contains("Download Flagged Files"), "capture confirmation is missing:\n{output}");
            assert!(!output.contains("Protocol Type for Transfer"), "declined capture reached transfer:\n{output}");
        } else {
            assert!(output.contains("Protocol Type for Transfer"), "capture protocol prompt is missing:\n{output}");
            assert!(output.contains("Transfer Aborted"), "protocol cancellation was ignored:\n{output}");
        }
        assert!(!output.contains("Sending File(s)"), "cancel started a transfer:\n{output}");
        assert!(!output.contains("Body of message"), "{output}");
        assert!(base.read_last_read_file().unwrap().is_empty());
        let header = base.read_header(1).unwrap();
        assert!(!header.is_read());
        assert_eq!(header.date_received, 0);
        assert!(header.is_receipt_req());
        assert_eq!(base.highest_message_number(), 3);
    }
}

#[test]
fn test_cmd_r_outer_kill_uses_its_number_instead_of_reading_it() {
    let (output, base) = persisted_read("R\nK 2\n\n", |_| {});
    assert!(output.contains("Message Killed"), "{output}");
    assert!(!output.contains("Body of message 2"), "{output}");
    assert!(base.read_header(2).map_or(true, |header| header.is_deleted()));
    assert_eq!(last_pointer(&base), 0);
}

#[test]
fn test_cmd_r_deselect_is_not_the_interactive_select_menu() {
    let (output, _) = persisted_read("R\n1\nDESELECT\n\n\n\n", |_| {});
    assert!(output.to_ascii_lowercase().contains("deselected"), "{output}");
    assert!(!output.contains("Conference Numbers"), "{output}");
    // READNEXT retains the original one-message range, rather than widening it.
    assert!(!output.contains("Body of message 2"), "{output}");
}

#[test]
fn test_cmd_r_inner_all_handoff_reaches_other_conferences() {
    let (output, _) = persisted_read("R\n1\nALL 1\n\n\n\n", second_conference_message);
    assert!(output.contains("OTHER-CONFERENCE-BODY"), "{output}");
}
