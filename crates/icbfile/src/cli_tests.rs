use super::*;

fn parse(args: &[&str]) -> Cli {
    icy_board_cli::try_parse_from::<Cli, _, _>(std::iter::once("icbfile").chain(args.iter().copied())).unwrap_or_else(|err| panic!("{err}"))
}

#[test]
fn cli_definition_and_root_defaults() {
    icy_board_cli::command::<Cli>().debug_assert();
    let cli = parse(&[]);
    assert!(!cli.version);
    assert!(cli.command.is_none());
    assert!(parse(&["--version", "--version"]).version);
    assert!(icy_board_cli::try_parse_from::<Cli, _, _>(["icbfile", "-V"]).is_err());
    assert!(icy_board_cli::try_parse_from::<Cli, _, _>(["icbfile", "list", "files", "--version"]).is_err());
}

#[test]
fn cli_subcommands_keep_their_names_positionals_and_short_options() {
    let Some(Command::Areas(cmd)) = parse(&["areas", "areas.toml"]).command else {
        panic!()
    };
    assert_eq!(cmd.areas, Path::new("areas.toml"));
    let Some(Command::List(cmd)) = parse(&["list", "areas.toml", "-a", "Games", "-l", "--long"]).command else {
        panic!()
    };
    assert_eq!(cmd.target, Path::new("areas.toml"));
    assert_eq!(cmd.area.as_deref(), Some("Games"));
    assert!(cmd.long);
    let Some(Command::Scan(cmd)) = parse(&["scan", "areas.toml", "--all", "-f"]).command else {
        panic!()
    };
    assert_eq!(cmd.target, Path::new("areas.toml"));
    assert!(cmd.all && cmd.force);
    assert!(cmd.area.is_none());
    let Some(Command::Check(cmd)) = parse(&["check", "areas.toml", "-a", "0", "--prune"]).command else {
        panic!()
    };
    assert_eq!(cmd.area.as_deref(), Some("0"));
    assert!(cmd.prune);
    let Some(Command::Export(cmd)) = parse(&["export", "areas.toml", "-a", "Games", "-o", "FILES.BBS"]).command else {
        panic!()
    };
    assert_eq!(cmd.output.as_deref(), Some(Path::new("FILES.BBS")));
    assert_eq!(cmd.area.as_deref(), Some("Games"));
    let Some(Command::Set(cmd)) = parse(&["set", "files", "TEST.ZIP", "-d", "new description", "--free", "true", "--locked", "false"]).command else {
        panic!()
    };
    assert_eq!(cmd.file, "TEST.ZIP");
    assert_eq!(cmd.desc.as_deref(), Some("new description"));
    assert_eq!(cmd.free, Some(true));
    assert_eq!(cmd.locked, Some(false));
    let Some(Command::Fingerprints(cmd)) = parse(&["fingerprints", "intros"]).command else {
        panic!()
    };
    assert_eq!(cmd.input, Path::new("intros"));
    assert_eq!(cmd.output, Path::new("bbstros.toml"));
    let Some(Command::Fingerprints(cmd)) = parse(&["fingerprints", "intros", "-o", "custom.toml"]).command else {
        panic!()
    };
    assert_eq!(cmd.output, Path::new("custom.toml"));
}

#[test]
fn cli_import_preserves_optional_listings_and_all_format_synonyms() {
    let Some(Command::Import(cmd)) = parse(&["import", "files"]).command else {
        panic!()
    };
    assert!(cmd.listings.is_empty());
    assert_eq!(cmd.format, Format::Auto);
    assert!(!cmd.dry_run && !cmd.overwrite && !cmd.keep_missing);
    for (value, expected) in [
        ("auto", Format::Auto),
        ("AUTO", Format::Auto),
        ("pcboard", Format::PcBoard),
        ("PcBoard", Format::PcBoard),
        ("DIR", Format::PcBoard),
        ("filesbbs", Format::FilesBbs),
        ("FILES.BBS", Format::FilesBbs),
        ("bbs", Format::FilesBbs),
    ] {
        let Some(Command::Import(cmd)) = parse(&["import", "files", "one.dir", "two.bbs", "-f", value, "-n", "--overwrite", "--keep-missing"]).command else {
            panic!()
        };
        assert_eq!(cmd.format, expected);
        assert_eq!(cmd.listings, [PathBuf::from("one.dir"), PathBuf::from("two.bbs")]);
        assert!(cmd.dry_run && cmd.overwrite && cmd.keep_missing);
    }
}

#[test]
fn cli_repack_preserves_every_limit_and_accepts_negative_compression() {
    let Some(Command::Repack(cmd)) = parse(&["repack", "files"]).command else {
        panic!()
    };
    assert_eq!(cmd.compression_level, 9);
    assert_eq!(cmd.max_members, 10_000);
    assert_eq!(cmd.max_member_size, 512 * 1024 * 1024);
    assert_eq!(cmd.max_expanded_size, 2 * 1024 * 1024 * 1024);
    assert_eq!(cmd.max_compression_ratio, 1_000);
    assert!(!cmd.dry_run && !cmd.keep_case);
    assert!(cmd.area.is_none() && cmd.fingerprints.is_none());
    let Some(Command::Repack(cmd)) = parse(&[
        "repack",
        "areas.toml",
        "-a",
        "Games",
        "-p",
        "rules.toml",
        "-n",
        "--keep-case",
        "--compression-level",
        "-1",
        "--max-members",
        "3",
        "--max-member-size",
        "1024",
        "--max-expanded-size",
        "2048",
        "--max-compression-ratio",
        "5",
    ])
    .command
    else {
        panic!()
    };
    assert_eq!(cmd.area.as_deref(), Some("Games"));
    assert_eq!(cmd.fingerprints.as_deref(), Some(Path::new("rules.toml")));
    assert!(cmd.dry_run && cmd.keep_case);
    assert_eq!(
        (
            cmd.compression_level,
            cmd.max_members,
            cmd.max_member_size,
            cmd.max_expanded_size,
            cmd.max_compression_ratio
        ),
        (-1, 3, 1024, 2048, 5)
    );
}

#[test]
fn cli_option_values_may_start_with_hyphens_but_positionals_need_separator() {
    let Some(Command::Set(cmd)) = parse(&["set", "files", "test.zip", "-d", "--not-a-flag", "-a", "-1"]).command else {
        panic!()
    };
    assert_eq!(cmd.desc.as_deref(), Some("--not-a-flag"));
    assert_eq!(cmd.area.as_deref(), Some("-1"));
    assert!(icy_board_cli::try_parse_from::<Cli, _, _>(["icbfile", "list", "-files"]).is_err());
    let Some(Command::List(cmd)) = parse(&["list", "--", "-files"]).command else {
        panic!()
    };
    assert_eq!(cmd.target, Path::new("-files"));
}

#[test]
fn cli_rejects_missing_values_unknown_commands_and_duplicate_options() {
    for args in [
        vec!["icbfile", "areas"],
        vec!["icbfile", "repack", "files", "--max-members", "-1"],
        vec!["icbfile", "import", "files", "--format", "unknown"],
        vec!["icbfile", "set", "files", "test.zip", "--free"],
        vec!["icbfile", "set", "files", "test.zip", "--locked", "yes"],
        vec!["icbfile", "list", "files", "-a", "0", "--area", "1"],
        vec!["icbfile", "unknown"],
    ] {
        assert!(icy_board_cli::try_parse_from::<Cli, _, _>(args.clone()).is_err(), "accepted {args:?}");
    }
}

#[cfg(unix)]
#[test]
fn cli_paths_preserve_non_utf8_bytes_in_positionals_lists_and_options() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let path = OsString::from_vec(b"files-\xff".to_vec());
    let cli =
        icy_board_cli::try_parse_from::<Cli, _, _>([OsString::from("icbfile"), "import".into(), path.clone(), path.clone()]).unwrap_or_else(|e| panic!("{e}"));
    let Some(Command::Import(cmd)) = cli.command else { panic!() };
    assert_eq!(cmd.target.as_os_str(), path);
    assert_eq!(cmd.listings[0].as_os_str(), path);
    let cli = icy_board_cli::try_parse_from::<Cli, _, _>([OsString::from("icbfile"), "export".into(), "files".into(), "-o".into(), path.clone()])
        .unwrap_or_else(|e| panic!("{e}"));
    let Some(Command::Export(cmd)) = cli.command else { panic!() };
    assert_eq!(cmd.output.unwrap().as_os_str(), path);
}
