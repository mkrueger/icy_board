use crate::tests::test_output;

/// The order of the W questions is a compatibility contract: PPEs pre-answer it
/// with KBDSTUFF, so an extra or missing question shifts every following stuffed
/// answer onto the wrong field. This is the sequence PCBoard walks, captured
/// from a live board.
#[test]
fn test_cmd_w_prompt_order() {
    // One Enter per question, plus a few spare so the command runs to the end.
    let output = test_output(format!("W\n{}", "\n".repeat(30)), |_| {});
    assert_prompt_order(
        &output,
        &[
            "Password (one word please)",
            "City and State calling from",
            "Business or data phone # is",
            "Home or voice phone # is",
            "Clear the screen between each message",
            "Scroll multi-screen messages",
            "Use long headers when reading messages",
            "Full Screen Editor Default",
            "Set editor workspace default to 79 columns",
            "Set default file description to SHORT",
            "Select Conference(s)",
            "Set Message Capture Limit",
            "Set Per-Conference Message Capture Limit",
            "Maximum Size to Auto-Include Personal Attachments in QWK Packet",
            "Maximum Size to Auto-Include Attachments in QWK Packet",
        ],
    );
}

/// An empty answer to the password question returns right away, so the
/// confirmation is never asked.
#[test]
fn test_cmd_w_empty_password_skips_confirmation() {
    let output = test_output(format!("W\n{}", "\n".repeat(30)), |_| {});
    assert!(!output.contains("Re-Enter"), "an empty password must not ask for a confirmation:\n{output}");
}

fn assert_prompt_order(output: &str, prompts: &[&str]) {
    let mut pos = 0;
    for prompt in prompts {
        match output[pos..].find(prompt) {
            Some(found) => pos += found + prompt.len(),
            None => panic!("prompt {prompt:?} is missing or out of order, searched from byte {pos} of:\n{output}"),
        }
    }
}
