use crate::tests::{setup_conference, test_output};
use icy_engine::{TextPane, TextScreen};
use icy_parser_core::{AnsiParser, CommandParser};

fn rendered_lines(output: &str) -> Vec<String> {
    let mut screen = TextScreen::new((80, 25));
    let mut parser = AnsiParser::default();
    parser.parse(output.as_bytes(), &mut icy_engine::ScreenSink::new(&mut screen));
    (0..25).map(|y| (0..80).map(|x| screen.char_at((x, y).into()).ch).collect::<String>()).collect()
}

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

#[test]
fn full_screen_enter_keeps_message_lines_above_the_help_footer() {
    let output = test_output("E\nALL\nSubject\nY\nfirst oracle line\rsecond oracle line".to_string(), |board| {
        setup_conference(board);
        board.users[0].flags.use_graphics = true;
    });

    let lines = rendered_lines(&output);
    assert!(
        lines[2].starts_with("first oracle line"),
        "first line is not on the first editor row: {:?}",
        lines.iter().enumerate().filter(|(_, line)| !line.trim().is_empty()).collect::<Vec<_>>()
    );
    assert!(
        lines[3].starts_with("second oracle line"),
        "second line disappeared after Enter: {:?}",
        lines[3]
    );
    assert!(
        lines[22].contains("Press (Esc) to Exit"),
        "the editor help footer was overwritten: {:?}",
        lines[22]
    );
}
