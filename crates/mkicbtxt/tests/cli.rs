use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_file(name: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("mkicbtxt-{name}-{}-{nonce}", std::process::id()))
}

fn mkicbtxt() -> Command {
    Command::new(env!("CARGO_BIN_EXE_mkicbtxt"))
}

#[test]
fn create_refuses_to_replace_a_file_without_force() {
    let path = temp_file("create");
    fs::write(&path, b"keep me").unwrap();

    let status = mkicbtxt().args(["--create", path.to_str().unwrap()]).status().unwrap();

    assert!(!status.success());
    assert_eq!(fs::read(&path).unwrap(), b"keep me");
    fs::remove_file(path).unwrap();
}

#[test]
fn forced_create_keeps_one_backup() {
    let path = temp_file("force");
    fs::write(&path, b"old text").unwrap();

    let status = mkicbtxt().args(["--create", "--force", path.to_str().unwrap()]).status().unwrap();

    assert!(status.success());
    let mut backup_name = path.file_name().unwrap().to_os_string();
    backup_name.push(".bak");
    let backup = path.with_file_name(backup_name);
    assert_eq!(fs::read(&backup).unwrap(), b"old text");
    assert_ne!(fs::read(&path).unwrap(), b"old text");
    fs::remove_file(path).unwrap();
    fs::remove_file(backup).unwrap();
}
