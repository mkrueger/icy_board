use std::process::Command;

fn ppld(language: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ppld"));
    command
        .env("LANG", language)
        .env("LC_ALL", language)
        .env("LANGUAGE", language)
        .env("NO_COLOR", "1")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("PPL_LANG_VERSION");
    command
}

#[test]
fn help_is_localized_and_successful_without_a_decompiler_banner() {
    for (language, about, description) in [
        (
            "en_US.UTF-8",
            "PCBoard Programming Language Decompiler",
            "output the disassembly instead of ppl",
        ),
        (
            "de_DE.UTF-8",
            "Decompiler für die PCBoard-Programmiersprache",
            "Disassemblierung statt PPL ausgeben",
        ),
    ] {
        let output = ppld(language).arg("--help").output().unwrap();
        let help = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(output.stderr.is_empty());
        assert!(help.contains(about) && help.contains(description), "{help}");
        for flag in [
            "--raw",
            "--disassemble",
            "--output",
            "--check",
            "--strict",
            "--cp437",
            "--style <style>",
            "--lang-version <lang-version>",
            "--version",
            "--help",
            "[file]",
        ] {
            assert!(help.contains(flag), "missing {flag}: {help}");
        }
        assert!(!help.contains("PPLD v"), "{help}");
    }
}

#[test]
fn invalid_arguments_are_localized_on_stderr_and_exit_one() {
    let mut errors = Vec::new();
    for language in ["en_US.UTF-8", "de_DE.UTF-8"] {
        let output = ppld(language).arg("--not-an-option").output().unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert_eq!(output.status.code(), Some(1), "{stderr}");
        assert!(output.stdout.is_empty());
        assert!(stderr.contains("--not-an-option"), "{stderr}");
        errors.push(stderr);
    }
    assert_ne!(errors[0], errors[1], "parse errors must be localized");
}

#[test]
fn version_output_stays_manual_and_locale_independent() {
    for language in ["en_US.UTF-8", "de_DE.UTF-8"] {
        for args in [vec!["--version", "--version"], vec!["--output", "--version"]] {
            let output = ppld(language).args(args).output().unwrap();
            assert!(output.status.success());
            assert!(output.stderr.is_empty());
            assert_eq!(
                String::from_utf8(output.stdout).unwrap(),
                format!("{}\n", icy_board_cli::version_line("ppld", env!("CARGO_PKG_VERSION"), env!("GIT_HASH")))
            );
        }
    }
}

#[test]
fn missing_input_keeps_the_banner_and_localized_help_on_stderr() {
    let output = ppld("de_DE.UTF-8").output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("PPLD v"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Decompiler für die PCBoard-Programmiersprache"));
}

#[test]
fn strict_requirement_errors_are_localized_and_exit_one() {
    let mut errors = Vec::new();
    for language in ["en_US.UTF-8", "de_DE.UTF-8"] {
        let output = ppld(language).args(["--strict", "--output", "input.ppe"]).output().unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert_eq!(output.status.code(), Some(1), "{stderr}");
        assert!(output.stdout.is_empty());
        assert!(stderr.contains("--check"), "{stderr}");
        assert!(
            !stderr.contains("PPLD v") && !stderr.contains("Can't read"),
            "validate before opening the input: {stderr}"
        );
        errors.push(stderr);
    }
    assert_ne!(errors[0], errors[1], "missing --check must use the localized parser diagnostics");
}

#[test]
fn strict_exit_policy_and_output_streams_are_documented_in_both_languages() {
    for (language, phrases) in [
        (
            "en_US.UTF-8",
            vec![
                "findings exit 0 unless --strict is used",
                "requires --check",
                "exit 1 for any unsupported, unimplemented or partially implemented reference, otherwise 0",
                "errors still exit 1",
                "source to stdout",
                "warnings go to stderr",
            ],
        ),
        (
            "de_DE.UTF-8",
            vec![
                "Funde liefern Exit-Code 0, außer mit --strict",
                "erfordert --check",
                "Exit-Code 1 bei jeder nicht unterstützten, nicht implementierten oder teilweise implementierten Referenz, sonst 0",
                "Fehler liefern weiterhin Exit-Code 1",
                "Quelltext auf stdout",
                "Warnungen gehen an stderr",
            ],
        ),
    ] {
        // Help still succeeds on stdout, even with --output and an otherwise
        // incomplete --strict invocation, before banner or validation logic.
        let output = ppld(language).args(["--output", "--strict", "--help"]).output().unwrap();
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let help = String::from_utf8(output.stdout).unwrap().split_whitespace().collect::<Vec<_>>().join(" ");
        for phrase in phrases {
            assert!(help.contains(phrase), "missing {phrase}: {help}");
        }
        assert!(!help.contains("PPLD v"));
    }
}
