use crate::tests::{setup_conference_with_messages, test_output};

/// The scan used to stop one short, so the newest message never showed up.
#[test]
fn quick_scan_lists_every_message() {
    let output = test_output("Q\n1\n".to_string(), setup_conference_with_messages);

    for subject in ["Subject 1", "Subject 2", "Subject 3"] {
        assert!(output.contains(subject), "{subject} is missing from the scan:\n{output}");
    }
}

/// The range the prompt offers has to be the range that exists.
#[test]
fn quick_scan_offers_the_real_message_range() {
    let output = test_output("Q\n\n".to_string(), setup_conference_with_messages);

    assert!(output.contains("(1-3)"), "the offered range does not match the base:\n{output}");
}

#[test]
fn quick_scan_starts_at_the_number_it_was_given() {
    let output = test_output("Q\n2\n".to_string(), setup_conference_with_messages);

    assert!(!output.contains("Subject 1"), "the scan started below the given number:\n{output}");
    assert!(output.contains("Subject 3"), "the scan stopped short:\n{output}");
}
