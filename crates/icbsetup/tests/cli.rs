use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use icy_board_engine::icy_board::{
    IcyBoard, IcyBoardSerializer,
    icb_config::IcbConfig,
    language::{Language, SupportedLanguages},
    lock::BoardLock,
};

fn temp_dir(name: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("icbsetup-{name}-{}-{nonce}", std::process::id()))
}

fn icbsetup() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_icbsetup"));
    command.env("LANG", "en_US.UTF-8").env("LC_ALL", "en_US.UTF-8").env("LANGUAGE", "en");
    command
}

#[test]
fn cli_help_errors_and_version_are_localized() {
    for (locale, help, error) in [("en", "Use the full screen", "error"), ("de", "Vollbild verwenden", "Fehler")] {
        let run = |args: &[&str]| {
            icbsetup()
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
        for subcommand in ["import", "create", "ppe-convert", "check", "dos-image", "dos-copy"] {
            assert!(stdout.contains(subcommand), "{stdout}");
        }
        let output = run(&["--unknown-option"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(error) && stderr.contains("--unknown-option"), "{stderr}");
        let output = run(&["--version"]);
        assert!(output.status.success() && output.stderr.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            format!("{}\n", icy_board_cli::version_line("icbsetup", env!("CARGO_PKG_VERSION"), env!("GIT_HASH")))
        );
    }
}

#[test]
fn cli_subcommand_help_and_errors_are_localized_recursively() {
    for (locale, descriptions, error) in [
        (
            "en",
            [
                "PCBOARD.DAT file",
                "Output directory",
                "Directory to convert",
                "Offer to create",
                "Board directory",
                "Host file to copy",
            ],
            "error",
        ),
        (
            "de",
            [
                "PCBOARD.DAT-Datei",
                "Ausgabeverzeichnis",
                "Zu konvertierendes Verzeichnis",
                "Das Erstellen",
                "Mailbox-Verzeichnis",
                "Zu kopierende Host-Datei",
            ],
            "Fehler",
        ),
    ] {
        for (subcommand, description) in ["import", "create", "ppe-convert", "check", "dos-image", "dos-copy"]
            .into_iter()
            .zip(descriptions)
        {
            let run = |arg: &str| {
                icbsetup()
                    .env("LANG", locale)
                    .env("LC_ALL", locale)
                    .env("LANGUAGE", locale)
                    .args([subcommand, arg])
                    .output()
                    .unwrap()
            };
            let output = run("--help");
            assert!(output.status.success() && output.stderr.is_empty());
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(stdout.contains(description) && stdout.contains(subcommand), "{stdout}");
            let output = run("--unknown-option");
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(stderr.contains(error) && stderr.contains("--unknown-option"), "{stderr}");
        }
    }
}

#[test]
fn missing_board_configuration_explains_how_to_start() {
    let path = temp_dir("missing-board");
    let output = icbsetup().arg(&path).env("LANG", "en_US.UTF-8").env_remove("ICB_PATH").output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("IcyBoard configuration not found:"));
    assert!(stderr.contains("Usage: "));
    assert!(stderr.contains("icbsetup create mybbs"));
    assert!(stderr.contains("docs/gettingstarted.md"));
}

#[test]
fn no_arguments_reports_a_missing_board_on_stderr() {
    let directory = temp_dir("no-arguments");
    fs::create_dir(&directory).unwrap();
    let output = icbsetup().current_dir(&directory).env_remove("ICB_PATH").output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("IcyBoard configuration not found:"));
    fs::remove_dir(directory).unwrap();
}

#[test]
fn check_with_missing_board_configuration_explains_how_to_start() {
    let path = temp_dir("missing-check-board");
    let output = icbsetup()
        .args(["check", path.to_str().unwrap()])
        .env("LANG", "en_US.UTF-8")
        .env_remove("ICB_PATH")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("IcyBoard configuration not found:"));
    assert!(stderr.contains("icbsetup check"));
    assert!(stderr.contains("icbsetup create mybbs"));
    assert!(stderr.contains("docs/gettingstarted.md"));
}

#[test]
fn missing_import_source_is_a_failure() {
    let source = temp_dir("missing-source");
    let output = temp_dir("missing-output");
    let status = icbsetup()
        .args(["import", source.to_str().unwrap(), output.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(!status.success());
    assert!(!output.exists());
}

#[test]
fn import_refuses_an_existing_destination() {
    let source = temp_dir("source");
    let output = temp_dir("existing-output");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("keep"), b"untouched").unwrap();

    let status = icbsetup()
        .args(["import", source.to_str().unwrap(), output.to_str().unwrap()])
        .status()
        .unwrap();

    assert!(!status.success());
    assert_eq!(fs::read(output.join("keep")).unwrap(), b"untouched");
    fs::remove_dir_all(output).unwrap();
}

#[test]
fn ppe_convert_keeps_the_root_and_lowercases_only_descendants() {
    let parent = temp_dir("MixedParent");
    let root = parent.join("SubDir");
    fs::create_dir_all(root.join("NestedDir")).unwrap();
    fs::write(root.join("FILE.TXT"), b"HELLO\r\n").unwrap();
    fs::write(root.join("NestedDir/README.DOC"), b"DOC\r\n").unwrap();

    let status = icbsetup().args(["ppe-convert", root.to_str().unwrap()]).status().unwrap();

    assert!(status.success());
    assert!(root.is_dir());
    assert!(root.join("file.txt").is_file());
    assert!(root.join("nesteddir/readme.doc").is_file());
    assert!(fs::read(root.join("file.txt")).unwrap().starts_with(&[0xEF, 0xBB, 0xBF]));
    fs::remove_dir_all(parent).unwrap();
}

#[test]
fn create_gives_sysop_a_password() {
    let output = temp_dir("create");

    let result = icbsetup().args(["create", output.to_str().unwrap()]).output().unwrap();

    assert!(result.status.success());
    let board = IcyBoard::load(&output.join("icboard.toml")).unwrap();
    assert!(!board.users[0].password.password.is_empty());
    assert!(String::from_utf8_lossy(&result.stdout).contains("Initial SYSOP password:"));
    let help = output.join(&board.config.paths.help_path);
    let sources = icy_board_help::catalog::sources(None).unwrap();
    assert_eq!(sources.len(), 68);
    assert_eq!(fs::read_dir(&help).unwrap().count(), 68);
    for number in 1..=16 {
        assert!(help.join(format!("hlp{number}.pcb")).is_file());
    }
    for source in sources {
        assert!(help.join(format!("{}.pcb", source.topic)).is_file());
        assert!(!help.join(format!("{}.de.pcb", source.topic)).exists());
    }
    fs::remove_dir_all(output).unwrap();
}

fn genhelp_command(root: &std::path::Path) -> Command {
    let mut command = icbsetup();
    command.current_dir(root).env_remove("ICB_PATH").arg("genhelp");
    command
}

fn save_help_config(root: &std::path::Path, config: &IcbConfig) {
    config.save(&root.join("icboard.toml")).unwrap();
}

fn tree_snapshot(root: &std::path::Path) -> Vec<(std::path::PathBuf, Option<Vec<u8>>)> {
    let mut result = walkdir::WalkDir::new(root)
        .into_iter()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.path().strip_prefix(root).unwrap().to_path_buf(),
                entry.file_type().is_file().then(|| fs::read(entry.path()).unwrap()),
            )
        })
        .collect::<Vec<_>>();
    result.sort();
    result
}

fn assert_help_success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn genhelp_nested_help_and_errors_are_localized() {
    let root = tempfile::tempdir().unwrap();
    for (locale, theme, destination, error, missing) in [
        (
            "en",
            "Theme: classic, minimal",
            "empty destination",
            "error",
            "IcyBoard configuration not found",
        ),
        (
            "de",
            "Design: classic, minimal",
            "leeres Zielverzeichnis",
            "Fehler",
            "IcyBoard-Konfiguration nicht gefunden",
        ),
    ] {
        let run = |args: &[&str]| {
            genhelp_command(root.path())
                .env("LANGUAGE", locale)
                .env("LANG", locale)
                .env("LC_ALL", locale)
                .args(args)
                .output()
                .unwrap()
        };
        let output = run(&["--help"]);
        assert_help_success(&output);
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        assert!(stdout.contains(theme), "{stdout}");
        for leaf in ["check", "export"] {
            assert!(stdout.contains(leaf), "{stdout}");
        }
        assert!(!stdout.contains("preview"), "{stdout}");
        let output = run(&["check", "--help"]);
        assert_help_success(&output);
        assert!(String::from_utf8_lossy(&output.stdout).contains(theme));
        let output = run(&["export", "--help"]);
        assert_help_success(&output);
        assert!(String::from_utf8_lossy(&output.stdout).contains(destination));
        for args in [vec!["--unknown-option"], vec!["check", "--unknown-option"], vec!["export", "--unknown-option"]] {
            let output = run(&args);
            assert!(!output.status.success(), "{args:?}");
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(stderr.contains(error) && stderr.contains("--unknown-option"), "{stderr}");
        }
        for args in [vec!["missing-board"], vec!["check", "missing-board"]] {
            let output = run(&args);
            assert!(!output.status.success(), "{args:?}");
            assert!(String::from_utf8_lossy(&output.stderr).contains(missing), "{args:?}");
        }
    }
    assert_eq!(tree_snapshot(root.path()).len(), 1);
}

#[test]
fn genhelp_check_and_validation_never_write() {
    let root = tempfile::tempdir().unwrap();
    let before = tree_snapshot(root.path());
    let output = genhelp_command(root.path()).arg("check").output().unwrap();
    assert_help_success(&output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "Validated help outputs: 68");
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(tree_snapshot(root.path()), before);
    for args in [
        vec!["preview"],
        vec!["check", "--board", "."],
        vec!["check", "--topic", "hlpa"],
        vec!["check", "--encoding", "cp437"],
        vec!["check", "--theme", "unknown"],
        vec!["check", "--width", "39"],
        vec!["check", "--width", "80"],
        vec!["check", "missing-board"],
        vec!["--dry-run"],
        vec!["--output", "out", "--replace-modified"],
    ] {
        let output = genhelp_command(root.path()).args(&args).output().unwrap();
        assert!(!output.status.success(), "{args:?}");
        assert_eq!(tree_snapshot(root.path()), before, "{args:?}");
    }
}

#[test]
fn genhelp_title_only_overrides_fail_before_any_writes() {
    for locale in ["en", "de"] {
        let root = tempfile::tempdir().unwrap();
        let sources = root.path().join("sources");
        fs::create_dir(&sources).unwrap();
        let run = |args: &[&str]| {
            genhelp_command(root.path())
                .env("LANGUAGE", locale)
                .env("LANG", locale)
                .env("LC_ALL", locale)
                .args(args)
                .args(["--sources", "sources"])
                .output()
                .unwrap()
        };
        for markdown in ["# Local title\n\n", "Local title\n===========\n\n"] {
            fs::write(sources.join("hlpz.md"), markdown).unwrap();
            let before = tree_snapshot(root.path());
            for args in [vec!["check"], vec!["--output", "rejected"]] {
                let output = run(&args);
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(!output.status.success(), "{locale}: {args:?}");
                assert!(stderr.contains("hlpz.md"), "{locale}: {args:?}: {stderr}");
                assert!(output.stdout.is_empty(), "{locale}: {args:?}");
                assert_eq!(tree_snapshot(root.path()), before, "{locale}: {args:?}");
            }
        }

        let config = IcbConfig::default();
        save_help_config(root.path(), &config);
        let help = root.path().join(&config.paths.help_path);
        fs::create_dir_all(&help).unwrap();
        fs::write(help.join("hlpa.pcb"), b"custom help").unwrap();
        fs::create_dir(root.path().join("existing")).unwrap();
        fs::write(root.path().join("existing/hlpa.pcb"), b"custom output").unwrap();
        let before = tree_snapshot(root.path());
        for args in [
            vec!["check", "."],
            vec![".", "--adopt"],
            vec![".", "--adopt", "--dry-run"],
            vec!["--output", "rejected"],
            vec!["--output", "existing", "--adopt"],
        ] {
            let output = run(&args);
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(!output.status.success(), "{locale}: {args:?}");
            assert!(stderr.contains("hlpz.md"), "{locale}: {args:?}: {stderr}");
            assert!(output.stdout.is_empty(), "{locale}: {args:?}");
            assert_eq!(tree_snapshot(root.path()), before, "{locale}: {args:?}");
        }
    }
}

#[test]
fn genhelp_resolves_the_board_positionally() {
    let root = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let config = IcbConfig::default();
    save_help_config(root.path(), &config);
    let before = tree_snapshot(root.path());
    for board in [root.path().to_path_buf(), root.path().join("icboard.toml")] {
        let output = genhelp_command(elsewhere.path()).arg(&board).arg("--dry-run").output().unwrap();
        assert_help_success(&output);
        assert_eq!(tree_snapshot(root.path()), before);
    }
    let output = genhelp_command(root.path()).arg("--dry-run").output().unwrap();
    assert_help_success(&output);
    assert_eq!(tree_snapshot(root.path()), before);
    let output = genhelp_command(elsewhere.path()).arg(root.path().join("missing.toml")).output().unwrap();
    assert!(!output.status.success());
    assert_eq!(tree_snapshot(root.path()), before);
    // Installing from elsewhere proves the resolved board root, not the invocation directory, is written.
    let output = genhelp_command(elsewhere.path()).arg(root.path()).output().unwrap();
    assert_help_success(&output);
    assert_eq!(fs::read_dir(root.path().join(&config.paths.help_path)).unwrap().count(), 68);
    assert!(root.path().join("main/help-generation.toml").is_file());
    assert_eq!(tree_snapshot(elsewhere.path()).len(), 1);
}

#[test]
fn genhelp_needs_no_board_configuration_and_ignores_a_stale_section() {
    let root = tempfile::tempdir().unwrap();
    let config = IcbConfig::default();
    let defaults = toml::Value::try_from(&config).unwrap();
    // Generation settings are CLI flags only; the board configuration has no section for them.
    assert!(defaults.get("help_generation").is_none(), "{defaults}");
    let base = toml::to_string(&defaults).unwrap();
    let from_base = toml::Value::try_from(toml::from_str::<IcbConfig>(&base).unwrap()).unwrap();

    let path = root.path().join("icboard.toml");
    fs::write(&path, &base).unwrap();
    let before = tree_snapshot(root.path());
    for args in [vec!["check"], vec!["--dry-run"]] {
        let output = genhelp_command(root.path()).args(&args).output().unwrap();
        assert_help_success(&output);
        assert_eq!(tree_snapshot(root.path()), before, "{args:?}");
    }

    // A leftover section from an older board is ignored and must never stop the board from loading.
    let stale = format!("{base}\n[help_generation]\nlanguages = [\"en\", \"de\"]\ntheme = \"classic\"\nwidht = 40\n");
    let from_stale = toml::Value::try_from(toml::from_str::<IcbConfig>(&stale).unwrap()).unwrap();
    assert_eq!(from_stale, from_base);
    fs::write(&path, &stale).unwrap();
    IcbConfig::load(&path).unwrap();
    let before = tree_snapshot(root.path());
    for args in [vec!["check"], vec!["--dry-run"]] {
        let output = genhelp_command(root.path()).args(&args).output().unwrap();
        assert_help_success(&output);
        assert_eq!(tree_snapshot(root.path()), before, "{args:?}");
    }
    let output = genhelp_command(root.path()).output().unwrap();
    assert_help_success(&output);
    assert_eq!(fs::read_dir(root.path().join(&config.paths.help_path)).unwrap().count(), 68);
    assert_eq!(fs::read_to_string(&path).unwrap(), stale);
}

#[test]
fn genhelp_output_directory_writes_without_a_board_ledger() {
    let root = tempfile::tempdir().unwrap();
    let config = IcbConfig::default();
    save_help_config(root.path(), &config);
    let out = root.path().join("out");
    let before = tree_snapshot(root.path());
    let output = genhelp_command(root.path()).args(["--output", "out", "--dry-run"]).output().unwrap();
    assert_help_success(&output);
    assert!(String::from_utf8_lossy(&output.stdout).contains("68"));
    assert!(!out.exists());
    assert_eq!(tree_snapshot(root.path()), before);

    let output = genhelp_command(root.path()).args(["--output", "out"]).output().unwrap();
    assert_help_success(&output);
    assert_eq!(fs::read_dir(&out).unwrap().count(), 68);
    assert!(out.join("hlpa.pcb").is_file());
    // Unmanaged mode keeps no ledger or backups and leaves the board's own help directory alone.
    assert!(!root.path().join("main").exists());
    assert!(!root.path().join(&config.paths.help_path).exists());

    let before = tree_snapshot(root.path());
    let output = genhelp_command(root.path()).args(["--output", "out"]).output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--adopt"));
    assert_eq!(tree_snapshot(root.path()), before);
    let output = genhelp_command(root.path()).args(["--output", "out", "--adopt"]).output().unwrap();
    assert_help_success(&output);
    assert_eq!(tree_snapshot(root.path()), before);

    for args in [vec!["--output", "rejected", "--replace-modified"], vec![".", "--output", "rejected"]] {
        let output = genhelp_command(root.path()).args(&args).output().unwrap();
        assert!(!output.status.success(), "{args:?}");
        assert!(!root.path().join("rejected").exists(), "{args:?}");
    }
    assert_eq!(tree_snapshot(root.path()), before);
}

#[test]
fn genhelp_language_suffixes_outputs_and_rejects_invalid_extensions() {
    let root = tempfile::tempdir().unwrap();
    let output = genhelp_command(root.path()).args(["--output", "german", "--language", "ger"]).output().unwrap();
    assert_help_success(&output);
    let german = root.path().join("german");
    assert_eq!(fs::read_dir(&german).unwrap().count(), 68);
    assert!(german.join("hlpa.ger.pcb").is_file());
    assert!(!german.join("hlpa.pcb").exists());
    let output = genhelp_command(root.path()).args(["--output", "english"]).output().unwrap();
    assert_help_success(&output);
    let english = root.path().join("english");
    assert!(english.join("hlpa.pcb").is_file());
    assert!(!english.join("hlpa.ger.pcb").exists());
    // Only the file name differs: --language does not select a translated source.
    assert_eq!(fs::read(german.join("hlpa.ger.pcb")).unwrap(), fs::read(english.join("hlpa.pcb")).unwrap());
    let long = "g".repeat(33);
    for extension in ["", "DE", "de-DE", "../de", long.as_str(), "1de"] {
        let output = genhelp_command(root.path())
            .args(["--output", "rejected", "--language", extension])
            .output()
            .unwrap();
        assert!(!output.status.success(), "{extension:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("language"), "{extension:?}");
        assert!(!root.path().join("rejected").exists(), "{extension:?}");
    }
}

#[test]
fn genhelp_cp437_flag_is_tristate() {
    let root = tempfile::tempdir().unwrap();
    let bom = [0xEF, 0xBB, 0xBF];
    for (directory, flag) in [("default", None), ("cp437", Some("--cp437")), ("utf8", Some("--cp437=false"))] {
        let mut command = genhelp_command(root.path());
        command.args(["--output", directory]);
        if let Some(flag) = flag {
            command.arg(flag);
        }
        let output = command.output().unwrap();
        assert_help_success(&output);
        let bytes = fs::read(root.path().join(directory).join("hlpa.pcb")).unwrap();
        assert_eq!(bytes.starts_with(&bom), directory != "cp437", "{directory}");
    }
    assert_eq!(
        fs::read(root.path().join("default/hlpa.pcb")).unwrap(),
        fs::read(root.path().join("utf8/hlpa.pcb")).unwrap()
    );
    // require_equals: the space form leaves "false" as the board positional instead of a flag value.
    let output = genhelp_command(root.path())
        .args(["--output", "rejected", "--cp437", "false"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!root.path().join("rejected").exists());

    let config = IcbConfig::default();
    save_help_config(root.path(), &config);
    let installed = root.path().join(&config.paths.help_path).join("hlpa.pcb");
    let output = genhelp_command(root.path()).arg("--cp437=false").output().unwrap();
    assert_help_success(&output);
    assert!(fs::read(&installed).unwrap().starts_with(&bom));
    let output = genhelp_command(root.path()).arg("--cp437").output().unwrap();
    assert_help_success(&output);
    assert!(!fs::read(&installed).unwrap().starts_with(&bom));
    // Without the switch the default is UTF-8; no board setting carries the previous CP437 choice over.
    let output = genhelp_command(root.path()).output().unwrap();
    assert_help_success(&output);
    assert!(fs::read(&installed).unwrap().starts_with(&bom));
}

#[test]
fn genhelp_export_writes_a_flat_bundle_and_refuses_nonempty_destinations() {
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join("sources");
    let output = genhelp_command(root.path()).arg("export").arg(&destination).output().unwrap();
    assert_help_success(&output);
    assert!(destination.join("catalog.toml").is_file());
    assert!(destination.join("hlpa.md").is_file());
    assert!(!destination.join("en").exists() && !destination.join("de").exists());
    assert_eq!(fs::read_dir(&destination).unwrap().count(), 69);
    for number in 1..=16 {
        assert!(destination.join(format!("hlp{number}.md")).is_file());
    }
    assert_eq!(
        icy_board_help::catalog::sources(Some(&destination)).unwrap(),
        icy_board_help::catalog::sources(None).unwrap()
    );
    let before = tree_snapshot(root.path());
    let output = genhelp_command(root.path()).arg("export").arg(&destination).output().unwrap();
    assert!(!output.status.success());
    assert_eq!(tree_snapshot(root.path()), before);
    let output = genhelp_command(root.path()).args(["check", "--sources", "sources"]).output().unwrap();
    assert_help_success(&output);
    assert_eq!(tree_snapshot(root.path()), before);
}

#[test]
fn genhelp_flat_sources_override_topics_and_reject_locale_subdirectories() {
    let root = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let config = IcbConfig::default();
    save_help_config(root.path(), &config);
    let sources = root.path().join("sources");
    fs::create_dir_all(&sources).unwrap();
    let markdown = "# Local help\n\nLocal body.\n";
    fs::write(sources.join("hlpa.md"), markdown).unwrap();
    let theme_text = "title = 14\nbody = 10\n";
    let theme_file = root.path().join("theme.toml");
    fs::write(&theme_file, theme_text).unwrap();
    let output = genhelp_command(elsewhere.path())
        .arg(root.path())
        .args([
            "--sources",
            sources.to_str().unwrap(),
            "--theme",
            theme_file.to_str().unwrap(),
            "--width",
            "70",
            "--cp437=false",
        ])
        .output()
        .unwrap();
    assert_help_success(&output);
    let path = root.path().join(&config.paths.help_path).join("hlpa.pcb");
    let expected = icy_board_help::render(
        markdown,
        &icy_board_help::RenderOptions {
            width: 70,
            encoding: icy_board_help::Encoding::Utf8,
            theme: toml::from_str(theme_text).unwrap(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(fs::read(&path).unwrap(), expected.bytes);
    let ledger = root.path().join("main/help-generation.toml");
    let before: toml::Value = toml::from_str(&fs::read_to_string(&ledger).unwrap()).unwrap();

    // 68 columns is the narrowest width the whole embedded corpus still renders at.
    let minimal = icy_board_help::RenderOptions {
        width: 68,
        encoding: icy_board_help::Encoding::Cp437,
        theme: icy_board_help::HelpTheme::preset("minimal").unwrap(),
        ..Default::default()
    };
    let output = genhelp_command(elsewhere.path())
        .arg(root.path().join("icboard.toml"))
        .args(["--sources", sources.to_str().unwrap(), "--theme", "minimal", "--width", "68", "--cp437"])
        .output()
        .unwrap();
    assert_help_success(&output);
    assert_eq!(fs::read(&path).unwrap(), icy_board_help::render(markdown, &minimal).unwrap().bytes);
    let after: toml::Value = toml::from_str(&fs::read_to_string(&ledger).unwrap()).unwrap();
    assert_eq!(before["entries"][0]["source_hash"], after["entries"][0]["source_hash"]);
    assert_ne!(before["entries"][0]["settings_hash"], after["entries"][0]["settings_hash"]);

    // A relative --sources is resolved against the invocation, never against the board root.
    let unchanged = tree_snapshot(root.path());
    let output = genhelp_command(elsewhere.path())
        .arg(root.path())
        .args(["--sources", "sources"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(tree_snapshot(root.path()), unchanged);

    fs::create_dir_all(elsewhere.path().join("overrides")).unwrap();
    let cli_markdown = "# CLI source\n\nOverride.\n";
    fs::write(elsewhere.path().join("overrides/hlpa.md"), cli_markdown).unwrap();
    let output = genhelp_command(elsewhere.path())
        .arg(root.path())
        .args(["--sources", "overrides", "--theme", "minimal", "--width", "68", "--cp437"])
        .output()
        .unwrap();
    assert_help_success(&output);
    assert_eq!(fs::read(&path).unwrap(), icy_board_help::render(cli_markdown, &minimal).unwrap().bytes);

    // A locale subdirectory is the old layout and must be refused before anything is written.
    fs::create_dir_all(elsewhere.path().join("overrides/en")).unwrap();
    let before = tree_snapshot(root.path());
    for args in [vec!["--sources", "overrides"], vec!["--sources", "overrides", "--dry-run"]] {
        let output = genhelp_command(elsewhere.path()).arg(root.path()).args(&args).output().unwrap();
        assert!(!output.status.success(), "{args:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("subdirectory"), "{args:?}");
        assert_eq!(tree_snapshot(root.path()), before, "{args:?}");
    }
}

#[test]
fn genhelp_theme_accepts_a_file_path_and_rejects_unreadable_ones() {
    let root = tempfile::tempdir().unwrap();
    let theme_text = "title = 9\nheading = 9\nbody = 7\nemphasis = 15\ncode = 2\nnote = 5\nborder = 3\nmargin = 1\ndecoration = false\n";
    fs::write(root.path().join("custom.toml"), theme_text).unwrap();
    let options = icy_board_help::RenderOptions {
        theme: toml::from_str(theme_text).unwrap(),
        ..Default::default()
    };
    let output = genhelp_command(root.path()).args(["--output", "themed", "--theme", "custom.toml"]).output().unwrap();
    assert_help_success(&output);
    let themed = root.path().join("themed");
    assert_eq!(fs::read_dir(&themed).unwrap().count(), 68);
    for source in icy_board_help::catalog::sources(None).unwrap() {
        let expected = icy_board_help::render(&source.markdown, &options).unwrap();
        assert_eq!(
            fs::read(themed.join(format!("{}.pcb", source.topic))).unwrap(),
            expected.bytes,
            "{}",
            source.topic
        );
    }
    let output = genhelp_command(root.path()).args(["--output", "classic"]).output().unwrap();
    assert_help_success(&output);
    assert_ne!(
        fs::read(themed.join("hlpa.pcb")).unwrap(),
        fs::read(root.path().join("classic/hlpa.pcb")).unwrap()
    );

    fs::write(root.path().join("broken.toml"), "title = ").unwrap();
    fs::write(root.path().join("unknown-field.toml"), "colour = 3\n").unwrap();
    fs::write(root.path().join("blinking.toml"), "title = 200\n").unwrap();
    fs::create_dir(root.path().join("directory.toml")).unwrap();
    let before = tree_snapshot(root.path());
    for theme in ["missing.toml", "broken.toml", "unknown-field.toml", "blinking.toml", "directory.toml"] {
        let output = genhelp_command(root.path())
            .args(["--output", "rejected", "--theme", theme])
            .output()
            .unwrap();
        assert!(!output.status.success(), "{theme}");
        assert!(String::from_utf8_lossy(&output.stderr).to_lowercase().contains("theme"), "{theme}");
        assert!(!root.path().join("rejected").exists(), "{theme}");
        assert_eq!(tree_snapshot(root.path()), before, "{theme}");
    }
}

#[test]
fn genhelp_ignores_board_language_definitions() {
    let root = tempfile::tempdir().unwrap();
    let mut config = IcbConfig::default();
    config.paths.language_file = "languages.toml".into();
    save_help_config(root.path(), &config);
    let mut languages = SupportedLanguages::default();
    for (locale, extension) in [("en_US", "eng"), ("de_DE", "ger"), ("DE-at", "../de")] {
        languages.push(Language {
            locale: locale.to_string(),
            extension: extension.to_string(),
            ..Default::default()
        });
    }
    languages.save(&root.path().join("languages.toml")).unwrap();
    let languages_before = fs::read(root.path().join("languages.toml")).unwrap();
    let help = root.path().join(&config.paths.help_path);
    fs::create_dir_all(&help).unwrap();
    fs::write(help.join("hlpa.de.pcb"), b"custom de help").unwrap();
    fs::write(help.join("hlpa.ger.pcb"), b"custom ger help").unwrap();
    let output = genhelp_command(root.path()).output().unwrap();
    assert_help_success(&output);
    let sources = icy_board_help::catalog::sources(None).unwrap();
    assert_eq!(sources.len(), 68);
    assert_eq!(fs::read_dir(&help).unwrap().count(), 70);
    for source in sources {
        let expected = icy_board_help::render(&source.markdown, &Default::default()).unwrap();
        assert_eq!(fs::read(help.join(format!("{}.pcb", source.topic))).unwrap(), expected.bytes);
    }
    assert_eq!(fs::read(help.join("hlpa.de.pcb")).unwrap(), b"custom de help");
    assert_eq!(fs::read(help.join("hlpa.ger.pcb")).unwrap(), b"custom ger help");
    assert_eq!(fs::read(root.path().join("languages.toml")).unwrap(), languages_before);
    // A malformed language file is never read, and regeneration stays idempotent.
    let malformed = b"[[languages\nnot valid TOML";
    fs::write(root.path().join("languages.toml"), malformed).unwrap();
    let before = tree_snapshot(root.path());
    for args in [vec!["check"], vec!["--dry-run"], vec![]] {
        let output = genhelp_command(root.path()).args(args).output().unwrap();
        assert_help_success(&output);
        assert_eq!(tree_snapshot(root.path()), before);
    }
    assert_eq!(fs::read(root.path().join("languages.toml")).unwrap(), malformed);
}

#[test]
fn genhelp_conflicts_require_separate_adoption_and_replacement_flags() {
    let root = tempfile::tempdir().unwrap();
    let config = IcbConfig::default();
    save_help_config(root.path(), &config);
    let path = root.path().join(&config.paths.help_path).join("hlpa.pcb");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"custom help").unwrap();
    for flags in [vec!["--dry-run"], vec!["--replace-modified", "--dry-run"]] {
        let before = tree_snapshot(root.path());
        let output = genhelp_command(root.path()).args(flags).output().unwrap();
        assert!(!output.status.success());
        assert_eq!(tree_snapshot(root.path()), before);
    }
    let before = tree_snapshot(root.path());
    let output = genhelp_command(root.path()).args(["--adopt", "--dry-run"]).output().unwrap();
    assert_help_success(&output);
    assert_eq!(tree_snapshot(root.path()), before);
    let output = genhelp_command(root.path()).arg("--adopt").output().unwrap();
    assert_help_success(&output);
    let generated = fs::read(&path).unwrap();
    fs::write(&path, b"edited managed help").unwrap();
    let before = tree_snapshot(root.path());
    for command in [vec!["check"], vec!["--adopt"]] {
        let output = genhelp_command(root.path()).args(command).output().unwrap();
        assert!(!output.status.success());
        assert_eq!(tree_snapshot(root.path()), before);
    }
    let output = genhelp_command(root.path()).arg("--replace-modified").output().unwrap();
    assert_help_success(&output);
    assert_eq!(fs::read(&path).unwrap(), generated);
    assert!(
        tree_snapshot(root.path())
            .iter()
            .any(|(_, bytes)| bytes.as_deref() == Some(b"custom help".as_slice()))
    );
    assert!(
        tree_snapshot(root.path())
            .iter()
            .any(|(_, bytes)| bytes.as_deref() == Some(b"edited managed help".as_slice()))
    );
}

#[test]
fn genhelp_apply_requires_lock_but_check_and_dry_run_do_not() {
    let root = tempfile::tempdir().unwrap();
    save_help_config(root.path(), &IcbConfig::default());
    let _lock = BoardLock::acquire(root.path()).unwrap();
    let before = tree_snapshot(root.path());
    for command in [vec!["check"], vec!["--dry-run"]] {
        let output = genhelp_command(root.path()).args(command).output().unwrap();
        assert_help_success(&output);
        assert_eq!(tree_snapshot(root.path()), before);
    }
    // export and --output never touch board files, so the board lock does not apply to them.
    for command in [vec!["export", "exported"], vec!["--output", "unmanaged"]] {
        let output = genhelp_command(root.path()).args(command).output().unwrap();
        assert_help_success(&output);
    }
    assert!(root.path().join("exported/catalog.toml").is_file());
    assert!(root.path().join("unmanaged/hlpa.pcb").is_file());
    let before = tree_snapshot(root.path());
    let output = genhelp_command(root.path()).output().unwrap();
    assert!(!output.status.success());
    assert_eq!(tree_snapshot(root.path()), before);
}
