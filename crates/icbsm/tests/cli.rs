use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn starting_without_a_board_explains_how_to_create_one() {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let directory = std::env::temp_dir().join(format!("icbsm-missing-{}-{nonce}", std::process::id()));
    fs::create_dir(&directory).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_icbsm"))
        .current_dir(&directory)
        .env("LANG", "en_US.UTF-8")
        .env_remove("ICB_PATH")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("IcyBoard configuration not found:"));
    assert!(stderr.contains("./icboard.toml"));
    assert!(stderr.contains("Usage: "));
    assert!(stderr.contains("icbsetup create mybbs"));
    assert!(stderr.contains("docs/gettingstarted.md"));

    fs::remove_dir(directory).unwrap();
}
