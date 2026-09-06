use std::process::{Command, Output};

fn run(locale: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_icbmailer"))
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
        ("en_US", "Exchange FTN and QWKnet mail with configured systems", "Options:"),
        ("de_DE", "FTN- und QWKnet-Nachrichten mit konfigurierten Systemen austauschen", "Optionen:"),
    ] {
        let help = run(locale, &["--help"]);
        assert_eq!(help.status.code(), Some(0), "{}", decoded(&help.stderr));
        assert!(help.stderr.is_empty());
        let stdout = decoded(&help.stdout);
        assert!(stdout.contains(about) && stdout.contains(options), "{stdout}");
        for command in ["links", "poll", "scan", "show", "toss", "qwk-links", "qwk-poll", "qwk-scan", "qwk-toss"] {
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
fn every_subcommand_help_is_localized_and_keeps_command_and_value_names() {
    let commands = [
        ("links", "list the configured links", "konfigurierte Verbindungen"),
        ("poll", "report what the session is doing", "den Sitzungsablauf protokollieren"),
        ("scan", "report what the scanner is doing", "das Sammeln ausgehender Nachrichten protokollieren"),
        ("show", "print the message text as well", "auch den Nachrichtentext ausgeben"),
        ("toss", "report what the tosser is doing", "den Importvorgang protokollieren"),
        ("qwk-links", "list configured QWKnet hubs", "konfigurierte QWKnet-Hubs auflisten"),
        ("qwk-poll", "scan, exchange and import mail", "sammeln, austauschen und importieren"),
        ("qwk-scan", "create REP packets", "REP-Pakete"),
        ("qwk-toss", "import QWK packets", "wartende QWK-Pakete"),
    ];
    for (command, english, german) in commands {
        for (locale, expected) in [("en_US", english), ("de_DE", german)] {
            let output = run(locale, &[command, "--help"]);
            assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
            assert!(output.stderr.is_empty());
            let stdout = decoded(&output.stdout);
            assert!(stdout.contains(expected), "{locale} {command}: {stdout}");
            assert!(stdout.contains(&format!("icbmailer {command}")), "{stdout}");
            assert!(stdout.contains("--help"), "{stdout}");
            assert!(stdout.contains(if command == "show" { "<file>" } else { "<config>" }), "{stdout}");
            if locale == "de_DE" {
                assert!(
                    !stdout.contains("Usage:") && !stdout.contains("Options:") && !stdout.contains("Arguments:"),
                    "{stdout}"
                );
            }
        }
    }
    let help = decoded(&run("de_DE", &["qwk-poll", "--help"]).stdout);
    assert!(help.contains("[hub]"), "{help}");
    let help = decoded(&run("de_DE", &["poll", "--help"]).stdout);
    assert!(help.contains("--keep") && help.contains("--verbose") && help.contains("[address]"), "{help}");
}

#[test]
fn parser_errors_are_localized_on_stderr_with_exit_one() {
    for locale in ["en_US", "de_DE"] {
        for args in [
            vec!["--not-an-option"],
            vec!["qwk-poll"],
            vec!["poll", "board.toml", "--not-an-option"],
            vec!["qwk_poll"],
        ] {
            let output = run(locale, &args);
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            let stderr = decoded(&output.stderr);
            assert!(stderr.contains("--help"), "{stderr}");
            let prefix = if locale == "de_DE" { "fehler" } else { "error" };
            assert!(stderr.to_lowercase().contains(prefix), "{stderr}");
        }
    }
}

#[test]
fn version_remains_manual_and_locale_independent() {
    for locale in ["en_US", "de_DE"] {
        let output = run(locale, &["--version"]);
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());
        assert_eq!(decoded(&output.stdout), format!("icbmailer {}\n", env!("CARGO_PKG_VERSION")));
        let short = run(locale, &["-V"]);
        assert_eq!(short.status.code(), Some(1));
        assert!(short.stdout.is_empty());
    }
}
