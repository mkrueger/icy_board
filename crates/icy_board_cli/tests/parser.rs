use clap::{Args, Parser, Subcommand, error::ErrorKind};
use icy_board_cli::try_parse_from;

#[derive(Debug, Parser)]
#[command(name = "probe", disable_version_flag = true, subcommand_precedence_over_arg = true)]
struct Cli {
    #[arg(long)]
    version: bool,
    #[arg(long)]
    value: Option<String>,
    #[arg(long)]
    number: Option<i32>,
    #[arg(long)]
    map: Vec<String>,
    #[arg(long, action = clap::ArgAction::Set, num_args = 0, default_missing_value = "true", overrides_with = "cp437")]
    cp437: Option<bool>,
    file: Option<std::path::PathBuf>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Import(Import),
}

#[derive(Debug, Args)]
struct Import {
    #[arg(long)]
    force: bool,
    source: String,
}

fn parse(arguments: &[&str]) -> Result<Cli, clap::error::Error<clap_i18n_richformatter::ClapI18nRichFormatter>> {
    try_parse_from(arguments)
}

#[test]
fn defaults_and_switch_presence_are_preserved() {
    let cli = parse(&["probe"]).unwrap();
    assert!(!cli.version);
    assert_eq!(cli.cp437, None);
    let cli = parse(&["probe", "--version", "--version", "--cp437", "--cp437"]).unwrap();
    assert!(cli.version);
    assert_eq!(cli.cp437, Some(true));
}

#[test]
fn scalar_options_reject_duplicates_but_vectors_accumulate() {
    assert!(parse(&["probe", "--value", "a", "--value", "b"]).is_err());
    assert_eq!(parse(&["probe", "--map", "a", "--map", "b"]).unwrap().map, ["a", "b"]);
}

#[test]
fn option_values_and_literal_filenames_are_not_help() {
    assert_eq!(parse(&["probe", "--value", "help"]).unwrap().value.as_deref(), Some("help"));
    assert_eq!(parse(&["probe", "--value", "--help"]).unwrap().value.as_deref(), Some("--help"));
    assert_eq!(parse(&["probe", "--", "help"]).unwrap().file.unwrap().to_str(), Some("help"));
    assert_eq!(parse(&["probe", "--number", "-5"]).unwrap().number, Some(-5));
}

#[test]
fn legacy_help_forms_select_the_right_command() {
    for arguments in [vec!["probe", "help"], vec!["probe", "--help"], vec!["probe", "-h"]] {
        let error = parse(&arguments).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::DisplayHelp);
        assert!(!error.use_stderr());
        assert!(error.to_string().contains("--value"));
    }
    for arguments in [
        vec!["probe", "help", "import"],
        vec!["probe", "--help", "import"],
        vec!["probe", "import", "help"],
        vec!["probe", "import", "--help"],
    ] {
        let error = parse(&arguments).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::DisplayHelp);
        let help = error.to_string();
        assert!(help.contains("probe import") && help.contains("--force"), "{help}");
        assert!(!help.contains("--value"), "{help}");
    }
}

#[test]
fn subcommands_win_over_optional_positionals() {
    let cli = parse(&["probe", "import", "--force", "--force", "source"]).unwrap();
    let Some(Commands::Import(import)) = cli.command else {
        panic!("missing import")
    };
    assert!(import.force);
    assert_eq!(import.source, "source");
}

#[test]
fn rich_errors_are_printable_without_bidi_or_color_in_text() {
    for arguments in [vec!["probe", "--unknown"], vec!["probe", "--number", "invalid"], vec!["probe", "import"]] {
        let error = parse(&arguments).unwrap_err();
        assert!(error.use_stderr());
        let text = error.to_string();
        assert!(text.contains("--help"), "{text}");
        assert!(!text.contains(['\u{2068}', '\u{2069}', '\x1b']), "{text}");
    }
}
