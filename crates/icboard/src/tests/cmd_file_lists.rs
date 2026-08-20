use std::path::{Path, PathBuf};

use dizbase::file_base::FileBase;
use icy_board_engine::icy_board::{
    IcyBoard,
    conferences::Conference,
    file_directory::{DirectoryList, FileDirectory},
};

use crate::tests::{test_dir, test_output};

/// A scratch area holding two files. `BETA.ZIP` carries a description that its name
/// gives no hint of, which is what tells a name search apart from a text search.
fn file_area() -> (PathBuf, PathBuf) {
    let root = test_dir();
    let dir = root.join("files");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("ALPHA.TXT"), b"alpha").unwrap();
    std::fs::write(dir.join("BETA.ZIP"), b"beta").unwrap();

    let metadata_path = root.join("dir1");
    let mut base = FileBase::open(&dir, &metadata_path).unwrap();
    base.set_description(&dir.join("BETA.ZIP"), "a wombat utility").unwrap();

    (dir, metadata_path)
}

fn setup_area(board: &mut IcyBoard, dir: &Path, metadata_path: &Path) {
    let mut directories = DirectoryList::default();
    directories.push(FileDirectory {
        name: "Test Files".to_string(),
        path: dir.to_path_buf(),
        metadata_path: metadata_path.to_path_buf(),
        ..Default::default()
    });
    board.conferences.push(Conference {
        name: "Main Board".to_string(),
        directories: Some(directories),
        ..Default::default()
    });
}

/// `L` matches the name, so only the file that was asked for comes back.
#[test]
fn test_locate_lists_only_matching_names() {
    let (dir, metadata_path) = file_area();
    let output = test_output("L ALPHA* A\n\n".to_string(), |board| setup_area(board, &dir, &metadata_path));
    assert!(output.contains("ALPHA.TXT"), "{output}");
    assert!(!output.contains("BETA.ZIP"), "{output}");
}

/// `L` never looks at descriptions - that is what `Z` is for. A file whose text
/// matches but whose name does not stays out of the listing.
#[test]
fn test_locate_ignores_descriptions() {
    let (dir, metadata_path) = file_area();
    let output = test_output("L WOMBAT* A\n\n".to_string(), |board| setup_area(board, &dir, &metadata_path));
    assert!(!output.contains("BETA.ZIP"), "{output}");
    assert!(!output.contains("ALPHA.TXT"), "{output}");
}

/// `Z` is the one command that reads descriptions, and it finds a file by its text
/// alone.
#[test]
fn test_zippy_scan_matches_description() {
    let (dir, metadata_path) = file_area();
    let output = test_output("Z WOMBAT A\n\n".to_string(), |board| setup_area(board, &dir, &metadata_path));
    assert!(output.contains("BETA.ZIP"), "{output}");
    assert!(!output.contains("ALPHA.TXT"), "{output}");
}

/// `Z` still matches a name, so a caller can use it as a wider `L`. The matched part
/// is highlighted, which breaks the name up, so only the stem is checked here.
#[test]
fn test_zippy_scan_matches_name() {
    let (dir, metadata_path) = file_area();
    let output = test_output("Z ALPHA A\n\n".to_string(), |board| setup_area(board, &dir, &metadata_path));
    assert!(output.contains("ALPHA"), "{output}");
    assert!(!output.contains("BETA"), "{output}");
}

/// Listing a directory shows everything in it.
#[test]
fn test_file_directory_lists_every_file() {
    let (dir, metadata_path) = file_area();
    let output = test_output("F 1\n\n".to_string(), |board| setup_area(board, &dir, &metadata_path));
    assert!(output.contains("ALPHA.TXT"), "{output}");
    assert!(output.contains("BETA.ZIP"), "{output}");
}
