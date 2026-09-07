use super::*;
use crate::icy_board::ftn::FtnLink;

fn config(directory: &Path) -> FtnConfig {
    let mut config = FtnConfig {
        inbound: directory.join("inbound"),
        new_file_areas: directory.join("file_areas"),
        links: vec![FtnLink {
            address: EchomailAddress::parse("21:1/1").unwrap(),
            ..Default::default()
        }],
        ..Default::default()
    };
    config.options.auto_add_files = true;
    config
}

fn arrive(config: &FtnConfig, name: &str, content: &[u8], lines: &str) -> PathBuf {
    fs::create_dir_all(&config.inbound).unwrap();
    fs::write(config.inbound.join(name), content).unwrap();
    let tic = config.inbound.join("00000001.tic");
    fs::write(
        &tic,
        format!("File {name}\r\nSize {}\r\nCrc {:08X}\r\n{lines}", content.len(), crc32fast::hash(content)),
    )
    .unwrap();
    tic
}

#[test]
fn registers_once_for_case_insensitive_tags_and_indexes_descriptions() {
    let directory = tempfile::tempdir().unwrap();
    let config = config(directory.path());
    let first = arrive(&config, "INFO1.ZIP", b"first", "Area AGN_INFO\r\nDesc Network information\r\n");
    let first_saved = config.inbound.join("00000000.tic");
    fs::rename(first, &first_saved).unwrap();
    let second = arrive(&config, "INFO2.ZIP", b"second", "Area agn_info\r\n");
    let mut registered = Vec::new();

    let report = toss_tics_with_area_registration(&config, &[], |area| {
        assert!(first_saved.exists());
        assert!(config.inbound.join("INFO1.ZIP").exists());
        registered.push(area.clone());
        Ok(())
    })
    .unwrap();

    assert!(report.failed.is_empty(), "{:?}", report.failed);
    assert!(report.unknown.is_empty());
    assert_eq!(report.arrived.len(), 2);
    assert_eq!(report.added, registered);
    assert_eq!(registered.len(), 1);
    let area = &registered[0];
    assert_eq!(area.tag, "AGN_INFO");
    assert_eq!(area.path, config.new_file_areas.join("agn_info"));
    assert!(!first_saved.exists());
    assert!(!second.exists());
    let mut base = FileBase::open(&area.path, &area.metadata_path).unwrap();
    assert_eq!(base.description(&area.path.join("INFO1.ZIP")).unwrap().as_deref(), Some("Network information"));
    assert!(base.contains_name("INFO2.ZIP"));
}

#[test]
fn does_not_replace_existing_tag_or_name_mappings() {
    for by_name in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let existing = FileArea {
            tag: if by_name { String::new() } else { "AGN_INFO".into() },
            name: if by_name { "AGN_INFO".into() } else { "Information".into() },
            path: directory.path().join("existing"),
            metadata_path: PathBuf::new(),
        };
        arrive(&config, "INFO.ZIP", b"info", "Area agn_info\r\n");
        let report = toss_tics_with_area_registration(&config, &[existing.clone()], |_| panic!("existing area must be reused")).unwrap();
        assert!(report.failed.is_empty());
        assert!(report.added.is_empty());
        assert!(existing.path.join("INFO.ZIP").exists());
        assert!(!config.new_file_areas.exists());
    }
}

#[test]
fn message_auto_add_does_not_enable_file_auto_add() {
    let directory = tempfile::tempdir().unwrap();
    let mut config = config(directory.path());
    config.options.auto_add_files = false;
    config.options.auto_add = true;
    let tic = arrive(&config, "INFO.ZIP", b"info", "Area AGN_INFO\r\n");
    let report = toss_tics(&config, &[]).unwrap();
    assert_eq!(report.unknown.get("AGN_INFO"), Some(&1));
    assert!(report.added.is_empty());
    assert!(tic.exists());
    assert!(config.inbound.join("INFO.ZIP").exists());
}

#[test]
fn validates_before_registering_or_creating_directories() {
    for failure in ["crc", "size", "missing", "source", "password"] {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.options.secure = true;
        config.links[0].tic_password = "secret".into();
        let from = if failure == "source" { "21:1/999" } else { "21:1/1" };
        let password = if failure == "password" { "wrong" } else { "secret" };
        let tic = arrive(&config, "INFO.ZIP", b"info", &format!("Area AGN_INFO\r\nFrom {from}\r\nPw {password}\r\n"));
        let source = config.inbound.join("INFO.ZIP");
        match failure {
            "crc" => fs::write(&source, b"Info").unwrap(),
            "size" => fs::write(&source, b"truncated").unwrap(),
            "missing" => fs::remove_file(&source).unwrap(),
            _ => {}
        }
        let report = toss_tics_with_area_registration(&config, &[], |_| panic!("invalid file must not register an area")).unwrap();
        assert_eq!(report.failed.len(), 1, "{failure}");
        assert!(report.added.is_empty());
        assert!(report.arrived.is_empty());
        assert!(tic.exists());
        assert_eq!(source.exists(), failure != "missing");
        assert!(!config.new_file_areas.exists());
    }
}

#[test]
fn registration_failure_preserves_inputs_for_retry() {
    let directory = tempfile::tempdir().unwrap();
    let config = config(directory.path());
    let tic = arrive(&config, "INFO.ZIP", b"info", "Area AGN_INFO\r\n");
    let report = toss_tics_with_area_registration(&config, &[], |_| Err("cannot save directory list".into())).unwrap();
    assert_eq!(report.failed.len(), 1);
    assert!(report.added.is_empty());
    assert!(tic.exists());
    assert_eq!(fs::read(config.inbound.join("INFO.ZIP")).unwrap(), b"info");
    assert!(!config.new_file_areas.exists());
    let retry = toss_tics(&config, &[]).unwrap();
    assert_eq!(retry.arrived.len(), 1);
    assert!(!tic.exists());
}

#[test]
fn rejects_unsafe_and_overlong_tags() {
    for tag in [
        "",
        ".",
        "..",
        "../escape",
        "/tmp/escape",
        "a/b",
        "a\\b",
        "C:escape",
        "bad\0tag",
        "two words",
        &"A".repeat(65),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let tic = arrive(&config, "INFO.ZIP", b"info", &format!("Area {tag}\r\n"));
        let report = toss_tics_with_area_registration(&config, &[], |_| panic!("invalid tag must not register")).unwrap();
        assert_eq!(report.failed.len(), 1, "{tag:?}");
        assert!(tic.exists());
        assert!(config.inbound.join("INFO.ZIP").exists());
        assert!(!config.new_file_areas.exists());
    }
}

#[test]
fn escapes_punctuation_without_path_collisions() {
    let directory = tempfile::tempdir().unwrap();
    let config = config(directory.path());
    let mut paths = std::collections::HashSet::new();
    for tag in ["AGN.INFO", "AGN%2eINFO", "AGN_INFO"] {
        arrive(&config, "INFO.ZIP", b"info", &format!("Area {tag}\r\n"));
        let report = toss_tics(&config, &[]).unwrap();
        assert!(report.failed.is_empty(), "{:?}", report.failed);
        let area = &report.added[0];
        assert_eq!(area.path.parent(), Some(config.new_file_areas.as_path()));
        assert!(paths.insert(area.path.clone()));
    }
}

#[cfg(unix)]
#[test]
fn refuses_existing_directory_symlinks() {
    let directory = tempfile::tempdir().unwrap();
    let config = config(directory.path());
    fs::create_dir_all(&config.new_file_areas).unwrap();
    let outside = directory.path().join("outside");
    fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, config.new_file_areas.join("agn_info")).unwrap();
    let tic = arrive(&config, "INFO.ZIP", b"info", "Area AGN_INFO\r\n");
    let report = toss_tics_with_area_registration(&config, &[], |_| panic!("symlink must not register")).unwrap();
    assert_eq!(report.failed.len(), 1);
    assert!(tic.exists());
    assert!(config.inbound.join("INFO.ZIP").exists());
    assert_eq!(fs::read_dir(outside).unwrap().count(), 0);
}

#[test]
fn refuses_paths_already_assigned_to_another_area() {
    let directory = tempfile::tempdir().unwrap();
    let config = config(directory.path());
    let existing = FileArea {
        tag: "OTHER".into(),
        name: "Other files".into(),
        path: config.new_file_areas.join("agn_info"),
        metadata_path: PathBuf::new(),
    };
    let tic = arrive(&config, "INFO.ZIP", b"info", "Area AGN_INFO\r\n");
    let report = toss_tics_with_area_registration(&config, &[existing], |_| panic!("conflicting path must not register")).unwrap();
    assert_eq!(report.failed.len(), 1);
    assert!(tic.exists());
    assert!(config.inbound.join("INFO.ZIP").exists());
    assert!(!config.new_file_areas.exists());
}

#[test]
fn disabled_processing_never_registers_areas() {
    for master in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.options.enabled = master;
        config.options.process_in = !master;
        let tic = arrive(&config, "INFO.ZIP", b"info", "Area AGN_INFO\r\n");
        let report = toss_tics_with_area_registration(&config, &[], |_| panic!("processing disabled")).unwrap();
        assert!(report.added.is_empty());
        assert!(report.arrived.is_empty());
        assert!(tic.exists());
        assert!(config.inbound.join("INFO.ZIP").exists());
        assert!(!config.new_file_areas.exists());
    }
}
