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
        let output = ppld(language).args(["--version", "--version"]).output().unwrap();
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert_eq!(String::from_utf8(output.stdout).unwrap(), format!("ppld {}\n", env!("CARGO_PKG_VERSION")));
    }
}

#[test]
fn missing_input_keeps_the_banner_and_localized_help_on_stderr() {
    let output = ppld("de_DE.UTF-8").output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("PPLD v"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Decompiler für die PCBoard-Programmiersprache"));
}
