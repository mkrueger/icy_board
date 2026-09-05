use std::sync::Arc;

use icy_board_engine::icy_board::{
    commands::{Command, CommandAction, CommandType},
    conferences::Conference,
    doors::{Door, DoorList, DoorType, DropFile},
};

use crate::tests::{compile_test_ppe, test_user_output};

/// A door of its own, so the test does not depend on anything the board ships.
fn board_with_door(board: &mut icy_board_engine::icy_board::IcyBoard, keyword: &str, parameter: &str) {
    let ppe = compile_test_ppe("PRINTLN \"door yyy running\"");
    board.conferences.push(Conference {
        name: "Main Board".to_string(),
        doors: Some(Arc::new(DoorList {
            accounts: Vec::new(),
            doors: vec![Door {
                name: "MRC".to_string(),
                door_type: DoorType::Local,
                path: ppe.to_string_lossy().to_string(),
                drop_file: DropFile::None,
                ..Default::default()
            }],
        })),
        ..Default::default()
    });
    board.commands.push(Command {
        keyword: keyword.to_string(),
        actions: vec![CommandAction {
            command_type: CommandType::Door,
            parameter: parameter.to_string(),
            ..Default::default()
        }],
        ..Default::default()
    });
}

#[test]
fn test_a_command_can_open_the_door_it_names() {
    let output = test_user_output("MRC\n".to_string(), |board| board_with_door(board, "MRC", "mrc"));

    assert!(output.contains("door yyy running"), "{output}");
}

#[test]
fn test_a_command_naming_no_door_says_which_one_it_missed() {
    let output = test_user_output("GAME\n".to_string(), |board| board_with_door(board, "GAME", "tetris"));

    assert!(!output.contains("door yyy running"), "{output}");
    assert!(output.contains("tetris") && output.contains("invalid DOOR"), "{output}");
}
