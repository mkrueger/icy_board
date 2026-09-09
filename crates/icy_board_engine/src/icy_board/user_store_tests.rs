use super::*;
use crate::icy_board::{
    password_recovery::{RecoveryChallenge, normalize_security, security_fingerprint},
    user_base::{FSEMode, Password, TpaRecord, UserContact},
};
use chrono::{DateTime, TimeZone, Utc};
use std::{path::PathBuf, sync::LazyLock};
use tempfile::TempDir;

fn date(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, day, 12, 0, 0).unwrap()
}

fn session(day: u32) -> UserUpdateMode {
    UserUpdateMode::Session { day: date(day).date_naive() }
}

fn user() -> User {
    static PASSWORD: LazyLock<Password> = LazyLock::new(|| Password::new_argon2("user-store-test"));
    let mut user = User {
        name: "Primary Name".into(),
        email: "caller@example.invalid".into(),
        ..Default::default()
    };
    user.password.password = PASSWORD.clone();
    user.stats.first_date_on = date(1);
    user.stats.last_on = date(8);
    normalize_security(&mut user);
    user
}

fn challenge(user: &User) -> RecoveryChallenge {
    // Deserialization exercises the persisted challenge shape without exposing its internals.
    let Password::Argon2(hash) = &user.password.password else {
        panic!("hashed fixture")
    };
    toml::from_str(&format!(
        "id = 'test-challenge'\nhash = '{hash}'\ncontext = 'test-context'\nrevision = {}\nissued = '2026-09-08T12:00:00Z'\nexpires = '2026-09-08T12:10:00Z'\nattempts = 0\n",
        user.credential_revision,
    )).unwrap()
}

fn fixture() -> (TempDir, IcyBoard) {
    let dir = tempfile::tempdir().unwrap();
    let mut board = IcyBoard::new();
    board.root_path = dir.path().into();
    board.config.paths.user_file = "users.toml".into();
    board.config.password_recovery.enabled = true;
    board.users.new_user(user());
    board.edit_users(|_| Ok(())).unwrap();
    (dir, board)
}

fn snapshot(board: &IcyBoard) -> String {
    toml::to_string(&board.users).unwrap()
}

fn assert_conflict<T>(result: Res<T>, field: &str) {
    let error = result.err().expect("conflict required");
    assert_eq!(
        error.downcast_ref::<UserUpdateError>(),
        Some(&UserUpdateError::Conflict { field: field.into() })
    );
    assert_eq!(error.to_string(), format!("Concurrent user update conflicts with field {field}"));
}

fn assert_identity_error(result: Res<User>, expected: UserUpdateError) {
    let error = result.err().expect("identity error required");
    assert_eq!(error.downcast_ref::<UserUpdateError>(), Some(&expected));
}

#[test]
fn overflowing_account_balance_rejects_the_entire_update() {
    let (dir, mut board) = fixture();
    board.config.accounting.concurrent_tracking = false;
    let baseline = board.users[0].clone();
    let mut edited = baseline.clone();
    edited.city = "not published".into();
    edited.account = Some(AccountUserInf {
        debit_call: f64::MAX,
        debit_time: f64::MAX,
        ..Default::default()
    });
    let before = snapshot(&board);
    let bytes = std::fs::read(dir.path().join("users.toml")).unwrap();
    for mode in [UserUpdateMode::Edit, session(8)] {
        assert!(board.update_user(&baseline, &edited, mode).is_err());
        assert_eq!(snapshot(&board), before);
        assert_eq!(std::fs::read(dir.path().join("users.toml")).unwrap(), bytes);
    }
}

#[test]
fn disjoint_profile_and_individual_flags_merge() {
    let baseline = user();
    let mut edited = baseline.clone();
    edited.city = "Edited city".into();
    edited.flags.expert_mode = true;
    let mut latest = baseline.clone();
    latest.language = "de".into();
    latest.flags.use_graphics = true;
    for mode in [UserUpdateMode::Edit, session(8)] {
        let merged = merge_user(&baseline, &edited, &latest, mode).unwrap();
        assert_eq!(merged.city, "Edited city");
        assert_eq!(merged.language, "de");
        assert!(merged.flags.expert_mode && merged.flags.use_graphics);
        assert_eq!(security_fingerprint(&merged), security_fingerprint(&baseline));
    }
}

#[test]
fn same_field_conflicts_do_not_change_full_base_or_disk() {
    let (dir, mut board) = fixture();
    let baseline = board.users[0].clone();
    board
        .edit_users(|users| {
            users[0].city = "Live private value".into();
            users.new_user(User {
                name: "Other".into(),
                ..user()
            });
            Ok(())
        })
        .unwrap();
    let mut edited = baseline.clone();
    edited.city = "Edited private value".into();
    edited.language = "de".into();
    let before = snapshot(&board);
    let disk = std::fs::read(dir.path().join("users.toml")).unwrap();
    assert_conflict(board.update_user(&baseline, &edited, UserUpdateMode::Edit), "city");
    assert_eq!(snapshot(&board), before);
    assert_eq!(std::fs::read(dir.path().join("users.toml")).unwrap(), disk);
}

#[test]
fn unequal_enum_flags_conflict_but_identical_edits_coalesce() {
    let baseline = user();
    let mut edited = baseline.clone();
    let mut latest = baseline.clone();
    edited.flags.fse_mode = FSEMode::No;
    latest.flags.fse_mode = FSEMode::Ask;
    assert_conflict(merge_user(&baseline, &edited, &latest, UserUpdateMode::Edit), "flags.fse_mode");
    latest.flags.fse_mode = FSEMode::No;
    assert_eq!(
        merge_user(&baseline, &edited, &latest, UserUpdateMode::Edit).unwrap().flags.fse_mode,
        FSEMode::No
    );
}

#[test]
fn stale_security_profile_preserves_authoritative_challenge_and_metadata() {
    let baseline = user();
    let mut edited = baseline.clone();
    edited.city = "New city".into();
    // None of these caller-supplied metadata values may replace authoritative state.
    edited.credential_revision = u64::MAX;
    edited.security_stamp = "forged".into();
    let mut latest = baseline.clone();
    latest.email = "new@example.invalid".into();
    normalize_security(&mut latest);
    latest.recovery = Some(challenge(&latest));
    latest.recovery_issues.push(date(8));
    let merged = merge_user(&baseline, &edited, &latest, UserUpdateMode::Edit).unwrap();
    assert_eq!(merged.city, "New city");
    assert_eq!(security_fingerprint(&merged), security_fingerprint(&latest));
    assert_eq!(merged.credential_revision, latest.credential_revision);
    assert_eq!(merged.security_stamp, latest.security_stamp);
    assert_eq!(toml::to_string(&merged.recovery).unwrap(), toml::to_string(&latest.recovery).unwrap());
    assert_eq!(merged.recovery_issues, latest.recovery_issues);
}

#[test]
fn credential_group_conflicts_and_intentional_change_revokes_challenge() {
    let mut baseline = user();
    baseline.recovery = Some(challenge(&baseline));
    let mut edited = baseline.clone();
    edited.password.password = Password::new_argon2("changed-user-store-test");
    let mut latest = baseline.clone();
    latest.security_level = 10;
    normalize_security(&mut latest);
    assert_conflict(merge_user(&baseline, &edited, &latest, UserUpdateMode::Edit), "credentials");
    let merged = merge_user(&baseline, &edited, &baseline, UserUpdateMode::Edit).unwrap();
    assert!(merged.recovery.is_none());
    assert_eq!(merged.credential_revision, baseline.credential_revision + 1);
    assert!(merged.password.password.is_valid("changed-user-store-test"));
    assert_eq!(merged.security_stamp, security_fingerprint(&merged));
}

#[test]
fn security_group_remains_conservative_even_for_identical_stale_credentials() {
    let baseline = user();
    let mut edited = baseline.clone();
    edited.email = "same-new@example.invalid".into();
    let mut latest = edited.clone();
    normalize_security(&mut latest);
    assert_conflict(merge_user(&baseline, &edited, &latest, UserUpdateMode::Edit), "credentials");
}

#[test]
fn session_deltas_across_two_baselines_refresh_and_retry() {
    let (dir, mut board) = fixture();
    board
        .edit_users(|users| {
            users[0].stats.messages_read = 10;
            users[0].stats.minutes_today = 20;
            users[0].account = Some(AccountUserInf {
                starting_balance: 100.0,
                ..Default::default()
            });
            Ok(())
        })
        .unwrap();
    let first_base = board.users[0].clone();
    let second_base = first_base.clone();
    let mut first = first_base.clone();
    first.stats.messages_read += 2;
    first.stats.minutes_today += 3;
    first.account.as_mut().unwrap().debit_msg_read += 4.0;
    let first_saved = board.update_user(&first_base, &first, session(8)).unwrap();
    let mut second = second_base.clone();
    second.stats.messages_read += 5;
    second.stats.minutes_today += 6;
    second.account.as_mut().unwrap().debit_msg_read += 10.0;

    board.config.paths.user_file = dir.path().into();
    let before = snapshot(&board);
    assert!(board.update_user(&second_base, &second, session(8)).is_err());
    assert_eq!(snapshot(&board), before);
    board.config.paths.user_file = "users.toml".into();
    let second_saved = board.update_user(&second_base, &second, session(8)).unwrap();
    assert_eq!(second_saved.stats.messages_read, 17);
    assert_eq!(second_saved.stats.minutes_today, 29);
    assert_eq!(second_saved.account.as_ref().unwrap().debit_msg_read, 14.0);

    let mut next_first = first_saved.clone();
    next_first.stats.messages_read += 1;
    next_first.account.as_mut().unwrap().debit_msg_read += 2.0;
    let saved = board.update_user(&first_saved, &next_first, session(8)).unwrap();
    assert_eq!(saved.stats.messages_read, 18);
    assert_eq!(saved.account.as_ref().unwrap().debit_msg_read, 16.0);
    let no_change = board.update_user(&saved, &saved, session(8)).unwrap();
    assert_eq!(toml::to_string(&no_change).unwrap(), toml::to_string(&saved).unwrap());
    assert_eq!(UserBase::load(&dir.path().join("users.toml")).unwrap()[0].stats.messages_read, 18);
}

#[test]
fn edit_stats_and_account_are_independent_absolute_fields() {
    let mut baseline = user();
    baseline.stats.messages_read = 10;
    baseline.account = Some(AccountUserInf {
        debit_call: 10.0,
        ..Default::default()
    });
    let mut edited = baseline.clone();
    edited.stats.messages_read = 3;
    edited.account.as_mut().unwrap().debit_call = 2.0;
    let mut latest = baseline.clone();
    latest.stats.messages_left = 20;
    latest.account.as_mut().unwrap().credit_special = 30.0;
    let merged = merge_user(&baseline, &edited, &latest, UserUpdateMode::Edit).unwrap();
    assert_eq!(merged.stats.messages_read, 3);
    assert_eq!(merged.stats.messages_left, 20);
    assert_eq!(merged.account.as_ref().unwrap().debit_call, 2.0);
    assert_eq!(merged.account.as_ref().unwrap().credit_special, 30.0);
    latest.account.as_mut().unwrap().debit_call = 4.0;
    assert_conflict(merge_user(&baseline, &edited, &latest, UserUpdateMode::Edit), "account.debit_call");
    latest.account.as_mut().unwrap().debit_call = 10.0;
    latest.stats.messages_read = 11;
    assert_conflict(merge_user(&baseline, &edited, &latest, UserUpdateMode::Edit), "stats.messages_read");
}

#[test]
fn daily_rollover_adds_only_current_epoch_and_ignores_older_session() {
    let mut baseline = user();
    baseline.stats.last_on = date(7);
    baseline.stats.minutes_today = 60;
    baseline.stats.today_num_downloads = 20;
    baseline.stats.today_dnld_bytes = 1000;
    let mut edited = baseline.clone();
    edited.stats.last_on = date(8);
    edited.stats.minutes_today = 3;
    edited.stats.today_num_downloads = 1;
    edited.stats.today_dnld_bytes = -50;
    let reset = merge_user(&baseline, &edited, &baseline, session(8)).unwrap();
    assert_eq!(reset.stats.minutes_today, 3);
    assert_eq!(reset.stats.today_num_downloads, 1);
    assert_eq!(reset.stats.today_dnld_bytes, -50);
    let mut latest = reset.clone();
    latest.stats.minutes_today = 7;
    latest.stats.today_num_downloads = 2;
    latest.stats.today_dnld_bytes = 100;
    let merged = merge_user(&baseline, &edited, &latest, session(8)).unwrap();
    assert_eq!(merged.stats.minutes_today, 10);
    assert_eq!(merged.stats.today_num_downloads, 3);
    assert_eq!(merged.stats.today_dnld_bytes, 50);
    assert_eq!(merged.stats.last_on, date(8));
    let mut old_session = baseline.clone();
    old_session.stats.minutes_today += 5;
    old_session.stats.today_dnld_bytes += 200;
    old_session.stats.messages_read += 2;
    let merged = merge_user(&baseline, &old_session, &latest, session(7)).unwrap();
    assert_eq!(merged.stats.minutes_today, latest.stats.minutes_today);
    assert_eq!(merged.stats.today_dnld_bytes, latest.stats.today_dnld_bytes);
    assert_eq!(merged.stats.messages_read, 2);
    assert_eq!(merged.stats.last_on, latest.stats.last_on);
}

#[test]
fn daily_latest_old_but_baseline_current_does_not_import_old_counts() {
    let baseline = user();
    let mut edited = baseline.clone();
    edited.stats.today_num_uploads = 2;
    edited.stats.today_upld_bytes = 30;
    let mut latest = baseline.clone();
    latest.stats.last_on = date(7);
    latest.stats.today_num_uploads = 100;
    latest.stats.today_upld_bytes = 1000;
    let saved = merge_user(&baseline, &edited, &latest, session(8)).unwrap();
    assert_eq!(saved.stats.today_num_uploads, 2);
    assert_eq!(saved.stats.today_upld_bytes, 30);
    assert_eq!(saved.stats.last_on.date_naive(), date(8).date_naive());
}

#[test]
fn counter_overflow_saturates_and_signed_credit_uses_wide_delta() {
    let mut baseline = user();
    baseline.stats.today_dnld_bytes = i64::MAX;
    let mut edited = baseline.clone();
    edited.stats.messages_read = u64::MAX;
    edited.stats.minutes_today = u16::MAX;
    edited.stats.today_dnld_bytes = i64::MIN;
    let mut latest = baseline.clone();
    latest.stats.messages_read = 1;
    latest.stats.minutes_today = 1;
    latest.stats.today_dnld_bytes = 0;
    let merged = merge_user(&baseline, &edited, &latest, session(8)).unwrap();
    assert_eq!(merged.stats.messages_read, u64::MAX);
    assert_eq!(merged.stats.minutes_today, u16::MAX);
    assert_eq!(merged.stats.today_dnld_bytes, i64::MIN);
    edited.stats.messages_read = 0;
    edited.stats.today_dnld_bytes = i64::MAX;
    baseline.stats.today_dnld_bytes = i64::MIN;
    assert_eq!(merge_user(&baseline, &edited, &latest, session(8)).unwrap().stats.today_dnld_bytes, i64::MAX);
}

#[test]
fn counter_reductions_are_checked_overrides() {
    let mut baseline = user();
    baseline.stats.messages_read = 10;
    let mut edited = baseline.clone();
    edited.stats.messages_read = 2;
    assert_eq!(merge_user(&baseline, &edited, &baseline, session(8)).unwrap().stats.messages_read, 2);
    let mut latest = baseline.clone();
    latest.stats.messages_read = 11;
    assert_conflict(merge_user(&baseline, &edited, &latest, session(8)), "stats.messages_read");
}

#[test]
fn maps_merge_disjoint_keys_bits_and_pointer_fields() {
    let mut baseline = user();
    baseline.conference_flags.insert(1, ConferenceFlags::Registered);
    baseline.lastread_ptr_flags.insert(
        (1, 0),
        LastReadStatus {
            last_read: 10,
            highest_msg_read: 20,
            include_qwk: true,
        },
    );
    let mut edited = baseline.clone();
    *edited.conference_flags.get_mut(&1).unwrap() |= ConferenceFlags::Selected;
    edited.conference_flags.insert(2, ConferenceFlags::Registered);
    edited.lastread_ptr_flags.get_mut(&(1, 0)).unwrap().last_read = 15;
    edited.lastread_ptr_flags.insert((2, 0), LastReadStatus::default());
    let mut latest = baseline.clone();
    *latest.conference_flags.get_mut(&1).unwrap() |= ConferenceFlags::Expired;
    latest.conference_flags.insert(3, ConferenceFlags::Registered);
    latest.lastread_ptr_flags.get_mut(&(1, 0)).unwrap().include_qwk = false;
    latest.lastread_ptr_flags.insert((3, 0), LastReadStatus::default());
    let merged = merge_user(&baseline, &edited, &latest, UserUpdateMode::Edit).unwrap();
    assert_eq!(
        merged.conference_flags[&1],
        ConferenceFlags::Registered | ConferenceFlags::Selected | ConferenceFlags::Expired
    );
    assert_eq!(merged.conference_flags.len(), 3);
    assert_eq!(merged.lastread_ptr_flags.len(), 3);
    assert_eq!(merged.lastread_ptr_flags[&(1, 0)].last_read, 15);
    assert!(!merged.lastread_ptr_flags[&(1, 0)].include_qwk);
}

#[test]
fn pointers_advance_monotonically_but_backward_changes_conflict() {
    let mut baseline = user();
    baseline.lastread_ptr_flags.insert(
        (0, 0),
        LastReadStatus {
            last_read: 10,
            highest_msg_read: 20,
            include_qwk: true,
        },
    );
    let mut edited = baseline.clone();
    edited.lastread_ptr_flags.get_mut(&(0, 0)).unwrap().last_read = 15;
    edited.lastread_ptr_flags.get_mut(&(0, 0)).unwrap().highest_msg_read = 25;
    let mut latest = baseline.clone();
    latest.lastread_ptr_flags.get_mut(&(0, 0)).unwrap().last_read = 18;
    latest.lastread_ptr_flags.get_mut(&(0, 0)).unwrap().highest_msg_read = 23;
    let merged = merge_user(&baseline, &edited, &latest, session(8)).unwrap();
    assert_eq!(merged.lastread_ptr_flags[&(0, 0)].last_read, 18);
    assert_eq!(merged.lastread_ptr_flags[&(0, 0)].highest_msg_read, 25);
    assert_conflict(merge_user(&baseline, &edited, &latest, UserUpdateMode::Edit), "lastread_ptr_flags.last_read");
    edited.lastread_ptr_flags.get_mut(&(0, 0)).unwrap().last_read = 3;
    assert_conflict(merge_user(&baseline, &edited, &latest, session(8)), "lastread_ptr_flags.last_read");
    assert_eq!(
        merge_user(&baseline, &edited, &baseline, session(8)).unwrap().lastread_ptr_flags[&(0, 0)].last_read,
        3
    );
    edited.lastread_ptr_flags.get_mut(&(0, 0)).unwrap().last_read = 10;
    edited.lastread_ptr_flags.get_mut(&(0, 0)).unwrap().highest_msg_read = 2;
    assert_conflict(merge_user(&baseline, &edited, &latest, session(8)), "lastread_ptr_flags.highest_msg_read");
}

#[test]
fn map_removal_conflicts_with_concurrent_update() {
    let mut baseline = user();
    baseline.conference_flags.insert(0, ConferenceFlags::Registered);
    let mut edited = baseline.clone();
    edited.conference_flags.clear();
    let mut latest = baseline.clone();
    latest.conference_flags.insert(0, ConferenceFlags::Selected);
    assert_conflict(merge_user(&baseline, &edited, &latest, UserUpdateMode::Edit), "conference_flags");
    assert!(
        merge_user(&baseline, &edited, &baseline, UserUpdateMode::Edit)
            .unwrap()
            .conference_flags
            .is_empty()
    );
    baseline.lastread_ptr_flags.insert((0, 0), LastReadStatus::default());
    edited = baseline.clone();
    edited.lastread_ptr_flags.clear();
    latest = baseline.clone();
    latest.lastread_ptr_flags.get_mut(&(0, 0)).unwrap().last_read = 10;
    assert_conflict(merge_user(&baseline, &edited, &latest, session(8)), "lastread_ptr_flags");
}

#[test]
fn atomic_vectors_conflict_without_discarding_concurrent_data() {
    let baseline = user();
    let mut edited = baseline.clone();
    edited.contacts.push(UserContact {
        service: "one".into(),
        account: "edited".into(),
    });
    let mut latest = baseline.clone();
    latest.contacts.push(UserContact {
        service: "two".into(),
        account: "live".into(),
    });
    assert_conflict(merge_user(&baseline, &edited, &latest, UserUpdateMode::Edit), "contacts");
    edited.contacts.clear();
    latest.contacts.clear();
    edited.tpa_records.push(TpaRecord {
        keyword: "one".into(),
        ..Default::default()
    });
    latest.tpa_records.push(TpaRecord {
        keyword: "two".into(),
        ..Default::default()
    });
    assert_conflict(merge_user(&baseline, &edited, &latest, UserUpdateMode::Edit), "tpa_records");
}

#[test]
fn save_failure_rolls_back_normalization_of_every_user_and_can_retry() {
    let (dir, mut board) = fixture();
    let mut other = user();
    other.name = "Other".into();
    other.recovery = Some(challenge(&other));
    // Deliberately leave the stamp stale to make staging normalize this unrelated user.
    board.users.new_user(other);
    let baseline = board.users[0].clone();
    let mut edited = baseline.clone();
    edited.city = "Saved after retry".into();
    let before = snapshot(&board);
    let disk = std::fs::read(dir.path().join("users.toml")).unwrap();
    board.config.paths.user_file = dir.path().into();
    board.config.password_recovery.enabled = false;
    assert!(board.update_user(&baseline, &edited, UserUpdateMode::Edit).is_err());
    assert_eq!(snapshot(&board), before);
    assert!(board.users[1].recovery.is_some());
    assert_eq!(std::fs::read(dir.path().join("users.toml")).unwrap(), disk);
    board.config.paths.user_file = "users.toml".into();
    board.update_user(&baseline, &edited, UserUpdateMode::Edit).unwrap();
    assert_eq!(board.users[0].city, "Saved after retry");
    assert!(board.users[1].recovery.is_none());
    assert_eq!(board.users[1].security_stamp, security_fingerprint(&board.users[1]));
    assert_eq!(UserBase::load(&dir.path().join("users.toml")).unwrap().len(), 2);
}

#[test]
fn staging_closure_error_rolls_back_insert_delete_and_existing_edits() {
    let (dir, mut board) = fixture();
    let before = snapshot(&board);
    let disk = std::fs::read(dir.path().join("users.toml")).unwrap();
    let result: Res<()> = board.edit_users(|users| {
        users[0].city = "not published".into();
        users.clear();
        users.new_user(user());
        Err("deliberate edit failure".into())
    });
    assert!(result.is_err());
    assert_eq!(snapshot(&board), before);
    assert_eq!(std::fs::read(dir.path().join("users.toml")).unwrap(), disk);
}

#[test]
fn identity_lookup_never_uses_alias_casefold_or_replaced_record() {
    let (dir, mut board) = fixture();
    let baseline = board.users[0].clone();
    let mut edited = baseline.clone();
    edited.city = "not saved".into();
    board.users[0].alias = baseline.name.clone();
    board.users[0].name = "Renamed".into();
    let before = snapshot(&board);
    let disk = std::fs::read(dir.path().join("users.toml")).unwrap();
    assert_identity_error(board.update_user(&baseline, &edited, UserUpdateMode::Edit), UserUpdateError::MissingIdentity);
    assert_eq!(snapshot(&board), before);
    assert_eq!(std::fs::read(dir.path().join("users.toml")).unwrap(), disk);
    board.users[0].name = baseline.name.to_lowercase();
    assert!(board.update_user(&baseline, &edited, UserUpdateMode::Edit).is_err());
    board.users[0] = baseline.clone();
    board.users[0].stats.first_date_on = date(2);
    assert_identity_error(board.update_user(&baseline, &edited, UserUpdateMode::Edit), UserUpdateError::IdentityChanged);
    board.users.clear();
    assert_identity_error(board.update_user(&baseline, &edited, UserUpdateMode::Edit), UserUpdateError::MissingIdentity);
}

#[test]
fn duplicate_primary_names_and_rename_collisions_are_rejected() {
    let (_dir, mut board) = fixture();
    let baseline = board.users[0].clone();
    board.users.new_user(baseline.clone());
    let before = snapshot(&board);
    assert_identity_error(
        board.update_user(&baseline, &baseline, UserUpdateMode::Edit),
        UserUpdateError::AmbiguousIdentity,
    );
    assert_eq!(snapshot(&board), before);
    board.users[1].name = "Other".into();
    let mut edited = baseline.clone();
    edited.name = "Other".into();
    assert_identity_error(board.update_user(&baseline, &edited, UserUpdateMode::Edit), UserUpdateError::AmbiguousIdentity);
}

#[test]
fn metadata_persists_and_no_change_saves_are_idempotent_with_argon2() {
    let (dir, mut board) = fixture();
    board
        .edit_users(|users| {
            users[0].recovery = Some(challenge(&users[0]));
            users[0].recovery_issues.push(date(8));
            users[0].path = Some(PathBuf::from("user-data/caller"));
            users[0].tpa_records.push(TpaRecord {
                keyword: "META".into(),
                data: "payload".into(),
                ..Default::default()
            });
            Ok(())
        })
        .unwrap();
    let before = snapshot(&board);
    let disk = std::fs::read(dir.path().join("users.toml")).unwrap();
    for mode in [UserUpdateMode::Edit, session(8), UserUpdateMode::Edit] {
        let baseline = board.users[0].clone();
        let saved = board.update_user(&baseline, &baseline, mode).unwrap();
        assert_eq!(saved.credential_revision, baseline.credential_revision);
        assert_eq!(snapshot(&board), before);
        assert_eq!(std::fs::read(dir.path().join("users.toml")).unwrap(), disk);
    }
    let loaded = UserBase::load(&dir.path().join("users.toml")).unwrap();
    assert_eq!(toml::to_string(&loaded).unwrap(), before);
}

#[test]
fn revoked_or_disabled_recovery_is_removed_only_on_success() {
    for disabled in [false, true] {
        let (dir, mut board) = fixture();
        board.users[0].recovery = Some(challenge(&board.users[0]));
        if disabled {
            board.config.password_recovery.enabled = false;
        } else {
            board.password_recovery_service.revoke_runtime_challenges();
        }
        board.config.paths.user_file = dir.path().into();
        assert!(board.edit_users(|_| Ok(())).is_err());
        assert!(board.users[0].recovery.is_some());
        board.config.paths.user_file = "users.toml".into();
        board.edit_users(|_| Ok(())).unwrap();
        assert!(board.users[0].recovery.is_none());
        assert!(UserBase::load(&dir.path().join("users.toml")).unwrap()[0].recovery.is_none());
    }
}

#[test]
fn accounting_delta_contract_handles_missing_accounts_drop_level_and_invalid_numbers() {
    let baseline = AccountUserInf {
        debit_call: 10.0,
        drop_sec_level: 5,
        ..Default::default()
    };
    let current = AccountUserInf {
        debit_call: 12.0,
        drop_sec_level: 6,
        ..baseline.clone()
    };
    let latest = AccountUserInf {
        debit_call: 13.0,
        drop_sec_level: 7,
        ..baseline.clone()
    };
    let merged = merge_account(Some(&current), Some(&latest), Some(&baseline)).unwrap().unwrap();
    assert_eq!(merged.debit_call, 15.0);
    assert_eq!(merged.drop_sec_level, 6);
    assert_eq!(merge_account(None, Some(&latest), Some(&baseline)).unwrap(), Some(latest.clone()));
    assert_eq!(merge_account(Some(&current), None, None).unwrap(), Some(current.clone()));
    assert_eq!(merge_account(Some(&current), None, Some(&baseline)).unwrap().unwrap().debit_call, 2.0);
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let invalid = AccountUserInf {
            debit_time: invalid,
            ..Default::default()
        };
        assert!(merge_account(Some(&invalid), Some(&latest), Some(&baseline)).is_err());
        assert!(merge_account(Some(&current), Some(&invalid), Some(&baseline)).is_err());
        assert!(merge_account(Some(&current), Some(&latest), Some(&invalid)).is_err());
    }
    let huge = AccountUserInf {
        debit_call: f64::MAX,
        ..Default::default()
    };
    assert!(merge_account(Some(&huge), Some(&huge), None).is_err());
}

#[test]
fn edit_account_presence_conflicts_and_creation_merges_disjoint_fields() {
    let baseline = AccountUserInf {
        debit_call: 10.0,
        ..Default::default()
    };
    let edited = AccountUserInf {
        debit_time: 2.0,
        ..baseline.clone()
    };
    assert_conflict(edit_account(Some(&baseline), None, Some(&edited)), "account");
    assert_eq!(edit_account(Some(&baseline), None, Some(&baseline)).unwrap(), None);
    assert_conflict(edit_account(Some(&baseline), Some(&edited), None), "account");
    let one = AccountUserInf {
        debit_call: 2.0,
        ..Default::default()
    };
    let two = AccountUserInf {
        debit_time: 3.0,
        ..Default::default()
    };
    let merged = edit_account(None, Some(&one), Some(&two)).unwrap().unwrap();
    assert_eq!(merged.debit_call, 2.0);
    assert_eq!(merged.debit_time, 3.0);
}
