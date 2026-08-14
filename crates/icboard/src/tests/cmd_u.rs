use std::path::PathBuf;

use icy_board_engine::icy_board::{IcyBoard, conferences::Conference};

use crate::tests::test_output;

fn setup_upload_directory(board: &mut IcyBoard) {
    board.conferences.push(Conference {
        name: "Main Board".to_string(),
        pub_upload_location: PathBuf::from("src/tests/main"),
        ..Default::default()
    });
}

/// TRANSFER.C asks for the description before anything is transferred.
#[test]
fn test_upload_asks_for_a_description() {
    let output = test_output("U\nTESTUP.ZIP\na proper description\n\n\n".to_string(), setup_upload_directory);
    assert!(output.contains("TESTUP.ZIP"), "{output}");
    assert!(output.contains("Private"), "the private hint is missing:\n{output}");
}

/// An empty first line abandons an upload that has not started yet.
#[test]
fn test_upload_empty_description_aborts() {
    let output = test_output("U\nTESTUP.ZIP\n\n\n".to_string(), setup_upload_directory);
    assert!(!output.contains("Protocol"), "the upload should have been abandoned:\n{output}");
}

/// Fewer than five characters is not a description, so the original asks again.
#[test]
fn test_upload_short_description_asks_again() {
    let output = test_output("U\nTESTUP.ZIP\nabc\na proper description\n\n\n".to_string(), setup_upload_directory);
    assert!(output.contains("longer description"), "{output}");
}
