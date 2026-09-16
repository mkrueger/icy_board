use std::collections::HashMap;

use chrono::DateTime;
use icy_board_engine::{
    datetime::*,
    icy_board::{PcbUser, icb_config::*, user_base::*, user_inf::*, users::*},
};
use tempfile::TempDir;

fn create_test_user(name: &str, idx: u8) -> User {
    let mut user = User {
        name: name.to_string(),
        alias: format!("Alias{}", idx),
        verify_answer: format!("Answer{}", idx),
        city_or_state: format!("City{}", idx),
        city: format!("City{}", idx),
        state: format!("ST{}", idx),
        street1: format!("123 Street {}", idx),
        street2: format!("Apt {}", idx),
        zip: format!("{:05}", (idx as i32) * 1000),
        country: "USA".to_string(),
        gender: if idx.is_multiple_of(2) { "M" } else { "F" }.to_string(),
        email: format!("user{}@example.com", idx),
        web: format!("http://example{}.com", idx),
        contacts: Vec::new(),
        date_format: DEFAULT_PCBOARD_DATE_FORMAT.to_string(),
        language: "EN".to_string(),
        bus_data_phone: format!("555-{:04}", (idx as u32) * 100),
        home_voice_phone: format!("555-{:04}", (idx as u32) * 101),
        birth_date: IcbDate::new(idx, idx % 12 + 1, 1980 + idx as u16).to_utc_date_time(),
        user_comment: format!("User comment {}", idx),
        sysop_comment: format!("Sysop comment {}", idx),
        custom_comment1: format!("Custom 1-{}", idx),
        custom_comment2: format!("Custom 2-{}", idx),
        custom_comment3: format!("Custom 3-{}", idx),
        custom_comment4: format!("Custom 4-{}", idx),
        custom_comment5: format!("Custom 5-{}", idx),
        password: PasswordInfo {
            password: Password::PlainText(format!("pass{}", idx)),
            prev_pwd: vec![
                Password::PlainText(format!("oldpass1-{}", idx)),
                Password::PlainText(format!("oldpass2-{}", idx)),
            ],
            last_change: DateTime::from_timestamp(1000000 + idx as i64 * 1000, 0).unwrap(),
            times_changed: idx as u64,
            expire_date: DateTime::from_timestamp(2000000 + idx as i64 * 1000, 0).unwrap(),
        },
        credential_revision: 0,
        security_stamp: String::new(),
        recovery: None,
        recovery_issues: Vec::new(),
        security_level: 10 + idx,
        expiration_date: IcbDate::new(12, 31, 2025).to_utc_date_time(),
        exp_security_level: 5 + idx,
        flags: UserFlags {
            expert_mode: idx.is_multiple_of(2),
            is_dirty: idx.is_multiple_of(3),
            msg_clear: idx.is_multiple_of(4),
            has_mail: idx.is_multiple_of(5),
            fse_mode: match idx % 3 {
                0 => FSEMode::Yes,
                1 => FSEMode::No,
                _ => FSEMode::Ask,
            },
            scroll_msg_body: idx % 2 == 1,
            use_short_filedescr: idx % 3 == 1,
            long_msg_header: idx % 4 == 1,
            wide_editor: idx % 5 == 1,
            delete_flag: false,
            disabled_flag: false,
            use_graphics: true,
            use_alias: idx.is_multiple_of(2),
        },
        protocol: match idx % 4 {
            0 => "Z",
            1 => "Y",
            2 => "X",
            _ => "G",
        }
        .to_string(),
        page_len: 20 + idx as u16,
        last_conference: idx as u16 % 10,
        elapsed_time_on: 100 + idx as u16 * 10,
        date_last_dir_read: DateTime::from_timestamp(1500000 + idx as i64 * 1000, 0).unwrap(),
        qwk_config: Some(QwkConfigUserInf {
            max_msgs: 500 + idx as u16 * 10,
            max_msgs_per_conf: 50 + idx as u16,
            personal_attach_limit: 1024 * idx as i32,
            public_attach_limit: 2048 * idx as i32,
            new_blt_limit: idx as i32,
            new_files: idx.is_multiple_of(2),
        }),
        account: if idx.is_multiple_of(2) {
            Some(AccountUserInf {
                starting_balance: 100.0 + idx as f64,
                start_this_session: 10.0 + idx as f64,
                debit_call: 0.5,
                debit_time: 0.1,
                debit_msg_read: 0.01,
                debit_msg_read_capture: 0.02,
                debit_msg_write: 0.05,
                debit_msg_write_echoed: 0.06,
                debit_msg_write_private: 0.07,
                debit_download_file: 1.0,
                debit_download_bytes: 0.001,
                debit_group_chat: 0.5,
                debit_tpu: 0.1,
                debit_special: 0.0,
                credit_upload_file: 2.0,
                credit_upload_bytes: 0.002,
                credit_special: 0.0,
                drop_sec_level: 5,
            })
        } else {
            None
        },
        bank: None,
        tpa_records: Vec::new(),
        stats: UserStats {
            first_date_on: DateTime::from_timestamp(500000 + idx as i64 * 1000, 0).unwrap(),
            last_on: DateTime::from_timestamp(1600000 + idx as i64 * 1000, 0).unwrap(),
            num_times_on: 10 + idx as u64,
            messages_read: 100 + idx as u64 * 10,
            messages_left: 20 + idx as u64 * 2,
            num_sec_viol: idx as u64 % 3,
            num_not_reg: idx as u64 % 2,
            num_reach_dnld_lim: idx as u64 % 4,
            num_file_not_found: idx as u64 % 5,
            num_password_failures: idx as u64 % 6,
            num_verify_errors: idx as u64 % 7,
            num_sysop_pages: idx as u64,
            num_group_chats: idx as u64 * 2,
            num_comments: idx as u64 * 3,
            num_uploads: 5 + idx as u64,
            num_downloads: 10 + idx as u64 * 2,
            total_dnld_bytes: 1024 * 1024 * idx as u64,
            total_upld_bytes: 512 * 1024 * idx as u64,
            today_dnld_bytes: 1024 * idx as i64,
            today_upld_bytes: 512 * idx as u64,
            today_num_downloads: idx as u64,
            today_num_uploads: idx as u64 / 2,
            total_doors_executed: idx as u64 * 5,
            minutes_today: 30 + idx as u16 * 5,
        },
        chat_status: if idx.is_multiple_of(2) {
            ChatStatus::Available
        } else {
            ChatStatus::Unavailable
        },
        conference_flags: HashMap::new(),
        lastread_ptr_flags: HashMap::new(),
        path: None,
    };

    // Add some conference flags
    for conf in 0..(idx % 5) {
        let mut flags = ConferenceFlags::None;
        if conf % 2 == 0 {
            flags |= ConferenceFlags::Registered;
        }
        if conf % 3 == 0 {
            flags |= ConferenceFlags::Selected;
        }
        user.conference_flags.insert(conf as usize, flags);
    }

    // Add some last read pointers
    for conf in 0..(idx % 3) {
        user.lastread_ptr_flags.insert(
            (conf as usize, 0),
            LastReadStatus {
                last_read: 100 + conf as usize * 10,
                highest_msg_read: 200 + conf as usize * 20,
                include_qwk: conf % 2 == 0,
            },
        );
    }

    user
}

#[test]
fn test_export_legacy_users_dates() {
    let temp_dir = TempDir::new().unwrap();
    let users_file = temp_dir.path().join("USERS");
    let users_inf_file = temp_dir.path().join("USERS.INF");
    let mut user = create_test_user("MILO", 1);
    user.stats.first_date_on = DateTime::parse_from_rfc3339("2026-09-07T00:00:00Z").unwrap().to_utc();
    user.stats.last_on = DateTime::parse_from_rfc3339("2026-09-15T12:34:00Z").unwrap().to_utc();
    user.date_last_dir_read = DateTime::parse_from_rfc3339("2026-08-31T00:00:00Z").unwrap().to_utc();
    user.expiration_date = DateTime::parse_from_rfc3339("2027-12-01T00:00:00Z").unwrap().to_utc();

    let mut user_base = UserBase::default();
    user_base.new_user(user.clone());
    user_base.export_pcboard(&users_file, &users_inf_file).unwrap();

    let record = std::fs::read(&users_file).unwrap();
    assert_eq!(record.len(), PcbUserRecord::RECORD_SIZE as usize);
    assert_eq!(&record[87..93], b"260915");
    assert_eq!(&record[101..107], b"260831");
    assert_eq!(&record[185..191], b"271201");
    let assigned = format!(
        "{}-{}-1983",
        std::str::from_utf8(&record[89..91]).unwrap(),
        std::str::from_utf8(&record[91..93]).unwrap(),
    );
    assert_eq!(assigned, "09-15-1983");

    let pcb_users = PcbUserRecord::read_users(&users_file).unwrap();
    assert_eq!(pcb_users[0].last_date_on, IcbDate::from_utc(&user.stats.last_on));
    assert_eq!(pcb_users[0].date_last_dir_read, IcbDate::from_utc(&user.date_last_dir_read));
    assert_eq!(pcb_users[0].exp_date, IcbDate::from_utc(&user.expiration_date));
    let pcb_infs = PcbUserInf::read_users(&users_inf_file).unwrap();
    assert_eq!(
        pcb_infs[0].call_stats.as_ref().unwrap().first_date_on,
        IcbDate::from_utc(&user.stats.first_date_on),
    );
    let imported = UserBase::import_pcboard(&[PcbUser {
        user: pcb_users[0].clone(),
        inf: pcb_infs[0].clone(),
    }]);
    assert_eq!(imported[0].stats.first_date_on, user.stats.first_date_on);
    assert_eq!(imported[0].stats.last_on.date_naive(), user.stats.last_on.date_naive());
}

#[test]
fn test_import_legacy_user_dates() {
    let temp_dir = TempDir::new().unwrap();
    let users_file = temp_dir.path().join("USERS");
    for (stored, expected) in [
        ("830907", IcbDate::new(9, 7, 1983)),
        ("000229", IcbDate::new(2, 29, 2000)),
        ("260915", IcbDate::new(9, 15, 2026)),
        ("000000", IcbDate::default()),
        ("      ", IcbDate::default()),
    ] {
        let mut record = vec![0; PcbUserRecord::RECORD_SIZE as usize];
        record[385..389].copy_from_slice(&1u32.to_le_bytes());
        for offset in [87, 101, 185] {
            record[offset..offset + 6].copy_from_slice(stored.as_bytes());
        }
        std::fs::write(&users_file, record).unwrap();
        let imported = PcbUserRecord::read_users(&users_file).unwrap();
        assert_eq!(imported[0].last_date_on, expected, "last on: {stored}");
        assert_eq!(imported[0].date_last_dir_read, expected, "directory scan: {stored}");
        assert_eq!(imported[0].exp_date, expected, "expiration: {stored}");

        let mut call_stats = [0; 30];
        call_stats[..2].copy_from_slice(&(expected.to_pcboard_date() as u16).to_le_bytes());
        assert_eq!(CallStatsUserInf::read(&call_stats).unwrap().first_date_on, expected, "first on: {stored}");
    }
}

#[test]
fn test_export_import_single_user() {
    let temp_dir = TempDir::new().unwrap();
    let users_file = temp_dir.path().join("USERS");
    let users_inf_file = temp_dir.path().join("USERS.INF");

    // Create a user base with a single user
    let mut user_base = UserBase::default();
    let user = create_test_user("John Doe", 1);
    user_base.new_user(user.clone());

    // Export to PCBoard format
    user_base.export_pcboard(&users_file, &users_inf_file).unwrap();

    // Verify files exist
    assert!(users_file.exists());
    assert!(users_inf_file.exists());

    // Read back the PCBoard files
    let pcb_users = PcbUserRecord::read_users(&users_file).unwrap();
    let pcb_infs = PcbUserInf::read_users(&users_inf_file).unwrap();

    assert_eq!(pcb_users.len(), 1);
    assert_eq!(pcb_infs.len(), 1);

    // Import back and compare
    let pcb_combined = vec![PcbUser {
        user: pcb_users[0].clone(),
        inf: pcb_infs[0].clone(),
    }];
    let imported_base = UserBase::import_pcboard(&pcb_combined);
    let imported_user = &imported_base[0];

    // Verify basic fields
    assert_eq!(imported_user.name, user.name);
    assert_eq!(imported_user.alias, user.alias);
    assert_eq!(imported_user.verify_answer, user.verify_answer);
    assert_eq!(imported_user.city_or_state, user.city_or_state);
    assert_eq!(imported_user.email, user.email);
    assert_eq!(imported_user.web, user.web);
    assert_eq!(imported_user.security_level, user.security_level);
    assert_eq!(imported_user.exp_security_level, user.exp_security_level);

    // Verify password info
    assert_eq!(imported_user.password.password.to_string(), user.password.password.to_string());
    assert_eq!(imported_user.password.prev_pwd.len(), user.password.prev_pwd.len());

    // Verify flags
    assert_eq!(imported_user.flags.expert_mode, user.flags.expert_mode);
    assert_eq!(imported_user.flags.scroll_msg_body, user.flags.scroll_msg_body);
    assert_eq!(imported_user.flags.fse_mode, user.flags.fse_mode);

    // Verify stats
    assert_eq!(imported_user.stats.num_uploads, user.stats.num_uploads);
    assert_eq!(imported_user.stats.num_downloads, user.stats.num_downloads);
    assert_eq!(imported_user.stats.total_dnld_bytes, user.stats.total_dnld_bytes);
}

#[test]
fn test_export_import_multiple_users() {
    let temp_dir = TempDir::new().unwrap();
    let users_file = temp_dir.path().join("USERS");
    let users_inf_file = temp_dir.path().join("USERS.INF");

    // Create a user base with multiple users
    let mut user_base = UserBase::default();
    let mut original_users = Vec::new();

    for i in 1..=5 {
        let user = create_test_user(&format!("User {}", i), i);
        original_users.push(user.clone());
        user_base.new_user(user);
    }

    // Export to PCBoard format
    user_base.export_pcboard(&users_file, &users_inf_file).unwrap();

    // Read back the PCBoard files
    let pcb_users = PcbUserRecord::read_users(&users_file).unwrap();
    let pcb_infs = PcbUserInf::read_users(&users_inf_file).unwrap();

    assert_eq!(pcb_users.len(), 5);
    assert_eq!(pcb_infs.len(), 5);

    // Import back
    let mut pcb_combined = Vec::new();
    for i in 0..5 {
        pcb_combined.push(PcbUser {
            user: pcb_users[i].clone(),
            inf: pcb_infs[i].clone(),
        });
    }
    let imported_base = UserBase::import_pcboard(&pcb_combined);

    // Verify each user
    for (i, original) in original_users.iter().enumerate() {
        let imported = &imported_base[i];

        assert_eq!(imported.name, original.name);
        assert_eq!(imported.alias, original.alias);
        assert_eq!(imported.security_level, original.security_level);
        assert_eq!(imported.protocol, original.protocol);
        assert_eq!(imported.page_len, original.page_len);

        // Check conference flags
        for (conf, flags) in &original.conference_flags {
            if let Some(imported_flags) = imported.conference_flags.get(conf) {
                assert_eq!(
                    imported_flags.contains(ConferenceFlags::Registered),
                    flags.contains(ConferenceFlags::Registered),
                    "Conference {} Registered flag mismatch for user {}",
                    conf,
                    i
                );
                assert_eq!(
                    imported_flags.contains(ConferenceFlags::Selected),
                    flags.contains(ConferenceFlags::Selected),
                    "Conference {} UserSelected flag mismatch for user {}",
                    conf,
                    i
                );
            }
        }
    }
}

#[test]
fn test_conference_flags_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let users_file = temp_dir.path().join("USERS");
    let users_inf_file = temp_dir.path().join("USERS.INF");

    let mut user_base = UserBase::default();
    let mut user = create_test_user("Conference Test", 1);

    // Set specific conference flags
    user.conference_flags.clear();
    user.conference_flags.insert(0, ConferenceFlags::Registered | ConferenceFlags::Selected);
    user.conference_flags.insert(7, ConferenceFlags::Expired);
    user.conference_flags.insert(8, ConferenceFlags::Registered);
    user.conference_flags.insert(15, ConferenceFlags::Selected);
    user.conference_flags.insert(31, ConferenceFlags::Registered | ConferenceFlags::Expired);
    user.conference_flags.insert(39, ConferenceFlags::all());

    user_base.new_user(user.clone());

    // Export and import
    user_base.export_pcboard(&users_file, &users_inf_file).unwrap();
    let pcb_users = PcbUserRecord::read_users(&users_file).unwrap();
    let pcb_infs = PcbUserInf::read_users(&users_inf_file).unwrap();

    let pcb_combined = vec![PcbUser {
        user: pcb_users[0].clone(),
        inf: pcb_infs[0].clone(),
    }];
    let imported_base = UserBase::import_pcboard(&pcb_combined);
    let imported = &imported_base[0];

    // Verify conference flags
    for conf in &[0, 7, 8, 15, 31, 39] {
        let original_flags = user.conference_flags.get(conf).copied().unwrap_or(ConferenceFlags::None);
        let imported_flags = imported.conference_flags.get(conf).copied().unwrap_or(ConferenceFlags::None);

        // Only compare the PCBoard-stored flags
        let mask = ConferenceFlags::Registered | ConferenceFlags::Expired | ConferenceFlags::Selected;
        assert_eq!(original_flags & mask, imported_flags & mask, "Conference {} flags mismatch", conf);
    }
}

#[test]
fn test_lastread_pointers_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let users_file = temp_dir.path().join("USERS");
    let users_inf_file = temp_dir.path().join("USERS.INF");

    let mut user_base = UserBase::default();
    let mut user = create_test_user("LastRead Test", 1);

    // Set specific last read pointers
    user.lastread_ptr_flags.clear();
    user.lastread_ptr_flags.insert(
        (0, 0),
        LastReadStatus {
            last_read: 100,
            highest_msg_read: 150,
            include_qwk: true,
        },
    );
    user.lastread_ptr_flags.insert(
        (5, 0),
        LastReadStatus {
            last_read: 200,
            highest_msg_read: 250,
            include_qwk: false,
        },
    );
    user.lastread_ptr_flags.insert(
        (39, 0),
        LastReadStatus {
            last_read: 999,
            highest_msg_read: 1500,
            include_qwk: true,
        },
    );

    user_base.new_user(user.clone());

    // Export and import
    user_base.export_pcboard(&users_file, &users_inf_file).unwrap();
    let pcb_users = PcbUserRecord::read_users(&users_file).unwrap();
    let pcb_infs = PcbUserInf::read_users(&users_inf_file).unwrap();

    let pcb_combined = vec![PcbUser {
        user: pcb_users[0].clone(),
        inf: pcb_infs[0].clone(),
    }];
    let imported_base = UserBase::import_pcboard(&pcb_combined);
    let imported = &imported_base[0];

    // Verify last read pointers (PCBoard only stores last_read, not highest_msg_read separately)
    for ((conf, area), status) in &user.lastread_ptr_flags {
        if let Some(imported_status) = imported.lastread_ptr_flags.get(&(*conf, *area)) {
            assert_eq!(imported_status.last_read, status.last_read, "Conference {} last_read mismatch", conf);
        }
    }
}

/// PCBoard stores the three byte counters as Basic doubles, so a record written
/// here has to read back the same way in `PCBSM` and in old PPEs.
#[test]
fn test_byte_counters_use_basic_doubles() {
    let temp_dir = TempDir::new().unwrap();
    let users_file = temp_dir.path().join("USERS");
    let users_inf_file = temp_dir.path().join("USERS.INF");

    let mut user = create_test_user("Byte Counters", 1);
    user.stats.total_dnld_bytes = 3_221_225_472;
    user.stats.total_upld_bytes = 1;
    user.stats.today_dnld_bytes = 3;

    let mut user_base = UserBase::default();
    user_base.new_user(user.clone());
    user_base.export_pcboard(&users_file, &users_inf_file).unwrap();

    let record = std::fs::read(&users_file).unwrap();
    // 1.0 and 3.0 in Microsoft Binary Format: exponent 0x81/0x82, no sign bit.
    assert_eq!(&record[216..224], &[0, 0, 0, 0, 0, 0, 0, 0x81]);
    assert_eq!(&record[115..123], &[0, 0, 0, 0, 0, 0, 0x40, 0x82]);

    let pcb_users = PcbUserRecord::read_users(&users_file).unwrap();
    assert_eq!(pcb_users[0].ul_tot_dnld_bytes, user.stats.total_dnld_bytes);
    assert_eq!(pcb_users[0].ul_tot_upld_bytes, user.stats.total_upld_bytes);
    assert_eq!(pcb_users[0].daily_downloaded_bytes as i64, user.stats.today_dnld_bytes);

    for value in [0u64, 1, 2, 3, 255, 1024, 65_535, 1_048_576, 4_294_967_296, 1 << 55] {
        let mut user = create_test_user("Byte Counters", 1);
        user.stats.total_dnld_bytes = value;
        let mut user_base = UserBase::default();
        user_base.new_user(user);
        user_base.export_pcboard(&users_file, &users_inf_file).unwrap();
        assert_eq!(PcbUserRecord::read_users(&users_file).unwrap()[0].ul_tot_dnld_bytes, value, "value {value}");
    }
}

/// `SingleLines` is set when the caller wants short descriptions, matching PCBSM.
#[test]
fn test_short_descriptions_flag_matches_pcboard() {
    let temp_dir = TempDir::new().unwrap();
    let users_file = temp_dir.path().join("USERS");
    let users_inf_file = temp_dir.path().join("USERS.INF");

    for short in [false, true] {
        let mut user = create_test_user("Flag Test", 1);
        user.flags.use_short_filedescr = short;
        user.chat_status = ChatStatus::Available;

        let mut user_base = UserBase::default();
        user_base.new_user(user);
        user_base.export_pcboard(&users_file, &users_inf_file).unwrap();

        let record = std::fs::read(&users_file).unwrap();
        assert_eq!(record[389] & 0b10, if short { 0b10 } else { 0 }, "short descriptions: {short}");
        assert_eq!(record[389] & 0b1, 0, "chat available should clear the UnAvailable bit");
        assert_eq!(PcbUserRecord::read_users(&users_file).unwrap()[0].short_file_descr, short);
    }
}

#[test]
fn test_last_time_on_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let users_file = temp_dir.path().join("USERS");
    let users_inf_file = temp_dir.path().join("USERS.INF");

    let mut user = create_test_user("Time Test", 1);
    user.stats.last_on = DateTime::parse_from_rfc3339("2026-09-15T21:07:00Z").unwrap().to_utc();

    let mut user_base = UserBase::default();
    user_base.new_user(user.clone());
    user_base.export_pcboard(&users_file, &users_inf_file).unwrap();

    let record = std::fs::read(&users_file).unwrap();
    assert_eq!(&record[93..98], b"21:07");

    let pcb_users = PcbUserRecord::read_users(&users_file).unwrap();
    let pcb_infs = PcbUserInf::read_users(&users_inf_file).unwrap();
    let imported = UserBase::import_pcboard(&[PcbUser {
        user: pcb_users[0].clone(),
        inf: pcb_infs[0].clone(),
    }]);
    assert_eq!(imported[0].stats.last_on, user.stats.last_on);
}

#[test]
fn test_message_counts_come_from_users_inf() {
    let temp_dir = TempDir::new().unwrap();
    let users_file = temp_dir.path().join("USERS");
    let users_inf_file = temp_dir.path().join("USERS.INF");

    let mut user = create_test_user("Counts", 1);
    user.stats.num_times_on = 7;
    user.stats.messages_read = 4321;
    user.stats.messages_left = 123;

    let mut user_base = UserBase::default();
    user_base.new_user(user.clone());
    user_base.export_pcboard(&users_file, &users_inf_file).unwrap();

    let pcb_users = PcbUserRecord::read_users(&users_file).unwrap();
    let pcb_infs = PcbUserInf::read_users(&users_inf_file).unwrap();
    let imported = UserBase::import_pcboard(&[PcbUser {
        user: pcb_users[0].clone(),
        inf: pcb_infs[0].clone(),
    }]);
    assert_eq!(imported[0].stats.num_times_on, 7);
    assert_eq!(imported[0].stats.messages_read, 4321);
    assert_eq!(imported[0].stats.messages_left, 123);
}

/// A foreign record may carry a zero USERS.INF pointer; reading it must not panic.
#[test]
fn test_missing_users_inf_pointer_is_tolerated() {
    let temp_dir = TempDir::new().unwrap();
    let users_file = temp_dir.path().join("USERS");
    std::fs::write(&users_file, vec![0; PcbUserRecord::RECORD_SIZE as usize]).unwrap();
    assert_eq!(PcbUserRecord::read_users(&users_file).unwrap()[0].rec_num, 0);
}

/// The optional USERS.INF sections are written whenever any of their fields is
/// set, not only when the string fields are.
#[test]
fn test_optional_inf_sections_keep_their_dates() {
    let temp_dir = TempDir::new().unwrap();
    let users_file = temp_dir.path().join("USERS");
    let users_inf_file = temp_dir.path().join("USERS.INF");

    let mut user = create_test_user("Dates Only", 1);
    user.gender = String::new();
    user.email = String::new();
    user.web = String::new();
    user.birth_date = DateTime::parse_from_rfc3339("1974-03-08T00:00:00Z").unwrap().to_utc();
    user.password.prev_pwd.clear();
    user.password.times_changed = 0;
    user.password.last_change = DateTime::parse_from_rfc3339("2026-01-02T00:00:00Z").unwrap().to_utc();
    user.password.expire_date = DateTime::parse_from_rfc3339("2026-12-31T00:00:00Z").unwrap().to_utc();

    let mut user_base = UserBase::default();
    user_base.new_user(user.clone());
    user_base.export_pcboard(&users_file, &users_inf_file).unwrap();

    let pcb_users = PcbUserRecord::read_users(&users_file).unwrap();
    let pcb_infs = PcbUserInf::read_users(&users_inf_file).unwrap();
    let imported = UserBase::import_pcboard(&[PcbUser {
        user: pcb_users[0].clone(),
        inf: pcb_infs[0].clone(),
    }]);
    assert_eq!(imported[0].birth_date.date_naive(), user.birth_date.date_naive());
    assert_eq!(imported[0].password.expire_date.date_naive(), user.password.expire_date.date_naive());
    assert_eq!(imported[0].password.last_change.date_naive(), user.password.last_change.date_naive());
}
