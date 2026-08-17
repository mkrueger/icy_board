use crate::tests::{setup_conference, test_output};

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
    assert!(
        !output.contains("Re-enter password"),
        "an empty password must not ask for a confirmation:\n{output}"
    );
}

fn setup_existing_alias(board: &mut icy_board_engine::icy_board::IcyBoard, allow_change: bool) {
    setup_conference(board);
    board.conferences[0].allow_aliases = true;
    board.users[0].alias = "OLDALIAS".to_string();
    board.config.new_user_settings.ask_alias = true;
    board.config.system_control.allow_alias_change = allow_change;
}

#[test]
fn test_cmd_w_keeps_an_existing_alias_when_changes_are_disabled() {
    let output = test_output(format!("W\n{}", "\n".repeat(30)), |board| setup_existing_alias(board, false));
    assert!(
        !output.contains("Alias Name"),
        "the alias question should be locked after choosing one:\n{output}"
    );
}

#[test]
fn test_cmd_w_asks_for_an_existing_alias_when_changes_are_allowed() {
    let output = test_output(format!("W\n{}", "\n".repeat(30)), |board| setup_existing_alias(board, true));
    assert!(output.contains("Alias Name"), "the alias question is missing:\n{output}");
}

/// Saying yes to the conference question hands over to the register mode of
/// SELECT, which asks which flags the numbers that follow should get.
#[test]
fn test_cmd_w_select_conferences_asks_for_the_flags() {
    let input = format!("W\n{}Y\n1\n\nQ\n{}", "\n".repeat(11), "\n".repeat(20));
    let output = test_output(input, setup_conference);
    assert!(output.contains("Conf. Flags"), "the conference flags prompt is missing:\n{output}");
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

/// PCBoard judged the password before it asked for the confirmation, and asked
/// the question again for every refusal.
fn change_password(answers: &str, min_len: u8) -> String {
    test_output(format!("W\n{answers}{}", "\n".repeat(30)), move |board| {
        board.config.limits.min_pwd_length = min_len;
    })
}

#[test]
fn test_cmd_w_refuses_a_short_password() {
    let output = change_password("abc\n", 6);
    assert!(output.contains("Password too short"), "a password under the minimum was taken:\n{output}");
    assert!(
        !output.contains("Re-enter password"),
        "a refused password must not reach the confirmation:\n{output}"
    );
}

#[test]
fn test_cmd_w_refuses_a_password_out_of_the_name() {
    let output = change_password("sysop\n", 0);
    assert!(
        output.contains("cannot be a subset of your name"),
        "a password taken from the caller's name was accepted:\n{output}"
    );
}

#[test]
fn test_cmd_w_asks_again_after_a_refusal() {
    let output = change_password("abc\nkaleidoscope\nkaleidoscope\n", 6);
    assert!(output.contains("Password too short"), "the short password was not refused:\n{output}");
    assert!(output.contains("Re-enter password"), "the retry never reached the confirmation:\n{output}");
    assert!(!output.contains("do not match"), "the confirmation was rejected:\n{output}");
}

#[test]
fn test_cmd_w_refuses_a_mistyped_confirmation() {
    let output = change_password("kaleidoscope\nkaleidoscopf\n", 0);
    assert!(output.contains("do not match"), "the mistyped confirmation was accepted:\n{output}");
}

#[test]
fn test_cmd_w_takes_a_good_password() {
    let output = change_password("kaleidoscope\nkaleidoscope\n", 6);
    for refusal in ["too short", "do not match", "subset of your name", "already used"] {
        assert!(!output.contains(refusal), "a good password was refused with {refusal:?}:\n{output}");
    }
}
