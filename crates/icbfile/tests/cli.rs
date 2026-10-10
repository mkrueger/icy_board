use std::{
    fs,
    process::{Command, Output},
};

use dizbase::file_base::FileBase;
use tempfile::TempDir;

fn run(locale: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_icbfile"))
        .args(args)
        .env("LANG", format!("{locale}.UTF-8"))
        .env("LC_ALL", format!("{locale}.UTF-8"))
        .env("LC_MESSAGES", format!("{locale}.UTF-8"))
        .env("LANGUAGE", locale)
        .env("NO_COLOR", "1")
        .output()
        .unwrap()
}

fn decoded(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn nested_area(root: &TempDir) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    fs::write(root.path().join("icboard.toml"), "").unwrap();
    let config = root.path().join("conferences/main/dir.toml");
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    fs::write(
        &config,
        r#"
            [[area]]
            name = "General"
            path = "files/general"
            metadata_path = "metadata/general"
            password = ""
        "#,
    )
    .unwrap();
    let path = root.path().join("files/general");
    fs::create_dir_all(&path).unwrap();
    let metadata = root.path().join("metadata/general");
    (config, path, metadata)
}

#[test]
fn check_nested_board_area_uses_the_live_database_and_prunes_deleted_files() {
    for locale in ["en_US", "de_DE"] {
        let root = TempDir::new().unwrap();
        let (config, path, metadata) = nested_area(&root);
        fs::write(path.join("pcb154f.zip"), "deleted archive").unwrap();
        fs::write(path.join("KEEP.TXT"), "keep this file").unwrap();
        let mut base = FileBase::open(&path, &metadata).unwrap();
        base.set_description(&path.join("pcb154f.zip"), "authored description").unwrap();
        base.iter_mut().find(|h| h.name == "pcb154f.zip").unwrap().dl_counter = 7;
        base.save().unwrap();
        drop(base);
        fs::remove_file(path.join("pcb154f.zip")).unwrap();

        let target = config.to_str().unwrap();
        let output = run(locale, &["check", target, "--area", "general"]);
        assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
        let stdout = decoded(&output.stdout);
        assert!(stdout.contains("missing: pcb154f.zip") && stdout.contains("2 file(s), 1 missing"), "{stdout}");
        let mut base = FileBase::open(&path, &metadata).unwrap();
        assert_eq!(base.description(&path.join("pcb154f.zip")).unwrap().as_deref(), Some("authored description"));
        assert_eq!(base.iter().find(|h| h.name == "pcb154f.zip").unwrap().dl_counter, 7);
        drop(base);

        let output = run(locale, &["check", target, "--prune"]);
        assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
        assert!(decoded(&output.stdout).contains("1 missing (removed)"));
        let base = FileBase::open(&path, &metadata).unwrap();
        assert_eq!(base.to_vec().len(), 1);
        assert_eq!(base[0].name, "KEEP.TXT");
        assert_eq!(fs::read_to_string(path.join("KEEP.TXT")).unwrap(), "keep this file");
        assert!(!config.parent().unwrap().join("metadata").exists());
    }
}

#[test]
fn check_unavailable_storage_fails_without_creating_or_pruning_a_database() {
    for locale in ["en_US", "de_DE"] {
        let root = TempDir::new().unwrap();
        let (config, path, metadata) = nested_area(&root);
        fs::remove_dir(&path).unwrap();
        let output = run(locale, &["check", config.to_str().unwrap(), "--area", "general", "--prune"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(decoded(&output.stderr).contains("can't read file directory"));
        assert!(!FileBase::database_path(&metadata).exists());
        assert!(!decoded(&output.stdout).contains("0 file(s)"));

        fs::create_dir(&path).unwrap();
        fs::write(path.join("OFFLINE.TXT"), "offline").unwrap();
        drop(FileBase::open(&path, &metadata).unwrap());
        let offline = root.path().join("offline");
        fs::rename(&path, &offline).unwrap();
        let output = run(locale, &["check", config.to_str().unwrap(), "--prune"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(decoded(&output.stderr).contains("can't read file directory"));
        fs::rename(&offline, &path).unwrap();
        assert!(FileBase::open(&path, &metadata).unwrap().contains_name("OFFLINE.TXT"));
    }
}

#[test]
fn nested_board_paths_are_shared_by_scan_set_import_export_and_list() {
    for locale in ["en_US", "de_DE"] {
        let root = TempDir::new().unwrap();
        let (config, path, metadata) = nested_area(&root);
        fs::write(path.join("RULES.TXT"), "rules").unwrap();
        let target = config.to_str().unwrap();
        for args in [
            vec!["scan", target, "--all"],
            vec!["set", target, "RULES.TXT", "--area", "General", "--desc", "Board rules"],
        ] {
            let output = run(locale, &args);
            assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
        }
        let listing = root.path().join("FILES.BBS");
        fs::write(&listing, "RULES.TXT Imported rules\n").unwrap();
        let output = run(locale, &["import", target, listing.to_str().unwrap(), "--area", "General", "--overwrite"]);
        assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
        for command in ["list", "export"] {
            let output = run(locale, &[command, target, "--area", "General"]);
            assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
            let stdout = decoded(&output.stdout);
            assert!(stdout.contains("RULES.TXT") && stdout.contains("Imported rules"), "{stdout}");
        }
        let mut base = FileBase::open(&path, &metadata).unwrap();
        assert_eq!(base.description(&path.join("RULES.TXT")).unwrap().as_deref(), Some("Imported rules"));
        assert!(!config.parent().unwrap().join("metadata").exists());
    }
}

#[test]
fn absolute_area_paths_and_default_metadata_are_preserved() {
    for locale in ["en_US", "de_DE"] {
        let root = TempDir::new().unwrap();
        let (config, path, _) = nested_area(&root);
        fs::write(
            &config,
            format!(
                "[[area]]\nname = \"General\"\npath = \"{}\"\nmetadata_path = \"\"\npassword = \"\"\n",
                path.display()
            ),
        )
        .unwrap();
        fs::write(path.join("MISSING.TXT"), "missing").unwrap();
        drop(FileBase::open(&path, path.join("dir")).unwrap());
        fs::remove_file(path.join("MISSING.TXT")).unwrap();
        let output = run(locale, &["check", config.to_str().unwrap(), "--area", "General", "--prune"]);
        assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
        assert!(decoded(&output.stdout).contains("1 missing (removed)"));
        assert!(FileBase::open(&path, path.join("dir")).unwrap().is_empty());
    }
}

#[test]
fn delete_removes_disk_files_and_missing_entries_from_the_live_board_database() {
    for locale in ["en_US", "de_DE"] {
        let root = TempDir::new().unwrap();
        let (config, path, metadata) = nested_area(&root);
        for name in ["pcb154f.zip", "MISSING.TXT", "KEEP.TXT"] {
            fs::write(path.join(name), name).unwrap();
        }
        let mut base = FileBase::open(&path, &metadata).unwrap();
        base.set_description(&path.join("pcb154f.zip"), "authored description").unwrap();
        base.set_description(&path.join("MISSING.TXT"), "missing description").unwrap();
        drop(base);
        fs::remove_file(path.join("MISSING.TXT")).unwrap();

        for name in ["PCB154F.ZIP", "missing.txt"] {
            let output = run(locale, &["delete", config.to_str().unwrap(), name, "--area", "General"]);
            assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
            let expected = if locale == "de_DE" { "gelöscht:" } else { "deleted:" };
            assert!(decoded(&output.stdout).contains(expected));
            assert!(!FileBase::open(&path, &metadata).unwrap().contains_name(name));
        }
        assert!(!path.join("pcb154f.zip").exists());
        assert!(!path.join("MISSING.TXT").exists());
        assert_eq!(fs::read_to_string(path.join("KEEP.TXT")).unwrap(), "KEEP.TXT");
        let output = run(locale, &["list", config.to_str().unwrap(), "--area", "0"]);
        assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
        let stdout = decoded(&output.stdout);
        assert!(
            stdout.contains("KEEP.TXT") && !stdout.contains("pcb154f.zip") && !stdout.contains("MISSING.TXT"),
            "{stdout}"
        );

        let output = run(locale, &["delete", config.to_str().unwrap(), "keep.txt", "--area", "0"]);
        assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
        assert!(!path.join("KEEP.TXT").exists());
        assert!(FileBase::open(&path, &metadata).unwrap().is_empty());
    }
}

#[test]
fn delete_rejects_unknown_names_and_retains_entries_when_disk_deletion_fails() {
    for locale in ["en_US", "de_DE"] {
        let root = TempDir::new().unwrap();
        let path = root.path().join("files");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("KEEP.TXT"), "keep").unwrap();
        fs::write(root.path().join("OUTSIDE.TXT"), "outside").unwrap();
        let metadata = path.join("dir");
        drop(FileBase::open(&path, &metadata).unwrap());
        for name in ["UNKNOWN.TXT", "*.TXT", "../OUTSIDE.TXT"] {
            let output = run(locale, &["delete", path.to_str().unwrap(), name]);
            assert_eq!(output.status.code(), Some(1));
            assert!(decoded(&output.stderr).contains("is not in this area"));
        }
        assert_eq!(fs::read_to_string(root.path().join("OUTSIDE.TXT")).unwrap(), "outside");
        fs::remove_file(path.join("KEEP.TXT")).unwrap();
        fs::create_dir(path.join("KEEP.TXT")).unwrap();
        let output = run(locale, &["delete", path.to_str().unwrap(), "KEEP.TXT"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(decoded(&output.stderr).contains("database entry retained"));
        assert!(path.join("KEEP.TXT").is_dir());
        assert!(FileBase::open(&path, &metadata).unwrap().contains_name("KEEP.TXT"));
        fs::remove_dir(path.join("KEEP.TXT")).unwrap();
        fs::write(path.join("KEEP.TXT"), "keep").unwrap();
        let output = run(locale, &["delete", path.to_str().unwrap(), "keep.txt"]);
        assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
        assert!(!path.join("KEEP.TXT").exists());
        assert!(FileBase::open(&path, metadata).unwrap().is_empty());
    }
}

#[cfg(unix)]
#[test]
fn check_inspection_errors_do_not_prune_and_delete_does_not_follow_symlinks() {
    use std::os::unix::fs::symlink;

    for locale in ["en_US", "de_DE"] {
        let root = TempDir::new().unwrap();
        let path = root.path().join("files");
        fs::create_dir(&path).unwrap();
        for name in ["LOOP.TXT", "MISSING.TXT"] {
            fs::write(path.join(name), name).unwrap();
        }
        let outside = root.path().join("OUTSIDE.TXT");
        fs::write(&outside, "outside").unwrap();
        symlink(&outside, path.join("LINK.TXT")).unwrap();
        let metadata = path.join("dir");
        drop(FileBase::open(&path, &metadata).unwrap());
        fs::remove_file(path.join("LOOP.TXT")).unwrap();
        fs::remove_file(path.join("MISSING.TXT")).unwrap();
        symlink("LOOP.TXT", path.join("LOOP.TXT")).unwrap();

        let output = run(locale, &["check", path.to_str().unwrap(), "--prune"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(decoded(&output.stderr).contains("can't inspect"));
        let base = FileBase::open(&path, &metadata).unwrap();
        assert!(base.contains_name("LOOP.TXT") && base.contains_name("MISSING.TXT"));
        drop(base);
        let output = run(locale, &["delete", path.to_str().unwrap(), "LINK.TXT"]);
        assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
        assert!(fs::symlink_metadata(path.join("LINK.TXT")).is_err());
        assert_eq!(fs::read_to_string(&outside).unwrap(), "outside");
        assert!(!FileBase::open(&path, &metadata).unwrap().contains_name("LINK.TXT"));
    }
}

#[test]
fn check_defaults_to_all_areas_and_only_prunes_when_requested() {
    for locale in ["en_US", "de_DE"] {
        let root = TempDir::new().unwrap();
        let config = root.path().join("file_areas.toml");
        fs::write(
            &config,
            r#"
                [[area]]
                name = "One"
                path = "one"
                metadata_path = "metadata/one"
                password = ""

                [[area]]
                name = "Two"
                path = "two"
                metadata_path = "metadata/two"
                password = ""
            "#,
        )
        .unwrap();
        for (area, missing) in [("one", "ONE.TXT"), ("two", "TWO.TXT")] {
            let path = root.path().join(area);
            fs::create_dir(&path).unwrap();
            fs::write(path.join(missing), "missing file").unwrap();
            fs::write(path.join("KEEP.TXT"), "keep this file").unwrap();
            let base = FileBase::open(&path, root.path().join("metadata").join(area)).unwrap();
            assert_eq!(base.to_vec().len(), 2);
            drop(base);
            fs::remove_file(path.join(missing)).unwrap();
        }

        let target = config.to_str().unwrap();
        let output = run(locale, &["check", target]);
        assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
        let stdout = decoded(&output.stdout);
        for expected in ["[0] One", "[1] Two", "missing: ONE.TXT", "missing: TWO.TXT"] {
            assert!(stdout.contains(expected), "{stdout}");
        }
        for area in ["one", "two"] {
            let base = FileBase::open(&root.path().join(area), root.path().join("metadata").join(area)).unwrap();
            assert_eq!(base.to_vec().len(), 2);
        }

        let output = run(locale, &["check", target, "--area", "1"]);
        assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
        let stdout = decoded(&output.stdout);
        assert!(stdout.contains("missing: TWO.TXT"), "{stdout}");
        assert!(!stdout.contains("ONE.TXT"), "{stdout}");

        let output = run(locale, &["check", target, "--area", "oNe", "--prune"]);
        assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
        for (area, expected_count) in [("one", 1), ("two", 2)] {
            let base = FileBase::open(&root.path().join(area), root.path().join("metadata").join(area)).unwrap();
            assert_eq!(base.to_vec().len(), expected_count);
        }

        let output = run(locale, &["check", target, "--prune"]);
        assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
        for area in ["one", "two"] {
            let path = root.path().join(area);
            let base = FileBase::open(&path, root.path().join("metadata").join(area)).unwrap();
            let headers = base.to_vec();
            assert_eq!(headers.len(), 1);
            assert_eq!(headers[0].name, "KEEP.TXT");
            assert_eq!(fs::read_to_string(path.join("KEEP.TXT")).unwrap(), "keep this file");
        }
    }
}

#[test]
fn check_still_accepts_a_directory_target() {
    for locale in ["en_US", "de_DE"] {
        let root = TempDir::new().unwrap();
        let path = root.path();
        fs::write(path.join("MISSING.TXT"), "missing file").unwrap();
        fs::write(path.join("KEEP.TXT"), "keep this file").unwrap();
        drop(FileBase::open(path, path.join("dir")).unwrap());
        fs::remove_file(path.join("MISSING.TXT")).unwrap();

        let output = run(locale, &["check", path.to_str().unwrap()]);
        assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
        assert!(decoded(&output.stdout).contains("missing: MISSING.TXT"));
        assert_eq!(FileBase::open(path, path.join("dir")).unwrap().to_vec().len(), 2);

        let output = run(locale, &["check", path.to_str().unwrap(), "--prune"]);
        assert_eq!(output.status.code(), Some(0), "{}", decoded(&output.stderr));
        let headers = FileBase::open(path, path.join("dir")).unwrap().to_vec();
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].name, "KEEP.TXT");
        assert_eq!(fs::read_to_string(path.join("KEEP.TXT")).unwrap(), "keep this file");
    }
}

#[test]
fn check_all_continues_after_an_area_failure_and_exits_with_an_error() {
    for locale in ["en_US", "de_DE"] {
        let root = TempDir::new().unwrap();
        let config = root.path().join("file_areas.toml");
        fs::write(
            &config,
            r#"
                [[area]]
                name = "Broken"
                path = "one"
                metadata_path = "blocked/dir"
                password = ""

                [[area]]
                name = "Working"
                path = "two"
                metadata_path = "two/dir"
                password = ""
            "#,
        )
        .unwrap();
        fs::create_dir(root.path().join("one")).unwrap();
        fs::create_dir(root.path().join("two")).unwrap();
        fs::write(root.path().join("blocked"), "not a directory").unwrap();
        let path = root.path().join("two");
        fs::write(path.join("MISSING.TXT"), "missing file").unwrap();
        drop(FileBase::open(&path, path.join("dir")).unwrap());
        fs::remove_file(path.join("MISSING.TXT")).unwrap();

        for prune in [false, true] {
            let mut args = vec!["check", config.to_str().unwrap()];
            if prune {
                args.push("--prune");
            }
            let output = run(locale, &args);
            assert_eq!(output.status.code(), Some(1));
            let stderr = decoded(&output.stderr);
            assert!(stderr.contains("area failed:") && stderr.contains("1 area(s) failed"), "{stderr}");
            let stdout = decoded(&output.stdout);
            for expected in ["[0] Broken", "[1] Working", "missing: MISSING.TXT"] {
                assert!(stdout.contains(expected), "{stdout}");
            }
            let base = FileBase::open(&path, path.join("dir")).unwrap();
            assert_eq!(base.to_vec().len(), if prune { 0 } else { 1 });
        }
    }
}

#[test]
fn help_and_no_arguments_keep_their_streams_and_exit_codes_in_both_languages() {
    for (locale, about, options) in [
        ("en_US", "Convert and maintain icy_board file bases", "Options:"),
        ("de_DE", "icy_board-Dateibereiche konvertieren und verwalten", "Optionen:"),
    ] {
        let help = run(locale, &["--help"]);
        assert_eq!(help.status.code(), Some(0), "{}", decoded(&help.stderr));
        assert!(help.stderr.is_empty());
        let stdout = decoded(&help.stdout);
        assert!(stdout.contains(about), "{stdout}");
        assert!(stdout.contains(options), "{stdout}");
        for command in ["areas", "list", "scan", "check", "import", "export", "set", "delete", "repack", "fingerprints"] {
            assert!(stdout.contains(command), "{stdout}");
        }
        let no_args = run(locale, &[]);
        assert_eq!(no_args.status.code(), Some(1));
        assert!(no_args.stdout.is_empty());
        let stderr = decoded(&no_args.stderr);
        assert!(stderr.contains(about) && stderr.contains(options), "{stderr}");
    }
}

#[test]
fn every_subcommand_help_is_localized_without_translating_identifiers() {
    let commands = [
        ("areas", "path to the area list", "Pfad zur Bereichsliste"),
        ("list", "show size, date and download count", "Download-Anzahl anzeigen"),
        ("scan", "scan every area", "jeden Bereich"),
        ("check", "drop entries", "Einträge entfernen"),
        (
            "import",
            "listing format: auto, pcboard or filesbbs",
            "Listenformat: auto, pcboard oder filesbbs",
        ),
        ("export", "encoded as cp437", "als cp437 kodiert"),
        ("set", "the new description", "die neue Beschreibung"),
        ("delete", "no wildcards", "keine Platzhalter"),
        ("repack", "zip deflate compression level", "zip-Deflate-Kompressionsstufe"),
        ("fingerprints", "where to write the fingerprints", "Ausgabepfad für die Fingerabdrücke"),
    ];
    for (command, english, german) in commands {
        for (locale, expected) in [("en_US", english), ("de_DE", german)] {
            let help = run(locale, &[command, "--help"]);
            assert_eq!(help.status.code(), Some(0), "{}", decoded(&help.stderr));
            assert!(help.stderr.is_empty());
            let stdout = decoded(&help.stdout);
            assert!(stdout.contains(expected), "{locale} {command}: {stdout}");
            assert!(stdout.contains(&format!("icbfile {command}")), "{stdout}");
            assert!(stdout.contains("--help"), "{stdout}");
            if command == "check" {
                let scope = if locale == "de_DE" { "alle Bereiche" } else { "all areas" };
                assert!(stdout.contains(scope), "{stdout}");
            }
            if locale == "de_DE" {
                assert!(
                    !stdout.contains("Usage:") && !stdout.contains("Options:") && !stdout.contains("Arguments:"),
                    "{stdout}"
                );
            }
        }
    }
    let help = decoded(&run("de_DE", &["repack", "--help"]).stdout);
    for identifier in [
        "--compression-level",
        "--max-members",
        "--max-member-size",
        "--max-expanded-size",
        "--max-compression-ratio",
        "--keep-case",
    ] {
        assert!(help.contains(identifier), "{help}");
    }
    assert!(help.contains("<target>"), "{help}");
}

#[test]
fn parser_errors_are_localized_on_stderr_with_exit_one() {
    for locale in ["en_US", "de_DE"] {
        for args in [vec!["--not-an-option"], vec!["areas"], vec!["repack", "files", "--max-members", "not-a-number"]] {
            let output = run(locale, &args);
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            let stderr = decoded(&output.stderr);
            assert!(stderr.contains("--help"), "{stderr}");
            let prefix = if locale == "de_DE" { "fehler" } else { "error" };
            assert!(stderr.to_lowercase().contains(prefix), "{stderr}");
        }
        let output = run(locale, &["import", "files", "--format", "bogus"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let stderr = decoded(&output.stderr);
        let expected = if locale == "de_DE" { "unbekanntes Format" } else { "unknown format" };
        assert!(stderr.contains(expected) && stderr.contains("bogus") && stderr.contains("--format"), "{stderr}");
    }
}

#[test]
fn version_remains_manual_and_locale_independent() {
    for locale in ["en_US", "de_DE"] {
        let output = run(locale, &["--version"]);
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());
        assert_eq!(
            decoded(&output.stdout),
            format!("{}\n", icy_board_cli::version_line("icbfile", env!("CARGO_PKG_VERSION"), env!("GIT_HASH")))
        );
        let short = run(locale, &["-V"]);
        assert_eq!(short.status.code(), Some(1));
        assert!(short.stdout.is_empty());
    }
}
