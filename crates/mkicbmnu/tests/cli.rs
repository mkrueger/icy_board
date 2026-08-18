use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_dir(name: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("mkicbmnu-{name}-{}-{nonce}", std::process::id()))
}

fn mkicbmnu() -> Command {
    Command::new(env!("CARGO_BIN_EXE_mkicbmnu"))
}

#[test]
fn missing_menu_explains_how_to_create_it() {
    let directory = temp_dir("missing-menu");
    fs::create_dir(&directory).unwrap();
    let menu = directory.join("main.mnu");

    let output = mkicbmnu().arg(&menu).env("LANG", "en_US.UTF-8").output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Input file not found:"));
    assert!(stderr.contains("--create"));
    assert!(stderr.contains("--help"));

    fs::remove_dir(directory).unwrap();
}

#[test]
fn missing_parent_board_explains_where_it_is_expected() {
    let directory = temp_dir("missing-board");
    fs::create_dir(&directory).unwrap();
    let menu = directory.join("main.mnu");

    let output = mkicbmnu()
        .args(["--create", menu.to_str().unwrap()])
        .env("LANG", "en_US.UTF-8")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("No icboard.toml found for:"));
    assert!(stderr.contains("file's directory and its parents"));
    assert!(stderr.contains("icbsetup create mybbs"));
    assert!(stderr.contains("docs/gettingstarted.md"));

    fs::remove_dir(directory).unwrap();
}
