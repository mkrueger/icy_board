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
fn without_overrides_the_stored_runtime_is_the_decompiler_default() {
    let executable = icy_board_ppl::executable::Executable::read_file(&fixture(), false).unwrap();
    let output = ppld().arg("-o").arg(fixture()).output().unwrap();
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(text.contains(&format!(";$LANGVERSION {}", executable.runtime)), "{text}");
}

#[test]
fn version_301_fixtures_keep_their_language_directive() {
    for name in ["test_pplc_301", "test_agsppc_301"] {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../icy_board_engine/tests/test_ppe/{name}.ppe"));
        for explicit in [false, true] {
            let mut command = ppld();
            if explicit {
                command.args(["--lang-version", "301"]);
            }
            let output = command.arg("-o").arg(&fixture).output().unwrap();
            let text = String::from_utf8_lossy(&output.stdout);
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
            assert!(text.contains(";$LANGVERSION 301"), "{name}: {text}");
        }
    }
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
