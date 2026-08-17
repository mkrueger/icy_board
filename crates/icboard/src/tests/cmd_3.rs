use crate::tests::{setup_conference_with_messages, test_output};

#[test]
fn sysop_pack_message_base_asks_pcboards_questions() {
    let output = test_output("3\nY\nN\n\nN\nN\n".to_string(), setup_conference_with_messages);

    assert!(output.contains("Pack the message base"), "the confirmation is missing:\n{output}");
    assert!(output.contains("Generate ONLY a New Index File"), "the index question is missing:\n{output}");
    assert!(output.contains("Purge older than"), "the date question is missing:\n{output}");
    assert!(output.contains("Purge RECEIVED+PRIVATE Msgs"), "the private mail question is missing:\n{output}");
    assert!(output.contains("Renumber during repack"), "the renumber question is missing:\n{output}");
    assert!(output.contains("Messages Successfully Packed"), "the pack did not report success:\n{output}");
}

#[test]
fn sysop_pack_message_base_answering_no_asks_nothing_else() {
    let output = test_output("3\nN\n".to_string(), setup_conference_with_messages);

    assert!(output.contains("Pack the message base"), "the confirmation is missing:\n{output}");
    assert!(!output.contains("Generate ONLY a New Index File"), "declining still ran the pack:\n{output}");
}

#[test]
fn sysop_pack_message_base_asks_for_the_new_low_number_when_renumbering() {
    let output = test_output("3\nY\nN\n\nN\nY\n1\n".to_string(), setup_conference_with_messages);

    assert!(output.contains("NEW low starting Message #"), "the new low number was not asked for:\n{output}");
    assert!(output.contains("Messages Successfully Packed"), "the pack did not report success:\n{output}");
}

#[test]
fn sysop_pack_message_base_removes_the_killed_message() {
    let output = test_output("K 2\n\n3\nY\nN\n\nN\nN\n".to_string(), setup_conference_with_messages);

    assert!(output.contains("1 message(s) removed."), "the killed message was not packed out:\n{output}");
}

/// The index rebuild is the one answer that skips every other question.
#[test]
fn sysop_pack_message_base_index_only_skips_the_criteria() {
    let output = test_output("3\nY\nY\n".to_string(), setup_conference_with_messages);

    assert!(output.contains("Generate ONLY a New Index File"), "the index question is missing:\n{output}");
    assert!(!output.contains("Purge older than"), "the criteria were asked for anyway:\n{output}");
    assert!(output.contains("Messages Successfully Packed"), "the index rebuild did not report success:\n{output}");
}
