use icy_board_engine::icy_board::{
    IcyBoard,
    commands::{Command, CommandAction, CommandType},
};

use crate::tests::{fixture, test_user_output};

fn with_command(board: &mut IcyBoard, keyword: &str, command_type: CommandType, parameter: &str) {
    board.commands.push(Command {
        keyword: keyword.to_string(),
        actions: vec![CommandAction {
            command_type,
            parameter: parameter.to_string(),
            ..Default::default()
        }],
        ..Default::default()
    });
}

/// What `CMD.LST` is made of: a keyword of the sysop's choosing standing for
/// what the caller would otherwise have typed.
#[test]
fn test_stuffed_text_is_acted_upon_as_if_it_were_typed() {
    let output = test_user_output("EXPERT\n".to_string(), |board| {
        with_command(board, "EXPERT", CommandType::StuffText, "X");
    });

    assert!(output.contains("Expert mode is now on"), "{output}");
}

#[test]
fn test_a_command_can_run_another_command() {
    let output = test_user_output("GO\n".to_string(), |board| {
        with_command(board, "GO", CommandType::Command, "X");
    });

    assert!(output.contains("Expert mode is now on"), "{output}");
}

#[test]
fn test_a_command_can_display_a_file() {
    let output = test_user_output("INFO\n".to_string(), |board| {
        with_command(board, "INFO", CommandType::DisplayFile, fixture("main/blt1").to_str().unwrap());
    });

    assert!(output.contains("BULLETIN1"), "{output}");
}

/// A disabled option is one the sysop kept but turned off, not a failure.
#[test]
fn test_a_disabled_command_does_nothing() {
    let output = test_user_output("NOPE\n".to_string(), |board| {
        with_command(board, "NOPE", CommandType::Disabled, "");
    });

    assert!(!output.contains("Invalid"), "{output}");
    assert!(!output.contains("action"), "{output}");
}
