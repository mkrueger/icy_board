use std::{fs, process::Command};

fn pplc() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pplc"))
}

fn check(source: &str) -> (i32, String) {
    let dir = std::env::temp_dir().join(format!("pplc_check_{}_{:?}", std::process::id(), std::thread::current().id()));
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("check.pps");
    fs::write(&file, source).unwrap();

    let output = pplc().arg("--check").arg("--lang-version").arg("400").arg(&file).output().unwrap();
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

#[test]
fn invalid_versions_fail() {
    for option in ["--runtime", "--lang-version"] {
        let output = pplc().args([option, "999", "ignored.pps"]).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
    }
}

#[test]
fn a_source_without_an_extension_is_found() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("hello.pps");
    fs::write(&source, "PRINTLN \"hello\"\n").unwrap();

    let status = pplc().arg(source.with_extension("")).status().unwrap();

    assert!(status.success());
    assert!(source.with_extension("ppe").is_file());
}

#[test]
fn init_refuses_an_existing_directory() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("keep"), b"untouched").unwrap();

    let status = pplc().args(["--init", dir.path().to_str().unwrap()]).status().unwrap();

    assert!(!status.success());
    assert_eq!(fs::read(dir.path().join("keep")).unwrap(), b"untouched");
}
