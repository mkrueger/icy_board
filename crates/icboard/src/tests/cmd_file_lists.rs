use std::path::{Path, PathBuf};

use dizbase::file_base::FileBase;
use icy_board_engine::icy_board::{
    IcyBoard,
    conferences::Conference,
    file_directory::{DirectoryList, FileDirectory},
};
use icy_engine::{TextPane, TextScreen};
use icy_parser_core::{AnsiParser, CommandParser};

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
        directories: Some(std::sync::Arc::new(directories)),
        ..Default::default()
    });
}

pub fn rendered_lines(output: &str) -> Vec<String> {
    let mut screen = TextScreen::new((80, 25));
    let mut parser = AnsiParser::default();
    parser.parse(output.as_bytes(), &mut icy_engine::ScreenSink::new(&mut screen));
    (0..25).map(|y| (0..80).map(|x| screen.char_at((x, y).into()).ch).collect::<String>()).collect()
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

/// The date field is prefilled, so typing over only part of it submits fewer than the
/// six digits the scan expects. That used to index past the end of the answer and took
/// the whole session down with it.
#[test]
fn test_new_scan_asks_again_for_a_partially_typed_date() {
    let (dir, metadata_path) = file_area();
    let output = test_output("N\n0801\n010180\nA\n\n".to_string(), |board| setup_area(board, &dir, &metadata_path));
    assert!(output.contains("ALPHA.TXT"), "{output}");
    assert!(output.contains("BETA.ZIP"), "{output}");
}

/// Only what was typed is submitted, so the rest of the prefilled default must not stay
/// on screen promising digits that are not part of the answer.
#[test]
fn test_a_prefilled_field_stops_showing_the_default_once_typing_starts() {
    let (dir, metadata_path) = file_area();
    let output = test_output("N\n1202\n\n\n".to_string(), |board| setup_area(board, &dir, &metadata_path));
    let lines = rendered_lines(&output);
    let prompt = lines
        .iter()
        .find(|line| line.contains("1202"))
        .unwrap_or_else(|| panic!("the typed date was never shown:\n{lines:#?}"));
    assert!(prompt.contains("(1202  )"), "the default was left behind: {prompt:?}");
}

/// The date can also arrive as a command line token, which no prompt ever validated.
#[test]
fn test_new_scan_survives_a_short_date_token() {
    let (dir, metadata_path) = file_area();
    let output = test_output("N 1\n010180\nA\n\n".to_string(), |board| setup_area(board, &dir, &metadata_path));
    assert!(output.contains("ALPHA.TXT"), "{output}");
}

/// `N` inside the file number prompt reads the date from the next token, which is not
/// required to be a date at all. A token that is not one stays a directory number.
#[test]
fn test_directory_numbers_survive_a_short_date_token() {
    let (dir, metadata_path) = file_area();
    let output = test_output("L ALPHA*\nN 1\n\n".to_string(), |board| setup_area(board, &dir, &metadata_path));
    assert!(output.contains("ALPHA.TXT"), "{output}");
}

/// A date given to `N` applies to the scan, and the directory numbers after it are
/// still directory numbers.
#[test]
fn test_directory_numbers_are_read_after_a_scan_date() {
    let (dir, metadata_path) = file_area();
    let output = test_output("L ALPHA*\nN 010180 1\n\n".to_string(), |board| setup_area(board, &dir, &metadata_path));
    assert!(output.contains("ALPHA.TXT"), "{output}");
}

/// A two digit year belongs to PCBoard's 1979..2078 window. Read as a bare number it
/// lands in antiquity instead, which makes every file look new.
#[test]
fn test_new_scan_reads_a_two_digit_year_as_the_far_end_of_the_window() {
    let (dir, metadata_path) = file_area();
    let output = test_output("N\n010178\nA\n\n".to_string(), |board| setup_area(board, &dir, &metadata_path));
    assert!(!output.contains("ALPHA.TXT"), "a file older than 2078 was listed as new:\n{output}");
    assert!(!output.contains("BETA.ZIP"), "a file older than 2078 was listed as new:\n{output}");
}

/// The near end of the same window still lists what came after it.
#[test]
fn test_new_scan_lists_files_newer_than_the_scan_date() {
    let (dir, metadata_path) = file_area();
    let output = test_output("N\n010179\nA\n\n".to_string(), |board| setup_area(board, &dir, &metadata_path));
    assert!(output.contains("ALPHA.TXT"), "{output}");
    assert!(output.contains("BETA.ZIP"), "{output}");
}

#[test]
fn a_long_filename_keeps_size_date_and_description_in_their_columns() {
    let root = test_dir();
    let dir = root.join("files");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("LONG-FILENAME.ZIP");
    std::fs::write(&file, b"data").unwrap();
    let metadata_path = root.join("dir1");
    let mut base = FileBase::open(&dir, &metadata_path).unwrap();
    base.set_description(&file, "A long name").unwrap();

    let output = test_output("F 1\n\n".to_string(), |board| setup_area(board, &dir, &metadata_path));
    let lines = rendered_lines(&output);
    let name_line = lines
        .iter()
        .position(|line| line.contains("LONG-FILENAME.ZIP"))
        .expect("filename was not listed");
    let fields: Vec<char> = lines[name_line + 1].chars().collect();

    assert!(
        fields[..13].iter().all(|ch| *ch == ' '),
        "size did not start in column 13: {:?}",
        lines[name_line + 1]
    );
    assert_eq!(fields[13..21].iter().collect::<String>().trim(), "4 B");
    assert_eq!(fields[21..23].iter().collect::<String>(), "  ");
    assert!(
        fields[23..31].iter().collect::<String>().contains('/'),
        "date did not start in column 23: {:?}",
        lines[name_line + 1]
    );
    assert_eq!(fields[31..33].iter().collect::<String>(), "  ");
    assert_eq!(fields[33..44].iter().collect::<String>(), "A long name");
}
