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
        .env("LC_ALL", "en_US.UTF-8")
        .env("LANGUAGE", "en")
        .env_remove("ICB_PATH")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(stderr.contains("IcyBoard configuration not found:"));
    assert!(stderr.contains("./icboard.toml"));
    assert!(stderr.contains("Usage: "));
    assert!(stderr.contains("icbsetup create mybbs"));
    assert!(stderr.contains("docs/gettingstarted.md"));

    fs::remove_dir(directory).unwrap();
}

#[test]
fn cli_help_errors_and_version_are_localized() {
    for (locale, help, error) in [("en", "Use the full screen", "error"), ("de", "Vollbild verwenden", "Fehler")] {
        let run = |args: &[&str]| {
            Command::new(env!("CARGO_BIN_EXE_icbsm"))
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
        for flag in [
            "--full-screen",
            "--pack",
            "--inactive-days",
            "--never-logged-on",
            "--no-delete-flagged",
            "--keep-security",
            "--pack-locked-out",
            "--standardize-phones",
            "--undo",
            "--dry-run",
            "--version",
        ] {
            assert!(stdout.contains(flag), "{stdout}");
        }
        let output = run(&["--unknown-option"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(error) && stderr.contains("--unknown-option"), "{stderr}");
        let output = run(&["--version"]);
        assert!(output.status.success() && output.stderr.is_empty());
        assert_eq!(String::from_utf8_lossy(&output.stdout), concat!("icbsm ", env!("CARGO_PKG_VERSION"), "\n"));
    }
}
