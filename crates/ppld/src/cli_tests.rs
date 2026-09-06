use super::Cli;

fn parse(args: &[&str]) -> Cli {
    icy_board_cli::try_parse_from::<Cli, _, _>(std::iter::once("ppld").chain(args.iter().copied())).unwrap()
}

#[test]
fn cli_defaults_preserve_decompiler_settings() {
    let cli = parse(&[]);
    assert!(!cli.raw && !cli.disassemble && !cli.output && !cli.check && !cli.cp437 && !cli.version);
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
        "--cp437",
        "--style",
        "l",
        "--lang-version",
        "350",
        "--version",
        "source.ppe",
    ]);
    assert!(cli.raw && cli.disassemble && cli.output && cli.check && cli.cp437 && cli.version);
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
}
