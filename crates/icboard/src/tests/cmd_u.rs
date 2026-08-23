use icy_board_engine::icy_board::{IcyBoard, conferences::Conference};

use crate::tests::{test_dir, test_output};

fn setup_upload_directory(board: &mut IcyBoard) {
    board.conferences.push(Conference {
        name: "Main Board".to_string(),
        pub_upload_location: test_dir(),
        private_upload_location: test_dir(),
        ..Default::default()
    });
}

/// PCBoard asks for the description before anything is transferred.
#[test]
fn test_upload_asks_for_a_description() {
    let output = test_output("U\nTESTUP.ZIP\na proper description\n\n\n".to_string(), setup_upload_directory);
    assert!(output.contains("TESTUP.ZIP"), "{output}");
    assert!(output.contains("Private"), "the private hint is missing:\n{output}");
}

/// An empty first line abandons the file, and the original then asks for a name
/// again rather than dropping out of the command.
#[test]
fn test_upload_empty_description_asks_for_another_name() {
    let output = test_output("U\nTESTUP.ZIP\n\n\n".to_string(), setup_upload_directory);
    assert!(!output.contains("Protocol"), "the upload should have been abandoned:\n{output}");
    assert!(
        output.matches("Filename to Upload").count() >= 2,
        "the filename prompt should come back:\n{output}"
    );
}

/// Fewer than five characters is not a description, so the original asks again.
#[test]
fn test_upload_short_description_asks_again() {
    let output = test_output("U\nTESTUP.ZIP\nabc\na proper description\n\n\n".to_string(), setup_upload_directory);
    assert!(output.contains("longer description"), "{output}");
}

/// The board says where the upload will land before it starts.
#[test]
fn test_upload_says_it_is_posted_immediately() {
    let output = test_output("U\nTESTUP.ZIP\na proper description\n\n\n".to_string(), setup_upload_directory);
    assert!(output.contains("Posted Immediately"), "{output}");
}

/// A leading slash asks for the upload to be screened instead.
#[test]
fn test_upload_slash_marks_it_for_screening() {
    let output = test_output("U\nTESTUP.ZIP\n/a private upload\n\n\n".to_string(), setup_upload_directory);
    assert!(output.contains("Screened Before Posting"), "{output}");
}

#[test]
fn test_upload_rejects_an_existing_filename_without_regard_to_case() {
    let upload = test_dir();
    std::fs::write(upload.join("existing.zip"), b"old").unwrap();
    let private = test_dir();

    let output = test_output("U\nEXISTING.ZIP\n\n".to_string(), |board| {
        board.conferences.push(Conference {
            name: "Main Board".to_string(),
            pub_upload_location: upload.clone(),
            private_upload_location: private.clone(),
            ..Default::default()
        });
    });

    assert!(output.contains("already exists on the system"), "{output}");
    assert!(
        !output.contains("Before beginning, enter a description"),
        "a duplicate reached the description prompt: {output}"
    );
}
