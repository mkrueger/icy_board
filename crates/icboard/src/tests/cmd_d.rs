use dizbase::file_base::FileBase;
use icy_board_engine::icy_board::{
    IcyBoard,
    conferences::Conference,
    file_directory::{DirectoryList, FileDirectory},
};

use crate::tests::{test_dir, test_output};

/// No directory to search, so every name comes back as missing - which is all
/// these tests are about.
fn setup_empty_file_base(board: &mut IcyBoard) {
    board.conferences.push(Conference {
        name: "Main Board".to_string(),
        directories: Some(DirectoryList::default()),
        ..Default::default()
    });
}

/// The original asks for another name after each one, until the caller answers
/// nothing - a PPE stuffing names with KBDSTUFF depends on that.
#[test]
fn test_download_asks_again_after_a_name() {
    let output = test_output("D\nNOSUCHFILE\n\n".to_string(), setup_empty_file_base);
    assert_eq!(output.matches("Enter the filename to Download").count(), 2, "{output}");
    assert!(output.contains("(NOSUCHFILE) not found on disk!"), "{output}");
}

/// A stacked token answers the first prompt, and the loop still runs.
#[test]
fn test_download_asks_again_after_a_token() {
    let output = test_output("D NOSUCHFILE\n\n".to_string(), setup_empty_file_base);
    assert_eq!(output.matches("Enter the filename to Download").count(), 1, "{output}");
    assert!(output.contains("(NOSUCHFILE) not found on disk!"), "{output}");
}

#[test]
fn a_filename_uppercased_by_the_prompt_flags_a_lowercase_file_on_disk() {
    let root = test_dir();
    let files = root.join("files");
    std::fs::create_dir_all(&files).unwrap();
    std::fs::write(files.join("gw-userstats.zip"), b"data").unwrap();
    let metadata = root.join("dir1");
    FileBase::open(&files, &metadata).unwrap();

    let output = test_output("F 1\nF\nGW-USERSTATS.ZIP\n\n".to_string(), |board| {
        let mut directories = DirectoryList::default();
        directories.push(FileDirectory {
            name: "Test Files".to_string(),
            path: files.clone(),
            metadata_path: metadata.clone(),
            ..Default::default()
        });
        board.conferences.push(Conference {
            name: "Main Board".to_string(),
            directories: Some(directories),
            ..Default::default()
        });
    });

    assert!(!output.contains("not found on disk"), "{output}");
    assert!(output.contains("gw-userstats.zip"), "the on-disk filename was not flagged: {output}");
}
