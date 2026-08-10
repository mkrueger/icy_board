use std::{fs, net::SocketAddr, path::PathBuf};

use std::sync::Arc;

use icbadmin::{
    backup::BoardLock,
    check_bind_address,
    dto::GeneralSettingsDto,
    error::AdminError,
    service::{AdminBackend, AdminService, LiveAdminBackend},
};
use icy_board_engine::icy_board::{
    IcyBoard, IcyBoardSerializer,
    conferences::{Conference, ConferenceBase},
    icb_config::IcbConfig,
};
use tokio::sync::Mutex;

struct Fixture {
    _dir: tempfile::TempDir,
    file: PathBuf,
    service: AdminService,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("icyboard.toml");
    IcbConfig::new().save(&file).unwrap();
    let service = AdminService::open(&file).unwrap();
    Fixture { _dir: dir, file, service }
}

fn patch_from(settings: &GeneralSettingsDto) -> GeneralSettingsDto {
    settings.clone()
}

#[test]
fn open_fails_for_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let Err(err) = AdminService::open(dir.path().join("nope.toml")) else {
        panic!("expected an error for a missing configuration file");
    };
    assert!(matches!(err, AdminError::NotFound(_)));
}

#[test]
fn update_writes_config_creates_backup_and_audit_entry() {
    let f = fixture();
    let current = f.service.get_general_settings().unwrap();

    let mut patch = patch_from(&current.settings);
    patch.board_name = "Test Board".to_string();
    patch.num_nodes = 8;
    patch.allow_iemsi = !current.settings.allow_iemsi;

    let result = f.service.update_general_settings(&patch, &current.fingerprint, "test").unwrap();
    assert!(result.changed_fields.contains(&"board_name".to_string()));
    assert!(result.backup.is_some());
    assert_ne!(result.fingerprint, current.fingerprint);

    let reloaded = IcbConfig::load(&f.file).unwrap();
    assert_eq!(reloaded.board.name, "Test Board");
    assert_eq!(reloaded.board.num_nodes, 8);

    let backups: Vec<_> = fs::read_dir(f.service.root_path().join("backups")).unwrap().collect();
    assert_eq!(backups.len(), 1);

    let audit = fs::read_to_string(f.service.root_path().join("icbadmin-audit.log")).unwrap();
    assert!(audit.contains("update_general_settings"));
    assert!(audit.contains("board_name"));
}

#[test]
fn password_is_preserved_and_never_exposed() {
    let f = fixture();
    let mut config = IcbConfig::load(&f.file).unwrap();
    config.sysop.password = icy_board_engine::icy_board::user_base::Password::PlainText("secret".to_string());
    config.save(&f.file).unwrap();

    let current = f.service.get_general_settings().unwrap();
    assert!(current.sysop_password_set);
    let json = serde_json::to_string(&current).unwrap();
    assert!(!json.contains("secret"));

    let mut patch = patch_from(&current.settings);
    patch.board_name = "Renamed".to_string();
    f.service.update_general_settings(&patch, &current.fingerprint, "test").unwrap();

    let reloaded = IcbConfig::load(&f.file).unwrap();
    assert_eq!(reloaded.sysop.password.to_string(), "secret");
}

#[test]
fn stale_fingerprint_is_rejected() {
    let f = fixture();
    let current = f.service.get_general_settings().unwrap();

    let mut patch = patch_from(&current.settings);
    patch.board_name = "First".to_string();
    f.service.update_general_settings(&patch, &current.fingerprint, "test").unwrap();

    // Second write still uses the fingerprint from before the first one.
    patch.board_name = "Second".to_string();
    let err = f.service.update_general_settings(&patch, &current.fingerprint, "test").unwrap_err();
    assert!(matches!(err, AdminError::Conflict));

    assert_eq!(IcbConfig::load(&f.file).unwrap().board.name, "First");
}

#[test]
fn invalid_settings_are_rejected_without_touching_the_file() {
    let f = fixture();
    let current = f.service.get_general_settings().unwrap();

    let mut patch = patch_from(&current.settings);
    patch.board_name = "   ".to_string();
    patch.num_nodes = 0;
    patch.date_format = "%Q".to_string();

    let err = f.service.update_general_settings(&patch, &current.fingerprint, "test").unwrap_err();
    let AdminError::Validation(details) = err else {
        panic!("expected validation error");
    };
    assert_eq!(details.len(), 3);
    assert_eq!(f.service.get_general_settings().unwrap().fingerprint, current.fingerprint);
}

#[test]
fn a_held_lock_blocks_writes() {
    let f = fixture();
    let current = f.service.get_general_settings().unwrap();
    let _lock = BoardLock::acquire(f.service.root_path()).unwrap();

    let mut patch = patch_from(&current.settings);
    patch.board_name = "Blocked".to_string();
    let err = f.service.update_general_settings(&patch, &current.fingerprint, "test").unwrap_err();
    assert!(matches!(err, AdminError::Locked));
}

#[test]
fn preview_reports_changes_without_writing() {
    let f = fixture();
    let current = f.service.get_general_settings().unwrap();

    let mut patch = patch_from(&current.settings);
    patch.board_name = "Preview".to_string();

    let diff = f.service.preview_general_settings(&patch).unwrap();
    assert_eq!(diff.changes.len(), 1);
    assert_eq!(diff.changes[0].field, "board_name");
    assert_eq!(f.service.get_general_settings().unwrap().fingerprint, current.fingerprint);
}

#[test]
fn overview_reports_an_unloadable_board_instead_of_failing() {
    let f = fixture();
    let overview = f.service.overview();
    // The fixture has no user base or conference file, so the full load must fail.
    assert!(!overview.config_loaded);
    assert!(overview.load_error.is_some());
    assert_eq!(overview.board_file, f.file.display().to_string());
}

#[test]
fn remote_binds_need_an_explicit_opt_in() {
    let local: SocketAddr = "127.0.0.1:8787".parse().unwrap();
    let any: SocketAddr = "0.0.0.0:8787".parse().unwrap();

    assert!(check_bind_address(&local, false).is_ok());
    assert!(check_bind_address(&any, false).is_err());
    assert!(check_bind_address(&any, true).is_ok());
}

#[test]
fn web_admin_defaults_are_local_and_disabled() {
    let config = IcbConfig::new();
    assert!(!config.board.web_admin.enabled);
    assert_eq!(config.board.web_admin.address, "127.0.0.1");
    assert_eq!(config.board.web_admin.port, 8787);
    assert!(!config.board.web_admin.allow_remote);
}

#[test]
fn config_without_web_admin_table_uses_secure_defaults() {
    let f = fixture();
    let text = fs::read_to_string(&f.file).unwrap();
    let table_start = text.find("[board.web_admin]").unwrap();
    let table_end = text[table_start..].find("\n[").map(|offset| table_start + offset + 1).unwrap_or(text.len());
    fs::write(&f.file, format!("{}{}", &text[..table_start], &text[table_end..])).unwrap();

    let config = IcbConfig::load(&f.file).unwrap();
    assert!(!config.board.web_admin.enabled);
    assert_eq!(config.board.web_admin.address, "127.0.0.1");
    assert_eq!(config.board.web_admin.port, 8787);
    assert!(!config.board.web_admin.allow_remote);
}


#[test]
fn general_settings_include_web_admin_and_editor_fields() {
    let f = fixture();
    let current = f.service.get_general_settings().unwrap();
    assert!(!current.settings.web_admin_enabled);
    assert_eq!(current.settings.web_admin_address, "127.0.0.1");
    assert_eq!(current.settings.web_admin_port, 8787);
    assert!(!current.settings.web_admin_allow_remote);

    let mut patch = patch_from(&current.settings);
    let editor = if current.settings.sysop_external_editor == "nano" { "vim" } else { "nano" };
    patch.sysop_external_editor = editor.to_string();
    patch.web_admin_enabled = true;
    patch.web_admin_port = 9090;
    patch.web_admin_address = "127.0.0.1".to_string();

    let result = f.service.update_general_settings(&patch, &current.fingerprint, "test").unwrap();
    assert!(result.changed_fields.contains(&"sysop_external_editor".to_string()), "{:?}", result.changed_fields);
    assert!(result.changed_fields.contains(&"web_admin_enabled".to_string()), "{:?}", result.changed_fields);
    assert!(result.changed_fields.contains(&"web_admin_port".to_string()), "{:?}", result.changed_fields);

    let reloaded = IcbConfig::load(&f.file).unwrap();
    assert_eq!(reloaded.sysop.external_editor, editor);
    assert!(reloaded.board.web_admin.enabled);
    assert_eq!(reloaded.board.web_admin.port, 9090);
}

#[test]
fn message_settings_round_trip() {
    let f = fixture();
    let current = f.service.get_message_settings().unwrap();
    let mut patch = current.settings.clone();
    patch.max_msg_lines = current.settings.max_msg_lines.saturating_add(5).max(10);
    patch.allow_esc_codes = !current.settings.allow_esc_codes;
    patch.prompt_to_read_mail = !current.settings.prompt_to_read_mail;

    let result = f
        .service
        .update_message_settings(&patch, &current.fingerprint, "test")
        .unwrap();
    assert!(result.changed_fields.contains(&"max_msg_lines".to_string()));
    assert!(result.changed_fields.contains(&"allow_esc_codes".to_string()));

    let reloaded = f.service.get_message_settings().unwrap();
    assert_eq!(reloaded.settings.max_msg_lines, patch.max_msg_lines);
    assert_eq!(reloaded.settings.allow_esc_codes, patch.allow_esc_codes);
    assert_eq!(reloaded.settings.prompt_to_read_mail, patch.prompt_to_read_mail);
}

#[test]
fn paths_settings_can_update_help_path() {
    let f = fixture();
    let current = f.service.get_paths_settings().unwrap();
    let mut patch = current.settings.clone();
    patch.help_path = "help".to_string();

    f.service
        .update_paths_settings(&patch, &current.fingerprint, "test")
        .unwrap();

    let reloaded = f.service.get_paths_settings().unwrap();
    assert_eq!(reloaded.settings.help_path, "help");
}

#[tokio::test]
async fn live_backend_updates_disk_and_running_board() {
    let f = fixture();
    let mut running_board = IcyBoard::new();
    running_board.config = IcbConfig::load(&f.file).unwrap();
    let running_board = Arc::new(Mutex::new(running_board));
    let backend = LiveAdminBackend::new(&f.file, running_board.clone()).unwrap();

    let current = backend.get_general_settings().await.unwrap();
    let mut patch = current.settings.clone();
    patch.board_name = "Live Board".to_string();
    patch.num_nodes = 12;

    let result = backend.update_general_settings(&patch, &current.fingerprint, "test").await.unwrap();
    assert!(result.changed_fields.contains(&"board_name".to_string()));
    assert_eq!(running_board.lock().await.config.board.name, "Live Board");
    assert_eq!(running_board.lock().await.config.board.num_nodes, 12);

    let saved = IcbConfig::load(&f.file).unwrap();
    assert_eq!(saved.board.name, "Live Board");
    assert_eq!(saved.board.num_nodes, 12);
}

fn conference_fixture() -> (Fixture, PathBuf) {
    let f = fixture();
    let path = f.service.root_path().join("main").join("conferences.toml");
    fs::create_dir_all(path.parent().unwrap()).unwrap();

    let mut base = ConferenceBase::default();
    base.push(Conference {
        name: "Main Board".to_string(),
        is_public: true,
        use_main_commands: true,
        ..Default::default()
    });
    base.push(Conference {
        name: "Second".to_string(),
        ..Default::default()
    });
    base.save(&path).unwrap();
    (f, path)
}

#[test]
fn conferences_are_listed_from_the_conference_file() {
    let (f, path) = conference_fixture();
    let list = f.service.list_conferences().unwrap();

    assert_eq!(list.conferences.len(), 2);
    assert_eq!(list.conferences[0].name, "Main Board");
    assert!(list.conferences[0].is_public);
    assert_eq!(list.conferences[1].index, 1);
    assert_eq!(list.file, path.display().to_string());
    assert!(!list.fingerprint.is_empty());
}

#[test]
fn conference_update_writes_file_backup_and_audit() {
    let (f, path) = conference_fixture();
    let current = f.service.get_conference(0).unwrap();

    let mut patch = current.settings.clone();
    patch.name = "Renamed".to_string();
    patch.required_security = "20".to_string();
    patch.is_read_only = true;

    let result = f.service.update_conference(0, &patch, &current.fingerprint, "test").unwrap();
    assert!(result.changed_fields.contains(&"name".to_string()));
    assert!(result.changed_fields.contains(&"required_security".to_string()));
    assert!(result.backup.is_some());

    let reloaded = ConferenceBase::load(&path).unwrap();
    assert_eq!(reloaded[0].name, "Renamed");
    assert!(reloaded[0].is_read_only);
    assert_eq!(reloaded[0].required_security.to_string(), "20");

    let audit = fs::read_to_string(f.service.root_path().join("icbadmin-audit.log")).unwrap();
    assert!(audit.contains("update_conference"));
}

#[test]
fn conference_can_be_created_and_deleted() {
    let (f, path) = conference_fixture();

    let list = f.service.list_conferences().unwrap();
    let mut patch = f.service.get_conference(0).unwrap().settings;
    patch.name = "Created".to_string();
    f.service.create_conference(&patch, &list.fingerprint, "test").unwrap();

    let after_create = ConferenceBase::load(&path).unwrap();
    assert_eq!(after_create.len(), 3);
    assert_eq!(after_create[2].name, "Created");

    let list = f.service.list_conferences().unwrap();
    f.service.delete_conference(2, &list.fingerprint, "test").unwrap();

    let after_delete = ConferenceBase::load(&path).unwrap();
    assert_eq!(after_delete.len(), 2);
}

#[test]
fn the_last_conference_cannot_be_deleted() {
    let (f, path) = conference_fixture();
    let list = f.service.list_conferences().unwrap();
    f.service.delete_conference(1, &list.fingerprint, "test").unwrap();

    let list = f.service.list_conferences().unwrap();
    let err = f.service.delete_conference(0, &list.fingerprint, "test").unwrap_err();
    assert!(matches!(err, AdminError::Validation(_)));
    assert_eq!(ConferenceBase::load(&path).unwrap().len(), 1);
}

#[test]
fn conference_join_password_is_write_only() {
    let (f, path) = conference_fixture();
    let mut base = ConferenceBase::load(&path).unwrap();
    base[0].password = icy_board_engine::icy_board::user_base::Password::PlainText("joinme".to_string());
    base.save(&path).unwrap();

    let current = f.service.get_conference(0).unwrap();
    assert!(current.password_set);
    let json = serde_json::to_string(&current).unwrap();
    assert!(!json.contains("joinme"));

    let mut patch = current.settings.clone();
    patch.new_password = "newsecret".to_string();
    f.service.update_conference(0, &patch, &current.fingerprint, "test").unwrap();
    assert_eq!(ConferenceBase::load(&path).unwrap()[0].password.to_string(), "newsecret");

    let current = f.service.get_conference(0).unwrap();
    let mut patch = current.settings.clone();
    patch.clear_password = true;
    f.service.update_conference(0, &patch, &current.fingerprint, "test").unwrap();
    assert!(ConferenceBase::load(&path).unwrap()[0].password.is_empty());
}

#[test]
fn invalid_conference_values_are_rejected() {
    let (f, path) = conference_fixture();
    let current = f.service.get_conference(0).unwrap();

    let mut patch = current.settings.clone();
    patch.name = "   ".to_string();
    patch.required_security = "20 &&& ".to_string();

    let err = f.service.update_conference(0, &patch, &current.fingerprint, "test").unwrap_err();
    assert!(matches!(err, AdminError::Validation(_)));
    assert_eq!(ConferenceBase::load(&path).unwrap()[0].name, "Main Board");
}

#[test]
fn conference_index_out_of_range_is_reported_as_missing() {
    let (f, _path) = conference_fixture();
    let err = f.service.get_conference(99).unwrap_err();
    assert!(matches!(err, AdminError::Missing(_)));
}

#[tokio::test]
async fn live_backend_updates_conferences_in_memory_and_on_disk() {
    let (f, path) = conference_fixture();
    let mut running_board = IcyBoard::new();
    running_board.config = IcbConfig::load(&f.file).unwrap();
    running_board.conferences = ConferenceBase::load(&path).unwrap();
    let running_board = Arc::new(Mutex::new(running_board));
    let backend = LiveAdminBackend::new(&f.file, running_board.clone()).unwrap();

    let current = backend.get_conference(1).await.unwrap();
    let mut patch = current.settings.clone();
    patch.name = "Live Conference".to_string();

    let result = backend.update_conference(1, &patch, &current.fingerprint, "test").await.unwrap();
    assert!(result.changed_fields.contains(&"name".to_string()));
    assert_eq!(running_board.lock().await.conferences[1].name, "Live Conference");
    assert_eq!(ConferenceBase::load(&path).unwrap()[1].name, "Live Conference");
}
