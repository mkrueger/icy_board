use crate::tests::{setup_conference, test_output};

/// PCBoard joins the command arguments into the recipient field but still asks
/// the question, with the name pre-filled. If the
/// token were consumed silently, a PPE stuffing the whole E sequence would have
/// every following answer land on the wrong question.
#[test]
fn test_cmd_e_token_prefills_but_still_asks() {
    let output = test_output("E SYSOP\n\n".to_string(), |_| {});
    assert!(output.contains("To (Enter)="), "the recipient prompt is missing:\n{output}");
    assert!(output.contains("SYSOP"), "the recipient token was not pre-filled:\n{output}");
}

/// Recipient, then subject, then the security question.
#[test]
fn test_cmd_e_prompt_order() {
    let output = test_output("E\nALL\nA subject\nN\n\n\n".to_string(), |_| {});
    let to = output.find("To (Enter)=").expect("recipient prompt missing");
    let subject = output[to..].find("Subject (Enter)=").expect("subject prompt missing") + to;
    let security = output[subject..].find("Message Security").expect("security prompt missing") + subject;
    assert!(to < subject && subject < security, "prompts are out of order:\n{output}");
}

/// An empty subject aborts before the security question is reached.
#[test]
fn test_cmd_e_empty_subject_aborts() {
    let output = test_output("E\nALL\n\n".to_string(), |_| {});
    assert!(!output.contains("Message Security"), "an empty subject must abort:\n{output}");
}

#[test]
fn test_cmd_e_validates_unknown_recipients() {
    let output = test_output("E\nNOBODY\nC\n\n".to_string(), |_| {});
    assert!(output.contains("Could not find"), "the unknown name was accepted:\n{output}");
    assert!(output.contains("e-enter user's name"), "the validation choice is missing:\n{output}");
}

#[test]
fn test_cmd_e_can_accept_unknown_recipients_when_validation_is_off() {
    let output = test_output("E\nNOBODY\n\n".to_string(), |board| {
        board.config.message.validate_to_name = false;
    });
    assert!(!output.contains("Could not find"), "validation ran despite the option being off:\n{output}");
}

#[test]
fn test_cmd_e_sc_asks_for_carbon_copies_when_enabled() {
    let output = test_output("E\nALL\nSubject\nN\n\nBody\n\nSC\n\n".to_string(), |board| {
        setup_conference(board);
        board.config.message.allow_carbon_copy = true;
        board.users[0].flags.fse_mode = icy_board_engine::icy_board::user_base::FSEMode::No;
    });
    assert!(output.contains("Carbon Copy To"), "the carbon-copy prompt is missing:\n{output}");
}

#[test]
fn test_cmd_e_sc_saves_without_carbon_copies_when_disabled() {
    let output = test_output("E\nALL\nSubject\nN\n\nBody\n\nSC\n".to_string(), |board| {
        setup_conference(board);
        board.config.message.allow_carbon_copy = false;
        board.users[0].flags.fse_mode = icy_board_engine::icy_board::user_base::FSEMode::No;
    });
    assert!(!output.contains("Carbon Copy To"), "the carbon-copy prompt was offered:\n{output}");
}
