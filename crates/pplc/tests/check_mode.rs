use std::{fs, process::Command};

fn check(source: &str) -> (i32, String) {
    let dir = std::env::temp_dir().join(format!("pplc_check_{}_{:?}", std::process::id(), std::thread::current().id()));
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("check.pps");
    fs::write(&file, source).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_pplc"))
        .arg("--check")
        .arg("--lang-version")
        .arg("400")
        .arg(&file)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&output.stdout).to_string() + &String::from_utf8_lossy(&output.stderr);
    let ppe_written = file.with_extension("ppe").exists();
    fs::remove_dir_all(&dir).unwrap();
    assert!(!ppe_written, "--check wrote an executable");
    (output.status.code().unwrap_or(-1), text)
}

#[test]
fn a_clean_source_passes() {
    let (code, _) = check("PRINTLN \"a\"\n");
    assert_eq!(code, 0);
}

#[test]
fn a_syntax_error_fails() {
    let (code, text) = check("PRINTLN (\n");
    assert_eq!(code, 1, "{text}");
    assert!(text.contains("Expected expression"), "{text}");
}

#[test]
fn a_missing_routine_fails() {
    let (code, text) = check("NoSuchProc(1)\n");
    assert_eq!(code, 1, "{text}");
    assert!(text.contains("Missing FUNCTION/PROCEDURE definition"), "{text}");
}

#[test]
fn an_unformatted_source_fails() {
    let (code, text) = check("PRINTLN   \"a\"\n");
    assert_eq!(code, 1, "{text}");
    assert!(text.contains("Diff in"), "{text}");
}
