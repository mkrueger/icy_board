use super::Cli;

fn parse(args: &[&str]) -> Cli {
    icy_board_cli::try_parse_from::<Cli, _, _>(std::iter::once("pplc").chain(args.iter().copied())).unwrap()
}

#[test]
fn cli_defaults_keep_encoding_autodetection_and_optional_versions() {
    let cli = parse(&[]);
    assert!(!cli.disassemble && !cli.nowarnings && !cli.version && !cli.mono);
    assert!(!cli.init && !cli.format && !cli.stdout && !cli.check);
    assert!(!cli.print_config && !cli.print_config_json);
    assert_eq!(cli.cp437, None);
    assert_eq!(cli.runtime, None);
    assert_eq!(cli.lang_version, None);
    assert_eq!(cli.defines, None);
    assert_eq!(cli.file, None);
}

#[test]
fn cli_accepts_all_existing_options_without_changing_values() {
    let cli = parse(&[
        "-d",
        "--nowarnings",
        "--version",
        "--mono",
        "--runtime",
        "340",
        "--lang-version",
        "400",
        "--cp437",
        "--init",
        "--defines",
        "DEBUG;TEST=1",
        "--format",
        "--stdout",
        "--check",
        "--print-config",
        "--print-config-json",
        "source.pps",
    ]);
    assert!(cli.disassemble && cli.nowarnings && cli.version && cli.mono);
    assert!(cli.init && cli.format && cli.stdout && cli.check);
    assert!(cli.print_config && cli.print_config_json);
    assert_eq!(cli.cp437, Some(true));
    assert_eq!(cli.runtime, Some(340));
    assert_eq!(cli.lang_version, Some(400));
    assert_eq!(cli.defines.as_deref(), Some("DEBUG;TEST=1"));
    assert_eq!(cli.file.as_deref(), Some(std::path::Path::new("source.pps")));
    assert!(parse(&["--disassemble"]).disassemble);
}

#[test]
fn cli_switches_can_repeat_and_cp437_never_consumes_the_file() {
    let cli = parse(&["--cp437", "--cp437", "--mono", "--mono", "-d", "-d", "--version", "--version", "source.pps"]);
    assert_eq!(cli.cp437, Some(true));
    assert!(cli.mono && cli.disassemble && cli.version);
    assert_eq!(cli.file.as_deref(), Some(std::path::Path::new("source.pps")));
    assert!(icy_board_cli::try_parse_from::<Cli, _, _>(["pplc", "--cp437=false"]).is_err());
}

#[test]
fn cli_keeps_runtime_validation_in_the_compiler() {
    assert_eq!(parse(&["--runtime", "999", "--lang-version", "999"]).runtime, Some(999));
    for args in [
        vec!["pplc", "--runtime", "text"],
        vec!["pplc", "--lang-version"],
        vec!["pplc", "--runtime", "340", "--runtime", "400"],
    ] {
        assert!(icy_board_cli::try_parse_from::<Cli, _, _>(args).is_err());
    }
}

#[test]
fn cli_help_is_a_successful_early_exit() {
    let error = icy_board_cli::try_parse_from::<Cli, _, _>(["pplc", "--help"]).err().unwrap();
    assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
    assert!(!error.use_stderr());
    let help = icy_board_cli::command::<Cli>().render_help().to_string();
    assert!(help.contains("--runtime <runtime>"), "{help}");
    assert!(help.contains("--lang-version <lang-version>"), "{help}");
    assert!(help.contains("[file]"), "{help}");
}

#[test]
fn cli_compression_is_explicit_and_debug_is_optional() {
    let defaults = parse(&[]);
    assert_eq!("none", defaults.compression);
    assert!(!defaults.debug);
    let options = parse(&["--compression", "zstd", "--debug", "source.pps"]);
    assert_eq!("zstd", options.compression);
    assert!(options.debug);
    assert!(icy_board_cli::try_parse_from::<Cli, _, _>(["pplc", "--compression", "rle"]).is_err());
}
