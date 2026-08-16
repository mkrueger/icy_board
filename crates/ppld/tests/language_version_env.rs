use std::{path::Path, process::Command};

fn ppld() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ppld"));
    command.env_remove("PPL_LANG_VERSION");
    command
}

fn fixture() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/beep.ppe").leak()
}

#[test]
fn the_environment_is_the_decompiler_default() {
    let output = ppld().env("PPL_LANG_VERSION", "350").arg("-o").arg(fixture()).output().unwrap();
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(text.contains(";$LANGVERSION 350"), "{text}");
}

#[test]
fn the_command_line_wins_over_the_environment() {
    let output = ppld()
        .env("PPL_LANG_VERSION", "latest")
        .args(["--lang-version", "400", "-o"])
        .arg(fixture())
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(text.contains(";$LANGVERSION 400"), "{text}");
}

#[test]
fn an_invalid_environment_language_version_fails() {
    let output = ppld().env("PPL_LANG_VERSION", "latest").arg("-o").arg(fixture()).output().unwrap();
    let text = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{text}");
    assert!(text.contains("Invalid PPL_LANG_VERSION 'latest'"), "{text}");
}
