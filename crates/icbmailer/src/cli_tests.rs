use super::*;

fn parse(args: &[&str]) -> Cli {
    icy_board_cli::try_parse_from::<Cli, _, _>(std::iter::once("icbmailer").chain(args.iter().copied())).unwrap_or_else(|err| panic!("{err}"))
}

#[test]
fn cli_definition_and_root_defaults() {
    icy_board_cli::command::<Cli>().debug_assert();
    let cli = parse(&[]);
    assert!(!cli.version);
    assert!(cli.command.is_none());
    assert!(parse(&["--version", "--version"]).version);
    assert!(icy_board_cli::try_parse_from::<Cli, _, _>(["icbmailer", "-V"]).is_err());
    assert!(icy_board_cli::try_parse_from::<Cli, _, _>(["icbmailer", "links", "board.toml", "--version"]).is_err());
}

#[test]
fn cli_ftn_subcommands_preserve_positionals_defaults_and_short_flags() {
    let Some(Command::Links(cmd)) = parse(&["links", "board.toml"]).command else {
        panic!()
    };
    assert_eq!(cmd.config, Path::new("board.toml"));
    let Some(Command::Poll(cmd)) = parse(&["poll", "board.toml"]).command else {
        panic!()
    };
    assert_eq!(cmd.config, Path::new("board.toml"));
    assert!(cmd.address.is_none());
    assert!(!cmd.keep && !cmd.verbose);
    let Some(Command::Poll(cmd)) = parse(&["poll", "board.toml", "2:240/100", "-k", "-v", "--verbose"]).command else {
        panic!()
    };
    assert_eq!(cmd.address.as_deref(), Some("2:240/100"));
    assert!(cmd.keep && cmd.verbose);
    let Some(Command::Show(cmd)) = parse(&["show", "mail.pkt", "-t"]).command else {
        panic!()
    };
    assert_eq!(cmd.file, Path::new("mail.pkt"));
    assert!(cmd.text);
    let Some(Command::Show(cmd)) = parse(&["show", "mail.pkt"]).command else {
        panic!()
    };
    assert!(!cmd.text);
    for flag in ["-v", "--verbose"] {
        let Some(Command::Scan(cmd)) = parse(&["scan", "board.toml", flag]).command else {
            panic!()
        };
        assert_eq!(cmd.config, Path::new("board.toml"));
        assert!(cmd.verbose);
        let Some(Command::Toss(cmd)) = parse(&["toss", "board.toml", flag]).command else {
            panic!()
        };
        assert_eq!(cmd.config, Path::new("board.toml"));
        assert!(cmd.verbose);
    }
    let Some(Command::Scan(cmd)) = parse(&["scan", "board.toml"]).command else {
        panic!()
    };
    assert!(!cmd.verbose);
    let Some(Command::Toss(cmd)) = parse(&["toss", "board.toml"]).command else {
        panic!()
    };
    assert!(!cmd.verbose);
}

#[test]
fn cli_qwk_command_names_and_optional_hubs_are_unchanged() {
    let Some(Command::QwkLinks(cmd)) = parse(&["qwk-links", "board.toml"]).command else {
        panic!()
    };
    assert_eq!(cmd.config, Path::new("board.toml"));
    for command in ["qwk-poll", "qwk-scan", "qwk-toss"] {
        for hub in [None, Some("MY-HUB")] {
            let mut args = vec![command, "board.toml"];
            args.extend(hub);
            let (config, actual_hub) = match parse(&args).command.unwrap() {
                Command::QwkPoll(cmd) => (cmd.config, cmd.hub),
                Command::QwkScan(cmd) => (cmd.config, cmd.hub),
                Command::QwkToss(cmd) => (cmd.config, cmd.hub),
                _ => panic!(),
            };
            assert_eq!(config, Path::new("board.toml"));
            assert_eq!(actual_hub.as_deref(), hub);
        }
    }
}

#[test]
fn cli_missing_positionals_unknown_names_and_flags_are_rejected() {
    for command in ["links", "poll", "scan", "show", "toss", "qwk-links", "qwk-poll", "qwk-scan", "qwk-toss"] {
        assert!(icy_board_cli::try_parse_from::<Cli, _, _>(["icbmailer", command]).is_err());
    }
    for args in [
        vec!["icbmailer", "qwk_poll", "board.toml"],
        vec!["icbmailer", "poll", "board.toml", "--unknown"],
        vec!["icbmailer", "qwk-links", "board.toml", "unexpected-hub"],
        vec!["icbmailer", "poll", "board.toml", "-1"],
    ] {
        assert!(icy_board_cli::try_parse_from::<Cli, _, _>(args.clone()).is_err(), "accepted {args:?}");
    }
    let Some(Command::Poll(cmd)) = parse(&["poll", "board.toml", "--", "-1"]).command else {
        panic!()
    };
    assert_eq!(cmd.address.as_deref(), Some("-1"));
}

#[cfg(unix)]
#[test]
fn cli_non_utf8_config_and_packet_paths_are_preserved() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let path = OsString::from_vec(b"mail-\xff".to_vec());
    for name in ["links", "poll", "scan", "show", "toss", "qwk-links", "qwk-poll", "qwk-scan", "qwk-toss"] {
        let cli = icy_board_cli::try_parse_from::<Cli, _, _>([OsString::from("icbmailer"), name.into(), path.clone()]).unwrap_or_else(|e| panic!("{e}"));
        let actual = match cli.command.unwrap() {
            Command::Links(cmd) => cmd.config,
            Command::Poll(cmd) => cmd.config,
            Command::Scan(cmd) => cmd.config,
            Command::Show(cmd) => cmd.file,
            Command::Toss(cmd) => cmd.config,
            Command::QwkLinks(cmd) => cmd.config,
            Command::QwkPoll(cmd) => cmd.config,
            Command::QwkScan(cmd) => cmd.config,
            Command::QwkToss(cmd) => cmd.config,
        };
        assert_eq!(actual.as_os_str(), path, "{name}");
    }
}
