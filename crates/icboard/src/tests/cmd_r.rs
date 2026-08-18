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

/// A command the reader parses but cannot run has to answer. Silence reads as a
/// broken board rather than a missing feature. X is the export PCBoard had.
#[test]
fn test_cmd_r_an_unrunnable_command_still_answers() {
    let output = test_output("R\n1\nX\n\n\n\n".to_string(), crate::tests::setup_conference_with_messages);
    assert!(output.contains("Invalid Entry"), "the reader stayed silent:\n{output}");
}
