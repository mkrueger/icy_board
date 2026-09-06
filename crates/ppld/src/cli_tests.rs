use super::Cli;

fn parse(args: &[&str]) -> Cli {
    icy_board_cli::try_parse_from::<Cli, _, _>(std::iter::once("ppld").chain(args.iter().copied())).unwrap()
}

#[test]
fn cli_defaults_preserve_decompiler_settings() {
    let cli = parse(&[]);
    assert!(!cli.raw && !cli.disassemble && !cli.output && !cli.check && !cli.strict && !cli.cp437 && !cli.version);
    assert_eq!(cli.style, None);
    assert_eq!(cli.lang_version, None);
    assert_eq!(cli.file, None);
}

#[test]
fn cli_accepts_all_existing_options_and_short_aliases() {
    let cli = parse(&[
        "-r",
        "-d",
        "-o",
        "--check",
        "--strict",
        "--cp437",
        "--style",
        "l",
        "--lang-version",
        "350",
        "--version",
        "source.ppe",
    ]);
    assert!(cli.raw && cli.disassemble && cli.output && cli.check && cli.strict && cli.cp437 && cli.version);
    assert_eq!(cli.style, Some('l'));
    assert_eq!(cli.lang_version, Some(350));
    assert_eq!(cli.file.as_deref(), Some("source.ppe"));
    let cli = parse(&["--raw", "--disassemble", "--output"]);
    assert!(cli.raw && cli.disassemble && cli.output);
}

#[test]
fn cli_boolean_switches_can_repeat() {
    let cli = parse(&["--raw", "-r", "-o", "--output", "--cp437", "--cp437", "--version", "--version"]);
    assert!(cli.raw && cli.output && cli.cp437 && cli.version);
}

#[test]
fn cli_keeps_language_and_style_validation_in_the_decompiler() {
    let cli = parse(&["--lang-version", "999", "--style", "x"]);
    assert_eq!(cli.lang_version, Some(999));
    assert_eq!(cli.style, Some('x'));
    for args in [
        vec!["ppld", "--lang-version", "text"],
        vec!["ppld", "--style", "upper"],
        vec!["ppld", "--style", "u", "--style", "l"],
    ] {
        assert!(icy_board_cli::try_parse_from::<Cli, _, _>(args).is_err());
    }
}

#[test]
fn cli_help_is_a_successful_early_exit() {
    let error = icy_board_cli::try_parse_from::<Cli, _, _>(["ppld", "--help"]).err().unwrap();
    assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
    assert!(!error.use_stderr());
    let help = icy_board_cli::command::<Cli>().render_help().to_string();
    assert!(help.contains("--style <style>"), "{help}");
    assert!(help.contains("--lang-version <lang-version>"), "{help}");
    assert!(help.contains("[file]"), "{help}");
    assert!(help.contains("--strict"), "{help}");
}

#[test]
fn cli_strict_requires_check_and_allows_repeated_switches() {
    let error = icy_board_cli::try_parse_from::<Cli, _, _>(["ppld", "--strict", "source.ppe"]).err().unwrap();
    assert_eq!(error.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    assert!(error.use_stderr());
    for args in [vec!["--strict", "--check"], vec!["--check", "--strict", "--check", "--strict"]] {
        let cli = parse(&args);
        assert!(cli.check && cli.strict);
    }
}

#[test]
fn cli_compatibility_summary_distinguishes_findings_from_errors() {
    use crate::compat_check::{CompatibilitySummary, check_compatibility};
    use icy_board_engine::executable::{Executable, OpCode};

    for (fixture, expected) in [
        ("beep", CompatibilitySummary::default()),
        (
            "dointr",
            CompatibilitySummary {
                unimplemented: 1,
                ..Default::default()
            },
        ),
        (
            "sound",
            CompatibilitySummary {
                unsupported: 1,
                ..Default::default()
            },
        ),
    ] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("test_data/{fixture}.ppe"));
        let executable = Executable::read_file(&path, false).unwrap();
        let report = check_compatibility(&executable).unwrap();
        assert_eq!(report.summary, expected, "{fixture}");
        assert_eq!(report.summary.has_findings(), fixture != "beep");
    }
    assert!(
        CompatibilitySummary {
            partial: 1,
            ..Default::default()
        }
        .has_findings()
    );

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/beep.ppe");
    let mut executable = Executable::read_file(&path, false).unwrap();
    executable.script_buffer = vec![OpCode::LET as i16];
    let error = check_compatibility(&executable).err().expect("truncated statement must fail analysis");
    assert!(error.to_string().contains("Failed to deserialize PPE"));
}

#[test]
fn cli_compatibility_report_is_deterministic_and_propagates_writer_errors() {
    use crate::compat_check::{CompatibilitySummary, check_compatibility};
    use icy_board_engine::executable::{Executable, FuncOpCode, OpCode, PPECommand, PPEExpr, PPEScript};
    use std::io::{self, Write};

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/dointr.ppe");
    let mut executable = Executable::read_file(&path, false).unwrap();
    let script = PPEScript::from_ppe_file(&executable).unwrap();
    let PPECommand::PredefinedCall(_, args) = &script.statements[0].command else {
        panic!("expected DOINTR")
    };
    executable.script_buffer.clear();
    // Put partial support first in source order, with repeated nested functions
    // at one offset. Reports group categories but must retain all references.
    PPECommand::PredefinedCall(OpCode::DLOCK.get_definition(), vec![args[0].clone()]).serialize(&mut executable.script_buffer);
    PPECommand::PredefinedCall(
        OpCode::PRINTLN.get_definition(),
        vec![
            PPEExpr::PredefinedFunctionCall(FuncOpCode::MODEM.get_definition(), vec![]),
            PPEExpr::PredefinedFunctionCall(FuncOpCode::REGAX.get_definition(), vec![]),
            PPEExpr::PredefinedFunctionCall(FuncOpCode::REGAX.get_definition(), vec![]),
        ],
    )
    .serialize(&mut executable.script_buffer);
    let expected = CompatibilitySummary {
        unimplemented: 2,
        unsupported: 1,
        partial: 1,
    };
    let report = check_compatibility(&executable).unwrap();
    assert_eq!(report.summary, expected);
    assert_eq!(report.summary.total(), 4);
    let mut first = Vec::new();
    report.write_report(&mut first).unwrap();
    let mut second = Vec::new();
    check_compatibility(&executable).unwrap().write_report(&mut second).unwrap();
    assert_eq!(first, second);
    let text = String::from_utf8(first).unwrap();
    assert_eq!(text.matches("FUNCTION REGAX").count(), 2, "{text}");
    assert!(text.find("Unimplemented:").unwrap() < text.find("Unsupported (stubbed):").unwrap(), "{text}");
    assert!(
        text.find("Unsupported (stubbed):").unwrap() < text.find("Partially Implemented:").unwrap(),
        "{text}"
    );

    struct FlushError;
    impl Write for FlushError {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed report stream"))
        }
    }
    assert_eq!(report.write_report(FlushError).unwrap_err().kind(), io::ErrorKind::BrokenPipe);
    assert_eq!(report.summary, expected, "report I/O must not change the analysis result");
}
