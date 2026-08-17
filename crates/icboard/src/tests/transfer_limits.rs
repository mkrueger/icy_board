use dizbase::file_base::FileBase;
use icy_board_engine::icy_board::{
    IcyBoard,
    conferences::Conference,
    file_directory::{DirectoryList, FileDirectory},
    sec_levels::SecurityLevel,
};

use crate::tests::{test_dir, test_output};

/// A scratch area holding one 4 KB file, and a security level the caller matches.
fn setup(board: &mut IcyBoard, level: SecurityLevel, free_area: bool) {
    let root = test_dir();
    let dir = root.join("files");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("BIG.ZIP"), vec![0u8; 4096]).unwrap();

    let metadata_path = root.join("dir1");
    FileBase::open(&dir, &metadata_path).unwrap();

    let mut directories = DirectoryList::default();
    directories.push(FileDirectory {
        name: "Test Files".to_string(),
        path: dir,
        metadata_path,
        is_free: free_area,
        ..Default::default()
    });
    board.conferences.push(Conference {
        name: "Main Board".to_string(),
        directories: Some(directories),
        ..Default::default()
    });

    board.config.system_control.enforce_transfer_limits = true;
    // The test caller logs in as the sysop record, whose level is 255.
    board.sec_levels.push(SecurityLevel { security: 255, ..level });
}

/// One kilobyte a day does not cover a four kilobyte file.
#[test]
fn test_daily_allowance_refuses_a_file_that_does_not_fit() {
    let level = SecurityLevel {
        daily_file_kb_limit: 1,
        ..Default::default()
    };
    let output = test_output("D BIG.ZIP\n\n".to_string(), move |board| setup(board, level.clone(), false));
    assert!(output.contains("download bytes left"), "{output}");
}

/// The same file goes out once the allowance covers it.
#[test]
fn test_daily_allowance_lets_a_file_through() {
    let level = SecurityLevel {
        daily_file_kb_limit: 100,
        ..Default::default()
    };
    let output = test_output("D BIG.ZIP\n\n".to_string(), move |board| setup(board, level.clone(), false));
    assert!(!output.contains("download bytes left"), "{output}");
}

/// The unlimited marker lifts the allowance entirely.
#[test]
fn test_the_unlimited_marker_lifts_the_daily_allowance() {
    let level = SecurityLevel {
        daily_file_kb_limit: 32767,
        ..Default::default()
    };
    let output = test_output("D BIG.ZIP\n\n".to_string(), move |board| setup(board, level.clone(), false));
    assert!(!output.contains("download bytes left"), "{output}");
}

/// A free area is exempt, which is what PCBoard's FSEC free password did.
#[test]
fn test_a_free_area_ignores_the_allowance() {
    let level = SecurityLevel {
        daily_file_kb_limit: 1,
        ..Default::default()
    };
    let output = test_output("D BIG.ZIP\n\n".to_string(), move |board| setup(board, level.clone(), true));
    assert!(!output.contains("download bytes left"), "{output}");
}

/// Nothing is enforced until the sysop asks for it.
#[test]
fn test_limits_are_not_enforced_by_default() {
    let level = SecurityLevel {
        daily_file_kb_limit: 1,
        ..Default::default()
    };
    let output = test_output("D BIG.ZIP\n\n".to_string(), move |board| {
        setup(board, level.clone(), false);
        board.config.system_control.enforce_transfer_limits = false;
    });
    assert!(!output.contains("download bytes left"), "{output}");
}

/// A ratio of 5.0:1 stops the caller who has downloaded five for every upload.
#[test]
fn test_the_file_ratio_refuses_a_download() {
    let level = SecurityLevel {
        daily_file_kb_limit: 32767,
        uldl_ratio_tenths: 50,
        ..Default::default()
    };
    let output = test_output("D BIG.ZIP\n\n".to_string(), move |board| {
        setup(board, level.clone(), false);
        board.users[0].stats.num_uploads = 1;
        board.users[0].stats.num_downloads = 5;
    });
    assert!(output.contains("would exceed your file ratio"), "{output}");
    assert!(output.contains("5.0:1"), "the ratio is not shown:\n{output}");
}

/// The total file limit is separate from the ratio and reports its own message.
#[test]
fn test_the_total_file_limit_refuses_a_download() {
    let level = SecurityLevel {
        daily_file_kb_limit: 32767,
        file_limit: 10,
        ..Default::default()
    };
    let output = test_output("D BIG.ZIP\n\n".to_string(), move |board| {
        setup(board, level.clone(), false);
        board.users[0].stats.num_downloads = 10;
    });
    assert!(output.contains("would exceed your file limit"), "{output}");
}
