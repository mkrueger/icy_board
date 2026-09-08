use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::SystemTime,
};

use icy_board_engine::{
    DEFAULT_ICYBOARD_FILE,
    icy_board::{
        IcyBoard, IcyBoardSerializer,
        commands::{Command as MenuCommand, CommandAction, CommandType},
        icb_text::DEFAULT_DISPLAY_TEXT,
        menu::Menu,
    },
};
use tempfile::TempDir;

fn temp_dir(name: &str) -> TempDir {
    tempfile::Builder::new().prefix(&format!("mkicbmnu-{name}-")).tempdir().unwrap()
}

fn mkicbmnu() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_mkicbmnu"));
    command.env("LANG", "en_US.UTF-8").env("LC_ALL", "en_US.UTF-8").env("LANGUAGE", "en");
    command
}

#[test]
fn cli_help_errors_version_and_no_arguments_keep_their_streams() {
    for (locale, help, error) in [("en", "Use the full screen", "error"), ("de", "Vollbild verwenden", "Fehler")] {
        let run = |args: &[&str]| {
            mkicbmnu()
                .env("LANG", locale)
                .env("LC_ALL", locale)
                .env("LANGUAGE", locale)
                .args(args)
                .output()
                .unwrap()
        };
        let output = run(&["--help"]);
        assert!(output.status.success() && output.stderr.is_empty());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains(help), "{stdout}");
        for flag in ["--create", "--board", "-b,", "DIRECTORY_OR_CONFIG", "--check", "--full-screen", "--version"] {
            assert!(stdout.contains(flag), "{stdout}");
        }
        let output = run(&[]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(help), "{stderr}");
        let output = run(&["--unknown-option"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(error) && stderr.contains("--unknown-option"), "{stderr}");
        let output = run(&["--version"]);
        assert!(output.status.success() && output.stderr.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            format!("{}\n", icy_board_cli::version_line("mkicbmnu", env!("CARGO_PKG_VERSION"), env!("GIT_HASH")))
        );
    }
}

#[test]
fn missing_menu_explains_how_to_create_it() {
    let temporary = temp_dir("missing-menu");
    let directory = temporary.path();
    let menu = directory.join("main.mnu");

    let output = mkicbmnu().arg(&menu).env("LANG", "en_US.UTF-8").output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Input file not found:"));
    assert!(stderr.contains("--create"));
    assert!(stderr.contains("--help"));
}

#[test]
fn missing_parent_board_explains_where_it_is_expected() {
    let temporary = temp_dir("missing-board");
    let directory = temporary.path();
    let menu = directory.join("main.mnu");

    let output = mkicbmnu()
        .args(["--create", menu.to_str().unwrap()])
        .env("LANG", "en_US.UTF-8")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("No icboard.toml found for:"));
    assert!(stderr.contains("file's directory and its parents"));
    assert!(stderr.contains("icbsetup create mybbs"));
    assert!(stderr.contains("docs/gettingstarted.md"));
}

#[test]
fn malformed_parent_board_reports_the_file_and_load_error() {
    let temporary = temp_dir("malformed-board");
    let directory = temporary.path();
    fs::write(directory.join("icboard.toml"), b"not valid toml").unwrap();
    let menu = directory.join("main.mnu");

    let output = mkicbmnu()
        .args(["--create", menu.to_str().unwrap()])
        .env("LANG", "en_US.UTF-8")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Can't load"));
    assert!(stderr.contains("icboard.toml"));
    assert!(!stderr.contains("No icboard.toml found"));
    assert!(!stderr.contains("panicked"));
}

/// Write real serialized dependencies, not just a syntactically valid config.
/// All configured load paths are relative to this disposable board directory.
fn write_board(directory: &Path) -> PathBuf {
    fs::create_dir_all(directory).unwrap();
    let mut board = IcyBoard::default();
    board.root_path = directory.to_path_buf();
    board.file_name = directory.join(DEFAULT_ICYBOARD_FILE);
    let paths = &mut board.config.paths;
    paths.user_file = "users.toml".into();
    paths.conferences = "conferences.toml".into();
    paths.icbtext = "icbtext.toml".into();
    paths.language_file = "languages.toml".into();
    paths.protocol_data_file = "protocols.toml".into();
    paths.pwrd_sec_level_file = "security.toml".into();
    paths.command_file = "commands.toml".into();
    paths.statistics_file = "statistics.toml".into();
    paths.group_file = "groups".into();
    paths.ftn_file = PathBuf::new();
    paths.qwknet_file = PathBuf::new();
    paths.zconnect_file = PathBuf::new();
    board.config.event.event_file = PathBuf::new();
    board.config.accounting.enabled = false;
    board.config.accounting.cfg_file = "unused-accounting.toml".into();

    board.config.save(&board.file_name).unwrap();
    IcyBoardSerializer::save(&board.users, &directory.join(&board.config.paths.user_file)).unwrap();
    board.conferences.save(&directory.join(&board.config.paths.conferences)).unwrap();
    DEFAULT_DISPLAY_TEXT.save(&directory.join(&board.config.paths.icbtext)).unwrap();
    board.languages.save(&directory.join(&board.config.paths.language_file)).unwrap();
    board.protocols.save(&directory.join(&board.config.paths.protocol_data_file)).unwrap();
    board.sec_levels.save(&directory.join(&board.config.paths.pwrd_sec_level_file)).unwrap();
    board.commands.save(&directory.join(&board.config.paths.command_file)).unwrap();
    board.statistics.save(&directory.join(&board.config.paths.statistics_file)).unwrap();
    board.groups.save(&directory.join(&board.config.paths.group_file)).unwrap();

    // A fixture failure must not masquerade as a CLI validation failure.
    IcyBoard::load(&board.file_name).expect("minimal CLI board fixture must load");
    board.file_name
}

fn valid_menu() -> Menu {
    Menu {
        title: "CLI fixture".into(),
        commands: vec![MenuCommand {
            keyword: "Q".into(),
            actions: vec![CommandAction {
                command_type: CommandType::QuitMenu,
                ..CommandAction::default()
            }],
            ..MenuCommand::default()
        }],
        ..Menu::default()
    }
}

// Include directories and modification times: rewriting identical bytes, creating
// a log/backup, or deleting a file must fail too. Reading may update atime only.
type TreeSnapshot = BTreeMap<PathBuf, (Option<Vec<u8>>, SystemTime)>;

fn snapshot(root: &Path) -> TreeSnapshot {
    fn visit(root: &Path, path: &Path, entries: &mut TreeSnapshot) {
        let metadata = fs::symlink_metadata(path).unwrap();
        assert!(!metadata.is_symlink(), "fixture unexpectedly contains a symlink: {}", path.display());
        let contents = if metadata.is_file() { Some(fs::read(path).unwrap()) } else { None };
        entries.insert(path.strip_prefix(root).unwrap().to_path_buf(), (contents, metadata.modified().unwrap()));
        if metadata.is_dir() {
            for entry in fs::read_dir(path).unwrap() {
                visit(root, &entry.unwrap().path(), entries);
            }
        }
    }

    let mut entries = BTreeMap::new();
    visit(root, root, &mut entries);
    entries
}

fn run_read_only(root: &Path, command: &mut Command) -> Output {
    let before = snapshot(root);
    let output = command.output().unwrap();
    assert_eq!(
        snapshot(root),
        before,
        "CLI modified the fixture: {command:?}\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn assert_check_output(output: &Output, code: i32, expected: &str) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(expected), "{stdout}");
    assert!(!stdout.contains('\u{1b}'), "--check must not initialize the terminal: {stdout:?}");
}

#[test]
fn check_accepts_long_and_short_board_directory_or_custom_config_absolute_and_relative() {
    for flag in ["--board", "-b"] {
        for config_argument in [false, true] {
            for relative in [false, true] {
                let temporary = temp_dir("explicit-board");
                let root = temporary.path();
                let board_dir = root.join("bbs directory");
                let mut config = write_board(&board_dir);
                if config_argument {
                    let custom = board_dir.join("custom board.toml");
                    fs::rename(&config, &custom).unwrap();
                    config = custom;
                }
                // This asset exists only under the BBS, whereas the menu is in CWD.
                fs::create_dir(board_dir.join("art")).unwrap();
                fs::write(board_dir.join("art/banner.ans"), b"Board display\r\n").unwrap();
                let menu = Menu {
                    display_file: "art/banner.ans".into(),
                    ..valid_menu()
                };
                menu.save(&root.join("external.mnu")).unwrap();
                let board_arg = if config_argument { config.clone() } else { board_dir.clone() };
                let board_arg = if relative {
                    board_arg.strip_prefix(root).unwrap().to_path_buf()
                } else {
                    board_arg
                };

                // Cover both absence of logs and preservation of an existing log.
                for existing_log in [false, true] {
                    if existing_log {
                        fs::write(config.with_extension("log"), b"existing log must not be appended\n").unwrap();
                    }
                    let output = run_read_only(root, mkicbmnu().current_dir(root).arg(flag).arg(&board_arg).args(["--check", "external.mnu"]));
                    assert_check_output(&output, 0, "No issues found");
                    if !existing_log {
                        assert!(!config.with_extension("log").exists());
                    }
                }
            }
        }
    }
}

#[test]
fn check_warnings_exit_zero_and_errors_exit_one_without_writes() {
    let temporary = temp_dir("check-severity");
    let root = temporary.path();
    let config = write_board(&root.join("bbs"));
    let menu_path = root.join("main.mnu");
    let mut warning_menu = valid_menu();
    let mut duplicate = warning_menu.commands[0].clone();
    duplicate.keyword = "q".into();
    warning_menu.commands.push(duplicate);
    let mut error_menu = valid_menu();
    error_menu.commands[0].charge_per_use = -1.0;
    let mixed_menu = Menu {
        display_file: "missing-display".into(),
        ..error_menu.clone()
    };
    for (menu, code, expected, warnings, errors) in [
        (valid_menu(), 0, "No issues found", false, false),
        (warning_menu, 0, "Warning #2: Duplicate ASCII-case-insensitive keyword", true, false),
        (Menu::default(), 1, "No commands", false, true),
        (error_menu, 1, "Error #1: Charges must be finite and non-negative", false, true),
        (mixed_menu, 1, "Error #1: Charges must be finite and non-negative", true, true),
    ] {
        menu.save(&menu_path).unwrap();
        let output = run_read_only(root, mkicbmnu().current_dir(root).arg("--board").arg(&config).arg("--check").arg(&menu_path));
        assert_check_output(&output, code, expected);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout.lines().any(|line| line.starts_with("Warning ")), warnings, "{stdout}");
        assert_eq!(stdout.lines().any(|line| line.starts_with("Error ")), errors, "{stdout}");
        assert_eq!(stdout.contains("No issues found"), !warnings && !errors, "{stdout}");
    }
}

#[test]
fn relative_menu_stays_cwd_relative_not_board_relative_and_normalizes_extension() {
    let temporary = temp_dir("cwd-menu");
    let root = temporary.path();
    let board_dir = root.join("bbs");
    write_board(&board_dir);
    // Opposite validation outcomes make selecting the wrong menu observable.
    for cwd_valid in [true, false] {
        let (cwd_menu, board_menu) = if cwd_valid {
            (valid_menu(), Menu::default())
        } else {
            (Menu::default(), valid_menu())
        };
        cwd_menu.save(&root.join("main.mnu")).unwrap();
        board_menu.save(&board_dir.join("main.mnu")).unwrap();
        fs::write(root.join("main.txt"), b"must not load this extension").unwrap();
        for input in ["main", "main.mnu", "main.txt"] {
            let output = run_read_only(root, mkicbmnu().current_dir(root).args(["-b", "bbs", "--check", input]));
            assert_check_output(
                &output,
                if cwd_valid { 0 } else { 1 },
                if cwd_valid { "No issues found" } else { "No commands" },
            );
        }
    }
    fs::remove_file(root.join("main.mnu")).unwrap();
    let output = run_read_only(root, mkicbmnu().current_dir(root).args(["--board", "bbs", "--check", "main.mnu"]));
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Input file not found:"));
}

#[test]
fn check_discovers_board_from_menu_parents_for_absolute_relative_and_bare_paths() {
    let temporary = temp_dir("parent-discovery");
    let root = temporary.path();
    let board_dir = root.join("bbs");
    write_board(&board_dir);
    let nested = board_dir.join("menus/deep");
    fs::create_dir_all(&nested).unwrap();
    valid_menu().save(&nested.join("main.mnu")).unwrap();
    let unrelated = root.join("unrelated");
    fs::create_dir(&unrelated).unwrap();
    // Discovery must follow the menu, not an unrelated config in CWD.
    fs::write(unrelated.join(DEFAULT_ICYBOARD_FILE), b"not valid toml").unwrap();
    for (cwd, menu) in [
        (unrelated, nested.join("main.mnu")),
        (root.to_path_buf(), PathBuf::from("bbs/menus/deep/main.mnu")),
        (board_dir.clone(), PathBuf::from("menus/deep/main.mnu")),
        (board_dir.join("menus"), PathBuf::from("deep/main.mnu")),
        (nested.clone(), PathBuf::from("main.mnu")),
        (nested, PathBuf::from("./main")),
    ] {
        let output = run_read_only(root, mkicbmnu().current_dir(&cwd).arg("--check").arg(&menu));
        assert_check_output(&output, 0, "No issues found");
    }
}

#[test]
fn nearest_malformed_parent_board_is_not_skipped_for_a_valid_ancestor() {
    let temporary = temp_dir("nearest-parent");
    let root = temporary.path();
    write_board(root);
    let nested = root.join("nested");
    fs::create_dir(&nested).unwrap();
    let invalid = nested.join(DEFAULT_ICYBOARD_FILE);
    fs::write(&invalid, b"not valid toml").unwrap();
    valid_menu().save(&nested.join("main.mnu")).unwrap();
    let output = run_read_only(root, mkicbmnu().current_dir(root).args(["--check", "nested/main.mnu"]));
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Can't load") && stderr.contains(invalid.to_str().unwrap()), "{stderr}");
    assert!(!stderr.contains("No icboard.toml found") && !stderr.contains("panicked"), "{stderr}");

    // An explicit valid board overrides that nearest (malformed) config.
    let output = run_read_only(root, mkicbmnu().current_dir(root).args(["--board", ".", "--check", "nested/main.mnu"]));
    assert_check_output(&output, 0, "No issues found");
}

#[test]
fn explicit_invalid_board_never_falls_back_to_valid_discovered_board() {
    let temporary = temp_dir("invalid-explicit-board");
    let root = temporary.path();
    let valid = root.join("valid");
    write_board(&valid);
    valid_menu().save(&valid.join("main.mnu")).unwrap();
    let empty = root.join("empty");
    fs::create_dir(&empty).unwrap();
    let malformed = root.join("malformed.toml");
    fs::write(&malformed, b"not valid toml").unwrap();
    let incomplete = root.join("incomplete");
    let incomplete_config = write_board(&incomplete);
    fs::remove_file(incomplete.join("users.toml")).unwrap();

    for flag in ["--board", "-b"] {
        for (argument, expected_path) in [
            (root.join("absent.toml"), root.join("absent.toml")),
            (root.join("absent-directory"), root.join("absent-directory")),
            (empty.clone(), empty.join(DEFAULT_ICYBOARD_FILE)),
            (malformed.clone(), malformed.clone()),
            (incomplete.clone(), incomplete_config.clone()),
            (incomplete_config.clone(), incomplete_config.clone()),
        ] {
            let output = run_read_only(root, mkicbmnu().current_dir(&valid).arg(flag).arg(&argument).args(["--check", "main.mnu"]));
            assert_eq!(output.status.code(), Some(1), "{argument:?}: {output:?}");
            assert!(output.stdout.is_empty(), "{output:?}");
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(stderr.contains("Can't load") && stderr.contains(expected_path.to_str().unwrap()), "{stderr}");
            assert!(!stderr.contains("No icboard.toml found") && !stderr.contains("panicked"), "{stderr}");
        }
    }
    let output = run_read_only(root, mkicbmnu().current_dir(&valid).args(["--check", "main.mnu"]));
    assert_check_output(&output, 0, "No issues found");
}

#[test]
fn create_refuses_existing_normalized_destination_before_loading_menu_or_board() {
    let temporary = temp_dir("create-existing");
    let root = temporary.path();
    // Both files are deliberately unloadable: destination refusal wins over both.
    fs::write(root.join("main.mnu"), b"existing menu bytes, not valid TOML\0").unwrap();
    fs::write(root.join(DEFAULT_ICYBOARD_FILE), b"not valid toml").unwrap();
    for create in ["--create", "-c"] {
        for input in ["main", "main.mnu", "main.txt"] {
            for board_args in [vec![], vec!["--board", "absent.toml"], vec!["-b", DEFAULT_ICYBOARD_FILE]] {
                let output = run_read_only(root, mkicbmnu().current_dir(root).arg(create).args(board_args).arg(input));
                assert_eq!(output.status.code(), Some(1));
                assert!(output.stdout.is_empty());
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(stderr.contains("already exists") && stderr.contains("main.mnu"), "{stderr}");
                assert!(
                    !stderr.contains("Can't load") && !stderr.contains("No icboard.toml found") && !stderr.contains("panicked"),
                    "{stderr}"
                );
            }
        }
    }
}

#[test]
fn check_argument_errors_are_reported_before_any_files_are_loaded_or_created() {
    let temporary = temp_dir("check-arguments");
    let root = temporary.path();
    for args in [
        vec!["--check", "--create", "new.mnu"],
        vec!["-c", "--check", "new.mnu"],
        vec!["--board"],
        vec!["-b"],
        vec!["--check"],
        vec!["--check", "--board", "absent.toml"],
    ] {
        let output = run_read_only(root, mkicbmnu().current_dir(root).args(&args));
        assert_eq!(output.status.code(), Some(1), "{args:?}: {output:?}");
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("--check") || stderr.contains("--board"), "{stderr}");
        if args.contains(&"--create") || args.contains(&"-c") {
            assert!(stderr.contains("--create") && stderr.contains("--check"), "{stderr}");
        }
        assert!(!stderr.contains("Can't load") && !stderr.contains("panicked"), "{stderr}");
    }
}

#[test]
fn check_missing_or_malformed_menu_fails_without_creating_files_or_logs() {
    let temporary = temp_dir("check-bad-menu");
    let root = temporary.path();
    write_board(root);
    let output = run_read_only(root, mkicbmnu().current_dir(root).args(["--check", "missing.mnu"]));
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Input file not found:") && stderr.contains("--create"), "{stderr}");

    fs::write(root.join("malformed.mnu"), b"not valid toml").unwrap();
    let output = run_read_only(root, mkicbmnu().current_dir(root).args(["--check", "malformed.mnu"]));
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("malformed.mnu"), "{stderr}");
    assert!(!stderr.contains("Can't load") && !stderr.contains("panicked"), "{stderr}");
}
