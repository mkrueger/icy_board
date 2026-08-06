use crate::tests::test_output;

/// PCBoard joins the command arguments into the recipient field but still asks
/// the question, with the name pre-filled. If the
/// token were consumed silently, a PPE stuffing the whole E sequence would have
/// every following answer land on the wrong question.
#[test]
fn test_cmd_e_token_prefills_but_still_asks() {
    let output = test_output("E JOHN DOE\n\n".to_string(), |_| {});
    assert!(output.contains("To (Enter)="), "the recipient prompt is missing:\n{output}");
    assert!(output.contains("JOHN DOE"), "the recipient tokens were not pre-filled:\n{output}");
}

/// Recipient, then subject, then the security question.
#[test]
fn test_cmd_e_prompt_order() {
    let output = test_output("E\nA subject\nN\n\n\n".to_string(), |_| {});
    let to = output.find("To (Enter)=").expect("recipient prompt missing");
    let subject = output[to..].find("Subject (Enter)=").expect("subject prompt missing") + to;
    let security = output[subject..].find("Message Security").expect("security prompt missing") + subject;
    assert!(to < subject && subject < security, "prompts are out of order:\n{output}");
}

/// An empty subject aborts before the security question is reached.
#[test]
fn test_cmd_e_empty_subject_aborts() {
    let output = test_output("E\n\n".to_string(), |_| {});
    assert!(!output.contains("Message Security"), "an empty subject must abort:\n{output}");
}
