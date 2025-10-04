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
        gender: if idx % 2 == 0 { "M" } else { "F" }.to_string(),
        email: format!("user{}@example.com", idx),
        web: format!("http://example{}.com", idx),
        date_format: DEFAULT_PCBOARD_DATE_FORMAT.to_string(),
        language: "EN".to_string(),
        bus_data_phone: format!("555-{:04}", (idx as u32) * 100),
        home_voice_phone: format!("555-{:04}", (idx as u32) * 101),
        birth_day: Some(IcbDate::new(idx, idx % 12 + 1, 1980 + idx as u16)),
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
            password_storage_method: PasswordStorageMethod::PlainText,
        },
        security_level: 10 + idx,
        exp_date: IcbDate::new(12, 31, 2025),
        exp_security_level: 5 + idx,
        flags: UserFlags {
            expert_mode: idx % 2 == 0,
            is_dirty: idx % 3 == 0,
            msg_clear: idx % 4 == 0,
            has_mail: idx % 5 == 0,
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
            use_alias: idx % 2 == 0,
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
            new_files: idx % 2 == 0,
        }),
        account: if idx % 2 == 0 {
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
            today_dnld_bytes: 1024 * idx as u64,
            today_upld_bytes: 512 * idx as u64,
            today_num_downloads: idx as u64,
            today_num_uploads: idx as u64 / 2,
            total_doors_executed: idx as u64 * 5,
            minutes_today: 30 + idx as u16 * 5,
        },
        chat_status: if idx % 2 == 0 { ChatStatus::Available } else { ChatStatus::Unavailable },
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
            flags |= ConferenceFlags::UserSelected;
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
                    imported_flags.contains(ConferenceFlags::UserSelected),
                    flags.contains(ConferenceFlags::UserSelected),
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
    user.conference_flags.insert(0, ConferenceFlags::Registered | ConferenceFlags::UserSelected);
    user.conference_flags.insert(7, ConferenceFlags::Expired);
    user.conference_flags.insert(8, ConferenceFlags::Registered);
    user.conference_flags.insert(15, ConferenceFlags::UserSelected);
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
        let mask = ConferenceFlags::Registered | ConferenceFlags::Expired | ConferenceFlags::UserSelected;
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
