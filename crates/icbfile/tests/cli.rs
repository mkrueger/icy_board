use std::process::{Command, Output};

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
        assert_eq!(decoded(&output.stdout), format!("icbfile {}\n", env!("CARGO_PKG_VERSION")));
        let short = run(locale, &["-V"]);
        assert_eq!(short.status.code(), Some(1));
        assert!(short.stdout.is_empty());
    }
}
