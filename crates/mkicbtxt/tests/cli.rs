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
    let mut command = Command::new(env!("CARGO_BIN_EXE_mkicbtxt"));
    command.env("LANG", "en_US.UTF-8").env("LC_ALL", "en_US.UTF-8").env("LANGUAGE", "en");
    command
}

#[test]
fn cli_help_errors_and_early_version_are_localized() {
    for (locale, help, error) in [("en", "Use the full screen", "error"), ("de", "Vollbild verwenden", "Fehler")] {
        let run = |args: &[&str]| {
            mkicbtxt()
                .env("LANG", locale)
                .env("LC_ALL", locale)
                .env("LANGUAGE", locale)
                .args(args)
                .output()
                .unwrap()
        };
        let output = run(&["--help"]);
        assert!(output.status.success() && output.stderr.is_empty());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains(help), "{stdout}");
        for flag in ["--create", "--update", "--full-screen", "--convert", "--force", "--version"] {
            assert!(stdout.contains(flag), "{stdout}");
        }
        for args in [&["--unknown-option"][..], &[]] {
            let output = run(args);
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(stderr.contains(error), "{stderr}");
        }
        for args in [&["--version"][..], &["--unknown-option", "--version"], &["--", "--version"]] {
            let output = run(args);
            assert!(output.status.success() && output.stderr.is_empty());
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                format!("{}\n", icy_board_cli::version_line("mkicbtxt", env!("CARGO_PKG_VERSION"), env!("GIT_HASH")))
            );
        }
    }
}

#[test]
fn missing_file_explains_how_to_create_it() {
    let path = temp_file("missing");
    let output = mkicbtxt().arg(&path).env("LANG", "en_US.UTF-8").output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Input file not found:"));
    assert!(stderr.contains("Usage: "));
    assert!(stderr.contains("--create"));
    assert!(stderr.contains("--help"));
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
