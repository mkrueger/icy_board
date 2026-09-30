use std::{
    fs,
    process::{Command, Output},
};

use dizbase::file_base::FileBase;
use tempfile::TempDir;

fn run(locale: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_icbfile"))
        .args(args)
        .env("LANG", format!("{locale}.UTF-8"))
        .env("LC_ALL", format!("{locale}.UTF-8"))
        .env("LC_MESSAGES", format!("{locale}.UTF-8"))
        .env("LANGUAGE", locale)
        .env("NO_COLOR", "1")
        .output()
        .unwrap()
}

fn decoded(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[test]
fn check_defaults_to_all_areas_and_only_prunes_when_requested() {
    for locale in ["en_US", "de_DE"] {
        let root = TempDir::new().unwrap();
        let config = root.path().join("file_areas.toml");
        fs::write(
            &config,
            r#"
                [[area]]
                name = "One"
                path = "one"
                metadata_path = "metadata/one"
                password = ""

                [[area]]
                name = "Two"
                path = "two"
                metadata_path = "metadata/two"
                password = ""
            "#,
        )
        .unwrap();
        for (area, missing) in [("one", "ONE.TXT"), ("two", "TWO.TXT")] {
            let path = root.path().join(area);
            fs::create_dir(&path).unwrap();
            fs::write(path.join(missing), "missing file").unwrap();
            fs::write(path.join("KEEP.TXT"), "keep this file").unwrap();
            let base = FileBase::open(&path, root.path().join("metadata").join(area)).unwrap();
            assert_eq!(base.to_vec().len(), 2);
            drop(base);
            fs::remove_file(path.join(missing)).unwrap();
        }

        let target = config.to_str().unwrap();
        let output = run(locale, &["check", target]);
        assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
        let stdout = decoded(&output.stdout);
        for expected in ["[0] One", "[1] Two", "missing: ONE.TXT", "missing: TWO.TXT"] {
            assert!(stdout.contains(expected), "{stdout}");
        }
        for area in ["one", "two"] {
            let base = FileBase::open(&root.path().join(area), root.path().join("metadata").join(area)).unwrap();
            assert_eq!(base.to_vec().len(), 2);
        }

        let output = run(locale, &["check", target, "--area", "1"]);
        assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
        let stdout = decoded(&output.stdout);
        assert!(stdout.contains("missing: TWO.TXT"), "{stdout}");
        assert!(!stdout.contains("ONE.TXT"), "{stdout}");

        let output = run(locale, &["check", target, "--area", "oNe", "--prune"]);
        assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
        for (area, expected_count) in [("one", 1), ("two", 2)] {
            let base = FileBase::open(&root.path().join(area), root.path().join("metadata").join(area)).unwrap();
            assert_eq!(base.to_vec().len(), expected_count);
        }

        let output = run(locale, &["check", target, "--prune"]);
        assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
        for area in ["one", "two"] {
            let path = root.path().join(area);
            let base = FileBase::open(&path, root.path().join("metadata").join(area)).unwrap();
            let headers = base.to_vec();
            assert_eq!(headers.len(), 1);
            assert_eq!(headers[0].name, "KEEP.TXT");
            assert_eq!(fs::read_to_string(path.join("KEEP.TXT")).unwrap(), "keep this file");
        }
    }
}

#[test]
fn check_still_accepts_a_directory_target() {
    for locale in ["en_US", "de_DE"] {
        let root = TempDir::new().unwrap();
        let path = root.path();
        fs::write(path.join("MISSING.TXT"), "missing file").unwrap();
        fs::write(path.join("KEEP.TXT"), "keep this file").unwrap();
        drop(FileBase::open(path, path.join("dir")).unwrap());
        fs::remove_file(path.join("MISSING.TXT")).unwrap();

        let output = run(locale, &["check", path.to_str().unwrap()]);
        assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
        assert!(decoded(&output.stdout).contains("missing: MISSING.TXT"));
        assert_eq!(FileBase::open(path, path.join("dir")).unwrap().to_vec().len(), 2);

        let output = run(locale, &["check", path.to_str().unwrap(), "--prune"]);
        assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
        let headers = FileBase::open(path, path.join("dir")).unwrap().to_vec();
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].name, "KEEP.TXT");
        assert_eq!(fs::read_to_string(path.join("KEEP.TXT")).unwrap(), "keep this file");
    }
}

#[test]
fn check_all_continues_after_an_area_failure_and_exits_with_an_error() {
    for locale in ["en_US", "de_DE"] {
        let root = TempDir::new().unwrap();
        let config = root.path().join("file_areas.toml");
        fs::write(
            &config,
            r#"
                [[area]]
                name = "Broken"
                path = "one"
                metadata_path = "blocked/dir"
                password = ""

                [[area]]
                name = "Working"
                path = "two"
                metadata_path = "two/dir"
                password = ""
            "#,
        )
        .unwrap();
        fs::create_dir(root.path().join("one")).unwrap();
        fs::create_dir(root.path().join("two")).unwrap();
        fs::write(root.path().join("blocked"), "not a directory").unwrap();
        let path = root.path().join("two");
        fs::write(path.join("MISSING.TXT"), "missing file").unwrap();
        drop(FileBase::open(&path, path.join("dir")).unwrap());
        fs::remove_file(path.join("MISSING.TXT")).unwrap();

        for prune in [false, true] {
            let mut args = vec!["check", config.to_str().unwrap()];
            if prune {
                args.push("--prune");
            }
            let output = run(locale, &args);
            assert_eq!(output.status.code(), Some(1));
            let stderr = decoded(&output.stderr);
            assert!(stderr.contains("area failed:") && stderr.contains("1 area(s) failed"), "{stderr}");
            let stdout = decoded(&output.stdout);
            for expected in ["[0] Broken", "[1] Working", "missing: MISSING.TXT"] {
                assert!(stdout.contains(expected), "{stdout}");
            }
            let base = FileBase::open(&path, path.join("dir")).unwrap();
            assert_eq!(base.to_vec().len(), if prune { 0 } else { 1 });
        }
    }
}

#[test]
fn help_and_no_arguments_keep_their_streams_and_exit_codes_in_both_languages() {
    for (locale, about, options) in [
        ("en_US", "Convert and maintain icy_board file bases", "Options:"),
        ("de_DE", "icy_board-Dateibereiche konvertieren und verwalten", "Optionen:"),
    ] {
        let help = run(locale, &["--help"]);
        assert_eq!(help.status.code(), Some(0), "{}", decoded(&help.stderr));
        assert!(help.stderr.is_empty());
        let stdout = decoded(&help.stdout);
        assert!(stdout.contains(about), "{stdout}");
        assert!(stdout.contains(options), "{stdout}");
        for command in ["areas", "list", "scan", "check", "import", "export", "set", "repack", "fingerprints"] {
            assert!(stdout.contains(command), "{stdout}");
        }
        let no_args = run(locale, &[]);
        assert_eq!(no_args.status.code(), Some(1));
        assert!(no_args.stdout.is_empty());
        let stderr = decoded(&no_args.stderr);
        assert!(stderr.contains(about) && stderr.contains(options), "{stderr}");
    }
}

#[test]
fn every_subcommand_help_is_localized_without_translating_identifiers() {
    let commands = [
        ("areas", "path to the area list", "Pfad zur Bereichsliste"),
        ("list", "show size, date and download count", "Download-Anzahl anzeigen"),
        ("scan", "scan every area", "jeden Bereich"),
        ("check", "drop entries", "Einträge entfernen"),
        (
            "import",
            "listing format: auto, pcboard or filesbbs",
            "Listenformat: auto, pcboard oder filesbbs",
        ),
        ("export", "encoded as cp437", "als cp437 kodiert"),
        ("set", "the new description", "die neue Beschreibung"),
        ("repack", "zip deflate compression level", "zip-Deflate-Kompressionsstufe"),
        ("fingerprints", "where to write the fingerprints", "Ausgabepfad für die Fingerabdrücke"),
    ];
    for (command, english, german) in commands {
        for (locale, expected) in [("en_US", english), ("de_DE", german)] {
            let help = run(locale, &[command, "--help"]);
            assert_eq!(help.status.code(), Some(0), "{}", decoded(&help.stderr));
            assert!(help.stderr.is_empty());
            let stdout = decoded(&help.stdout);
            assert!(stdout.contains(expected), "{locale} {command}: {stdout}");
            assert!(stdout.contains(&format!("icbfile {command}")), "{stdout}");
            assert!(stdout.contains("--help"), "{stdout}");
            if command == "check" {
                let scope = if locale == "de_DE" { "alle Bereiche" } else { "all areas" };
                assert!(stdout.contains(scope), "{stdout}");
            }
            if locale == "de_DE" {
                assert!(
                    !stdout.contains("Usage:") && !stdout.contains("Options:") && !stdout.contains("Arguments:"),
                    "{stdout}"
                );
            }
        }
    }
    let help = decoded(&run("de_DE", &["repack", "--help"]).stdout);
    for identifier in [
        "--compression-level",
        "--max-members",
        "--max-member-size",
        "--max-expanded-size",
        "--max-compression-ratio",
        "--keep-case",
    ] {
        assert!(help.contains(identifier), "{help}");
    }
    assert!(help.contains("<target>"), "{help}");
}

#[test]
fn parser_errors_are_localized_on_stderr_with_exit_one() {
    for locale in ["en_US", "de_DE"] {
        for args in [vec!["--not-an-option"], vec!["areas"], vec!["repack", "files", "--max-members", "not-a-number"]] {
            let output = run(locale, &args);
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            let stderr = decoded(&output.stderr);
            assert!(stderr.contains("--help"), "{stderr}");
            let prefix = if locale == "de_DE" { "fehler" } else { "error" };
            assert!(stderr.to_lowercase().contains(prefix), "{stderr}");
        }
        let output = run(locale, &["import", "files", "--format", "bogus"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let stderr = decoded(&output.stderr);
        let expected = if locale == "de_DE" { "unbekanntes Format" } else { "unknown format" };
        assert!(stderr.contains(expected) && stderr.contains("bogus") && stderr.contains("--format"), "{stderr}");
    }
}

#[test]
fn version_remains_manual_and_locale_independent() {
    for locale in ["en_US", "de_DE"] {
        let output = run(locale, &["--version"]);
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());
        assert_eq!(
            decoded(&output.stdout),
            format!("{}\n", icy_board_cli::version_line("icbfile", env!("CARGO_PKG_VERSION"), env!("GIT_HASH")))
        );
        let short = run(locale, &["-V"]);
        assert_eq!(short.status.code(), Some(1));
        assert!(short.stdout.is_empty());
    }
}
