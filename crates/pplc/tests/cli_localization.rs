use std::process::Command;

fn pplc(language: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_pplc"));
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
fn help_is_localized_and_successful_without_a_compiler_banner() {
    for (language, about, description) in [
        ("en_US.UTF-8", "PCBoard Programming Language Compiler", "don't report any warnings"),
        ("de_DE.UTF-8", "Compiler für die PCBoard-Programmiersprache", "keine Warnungen ausgeben"),
    ] {
        let output = pplc(language).arg("--help").output().unwrap();
        let help = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(output.stderr.is_empty());
        assert!(help.contains(about) && help.contains(description), "{help}");
        for flag in [
            "--disassemble",
            "--nowarnings",
            "--version",
            "--mono",
            "--runtime <runtime>",
            "--lang-version <lang-version>",
            "--cp437",
            "--init",
            "--defines <defines>",
            "--format",
            "--stdout",
            "--check",
            "--print-config",
            "--print-config-json",
            "--help",
            "[file]",
        ] {
            assert!(help.contains(flag), "missing {flag}: {help}");
        }
        assert!(!help.contains("PPLC v"), "{help}");
    }
}

#[test]
fn invalid_arguments_are_localized_on_stderr_and_exit_one() {
    let mut errors = Vec::new();
    for language in ["en_US.UTF-8", "de_DE.UTF-8"] {
        let output = pplc(language).arg("--not-an-option").output().unwrap();
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
        let output = pplc(language).args(["--version", "--version"]).output().unwrap();
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert_eq!(String::from_utf8(output.stdout).unwrap(), format!("pplc {}\n", env!("CARGO_PKG_VERSION")));
    }
}

#[test]
fn missing_input_keeps_the_banner_and_localized_help_on_stderr() {
    let dir = tempfile::tempdir().unwrap();
    let output = pplc("de_DE.UTF-8").current_dir(dir.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("PPLC v"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Compiler für die PCBoard-Programmiersprache"));
}

#[test]
fn json_configuration_keeps_encoding_defaults_and_is_locale_independent() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.pps");
    std::fs::write(&source, "PRINTLN 1\n").unwrap();
    for (switches, encoding) in [(vec![], "detect"), (vec!["--cp437"], "cp437")] {
        let mut reports = Vec::new();
        for language in ["en_US.UTF-8", "de_DE.UTF-8"] {
            let output = pplc(language).arg("--print-config-json").args(&switches).arg(&source).output().unwrap();
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
            assert!(output.stderr.is_empty());
            let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(report["encoding"], encoding);
            assert_eq!(report["runtimeVersion"]["effective"], 400);
            reports.push(output.stdout);
        }
        assert_eq!(reports[0], reports[1]);
    }
    assert!(!source.with_extension("ppe").exists());
}
