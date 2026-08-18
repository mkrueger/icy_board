use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use icy_board_engine::icy_board::IcyBoard;

fn temp_dir(name: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("icbsetup-{name}-{}-{nonce}", std::process::id()))
}

fn icbsetup() -> Command {
    Command::new(env!("CARGO_BIN_EXE_icbsetup"))
}

#[test]
fn missing_board_configuration_explains_how_to_start() {
    let path = temp_dir("missing-board");
    let output = icbsetup().arg(&path).env("LANG", "en_US.UTF-8").env_remove("ICB_PATH").output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("IcyBoard configuration not found:"));
    assert!(stderr.contains("Usage: "));
    assert!(stderr.contains("icbsetup create mybbs"));
    assert!(stderr.contains("docs/gettingstarted.md"));
}

#[test]
fn check_with_missing_board_configuration_explains_how_to_start() {
    let path = temp_dir("missing-check-board");
    let output = icbsetup()
        .args(["check", path.to_str().unwrap()])
        .env("LANG", "en_US.UTF-8")
        .env_remove("ICB_PATH")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("IcyBoard configuration not found:"));
    assert!(stderr.contains("icbsetup check"));
    assert!(stderr.contains("icbsetup create mybbs"));
    assert!(stderr.contains("docs/gettingstarted.md"));
}

#[test]
fn missing_import_source_is_a_failure() {
    let source = temp_dir("missing-source");
    let output = temp_dir("missing-output");
    let status = icbsetup()
        .args(["import", source.to_str().unwrap(), output.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(!status.success());
    assert!(!output.exists());
}

#[test]
fn import_refuses_an_existing_destination() {
    let source = temp_dir("source");
    let output = temp_dir("existing-output");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("keep"), b"untouched").unwrap();

    let status = icbsetup()
        .args(["import", source.to_str().unwrap(), output.to_str().unwrap()])
        .status()
        .unwrap();

    assert!(!status.success());
    assert_eq!(fs::read(output.join("keep")).unwrap(), b"untouched");
    fs::remove_dir_all(output).unwrap();
}

#[test]
fn ppe_convert_keeps_the_root_and_lowercases_only_descendants() {
    let parent = temp_dir("MixedParent");
    let root = parent.join("SubDir");
    fs::create_dir_all(root.join("NestedDir")).unwrap();
    fs::write(root.join("FILE.TXT"), b"HELLO\r\n").unwrap();
    fs::write(root.join("NestedDir/README.DOC"), b"DOC\r\n").unwrap();

    let status = icbsetup().args(["ppe-convert", root.to_str().unwrap()]).status().unwrap();

    assert!(status.success());
    assert!(root.is_dir());
    assert!(root.join("file.txt").is_file());
    assert!(root.join("nesteddir/readme.doc").is_file());
    assert!(fs::read(root.join("file.txt")).unwrap().starts_with(&[0xEF, 0xBB, 0xBF]));
    fs::remove_dir_all(parent).unwrap();
}

#[test]
fn create_gives_sysop_a_password() {
    let output = temp_dir("create");

    let result = icbsetup().args(["create", output.to_str().unwrap()]).output().unwrap();

    assert!(result.status.success());
    let board = IcyBoard::load(&output.join("icboard.toml")).unwrap();
    assert!(!board.users[0].password.password.is_empty());
    assert!(String::from_utf8_lossy(&result.stdout).contains("Initial SYSOP password:"));
    fs::remove_dir_all(output).unwrap();
}
