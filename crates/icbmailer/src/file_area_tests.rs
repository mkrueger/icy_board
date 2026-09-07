use super::*;
use icy_board_engine::icy_board::{conferences::Conference, file_directory::DirectoryList};

fn board(root: &Path) -> IcyBoard {
    let mut board = IcyBoard::new();
    board.root_path = root.to_path_buf();
    board.ftn.options.auto_add_files = true;
    board.ftn.options.auto_add_file_conference = 1;
    board.ftn.inbound = root.join("inbound");
    board.ftn.new_file_areas = root.join("files");
    board.conferences.clear();
    for name in ["Main", "File echoes"] {
        board.conferences.push(Conference {
            name: name.into(),
            dir_file: root.join(format!("{name}.toml")),
            ..Default::default()
        });
    }
    board
}

fn arrive(board: &IcyBoard, tag: &str, name: &str) -> PathBuf {
    fs::create_dir_all(&board.ftn.inbound).unwrap();
    fs::write(board.ftn.inbound.join(name), b"network information").unwrap();
    let tic = board.ftn.inbound.join("test.tic");
    fs::write(&tic, format!("Area {tag}\r\nFile {name}\r\nDesc Information\r\n")).unwrap();
    tic
}

#[test]
fn file_auto_add_persists_in_selected_conference_and_reuses_mapping_after_reload() {
    let root = tempfile::tempdir().unwrap();
    let mut board = board(root.path());
    let mut existing = DirectoryList::default();
    existing.push(FileDirectory {
        name: "Local files".into(),
        path: root.path().join("local"),
        ..Default::default()
    });
    existing.save(&board.conferences[1].dir_file).unwrap();
    board.conferences[1].directories = Some(std::sync::Arc::new(existing));
    let tic = arrive(&board, "AGN_INFO", "INFO.ZIP");

    toss_files(&mut board).unwrap();

    assert!(!tic.exists());
    assert!(board.conferences[0].directories.is_none());
    assert!(!board.conferences[0].dir_file.exists());
    let stored = DirectoryList::load(&board.conferences[1].dir_file).unwrap();
    assert_eq!(stored.len(), 2);
    assert_eq!(stored[0].name, "Local files");
    assert_eq!(stored[1].ftn_area_tag, "AGN_INFO");
    assert_eq!(stored[1].path, root.path().join("files/agn_info"));
    assert_eq!(stored[1].metadata_path, root.path().join("files/agn_info/dir"));
    assert!(stored[1].path.join("INFO.ZIP").exists());

    board.conferences[1].directories = Some(std::sync::Arc::new(stored));
    let second = arrive(&board, "agn_info", "INFO2.ZIP");
    toss_files(&mut board).unwrap();
    assert!(!second.exists());
    assert_eq!(DirectoryList::load(&board.conferences[1].dir_file).unwrap().len(), 2);
    assert!(root.path().join("files/agn_info/INFO2.ZIP").exists());
}

#[test]
fn file_auto_add_invalid_conference_preserves_inputs_until_fixed() {
    let root = tempfile::tempdir().unwrap();
    let mut board = board(root.path());
    board.ftn.options.auto_add_file_conference = 20;
    let tic = arrive(&board, "AGN_NODE", "NODE.ZIP");
    // Per-TIC failures are printed, just like other failed file imports.
    toss_files(&mut board).unwrap();
    assert!(tic.exists());
    assert!(board.ftn.inbound.join("NODE.ZIP").exists());
    assert!(!board.ftn.new_file_areas.exists());
    board.ftn.options.auto_add_file_conference = 1;
    toss_files(&mut board).unwrap();
    assert!(!tic.exists());
    assert_eq!(DirectoryList::load(&board.conferences[1].dir_file).unwrap().len(), 1);
}

#[test]
fn file_auto_add_failed_directory_list_write_does_not_consume_files_or_modify_memory() {
    let root = tempfile::tempdir().unwrap();
    let mut board = board(root.path());
    let blocker = root.path().join("not-a-directory");
    fs::write(&blocker, b"keep").unwrap();
    board.conferences[1].dir_file = blocker.join("dirs.toml");
    let tic = arrive(&board, "AGN_DIFF", "DIFF.ZIP");
    toss_files(&mut board).unwrap();
    assert!(tic.exists());
    assert!(board.ftn.inbound.join("DIFF.ZIP").exists());
    assert!(board.conferences[1].directories.is_none());
    assert!(!board.ftn.new_file_areas.exists());
    assert_eq!(fs::read(&blocker).unwrap(), b"keep");
}

#[test]
fn file_auto_add_refuses_missing_or_unloaded_directory_list_configuration() {
    for invalid in ["empty", "directory", "unloaded"] {
        let root = tempfile::tempdir().unwrap();
        let mut board = board(root.path());
        match invalid {
            "empty" => board.conferences[1].dir_file.clear(),
            "directory" => board.conferences[1].dir_file = root.path().to_path_buf(),
            "unloaded" => fs::write(&board.conferences[1].dir_file, b"invalid configuration").unwrap(),
            _ => unreachable!(),
        }
        let tic = arrive(&board, "AGN_INFO", "INFO.ZIP");
        toss_files(&mut board).unwrap();
        assert!(tic.exists(), "{invalid}");
        assert!(board.ftn.inbound.join("INFO.ZIP").exists());
        assert!(board.conferences[1].directories.is_none());
        assert!(!board.ftn.new_file_areas.exists());
        if invalid == "unloaded" {
            assert_eq!(fs::read(&board.conferences[1].dir_file).unwrap(), b"invalid configuration");
        }
    }
}

#[test]
fn file_auto_add_base_path_is_relative_to_board_root() {
    let root = tempfile::tempdir().unwrap();
    let mut board = board(root.path());
    board.ftn.new_file_areas = PathBuf::from("file-echoes");
    board.resolve_paths();
    assert_eq!(board.ftn.new_file_areas, root.path().join("file-echoes"));
    arrive(&board, "AGN_INFO", "INFO.ZIP");
    toss_files(&mut board).unwrap();
    assert!(root.path().join("file-echoes/agn_info/INFO.ZIP").exists());
}
