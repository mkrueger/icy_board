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
    for command in [
        "links",
        "poll",
        "scan",
        "show",
        "toss",
        "qwk-links",
        "qwk-poll",
        "qwk-scan",
        "qwk-toss",
        "zconnect-links",
        "zconnect-poll",
        "zconnect-scan",
        "zconnect-toss",
        "zconnect-ack",
    ] {
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
    for name in [
        "links",
        "poll",
        "scan",
        "show",
        "toss",
        "qwk-links",
        "qwk-poll",
        "qwk-scan",
        "qwk-toss",
        "zconnect-links",
        "zconnect-poll",
        "zconnect-scan",
        "zconnect-toss",
        "zconnect-ack",
    ] {
        let mut args = vec![OsString::from("icbmailer"), name.into(), path.clone()];
        if name == "zconnect-ack" {
            args.push("PEER".into());
        }
        let cli = icy_board_cli::try_parse_from::<Cli, _, _>(args).unwrap_or_else(|e| panic!("{e}"));
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
            Command::ZconnectLinks(cmd) => cmd.config,
            Command::ZconnectPoll(cmd) => cmd.config,
            Command::ZconnectScan(cmd) => cmd.config,
            Command::ZconnectToss(cmd) => cmd.config,
            Command::ZconnectAck(cmd) => cmd.config,
        };
        assert_eq!(actual.as_os_str(), path, "{name}");
    }
}

#[test]
fn cli_zconnect_optional_selection_and_explicit_ack() {
    for command in ["zconnect-links", "zconnect-poll", "zconnect-scan", "zconnect-toss"] {
        for link in [None, Some("MY-PEER")] {
            let mut args = vec![command, "board.toml"];
            args.extend(link);
            let cmd = match parse(&args).command.unwrap() {
                Command::ZconnectLinks(cmd) | Command::ZconnectPoll(cmd) | Command::ZconnectScan(cmd) | Command::ZconnectToss(cmd) => cmd,
                _ => panic!(),
            };
            assert_eq!(cmd.config, Path::new("board.toml"));
            assert_eq!(cmd.link.as_deref(), link);
        }
    }
    for args in [
        vec!["icbmailer", "zconnect-ack", "board.toml"],
        vec!["icbmailer", "zconnect-ack", "board.toml", "--all"],
        vec!["icbmailer", "zconnect-ack", "board.toml", "one", "two"],
        vec!["icbmailer", "zconnect-poll", "board.toml", "--keep"],
    ] {
        assert!(icy_board_cli::try_parse_from::<Cli, _, _>(args).is_err());
    }
    let Some(Command::ZconnectAck(cmd)) = parse(&["zconnect-ack", "board.toml", "PEER"]).command else {
        panic!()
    };
    assert_eq!(cmd.config, Path::new("board.toml"));
    assert_eq!(cmd.link, "PEER");
}

fn zconnect_config() -> icy_board_engine::icy_board::zconnect::ZconnectConfig {
    use icy_board_engine::icy_board::zconnect::{ZconnectConfig, ZconnectLink};
    ZconnectConfig {
        enabled: true,
        local_system: "local.example".into(),
        links: ["One", "Two"]
            .into_iter()
            .map(|id| ZconnectLink {
                id: id.into(),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}

#[test]
fn zconnect_link_selection_is_exact_case_insensitive_and_rejects_unknown() {
    let config = zconnect_config();
    assert_eq!(zconnect::selected_links(&config, None).unwrap().len(), 2);
    let links = zconnect::selected_links(&config, Some("oNe")).unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].id, "One");
    for id in ["", "O", "*", "all", "../One"] {
        assert!(zconnect::selected_links(&config, Some(id)).is_err());
    }
    let mut invalid = config;
    invalid.links[0].id = "../escape".into();
    assert!(zconnect::selected_links(&invalid, None).is_err());
}

#[test]
fn zconnect_packet_selection_is_per_link_regular_zip_only() {
    let temp = tempfile::tempdir().unwrap();
    let one = temp.path().join("One");
    let two = temp.path().join("Two");
    fs::create_dir_all(one.join("processed")).unwrap();
    fs::create_dir_all(one.join("retained")).unwrap();
    fs::create_dir_all(one.join("directory.zip")).unwrap();
    fs::create_dir_all(&two).unwrap();
    for name in ["z.zip", "A.ZIP", "partial.zip.tmp", "processed/old.zip", "retained/private.zip"] {
        fs::write(one.join(name), name).unwrap();
    }
    fs::write(two.join("other.zip"), b"other link").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(two.join("other.zip"), one.join("alias.zip")).unwrap();
    assert_eq!(zconnect::inbound_packets(&one).unwrap(), vec![one.join("A.ZIP"), one.join("z.zip")]);
    assert_eq!(zconnect::inbound_packets(&two).unwrap(), vec![two.join("other.zip")]);
    assert!(zconnect::inbound_packets(&temp.path().join("missing")).unwrap().is_empty());
}

#[test]
fn zconnect_archives_never_overwrite_and_retained_bytes_survive() {
    let temp = tempfile::tempdir().unwrap();
    let packet = temp.path().join("mail.zip");
    fs::write(&packet, b"first").unwrap();
    let first = zconnect::archive_packet(&packet, false).unwrap();
    fs::write(&packet, b"second").unwrap();
    let second = zconnect::archive_packet(&packet, false).unwrap();
    assert_ne!(first, second);
    assert_eq!(fs::read(first).unwrap(), b"first");
    assert_eq!(fs::read(second).unwrap(), b"second");
    fs::write(&packet, b"private original").unwrap();
    let retained = zconnect::archive_packet(&packet, true).unwrap();
    assert_eq!(retained.parent(), Some(temp.path().join("retained").as_path()));
    assert_eq!(fs::read(retained).unwrap(), b"private original");
    assert!(zconnect::inbound_packets(temp.path()).unwrap().is_empty());
}

#[test]
fn zconnect_toss_retains_unsupported_unknown_and_errors_but_continues() {
    use icy_board_engine::icy_board::zconnect::TossReport;
    let temp = tempfile::tempdir().unwrap();
    for name in ["a-error.zip", "b-unsupported.zip", "c-unknown.zip", "d-good.zip"] {
        fs::write(temp.path().join(name), name).unwrap();
    }
    let mut calls = Vec::new();
    let result = zconnect::toss_packets(temp.path(), |packet| {
        let name = packet.file_name().unwrap().to_str().unwrap();
        calls.push(name.to_owned());
        match name {
            "a-error.zip" => Err("bad ZIP".into()),
            "b-unsupported.zip" => Ok(TossReport {
                imported: 1,
                unsupported: 1,
                ..Default::default()
            }),
            "c-unknown.zip" => Ok(TossReport {
                unknown_boards: 1,
                ..Default::default()
            }),
            _ => Ok(TossReport {
                duplicates: 2,
                loops: 1,
                ..Default::default()
            }),
        }
    });
    assert!(result.unwrap_err().to_string().contains("bad ZIP"));
    assert_eq!(calls.len(), 4);
    for name in ["b-unsupported.zip", "c-unknown.zip"] {
        assert_eq!(fs::read(temp.path().join("retained").join(name)).unwrap(), name.as_bytes());
    }
    assert!(temp.path().join("processed/d-good.zip").exists());
    assert_eq!(fs::read(temp.path().join("a-error.zip")).unwrap(), b"a-error.zip");
    assert_eq!(zconnect::inbound_packets(temp.path()).unwrap(), vec![temp.path().join("a-error.zip")]);
}

#[test]
fn zconnect_archive_failure_preserves_inbox_original() {
    let temp = tempfile::tempdir().unwrap();
    let packet = temp.path().join("mail.zip");
    fs::write(&packet, b"original").unwrap();
    fs::write(temp.path().join("processed"), b"not a directory").unwrap();
    assert!(zconnect::toss_packets(temp.path(), |_| Ok(Default::default())).is_err());
    assert_eq!(fs::read(packet).unwrap(), b"original");
}

#[test]
fn zconnect_engine_rejection_keeps_the_original_packet() {
    let temp = tempfile::tempdir().unwrap();
    let config = zconnect_config();
    let inbound = temp.path().join(&config.inbound).join("One");
    fs::create_dir_all(&inbound).unwrap();
    let packet = inbound.join("broken.zip");
    fs::write(&packet, b"not a ZIP archive").unwrap();
    let result = zconnect::toss_packets(&inbound, |packet| {
        icy_board_engine::icy_board::zconnect::toss(&config, temp.path(), "One", packet)
    });
    assert!(result.is_err());
    assert_eq!(fs::read(&packet).unwrap(), b"not a ZIP archive");
    assert!(!inbound.join("processed").exists());
}

#[test]
fn zconnect_offline_lock_excludes_same_link_and_keeps_stable_inode() {
    let temp = tempfile::tempdir().unwrap();
    let config = zconnect_config();
    let guard = zconnect::offline_lock(&config, temp.path(), &config.links[0]).unwrap();
    assert!(zconnect::offline_lock(&config, temp.path(), &config.links[0]).is_err());
    assert!(zconnect::offline_lock(&config, temp.path(), &config.links[1]).is_ok());
    let lock_path = temp.path().join(&config.outbound).join("One/poll.lock");
    assert!(lock_path.exists());
    drop(guard);
    assert!(lock_path.exists());
    assert!(zconnect::offline_lock(&config, temp.path(), &config.links[0]).is_ok());
}

#[tokio::test]
async fn zconnect_poll_failure_still_tosses_all_durable_archives_and_fails() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("old.zip"), b"previous poll").unwrap();
    let exchange = async {
        fs::write(temp.path().join("partial-session.zip"), b"complete download before disconnect").unwrap();
        Err("late protocol failure".into())
    };
    let mut calls = 0;
    let result = zconnect::exchange_and_toss(exchange, || {
        zconnect::toss_packets(temp.path(), |_| {
            calls += 1;
            Ok(Default::default())
        })
    })
    .await;
    assert!(result.unwrap_err().to_string().contains("late protocol failure"));
    assert_eq!(calls, 2);
    assert!(zconnect::inbound_packets(temp.path()).unwrap().is_empty());
    assert!(temp.path().join("processed/old.zip").exists());
    assert!(temp.path().join("processed/partial-session.zip").exists());
}

#[tokio::test]
async fn zconnect_poll_reports_both_exchange_and_toss_errors() {
    let result = zconnect::exchange_and_toss(async { Err("exchange broke".into()) }, || Err("toss broke".into())).await;
    let error = result.unwrap_err().to_string();
    assert!(error.contains("exchange broke"));
    assert!(error.contains("toss broke"));
    assert!(
        zconnect::exchange_and_toss(async { Ok(Default::default()) }, || Err("toss only".into()))
            .await
            .is_err()
    );
}
