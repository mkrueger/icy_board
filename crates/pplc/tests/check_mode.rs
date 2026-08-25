use std::{fs, process::Command};

fn pplc() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_pplc"));
    command.env_remove("PPL_LANG_VERSION");
    command
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
fn the_environment_is_the_default_for_a_loose_source() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("const.pps");
    fs::write(&source, "CONST INTEGER Answer = 42\nPRINTLN Answer\n").unwrap();

    let old = pplc().env("PPL_LANG_VERSION", "340").arg(&source).output().unwrap();
    assert_eq!(old.status.code(), Some(1), "{}", String::from_utf8_lossy(&old.stderr));

    let modern = pplc().env("PPL_LANG_VERSION", "350").arg(&source).output().unwrap();
    assert!(modern.status.success(), "{}", String::from_utf8_lossy(&modern.stderr));
}

#[test]
fn command_line_and_source_win_over_the_environment() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("const.pps");
    fs::write(&source, "CONST INTEGER Answer = 42\nPRINTLN Answer\n").unwrap();

    let cli = pplc()
        .env("PPL_LANG_VERSION", "340")
        .args(["--lang-version", "350"])
        .arg(&source)
        .output()
        .unwrap();
    assert!(cli.status.success(), "{}", String::from_utf8_lossy(&cli.stderr));

    fs::write(&source, ";$LANGVERSION 350\nCONST INTEGER Answer = 42\nPRINTLN Answer\n").unwrap();
    let declared = pplc().env("PPL_LANG_VERSION", "340").arg(&source).output().unwrap();
    assert!(declared.status.success(), "{}", String::from_utf8_lossy(&declared.stderr));
}

#[test]
fn a_manifest_wins_over_the_environment() {
    let dir = tempfile::tempdir().unwrap();
    let package = dir.path().join("package");
    let init = pplc().args(["--init", "--lang-version", "350"]).arg(&package).output().unwrap();
    assert!(init.status.success(), "{}", String::from_utf8_lossy(&init.stderr));
    fs::write(package.join("src/main.pps"), "CONST INTEGER Answer = 42\nPRINTLN Answer\n").unwrap();

    let output = pplc().env("PPL_LANG_VERSION", "340").current_dir(&package).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn explicit_versions_ignore_an_invalid_environment() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.pps");
    fs::write(&source, "PRINTLN 1\n").unwrap();

    let output = pplc()
        .env("PPL_LANG_VERSION", "latest")
        .args(["--lang-version", "350"])
        .arg(&source)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    fs::write(&source, ";$LANGVERSION 350\nPRINTLN 1\n").unwrap();
    let declared = pplc().env("PPL_LANG_VERSION", "latest").arg(&source).output().unwrap();
    assert!(declared.status.success(), "{}", String::from_utf8_lossy(&declared.stderr));
}

#[test]
fn an_invalid_environment_language_version_fails() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.pps");
    fs::write(&source, "PRINTLN 1\n").unwrap();

    let output = pplc().env("PPL_LANG_VERSION", "latest").arg(&source).output().unwrap();
    let text = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{text}");
    assert!(text.contains("Invalid PPL_LANG_VERSION 'latest'"), "{text}");
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

#[test]
fn print_config_explains_a_loose_source() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.pps");
    fs::write(&source, ";$LANGVERSION 350\nPRINTLN 1\n").unwrap();

    let output = pplc()
        .env("PPL_LANG_VERSION", "340")
        .args(["--print-config", "--lang-version", "400"])
        .arg(&source)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(text.contains("Language version       350"), "{text}");
    assert!(text.contains("From                 sourceDirective"), "{text}");
    assert!(text.contains("Command line         400"), "{text}");
    assert!(text.contains("Environment          340"), "{text}");
    assert!(!source.with_extension("ppe").exists(), "--print-config built the source");
}

#[test]
fn print_config_json_is_machine_readable() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.pps");
    fs::write(&source, "PRINTLN 1\n").unwrap();

    let output = pplc().env("PPL_LANG_VERSION", "350").arg("--print-config-json").arg(&source).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let config: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(config["languageVersion"]["effective"], 350);
    assert_eq!(config["languageVersion"]["source"], "environment");
    assert_eq!(config["runtimeVersion"]["effective"], 400);
    assert_eq!(config["sources"].as_array().unwrap().len(), 1);
}

#[test]
fn print_config_reads_a_package_without_building_it() {
    let dir = tempfile::tempdir().unwrap();
    let package = dir.path().join("package");
    let init = pplc().args(["--init", "--lang-version", "350"]).arg(&package).output().unwrap();
    assert!(init.status.success(), "{}", String::from_utf8_lossy(&init.stderr));

    let output = pplc().current_dir(&package).arg("--print-config-json").output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let config: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(config["package"]["name"], "package");
    assert_eq!(config["languageVersion"]["source"], "manifest");
    assert!(config["output"].as_str().unwrap().ends_with("target/icboard/package.ppe"));
    assert!(!package.join("target").exists(), "printing config created the target directory");
}

#[test]
fn print_config_formats_are_mutually_exclusive() {
    let output = pplc().args(["--print-config", "--print-config-json", "ignored.pps"]).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
}
