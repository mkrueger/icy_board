use super::IcyBoardState;
use crate::icy_board::{
    IcyBoard, IcyBoardSerializer,
    accounting_cfg::AccountingConfig,
    bbs::BBS,
    icb_config::PasswordStorageMethod,
    password_recovery::{MailSender, PasswordRecoveryConfig, RecoveryService, security_fingerprint},
    sec_levels::SecurityLevel,
    user_base::{ConferenceFlags, LastReadStatus, Password, User, UserBase},
    user_inf::AccountUserInf,
    user_store::UserUpdateError,
};
use chrono::{DateTime, TimeZone, Utc};
use icy_net::{ConnectionType, channel::ChannelConnection};
use std::{
    future::{Future, poll_fn},
    pin::Pin,
    sync::{Arc, LazyLock, mpsc},
    task::Poll,
    time::Duration,
};
use tempfile::TempDir;
use tokio::sync::Mutex;

const CALLER: usize = 1;

fn date(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, day, 12, 0, 0).unwrap()
}

struct Fixture {
    root: TempDir,
    first: IcyBoardState,
    second: IcyBoardState,
    _peers: [ChannelConnection; 2],
}

impl Fixture {
    async fn new(last_day: u32, login_days: [u32; 2]) -> Self {
        static PASSWORD: LazyLock<Password> = LazyLock::new(|| Password::new_argon2("old-secret"));
        let root = tempfile::tempdir().unwrap();
        let mut board = IcyBoard::new();
        board.root_path = root.path().into();
        board.config.paths.user_file = root.path().join("users.toml");
        board.config.paths.statistics_file = root.path().join("statistics.toml");
        board.config.paths.caller_log.clear();
        board.config.system_control.password_storage_method = PasswordStorageMethod::Argon2;
        board.config.limits.min_pwd_length = 6;
        board.config.sysop_command_level.sysop = 100;
        board.config.accounting.enabled = true;
        board.config.accounting.ignore_empty_sec_level = false;
        board.config.accounting.concurrent_tracking = false;
        board.config.accounting.info_file.clear();
        board.config.accounting.warning_file.clear();
        board.config.accounting.logoff_file.clear();
        board.config.accounting.tracking_file.clear();
        board.config.accounting.peak_holiday_list_file.clear();
        board.config.accounting.accounting_config = Some(AccountingConfig::default());
        board.sec_levels.clear();
        board.sec_levels.push(SecurityLevel {
            security: 10,
            is_enabled: true,
            time_per_day: 60,
            ..Default::default()
        });
        board.users = UserBase::default();
        board.users.new_user(User {
            name: "Sysop sentinel".into(),
            ..Default::default()
        });
        let mut user = User {
            name: "Session Caller".into(),
            email: "caller@example.invalid".into(),
            city: "Original city".into(),
            security_level: 10,
            account: Some(AccountUserInf {
                starting_balance: 100.0,
                start_this_session: 80.0,
                ..Default::default()
            }),
            ..Default::default()
        };
        user.password.password = PASSWORD.clone();
        user.stats.first_date_on = date(1);
        user.stats.last_on = date(last_day);
        user.stats.num_times_on = 10;
        user.stats.messages_read = 20;
        user.stats.minutes_today = 7;
        user.stats.today_num_downloads = 3;
        user.stats.today_num_uploads = 4;
        user.stats.today_dnld_bytes = 300;
        user.stats.today_upld_bytes = 400;
        user.flags.expert_mode = false;
        user.flags.use_graphics = false;
        board.users.new_user(user);
        board.users.new_user(User {
            name: "Unrelated sentinel".into(),
            city: "Untouched".into(),
            ..Default::default()
        });
        board.edit_users(|_| Ok(())).unwrap();
        let board = Arc::new(Mutex::new(board));
        let bbs = Arc::new(Mutex::new(BBS::new(2)));
        let (first, first_peer) = Self::node(board.clone(), bbs.clone(), login_days[0]).await;
        let (second, second_peer) = Self::node(board, bbs, login_days[1]).await;
        assert_ne!(first.node, second.node);
        assert!(Arc::ptr_eq(&first.board, &second.board));
        Self {
            root,
            first,
            second,
            _peers: [first_peer, second_peer],
        }
    }

    async fn node(board: Arc<Mutex<IcyBoard>>, bbs: Arc<Mutex<BBS>>, day: u32) -> (IcyBoardState, ChannelConnection) {
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (peer, connection) = ChannelConnection::create_pair();
        let mut state = IcyBoardState::new(bbs, board, nodes, node, Box::new(connection)).await;
        state.session.login_date = date(day);
        state.session.disp_options.show_on_screen = false;
        state.session.disp_options.count_lines = false;
        state.set_current_user(CALLER, false).await.unwrap();
        assert!(state.authorize_normal_login().await.unwrap());
        (state, peer)
    }

    fn disk(&self) -> UserBase {
        UserBase::load(&self.root.path().join("users.toml")).unwrap()
    }

    async fn assert_disk_matches_board(&self) {
        let disk = self.disk();
        let board = self.first.board.lock().await;
        assert_eq!(disk.len(), board.users.len());
        for (disk, live) in disk.iter().zip(board.users.iter()) {
            assert_user_eq(disk, live);
        }
    }
}

fn local(state: &IcyBoardState) -> &User {
    state.session.current_user.as_ref().unwrap()
}

fn edit(state: &mut IcyBoardState) -> &mut User {
    state.session.current_user.as_mut().unwrap()
}

fn assert_user_eq(actual: &User, expected: &User) {
    assert_eq!(actual.conference_flags, expected.conference_flags);
    assert_eq!(actual.lastread_ptr_flags, expected.lastread_ptr_flags);
    // Map serializers emit unordered strings, and hashed Password::eq is not reflexive.
    let mut actual = actual.clone();
    let mut expected = expected.clone();
    actual.conference_flags.clear();
    expected.conference_flags.clear();
    actual.lastread_ptr_flags.clear();
    expected.lastread_ptr_flags.clear();
    assert_eq!(toml::Value::try_from(actual).unwrap(), toml::Value::try_from(expected).unwrap());
}

fn assert_refreshed(state: &IcyBoardState) {
    assert_user_eq(state.session.user_baseline.as_ref().unwrap(), local(state));
    assert_user_eq(state.session.security_baseline.as_ref().unwrap(), local(state));
    let mut accounting_baseline = local(state).clone();
    state.accounting_set_update_baseline(&mut accounting_baseline);
    assert_eq!(accounting_baseline.account, local(state).account);
}

#[derive(Debug, PartialEq)]
struct SessionSnapshot {
    pending: String,
    baseline: String,
    security: String,
    accounting_baseline: String,
    authenticated: Option<(u64, String)>,
    saved_minutes: i64,
    request_logoff: bool,
}

fn session_snapshot(state: &IcyBoardState) -> SessionSnapshot {
    let mut accounting_baseline = local(state).clone();
    state.accounting_set_update_baseline(&mut accounting_baseline);
    SessionSnapshot {
        pending: toml::to_string(local(state)).unwrap(),
        baseline: toml::to_string(state.session.user_baseline.as_ref().unwrap()).unwrap(),
        security: toml::to_string(state.session.security_baseline.as_ref().unwrap()).unwrap(),
        accounting_baseline: toml::to_string(&accounting_baseline).unwrap(),
        authenticated: state.session.authenticated_security.clone(),
        saved_minutes: state.session.saved_session_minutes,
        request_logoff: state.session.request_logoff,
    }
}

const SAVE_DEADLINE: Duration = Duration::from_secs(10);

async fn poll_pending<F: Future>(mut future: Pin<&mut F>) {
    poll_fn(|cx| {
        assert!(future.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
}

async fn cancel_at_file_sync(state: &mut IcyBoardState) -> mpsc::Sender<()> {
    let (entered_tx, entered) = tokio::sync::oneshot::channel();
    let (release, released) = mpsc::channel();
    IcyBoard::ordered_persistence(&state.board, move || {
        crate::icy_board::BEFORE_FILE_SYNC.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move || {
                entered_tx.send(()).unwrap();
                released.recv_timeout(SAVE_DEADLINE).unwrap();
            }));
        });
    })
    .await
    .unwrap();
    let mut save = Box::pin(state.persist_current_user());
    tokio::select! {
        result = &mut save => panic!("save completed before cancellation: {result:?}"),
        result = tokio::time::timeout(SAVE_DEADLINE, entered) => result.unwrap().unwrap(),
    }
    drop(save);
    assert!(state.pending_user_save.is_some());
    release
}

#[tokio::test]
async fn cancelled_state_save_rebases_later_edits_and_posts_activity_once() {
    for (login_day, finalize) in [(8, false), (8, true), (9, false), (9, true)] {
        let mut f = Fixture::new(8, [login_day, login_day]).await;
        f.first.accounting_start().await.unwrap();
        f.second.accounting_start().await.unwrap();
        edit(&mut f.second).web = "https://example.invalid/other-node".into();
        edit(&mut f.second).stats.messages_read += 5;
        f.second.accounting_record(16, "CREDIT", "", 30.0, 1).unwrap();
        f.second.persist_current_user().await.unwrap();
        edit(&mut f.first).city = "Submitted city".into();
        edit(&mut f.first).stats.messages_read += 2;
        edit(&mut f.first).stats.today_num_downloads += 2;
        f.first.accounting_record(4, "READ", "", 2.0, 1).unwrap();
        let baseline = session_snapshot(&f.first);
        let release = cancel_at_file_sync(&mut f.first).await;
        assert_eq!(session_snapshot(&f.first), baseline);
        assert_eq!(f.disk()[CALLER].stats.messages_read, 25);
        edit(&mut f.first).city = "Later city".into();
        edit(&mut f.first).stats.messages_read += 3;
        edit(&mut f.first).stats.today_num_downloads += 3;
        f.first.accounting_record(4, "READ", "", 3.0, 1).unwrap();
        f.first.accounting_record(16, "CREDIT", "", 4.0, 1).unwrap();
        release.send(()).unwrap();
        tokio::time::timeout(SAVE_DEADLINE, IcyBoard::flush_persistence(&f.first.board))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(f.disk()[CALLER].stats.messages_read, 27);
        assert_eq!(f.disk()[CALLER].account.as_ref().unwrap().debit_msg_read, 2.0);
        if finalize {
            f.first.accounting_finish().await.unwrap();
            f.first.accounting_finish().await.unwrap();
        } else {
            f.first.persist_current_user().await.unwrap();
        }
        f.first.persist_current_user().await.unwrap();
        let disk = f.disk();
        assert_eq!(disk[CALLER].city, "Later city");
        assert_eq!(disk[CALLER].web, "https://example.invalid/other-node");
        assert_eq!(disk[CALLER].stats.num_times_on, 12);
        assert_eq!(disk[CALLER].stats.messages_read, 30);
        assert_eq!(disk[CALLER].stats.today_num_downloads, if login_day == 8 { 8 } else { 5 });
        let account = disk[CALLER].account.as_ref().unwrap();
        assert_eq!(account.debit_msg_read, 5.0);
        assert_eq!(account.credit_special, 34.0);
        let account = local(&f.first).account.as_ref().unwrap();
        assert_eq!(account.start_this_session, 100.0);
        assert_eq!(account.credit_special, 4.0);
        assert_eq!(account.debit_msg_read, 5.0);
        assert_eq!(f.first.session.calculate_balance(), 99.0);
        assert_refreshed(&f.first);
        f.assert_disk_matches_board().await;
    }
}

#[tokio::test]
async fn cancelled_save_does_not_reauthorize_credentials_changed_before_acknowledgement() {
    let mut f = Fixture::new(8, [8, 8]).await;
    f.first.accounting_start().await.unwrap();
    let authorization = f.first.session.authenticated_security.clone();
    edit(&mut f.first).stats.messages_read += 2;
    f.first.accounting_record(4, "READ", "", 2.0, 1).unwrap();
    let release = cancel_at_file_sync(&mut f.first).await;
    edit(&mut f.first).email = "later-local@example.invalid".into();
    edit(&mut f.first).stats.messages_read += 3;
    f.first.accounting_record(4, "READ", "", 3.0, 1).unwrap();
    release.send(()).unwrap();
    IcyBoard::write_users(&f.first.board, |board| {
        board.edit_users(|users| {
            users[CALLER].email = "authoritative@example.invalid".into();
            users[CALLER].flags.disabled_flag = true;
            Ok(())
        })
    })
    .await
    .unwrap();
    let live = security_fingerprint(&f.disk()[CALLER]);
    assert!(f.first.persist_current_user().await.is_err());
    f.first.accounting_finish().await.unwrap();
    f.first.accounting_finish().await.unwrap();
    assert_eq!(security_fingerprint(&f.disk()[CALLER]), live);
    assert_eq!(f.first.session.authenticated_security, authorization);
    assert!(f.first.session.request_logoff);
    assert!(!f.first.credentials_still_current().await);
    assert_eq!(f.disk()[CALLER].stats.messages_read, 25);
    assert_eq!(f.disk()[CALLER].account.as_ref().unwrap().debit_msg_read, 5.0);
    f.assert_disk_matches_board().await;
}

#[tokio::test]
async fn cancelled_failed_save_keeps_uncommitted_edits_for_retry() {
    let mut f = Fixture::new(8, [8, 8]).await;
    f.first.accounting_start().await.unwrap();
    edit(&mut f.first).city = "Submitted city".into();
    edit(&mut f.first).stats.messages_read += 2;
    f.first.accounting_record(4, "READ", "", 2.0, 1).unwrap();
    let baseline = f.first.session.user_baseline.clone().unwrap();
    let release = cancel_at_file_sync(&mut f.first).await;
    edit(&mut f.first).city = "Later city".into();
    edit(&mut f.first).stats.messages_read += 3;
    f.first.accounting_record(4, "READ", "", 3.0, 1).unwrap();
    let path = f.root.path().join("users.toml");
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    release.send(()).unwrap();
    let snapshot = session_snapshot(&f.first);
    assert!(f.first.persist_current_user().await.is_err());
    assert!(f.first.pending_user_save.is_none());
    assert_eq!(session_snapshot(&f.first), snapshot);
    assert_user_eq(f.first.session.user_baseline.as_ref().unwrap(), &baseline);
    assert_eq!(f.first.get_board().await.users[CALLER].stats.messages_read, 20);
    std::fs::remove_dir(&path).unwrap();
    f.first.persist_current_user().await.unwrap();
    f.first.accounting_finish().await.unwrap();
    assert_eq!(f.disk()[CALLER].city, "Later city");
    assert_eq!(f.disk()[CALLER].stats.messages_read, 25);
    assert_eq!(f.disk()[CALLER].account.as_ref().unwrap().debit_msg_read, 5.0);
    assert_refreshed(&f.first);
    f.assert_disk_matches_board().await;
}

#[tokio::test]
async fn cancelled_save_retains_successful_receipt_through_rebase_conflict() {
    let mut f = Fixture::new(8, [8, 8]).await;
    f.first.accounting_start().await.unwrap();
    edit(&mut f.second).city = "Other node city".into();
    f.second.persist_current_user().await.unwrap();
    edit(&mut f.first).stats.messages_read += 2;
    f.first.accounting_record(4, "READ", "", 2.0, 1).unwrap();
    let release = cancel_at_file_sync(&mut f.first).await;
    edit(&mut f.first).city = "Later conflicting city".into();
    edit(&mut f.first).web = "https://example.invalid/later".into();
    edit(&mut f.first).stats.messages_read += 3;
    f.first.accounting_record(4, "READ", "", 3.0, 1).unwrap();
    release.send(()).unwrap();
    for _ in 0..2 {
        let error = f.first.persist_current_user().await.unwrap_err();
        assert_eq!(
            error.downcast_ref::<UserUpdateError>(),
            Some(&UserUpdateError::Conflict { field: "city".into() })
        );
        assert!(f.first.pending_user_save.as_ref().unwrap().saved.is_some());
        assert_eq!(f.disk()[CALLER].stats.messages_read, 22);
    }
    f.first.accounting_finish().await.unwrap();
    f.first.persist_final_user().await.unwrap();
    assert_eq!(f.disk()[CALLER].city, "Other node city");
    assert_eq!(f.disk()[CALLER].web, "https://example.invalid/later");
    assert_eq!(f.disk()[CALLER].stats.messages_read, 25);
    assert_eq!(f.disk()[CALLER].account.as_ref().unwrap().debit_msg_read, 5.0);
    assert_refreshed(&f.first);
    f.assert_disk_matches_board().await;
}

#[tokio::test]
async fn cancelled_before_queue_admission_resumes_the_same_submission_once() {
    let mut f = Fixture::new(8, [8, 8]).await;
    f.first.accounting_start().await.unwrap();
    let board = f.first.board.clone();
    let (entered_tx, entered) = tokio::sync::oneshot::channel();
    let (release, released) = mpsc::channel();
    let mut blocker = Box::pin(IcyBoard::ordered_persistence(&board, move || {
        entered_tx.send(()).unwrap();
        released.recv_timeout(SAVE_DEADLINE).unwrap();
    }));
    poll_pending(blocker.as_mut()).await;
    tokio::time::timeout(SAVE_DEADLINE, entered).await.unwrap().unwrap();
    let mut queued = Vec::new();
    for _ in 0..32 {
        let mut request = Box::pin(IcyBoard::ordered_persistence(&board, || ()));
        poll_pending(request.as_mut()).await;
        queued.push(request);
    }
    edit(&mut f.first).city = "Submitted city".into();
    edit(&mut f.first).stats.messages_read += 2;
    f.first.accounting_record(4, "READ", "", 2.0, 1).unwrap();
    for _ in 0..2 {
        let mut save = Box::pin(f.first.persist_current_user());
        poll_pending(save.as_mut()).await;
        drop(save);
        assert!(f.first.pending_user_save.as_ref().unwrap().operation.is_some());
    }
    edit(&mut f.first).city = "Later city".into();
    edit(&mut f.first).stats.messages_read += 3;
    f.first.accounting_record(4, "READ", "", 3.0, 1).unwrap();
    release.send(()).unwrap();
    tokio::time::timeout(SAVE_DEADLINE, blocker).await.unwrap().unwrap();
    for request in queued {
        tokio::time::timeout(SAVE_DEADLINE, request).await.unwrap().unwrap();
    }
    assert_eq!(f.disk()[CALLER].stats.messages_read, 20);
    let revision = f.first.get_board().await.user_revision;
    f.first.accounting_finish().await.unwrap();
    assert_eq!(f.first.get_board().await.user_revision, revision + 2);
    f.first.accounting_finish().await.unwrap();
    assert_eq!(f.disk()[CALLER].city, "Later city");
    assert_eq!(f.disk()[CALLER].stats.messages_read, 25);
    assert_eq!(f.disk()[CALLER].account.as_ref().unwrap().debit_msg_read, 5.0);
    assert_refreshed(&f.first);
    f.assert_disk_matches_board().await;
}

#[tokio::test]
async fn switching_user_settles_cancelled_save_and_later_edits_without_active_accounting() {
    let mut f = Fixture::new(8, [8, 8]).await;
    edit(&mut f.first).stats.messages_read += 2;
    let release = cancel_at_file_sync(&mut f.first).await;
    edit(&mut f.first).city = "Later city".into();
    edit(&mut f.first).stats.messages_read += 3;
    release.send(()).unwrap();
    f.first.set_current_user(2, false).await.unwrap();
    assert!(f.first.pending_user_save.is_none());
    assert_eq!(f.first.session.cur_user_id, 2);
    assert_eq!(local(&f.first).name, "Unrelated sentinel");
    assert_eq!(local(&f.first).stats.messages_read, 0);
    assert_eq!(f.disk()[CALLER].city, "Later city");
    assert_eq!(f.disk()[CALLER].stats.messages_read, 25);
    assert_eq!(f.disk()[CALLER].stats.num_times_on, 11);
    f.assert_disk_matches_board().await;
}

#[tokio::test]
async fn finalization_retries_cancelled_failed_save_without_started_accounting() {
    let mut f = Fixture::new(8, [8, 8]).await;
    edit(&mut f.first).stats.messages_read += 2;
    let release = cancel_at_file_sync(&mut f.first).await;
    edit(&mut f.first).city = "Later city".into();
    edit(&mut f.first).stats.messages_read += 3;
    let path = f.root.path().join("users.toml");
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    release.send(()).unwrap();
    assert!(f.first.accounting_finish().await.is_err());
    assert!(f.first.pending_user_save.is_none());
    assert!(f.first.accounting_finish().await.is_err());
    std::fs::remove_dir(&path).unwrap();
    f.first.accounting_finish().await.unwrap();
    let revision = f.first.get_board().await.user_revision;
    f.first.accounting_finish().await.unwrap();
    assert_eq!(f.first.get_board().await.user_revision, revision);
    assert_eq!(f.disk()[CALLER].city, "Later city");
    assert_eq!(f.disk()[CALLER].stats.messages_read, 25);
    assert_eq!(f.disk()[CALLER].stats.num_times_on, 11);
    assert_refreshed(&f.first);
    f.assert_disk_matches_board().await;
}

#[tokio::test]
async fn independent_profile_edits_adopt_saved_profile_and_refresh_both_baselines() {
    let mut f = Fixture::new(8, [8, 8]).await;
    assert_eq!(f.first.session.user_baseline.as_ref().unwrap().stats.num_times_on, 10);
    assert_eq!(local(&f.first).stats.num_times_on, 11);
    edit(&mut f.first).city = "First city".into();
    edit(&mut f.second).web = "https://example.invalid/second".into();
    f.first.persist_current_user().await.unwrap();
    f.second.persist_current_user().await.unwrap();
    assert_eq!(local(&f.second).city, "First city");
    assert_eq!(local(&f.second).web, "https://example.invalid/second");
    assert_eq!(local(&f.second).stats.num_times_on, 12);
    assert_refreshed(&f.second);
    f.first.persist_current_user().await.unwrap();
    assert_user_eq(local(&f.first), local(&f.second));
    assert_refreshed(&f.first);
    f.assert_disk_matches_board().await;
}

#[tokio::test]
async fn statistics_and_local_account_deltas_are_posted_once_across_repeated_saves() {
    let mut f = Fixture::new(8, [8, 8]).await;
    f.first.accounting_start().await.unwrap();
    f.second.accounting_start().await.unwrap();
    edit(&mut f.first).stats.messages_read += 2;
    edit(&mut f.second).stats.messages_read += 5;
    f.first.accounting_record(4, "READ", "", 2.0, 1).unwrap();
    f.second.accounting_record(4, "READ", "", 5.0, 1).unwrap();
    f.second.accounting_record(16, "CREDIT", "", 4.0, 1).unwrap();
    f.first.persist_current_user().await.unwrap();
    f.second.persist_current_user().await.unwrap();
    f.first
        .board
        .lock()
        .await
        .edit_users(|users| {
            users[CALLER].account.as_mut().unwrap().credit_special += 30.0;
            Ok(())
        })
        .unwrap();
    for _ in 0..3 {
        f.first.persist_current_user().await.unwrap();
        f.second.persist_current_user().await.unwrap();
        assert_refreshed(&f.first);
        assert_refreshed(&f.second);
    }
    for state in [&f.first, &f.second] {
        assert!(state.accounting_active());
        assert_eq!(local(state).account.as_ref().unwrap().start_this_session, 100.0);
        assert_eq!(local(state).stats.messages_read, 27);
        assert_eq!(local(state).stats.num_times_on, 12);
    }
    assert_eq!(local(&f.first).account.as_ref().unwrap().credit_special, 0.0);
    assert_eq!(local(&f.second).account.as_ref().unwrap().credit_special, 4.0);
    assert_eq!(f.first.session.calculate_balance(), 98.0);
    assert_eq!(f.second.session.calculate_balance(), 99.0);
    edit(&mut f.first).stats.messages_read += 1;
    edit(&mut f.second).stats.messages_read += 3;
    f.first.accounting_record(4, "READ", "", 3.0, 1).unwrap();
    f.second.accounting_record(4, "READ", "", 1.0, 1).unwrap();
    f.first.persist_current_user().await.unwrap();
    f.second.persist_current_user().await.unwrap();
    f.first.persist_current_user().await.unwrap();
    f.second.persist_current_user().await.unwrap();
    let disk = f.disk();
    assert_eq!(disk[CALLER].stats.messages_read, 31);
    assert_eq!(disk[CALLER].stats.num_times_on, 12);
    let account = disk[CALLER].account.as_ref().unwrap();
    assert_eq!(account.debit_msg_read, 11.0);
    assert_eq!(account.credit_special, 34.0);
    assert_eq!(account.balance(false, 0.0), 123.0);
    f.assert_disk_matches_board().await;
}

#[tokio::test]
async fn same_field_conflict_preserves_pending_edits_and_retry_posts_only_once() {
    let mut f = Fixture::new(8, [8, 8]).await;
    f.first.accounting_start().await.unwrap();
    f.second.accounting_start().await.unwrap();
    edit(&mut f.first).city = "First city".into();
    edit(&mut f.second).city = "Second city".into();
    edit(&mut f.second).web = "https://example.invalid/pending".into();
    edit(&mut f.first).stats.messages_read += 2;
    edit(&mut f.second).stats.messages_read += 5;
    f.first.accounting_record(4, "READ", "", 2.0, 1).unwrap();
    f.second.accounting_record(4, "READ", "", 5.0, 1).unwrap();
    f.first.persist_current_user().await.unwrap();
    let board_before = toml::to_string(&f.first.board.lock().await.users).unwrap();
    let disk_before = std::fs::read(f.root.path().join("users.toml")).unwrap();
    let pending = session_snapshot(&f.second);
    let first_before = session_snapshot(&f.first);
    for _ in 0..2 {
        let error = f.second.persist_current_user().await.unwrap_err();
        assert_eq!(
            error.downcast_ref::<UserUpdateError>(),
            Some(&UserUpdateError::Conflict { field: "city".into() })
        );
        assert_eq!(toml::to_string(&f.first.board.lock().await.users).unwrap(), board_before);
        assert_eq!(std::fs::read(f.root.path().join("users.toml")).unwrap(), disk_before);
        assert_eq!(session_snapshot(&f.second), pending);
        assert_eq!(session_snapshot(&f.first), first_before);
    }
    // Resolve just the conflicting field, retaining all unposted session deltas.
    edit(&mut f.second).city = "First city".into();
    f.second.persist_current_user().await.unwrap();
    f.second.persist_current_user().await.unwrap();
    f.first.persist_current_user().await.unwrap();
    let disk = f.disk();
    assert_eq!(disk[CALLER].city, "First city");
    assert_eq!(disk[CALLER].web, "https://example.invalid/pending");
    assert_eq!(disk[CALLER].stats.num_times_on, 12);
    assert_eq!(disk[CALLER].stats.messages_read, 27);
    assert_eq!(disk[CALLER].account.as_ref().unwrap().debit_msg_read, 7.0);
    assert_eq!(local(&f.second).account.as_ref().unwrap().debit_msg_read, 5.0);
    assert_refreshed(&f.second);
    f.assert_disk_matches_board().await;
}

#[tokio::test]
async fn final_save_keeps_live_conflicts_and_posts_activity_once_even_after_io_retry() {
    for explicit_logoff in [false, true] {
        let mut f = Fixture::new(8, [8, 8]).await;
        f.first.accounting_start().await.unwrap();
        f.second.accounting_start().await.unwrap();
        edit(&mut f.first).city = "First city".into();
        edit(&mut f.second).city = "Second city".into();
        edit(&mut f.second).web = "https://example.invalid/pending".into();
        edit(&mut f.first).stats.messages_read += 2;
        edit(&mut f.second).stats.messages_read += 5;
        f.first.accounting_record(4, "READ", "", 2.0, 1).unwrap();
        f.second.accounting_record(4, "READ", "", 5.0, 1).unwrap();
        f.first.persist_current_user().await.unwrap();
        assert!(f.second.persist_current_user().await.is_err());
        let before = std::fs::read(f.root.path().join("users.toml")).unwrap();
        let baseline = f.second.session.user_baseline.clone().unwrap();
        f.second.get_board().await.config.paths.user_file = f.root.path().into();
        f.second.session.request_logoff = explicit_logoff;
        let result = if explicit_logoff {
            f.second.save_current_user().await
        } else {
            f.second.accounting_finish().await
        };
        assert!(result.is_err());
        assert_eq!(std::fs::read(f.root.path().join("users.toml")).unwrap(), before);
        assert_user_eq(f.second.session.user_baseline.as_ref().unwrap(), &baseline);
        assert_eq!(local(&f.second).city, "Second city");
        f.assert_disk_matches_board().await;

        f.second.get_board().await.config.paths.user_file = f.root.path().join("users.toml");
        if explicit_logoff {
            f.second.save_current_user().await.unwrap();
        }
        f.second.accounting_finish().await.unwrap();
        f.second.accounting_finish().await.unwrap();
        f.second.persist_final_user().await.unwrap();
        let disk = f.disk();
        assert_eq!(disk[CALLER].city, "First city");
        assert_eq!(disk[CALLER].web, "https://example.invalid/pending");
        assert_eq!(disk[CALLER].stats.messages_read, 27);
        assert_eq!(disk[CALLER].stats.num_times_on, 12);
        assert_eq!(disk[CALLER].account.as_ref().unwrap().debit_msg_read, 7.0);
        assert_refreshed(&f.second);
        f.assert_disk_matches_board().await;
    }
}

#[tokio::test]
async fn final_save_preserves_changed_credentials_without_reauthorizing_the_stale_session() {
    let mut f = Fixture::new(8, [8, 8]).await;
    f.second.accounting_start().await.unwrap();
    let authorization = f.second.session.authenticated_security.clone();
    edit(&mut f.second).email = "local@example.invalid".into();
    edit(&mut f.second).stats.messages_read += 3;
    f.second.accounting_record(4, "READ", "", 3.0, 1).unwrap();
    f.first
        .get_board()
        .await
        .edit_users(|users| {
            users[CALLER].email = "authoritative@example.invalid".into();
            users[CALLER].flags.disabled_flag = true;
            Ok(())
        })
        .unwrap();
    let live_security = security_fingerprint(&f.disk()[CALLER]);
    assert!(f.second.persist_current_user().await.is_err());
    f.second.accounting_finish().await.unwrap();
    f.second.accounting_finish().await.unwrap();
    let disk = f.disk();
    assert_eq!(security_fingerprint(&disk[CALLER]), live_security);
    assert_eq!(disk[CALLER].stats.messages_read, 23);
    assert_eq!(disk[CALLER].account.as_ref().unwrap().debit_msg_read, 3.0);
    assert_eq!(f.second.session.authenticated_security, authorization);
    assert!(f.second.session.request_logoff);
}

#[tokio::test]
async fn final_save_does_not_suppress_invalid_money_or_replaced_identity() {
    let mut f = Fixture::new(8, [8, 8]).await;
    f.second.accounting_start().await.unwrap();
    edit(&mut f.second).account.as_mut().unwrap().debit_msg_read = f64::NAN;
    let before = std::fs::read(f.root.path().join("users.toml")).unwrap();
    assert!(f.second.accounting_finish().await.is_err());
    assert_eq!(std::fs::read(f.root.path().join("users.toml")).unwrap(), before);
    edit(&mut f.second).account.as_mut().unwrap().debit_msg_read = 3.0;
    f.first
        .get_board()
        .await
        .edit_users(|users| {
            users[CALLER].stats.first_date_on = date(2);
            Ok(())
        })
        .unwrap();
    let before = std::fs::read(f.root.path().join("users.toml")).unwrap();
    let error = f.second.accounting_finish().await.unwrap_err();
    assert_eq!(error.downcast_ref::<UserUpdateError>(), Some(&UserUpdateError::IdentityChanged));
    assert_eq!(std::fs::read(f.root.path().join("users.toml")).unwrap(), before);
}

#[tokio::test]
async fn individual_flags_and_disjoint_conference_and_lastread_maps_survive_stale_saves() {
    let mut f = Fixture::new(8, [8, 8]).await;
    let first = edit(&mut f.first);
    first.flags.expert_mode = true;
    first.conference_flags.insert(1, ConferenceFlags::Registered);
    first.conference_flags.insert(2, ConferenceFlags::Selected);
    first.lastread_ptr_flags.insert(
        (1, 0),
        LastReadStatus {
            last_read: 5,
            highest_msg_read: 7,
            include_qwk: true,
        },
    );
    let second = edit(&mut f.second);
    second.flags.use_graphics = true;
    second.conference_flags.insert(1, ConferenceFlags::Selected);
    second.conference_flags.insert(3, ConferenceFlags::Registered);
    second.lastread_ptr_flags.insert(
        (3, 0),
        LastReadStatus {
            last_read: 9,
            highest_msg_read: 12,
            include_qwk: false,
        },
    );
    f.first.persist_current_user().await.unwrap();
    f.second.persist_current_user().await.unwrap();
    f.first.persist_current_user().await.unwrap();
    let disk = f.disk();
    let user = &disk[CALLER];
    assert!(user.flags.expert_mode && user.flags.use_graphics);
    assert_eq!(user.conference_flags.len(), 3);
    assert_eq!(user.conference_flags[&1], ConferenceFlags::Registered | ConferenceFlags::Selected);
    assert_eq!(user.conference_flags[&2], ConferenceFlags::Selected);
    assert_eq!(user.conference_flags[&3], ConferenceFlags::Registered);
    assert_eq!(user.lastread_ptr_flags.len(), 2);
    assert_eq!(
        user.lastread_ptr_flags[&(1, 0)],
        LastReadStatus {
            last_read: 5,
            highest_msg_read: 7,
            include_qwk: true
        }
    );
    assert_eq!(
        user.lastread_ptr_flags[&(3, 0)],
        LastReadStatus {
            last_read: 9,
            highest_msg_read: 12,
            include_qwk: false
        }
    );
    assert_user_eq(local(&f.first), user);
    assert_refreshed(&f.first);
    assert_refreshed(&f.second);
    f.assert_disk_matches_board().await;
}

#[tokio::test]
async fn failed_write_keeps_entire_base_baselines_and_pending_edits_for_retry() {
    let mut f = Fixture::new(8, [8, 8]).await;
    f.second.accounting_start().await.unwrap();
    edit(&mut f.first).city = "Persisted on other node".into();
    f.first.persist_current_user().await.unwrap();
    edit(&mut f.second).web = "https://example.invalid/retry".into();
    edit(&mut f.second).stats.messages_read += 5;
    f.second.accounting_record(4, "READ", "", 5.0, 1).unwrap();
    let pending = session_snapshot(&f.second);
    let first_before = session_snapshot(&f.first);
    let board_before = toml::to_string(&f.first.board.lock().await.users).unwrap();
    let disk_before = std::fs::read(f.root.path().join("users.toml")).unwrap();
    f.first.board.lock().await.config.paths.user_file = f.root.path().into();
    for _ in 0..2 {
        assert!(f.second.persist_current_user().await.is_err());
        assert_eq!(toml::to_string(&f.first.board.lock().await.users).unwrap(), board_before);
        assert_eq!(std::fs::read(f.root.path().join("users.toml")).unwrap(), disk_before);
        assert_eq!(session_snapshot(&f.second), pending);
        assert_eq!(session_snapshot(&f.first), first_before);
        assert!(f.second.accounting_active());
    }
    f.first.board.lock().await.config.paths.user_file = f.root.path().join("users.toml");
    f.second.persist_current_user().await.unwrap();
    f.second.persist_current_user().await.unwrap();
    let disk = f.disk();
    assert_eq!(disk[CALLER].city, "Persisted on other node");
    assert_eq!(disk[CALLER].web, "https://example.invalid/retry");
    assert_eq!(disk[CALLER].stats.messages_read, 25);
    assert_eq!(disk[CALLER].stats.num_times_on, 12);
    assert_eq!(disk[CALLER].account.as_ref().unwrap().debit_msg_read, 5.0);
    assert_refreshed(&f.second);
    f.assert_disk_matches_board().await;
}

#[derive(Default)]
struct CaptureMail(std::sync::Mutex<Vec<String>>);

#[async_trait::async_trait]
impl MailSender for CaptureMail {
    async fn send(&self, _: &PasswordRecoveryConfig, to: &str, _: &str, body: String) -> Result<(), ()> {
        assert_eq!(to, "caller@example.invalid");
        self.0.lock().unwrap().push(body);
        Ok(())
    }
}

#[tokio::test]
async fn native_recovery_issue_survives_two_stale_session_saves() {
    let mut f = Fixture::new(8, [8, 8]).await;
    let mail = Arc::new(CaptureMail::default());
    let service = Arc::new(RecoveryService::new(mail.clone()));
    {
        let mut board = f.first.board.lock().await;
        board.config.password_recovery = PasswordRecoveryConfig {
            enabled: true,
            smtp_host: "smtp.example.invalid".into(),
            sender: "bbs@example.invalid".into(),
            ..Default::default()
        };
        board.password_recovery_service = service.clone();
    }
    assert!(service.issue(&f.first.board, CALLER, date(8)).await.unwrap());
    assert_eq!(mail.0.lock().unwrap().len(), 1);
    let issued = f.disk()[CALLER].clone();
    assert!(issued.recovery.is_some());
    assert_eq!(issued.recovery_issues, vec![date(8)]);
    assert!(local(&f.first).recovery.is_none());
    edit(&mut f.first).city = "After recovery".into();
    edit(&mut f.second).web = "https://example.invalid/recovery".into();
    f.first.persist_current_user().await.unwrap();
    f.second.persist_current_user().await.unwrap();
    f.first.persist_current_user().await.unwrap();
    let disk = f.disk();
    for user in [&disk[CALLER], local(&f.first), local(&f.second)] {
        assert!(user.recovery.is_some());
        assert_eq!(
            toml::to_string(user.recovery.as_ref().unwrap()).unwrap(),
            toml::to_string(issued.recovery.as_ref().unwrap()).unwrap()
        );
        assert_eq!(user.recovery_issues, issued.recovery_issues);
        assert_eq!(user.credential_revision, issued.credential_revision);
        assert_eq!(security_fingerprint(user), security_fingerprint(&issued));
        assert_eq!(user.city, "After recovery");
        assert_eq!(user.web, "https://example.invalid/recovery");
    }
    assert!(f.first.credentials_still_current().await);
    assert!(f.second.credentials_still_current().await);
    assert_refreshed(&f.first);
    assert_refreshed(&f.second);
    f.assert_disk_matches_board().await;
}

#[tokio::test]
async fn change_password_persists_without_an_extra_save_and_stale_profile_cannot_undo_it() {
    let mut f = Fixture::new(8, [8, 8]).await;
    let revision = local(&f.first).credential_revision;
    assert!(f.first.change_password("fresh-secret").await.unwrap());
    let disk = f.disk();
    assert!(disk[CALLER].password.password.is_valid("fresh-secret"));
    assert!(!disk[CALLER].password.password.is_valid("old-secret"));
    assert_eq!(disk[CALLER].password.times_changed, 1);
    assert_eq!(disk[CALLER].credential_revision, revision + 1);
    assert_eq!(disk[CALLER].password.prev_pwd.len(), 1);
    assert!(disk[CALLER].password.prev_pwd[0].is_valid("old-secret"));
    assert!(f.first.credentials_still_current().await);
    assert_refreshed(&f.first);
    edit(&mut f.second).city = "Stale profile edit".into();
    f.second.persist_current_user().await.unwrap();
    assert!(f.second.session.request_logoff);
    f.first.persist_current_user().await.unwrap();
    let disk = f.disk();
    assert!(disk[CALLER].password.password.is_valid("fresh-secret"));
    assert_eq!(disk[CALLER].city, "Stale profile edit");
    assert_eq!(disk[CALLER].password.times_changed, 1);
    assert_eq!(disk[CALLER].stats.num_times_on, 12);
    f.assert_disk_matches_board().await;
}

#[tokio::test]
async fn change_password_write_failure_rolls_back_and_retry_preserves_pending_activity() {
    let mut f = Fixture::new(8, [8, 8]).await;
    f.first.accounting_start().await.unwrap();
    edit(&mut f.first).city = "Pending profile".into();
    edit(&mut f.first).stats.messages_read += 2;
    f.first.accounting_record(4, "READ", "", 3.0, 1).unwrap();
    let before = session_snapshot(&f.first);
    let other_before = session_snapshot(&f.second);
    let board_before = toml::to_string(&f.first.board.lock().await.users).unwrap();
    let disk_before = std::fs::read(f.root.path().join("users.toml")).unwrap();
    f.first.board.lock().await.config.paths.user_file = f.root.path().into();
    assert!(f.first.change_password("fresh-secret").await.is_err());
    assert_eq!(session_snapshot(&f.first), before);
    assert_eq!(session_snapshot(&f.second), other_before);
    assert_eq!(toml::to_string(&f.first.board.lock().await.users).unwrap(), board_before);
    assert_eq!(std::fs::read(f.root.path().join("users.toml")).unwrap(), disk_before);
    assert!(local(&f.first).password.password.is_valid("old-secret"));
    assert!(f.first.accounting_active());
    f.first.board.lock().await.config.paths.user_file = f.root.path().join("users.toml");
    assert!(f.first.change_password("fresh-secret").await.unwrap());
    f.first.persist_current_user().await.unwrap();
    let disk = f.disk();
    assert!(disk[CALLER].password.password.is_valid("fresh-secret"));
    assert_eq!(disk[CALLER].password.times_changed, 1);
    assert_eq!(disk[CALLER].city, "Pending profile");
    assert_eq!(disk[CALLER].stats.messages_read, 22);
    assert_eq!(disk[CALLER].stats.num_times_on, 11);
    assert_eq!(disk[CALLER].account.as_ref().unwrap().debit_msg_read, 3.0);
    assert_refreshed(&f.first);
    f.assert_disk_matches_board().await;
}

#[tokio::test]
async fn rollover_resets_all_daily_counters_before_merging_two_new_calls() {
    let mut f = Fixture::new(7, [8, 8]).await;
    for state in [&f.first, &f.second] {
        let stats = &local(state).stats;
        assert_eq!(
            (
                stats.minutes_today,
                stats.today_num_downloads,
                stats.today_num_uploads,
                stats.today_dnld_bytes,
                stats.today_upld_bytes
            ),
            (0, 0, 0, 0, 0)
        );
        assert_eq!(state.session.user_baseline.as_ref().unwrap().stats.minutes_today, 7);
        assert_eq!(state.session.user_baseline.as_ref().unwrap().stats.last_on, date(7));
    }
    edit(&mut f.first).stats.minutes_today = 2;
    edit(&mut f.first).stats.today_num_downloads = 1;
    edit(&mut f.first).stats.today_dnld_bytes = 100;
    edit(&mut f.second).stats.minutes_today = 3;
    edit(&mut f.second).stats.today_num_uploads = 2;
    edit(&mut f.second).stats.today_upld_bytes = 200;
    f.first.persist_current_user().await.unwrap();
    f.second.persist_current_user().await.unwrap();
    for _ in 0..2 {
        f.first.persist_current_user().await.unwrap();
        f.second.persist_current_user().await.unwrap();
    }
    let disk = f.disk();
    let stats = &disk[CALLER].stats;
    assert_eq!(
        (
            stats.minutes_today,
            stats.today_num_downloads,
            stats.today_num_uploads,
            stats.today_dnld_bytes,
            stats.today_upld_bytes
        ),
        (5, 1, 2, 100, 200)
    );
    assert_eq!(stats.last_on.date_naive(), date(8).date_naive());
    assert_eq!(stats.num_times_on, 12);
    assert_refreshed(&f.first);
    assert_refreshed(&f.second);
    f.assert_disk_matches_board().await;
}

#[tokio::test]
async fn old_call_after_midnight_cannot_repost_daily_usage_into_new_login_day() {
    let mut f = Fixture::new(7, [7, 8]).await;
    edit(&mut f.first).stats.minutes_today += 2;
    edit(&mut f.first).stats.messages_read += 2;
    edit(&mut f.first).stats.today_num_downloads += 1;
    edit(&mut f.first).stats.today_dnld_bytes += 100;
    edit(&mut f.second).stats.minutes_today = 3;
    edit(&mut f.second).stats.messages_read += 5;
    edit(&mut f.second).stats.today_num_uploads = 2;
    edit(&mut f.second).stats.today_upld_bytes = 200;
    f.second.persist_current_user().await.unwrap();
    f.first.persist_current_user().await.unwrap();
    assert_eq!(f.first.session.login_date, date(7));
    edit(&mut f.first).stats.minutes_today += 1;
    edit(&mut f.first).stats.messages_read += 1;
    edit(&mut f.first).stats.today_num_downloads += 1;
    edit(&mut f.first).stats.today_dnld_bytes += 100;
    f.first.persist_current_user().await.unwrap();
    f.first.persist_current_user().await.unwrap();
    f.second.persist_current_user().await.unwrap();
    let disk = f.disk();
    let stats = &disk[CALLER].stats;
    assert_eq!(
        (
            stats.minutes_today,
            stats.today_num_downloads,
            stats.today_num_uploads,
            stats.today_dnld_bytes,
            stats.today_upld_bytes
        ),
        (3, 0, 2, 0, 200)
    );
    assert_eq!(stats.last_on.date_naive(), date(8).date_naive());
    assert_eq!(stats.messages_read, 28);
    assert_eq!(stats.num_times_on, 12);
    assert_refreshed(&f.first);
    assert_refreshed(&f.second);
    f.assert_disk_matches_board().await;
}
