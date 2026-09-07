use icy_board_engine::icy_board::{IcyBoard, conferences::Conference, security_expr::SecurityExpression};

use crate::tests::{test_dir, test_remote_output};

fn setup_upload_directory(board: &mut IcyBoard) {
    // These fixtures describe files and cancel, without a protocol peer or U's
    // automatic promotion to the batch filename loop.
    board.users[0].protocol = "N".to_string();
    board.config.file_transfer.promote_to_batch_transfers = false;
    board.conferences.push(Conference {
        name: "Main Board".to_string(),
        pub_upload_location: test_dir(),
        private_upload_location: test_dir(),
        ..Default::default()
    });
}

fn assert_returned_to_main_command(output: &str) {
    assert_eq!(output.matches("Main Board Command?").count(), 2, "upload did not finish cleanly:\n{output}");
    assert!(!output.contains("Invalid Entry"), "upload input leaked into the command loop:\n{output}");
    assert!(!output.contains("Transfer Successful"), "cancel claimed a transfer:\n{output}");
}

/// PCBoard asks for the description before anything is transferred.
#[test]
fn test_upload_asks_for_a_description() {
    let output = test_remote_output("U\nTESTUP.ZIP\na proper description\n\nN\n".to_string(), setup_upload_directory);
    assert!(output.contains("TESTUP.ZIP"), "{output}");
    assert!(output.contains("Private"), "the private hint is missing:\n{output}");
    assert!(output.contains("Protocol Type for Transfer"), "{output}");
    assert_returned_to_main_command(&output);
}

/// An empty first line abandons the file, and the original then asks for a name
/// again rather than dropping out of the command.
#[test]
fn test_upload_empty_description_asks_for_another_name() {
    let output = test_remote_output("U\nTESTUP.ZIP\n\n\n".to_string(), setup_upload_directory);
    assert!(!output.contains("Protocol"), "the upload should have been abandoned:\n{output}");
    assert_eq!(
        output.matches("Filename to Upload").count(),
        2,
        "the filename prompt should come back:\n{output}"
    );
    assert_returned_to_main_command(&output);
}

/// Fewer than five characters is not a description, so the original asks again.
#[test]
fn test_upload_short_description_asks_again() {
    let output = test_remote_output("U\nTESTUP.ZIP\nabc\na proper description\n\nN\n".to_string(), setup_upload_directory);
    assert!(output.contains("longer description"), "{output}");
    assert!(output.contains("Protocol Type for Transfer"), "{output}");
    assert_returned_to_main_command(&output);
}

/// The board says where the upload will land before it starts.
#[test]
fn test_upload_says_it_is_posted_immediately() {
    let output = test_remote_output("U\nTESTUP.ZIP\na proper description\n\nN\n".to_string(), setup_upload_directory);
    assert!(output.contains("Posted Immediately"), "{output}");
    assert!(output.contains("Protocol Type for Transfer"), "{output}");
    assert_returned_to_main_command(&output);
}

/// A leading slash asks for the upload to be screened instead.
#[test]
fn test_upload_slash_marks_it_for_screening() {
    let output = test_remote_output("U\nTESTUP.ZIP\n/a private upload\n\nN\n".to_string(), setup_upload_directory);
    assert!(output.contains("Screened Before Posting"), "{output}");
    assert!(output.contains("Protocol Type for Transfer"), "{output}");
    assert_returned_to_main_command(&output);
}

#[test]
fn test_upload_rejects_an_existing_filename_without_regard_to_case() {
    let upload = test_dir();
    std::fs::write(upload.join("existing.zip"), b"old").unwrap();

    let output = test_remote_output("U\nEXISTING.ZIP\n\n".to_string(), |board| {
        setup_upload_directory(board);
        board.conferences[0].pub_upload_location = upload.clone();
    });

    assert!(output.contains("already exists on the system"), "{output}");
    assert!(
        !output.contains("Before beginning, enter a description"),
        "a duplicate reached the description prompt: {output}"
    );
    assert!(!output.contains("Protocol Type for Transfer"), "{output}");
    assert_returned_to_main_command(&output);
}

#[test]
fn test_batch_upload_collects_numbered_names_before_protocol() {
    let output = test_remote_output(
        "BU\nONE.ZIP\nfirst description\n\nTWO.ZIP\nsecond description\n\n\nN\n".to_string(),
        setup_upload_directory,
    );
    let mut remaining = output.as_str();
    for prompt in [
        "(1) Enter the Filename to Upload",
        "ONE.ZIP",
        "(2) Enter the Filename to Upload",
        "TWO.ZIP",
        "(3) Enter the Filename to Upload",
        "Protocol Type for Transfer",
    ] {
        remaining = remaining
            .split_once(prompt)
            .unwrap_or_else(|| panic!("missing or out-of-order {prompt}:\n{output}"))
            .1;
    }
    assert_eq!(output.matches("Posted Immediately").count(), 2, "{output}");
    assert_returned_to_main_command(&output);
}

#[test]
fn test_batch_upload_below_batch_security_falls_back_to_normal_upload() {
    let output = test_remote_output("BU\nONE.ZIP\nfirst description\n\nN\n".to_string(), |board| {
        setup_upload_directory(board);
        board.users[0].security_level = 10;
        board.config.user_command_level.cmd_u = SecurityExpression::from_req_security(0);
        board.config.user_command_level.batch_file_transfer = SecurityExpression::from_req_security(20);
    });
    assert_eq!(output.matches("Filename to Upload").count(), 1, "{output}");
    assert!(
        !output.contains("(1) Enter the Filename to Upload"),
        "BU did not fall back to normal U:\n{output}"
    );
    assert!(output.contains("Posted Immediately"), "{output}");
    assert!(output.contains("Protocol Type for Transfer"), "{output}");
    assert!(!output.contains("Menu Selection is not available"), "BU should still allow normal U:\n{output}");
    assert_returned_to_main_command(&output);
}
