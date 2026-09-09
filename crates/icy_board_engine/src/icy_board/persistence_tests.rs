use super::*;
use crate::icy_board::{
    user_base::{User, UserBase},
    user_store::UserUpdateMode,
};
use std::{
    future::{Future, poll_fn},
    pin::{Pin, pin},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc as sync_mpsc,
    },
    task::Poll,
    time::Duration,
};
use tempfile::TempDir;

const DEADLINE: Duration = Duration::from_secs(10);

struct WorkerBarrier {
    entered: sync_mpsc::Receiver<()>,
    release: sync_mpsc::Sender<()>,
}

impl WorkerBarrier {
    fn new() -> (Self, impl FnOnce() + Send + 'static) {
        let (entered_tx, entered) = sync_mpsc::channel();
        let (release, release_rx) = sync_mpsc::channel();
        let wait = move || {
            entered_tx.send(()).expect("test must still be waiting for worker");
            release_rx.recv_timeout(DEADLINE).expect("worker barrier was not released");
        };
        (Self { entered, release }, wait)
    }

    async fn entered(&mut self) {
        let (_, empty) = sync_mpsc::channel();
        receive(std::mem::replace(&mut self.entered, empty)).await;
    }

    fn release(self) {
        self.release.send(()).expect("worker must still be blocked");
    }
}

async fn receive<T: Send + 'static>(receiver: sync_mpsc::Receiver<T>) -> T {
    // Never wait on a synchronous channel on the single-threaded executor.
    bounded(tokio::task::spawn_blocking(move || receiver.recv_timeout(DEADLINE)))
        .await
        .expect("channel waiter panicked")
        .expect("worker did not respond before deadline")
}

async fn bounded<F: Future>(future: F) -> F::Output {
    tokio::time::timeout(DEADLINE, future).await.expect("persistence operation timed out")
}

async fn pending<F: Future>(mut future: Pin<&mut F>) {
    poll_fn(|cx| {
        assert!(future.as_mut().poll(cx).is_pending(), "acknowledgement arrived before release");
        Poll::Ready(())
    })
    .await;
}

fn fixture() -> (TempDir, SharedBoard) {
    let dir = tempfile::tempdir().unwrap();
    let mut board = IcyBoard::new();
    board.root_path = dir.path().into();
    board.config.paths.user_file = "users.toml".into();
    board.config.board.name = "Persistence fixture".into();
    let mut user = User {
        name: "Persistence User".into(),
        city: "Original city".into(),
        language: "en".into(),
        ..Default::default()
    };
    user.stats.first_date_on = "2026-09-01T12:00:00Z".parse().unwrap();
    user.stats.last_on = "2026-09-09T12:00:00Z".parse().unwrap();
    user.stats.messages_read = 100;
    user.stats.today_num_downloads = 10;
    board.users.new_user(user);
    board.edit_users(|_| Ok(())).unwrap();
    (dir, Arc::new(tokio::sync::Mutex::new(board)))
}

fn encoded(users: &UserBase) -> String {
    toml::to_string(users).unwrap()
}

fn disk_users(dir: &TempDir) -> UserBase {
    UserBase::load(&dir.path().join("users.toml")).unwrap()
}

async fn assert_one_queued(board: &SharedBoard) {
    let writer = bounded(board.lock()).await.persistence_writer.clone();
    // A single poll must actually enqueue the request, not merely construct its future.
    assert_eq!(writer.sender().unwrap().capacity(), 31);
}

#[tokio::test(flavor = "current_thread")]
async fn context_reads_and_path_resolution_match_board_without_mutable_snapshot_access() {
    let (dir, board) = fixture();
    std::fs::create_dir(dir.path().join("MixedCase")).unwrap();
    std::fs::write(dir.path().join("MixedCase/Users.txt"), "fixture").unwrap();
    let paths = vec![
        PathBuf::new(),
        PathBuf::from("users.toml"),
        PathBuf::from("mixedcase/users.TXT"),
        PathBuf::from("missing.toml"),
        dir.path().join("MixedCase/Users.txt"),
        dir.path().join("mixedcase/users.TXT"),
        dir.path().join("missing-absolute.toml"),
    ];
    let (expected, config, recovery, revision) = {
        let board = board.lock().await;
        (
            paths.iter().map(|path| board.resolve_file(path)).collect::<Vec<_>>(),
            board.configuration_snapshot(),
            board.password_recovery_service.clone(),
            board.user_revision,
        )
    };
    assert_eq!(expected[0], PathBuf::new());
    assert_eq!(expected[1], dir.path().join("users.toml"));
    assert_eq!(expected[2], dir.path().join("MixedCase/Users.txt"));
    assert_eq!(expected[3], dir.path().join("missing.toml"));
    assert_eq!(expected[4], expected[2]);
    assert_eq!(expected[5], expected[2]);
    assert_eq!(expected[6], dir.path().join("missing-absolute.toml"));
    bounded(IcyBoard::write_users(&board, move |context| {
        let view: &UserWriteSnapshot = context;
        assert!(view.config.ptr_eq(&config));
        assert!(Arc::ptr_eq(&view.password_recovery_service, &recovery));
        assert_eq!(view.user_revision, revision);
        assert_eq!(paths.iter().map(|path| context.resolve_file(path)).collect::<Vec<_>>(), expected);
        let mut detached_users = view.users.clone();
        let mut detached_config = view.config.clone();
        detached_users[0].city = "Detached edit".into();
        detached_config.board.name = "Detached config".into();
        assert_eq!(context.users[0].city, "Original city");
        assert_eq!(context.config.board.name, "Persistence fixture");
        Ok(())
    }))
    .await
    .unwrap();
    let live = board.lock().await;
    assert_eq!(live.user_revision, revision);
    assert_eq!(encoded(&disk_users(&dir)), encoded(&live.users));
}

#[tokio::test(flavor = "current_thread")]
async fn context_update_returns_saved_normalization_and_edits_refresh_its_view() {
    for revoked in [false, true] {
        let (dir, board) = fixture();
        let (baseline, revision) = {
            let mut board = board.lock().await;
            board.config.password_recovery.enabled = revoked;
            board.users[0].recovery = Some(toml::from_str(
                "id = 'normalization-test'\nhash = 'test'\ncontext = 'test'\nrevision = 1\nissued = '2026-09-09T12:00:00Z'\nexpires = '2026-09-09T12:10:00Z'\nattempts = 0\n",
            ).unwrap());
            if revoked {
                board.password_recovery_service.revoke_runtime_challenges();
            }
            (board.users[0].clone(), board.user_revision)
        };
        let (saved, edited) = bounded(IcyBoard::write_users(&board, move |context| {
            let mut edit = baseline.clone();
            edit.city = "Saved city".into();
            let saved = context.update_user(&baseline, &edit, UserUpdateMode::Edit)?;
            assert!(saved.recovery.is_none());
            assert_eq!(saved.city, "Saved city");
            assert_eq!(saved.credential_revision, baseline.credential_revision);
            assert_eq!(toml::to_string(&saved)?, toml::to_string(&context.users[0])?);
            assert_eq!(context.user_revision, revision + 1);
            context.edit_users(|users| {
                users[0].email = "changed@example.invalid".into();
                Ok(())
            })?;
            assert_eq!(context.users[0].credential_revision, saved.credential_revision + 1);
            assert_eq!(
                context.users[0].security_stamp,
                super::super::password_recovery::security_fingerprint(&context.users[0])
            );
            assert_eq!(context.user_revision, revision + 2);
            Ok((saved, context.users[0].clone()))
        }))
        .await
        .unwrap();
        let live = board.lock().await;
        assert_eq!(live.user_revision, revision + 2);
        assert_eq!(toml::to_string(&edited).unwrap(), toml::to_string(&live.users[0]).unwrap());
        assert_eq!(encoded(&disk_users(&dir)), encoded(&live.users));
        assert!(saved.recovery.is_none());
    }
}

#[tokio::test(flavor = "current_thread")]
async fn successful_save_is_published_after_later_callback_error_panic_or_io_failure() {
    for failure in ["error", "panic", "io"] {
        let (dir, board) = fixture();
        let revision = board.lock().await.user_revision;
        let path = dir.path().join("users.toml");
        let backup = dir.path().join("committed-users.toml");
        let blocked_path = path.clone();
        let saved_path = backup.clone();
        let error = bounded(IcyBoard::write_users(&board, move |context| {
            context.edit_users(|users| {
                users[0].city = "Committed before failure".into();
                Ok(())
            })?;
            if failure == "io" {
                std::fs::rename(&blocked_path, &saved_path)?;
                std::fs::create_dir(&blocked_path)?;
            }
            context.edit_users(|users| {
                users[0].language = "Must not publish".into();
                match failure {
                    "error" => Err("intentional callback error".into()),
                    "panic" => panic!("intentional callback panic after commit"),
                    _ => Ok(()),
                }
            })
        }))
        .await
        .unwrap_err();
        match failure {
            "error" => assert_eq!(error.to_string(), "intentional callback error"),
            "panic" => assert_eq!(error.downcast_ref::<UserUpdateError>(), Some(&UserUpdateError::WriterPanicked)),
            _ => {
                assert!(matches!(
                    error.downcast_ref::<crate::icy_board::IcyError>(),
                    Some(crate::icy_board::IcyError::ErrorGeneratingToml(_, _))
                ));
                std::fs::remove_dir(&path).unwrap();
                std::fs::rename(&backup, &path).unwrap();
            }
        }
        {
            let live = board.lock().await;
            assert_eq!(live.user_revision, revision + 1);
            assert_eq!(live.users[0].city, "Committed before failure");
            assert_eq!(live.users[0].language, "en");
            assert_eq!(encoded(&disk_users(&dir)), encoded(&live.users));
        }
        bounded(IcyBoard::write_users(&board, move |context| {
            assert_eq!(context.user_revision, revision + 1);
            assert_eq!(context.users[0].city, "Committed before failure");
            context.edit_users(|users| {
                users[0].language = "de".into();
                Ok(())
            })
        }))
        .await
        .unwrap();
        let live = board.lock().await;
        assert_eq!(live.user_revision, revision + 2);
        assert_eq!(live.users[0].language, "de");
        assert_eq!(encoded(&disk_users(&dir)), encoded(&live.users));
    }
}

#[tokio::test(flavor = "current_thread")]
async fn blocked_save_keeps_reads_snapshots_and_executor_live_until_ack() {
    let (dir, board) = fixture();
    let (old_users, old_config, revision) = {
        let board = board.lock().await;
        (board.users.clone(), board.configuration_snapshot(), board.user_revision)
    };
    let old_disk = std::fs::read(dir.path().join("users.toml")).unwrap();
    let (mut barrier, wait) = WorkerBarrier::new();
    let mut request = pin!(IcyBoard::write_users(&board, move |staging| {
        super::super::BEFORE_FILE_SYNC.with(|hook| *hook.borrow_mut() = Some(Box::new(wait)));
        staging.edit_users(|users| {
            users[0].city = "Published city".into();
            Ok(())
        })
    }));
    pending(request.as_mut()).await;
    barrier.entered().await;

    let read = bounded(board.lock()).await;
    assert_eq!(read.user_revision, revision);
    assert_eq!(encoded(&read.users), encoded(&old_users));
    assert!(read.configuration_snapshot().ptr_eq(&old_config));
    assert_eq!(read.config.board.name, "Persistence fixture");
    drop(read);
    assert_eq!(old_users[0].city, "Original city");
    assert_eq!(std::fs::read(dir.path().join("users.toml")).unwrap(), old_disk);
    assert_eq!(bounded(tokio::spawn(async { 42 })).await.unwrap(), 42);
    // The timer must fire on this executor while the writer is still blocked.
    assert!(tokio::time::timeout(Duration::from_millis(10), request.as_mut()).await.is_err());
    pending(request.as_mut()).await;

    barrier.release();
    bounded(request).await.unwrap();
    let read = bounded(board.lock()).await;
    assert_eq!(read.user_revision, revision + 1);
    assert_eq!(read.users[0].city, "Published city");
    assert_eq!(encoded(&disk_users(&dir)), encoded(&read.users));
    assert!(read.configuration_snapshot().ptr_eq(&old_config));
    assert_eq!(old_users[0].city, "Original city");
}

#[tokio::test(flavor = "current_thread")]
async fn fifo_updates_merge_same_baseline_and_accumulate_counter_deltas() {
    let (dir, board) = fixture();
    let (original, revision) = {
        let board = board.lock().await;
        (board.users.clone(), board.user_revision)
    };
    let baseline = original[0].clone();
    let mode = UserUpdateMode::Session {
        day: baseline.stats.last_on.date_naive(),
    };
    let mut edited_first = baseline.clone();
    edited_first.city = "First city".into();
    edited_first.stats.messages_read += 3;
    edited_first.stats.today_num_downloads += 2;
    let mut edited_second = baseline.clone();
    edited_second.language = "de".into();
    edited_second.stats.messages_read += 7;
    edited_second.stats.today_num_downloads += 4;
    let (mut first_barrier, first_wait) = WorkerBarrier::new();
    let (mut second_barrier, second_wait) = WorkerBarrier::new();
    let (observed_tx, observed_rx) = sync_mpsc::channel();
    let first_baseline = baseline.clone();
    let mut first = pin!(IcyBoard::write_users(&board, move |staging| {
        assert_eq!(staging.user_revision, revision);
        first_wait();
        let user = staging.update_user(&first_baseline, &edited_first, mode)?;
        Ok((user, staging.user_revision))
    }));
    pending(first.as_mut()).await;
    first_barrier.entered().await;
    let mut second = pin!(IcyBoard::write_users(&board, move |staging| {
        observed_tx.send((staging.users.clone(), staging.user_revision)).unwrap();
        second_wait();
        let user = staging.update_user(&baseline, &edited_second, mode)?;
        Ok((user, staging.user_revision))
    }));
    pending(second.as_mut()).await;
    assert_one_queued(&board).await;
    pending(first.as_mut()).await;
    first_barrier.release();
    let (first_user, first_revision) = bounded(first).await.unwrap();
    second_barrier.entered().await;
    let (second_baseline, second_revision) = receive(observed_rx).await;
    assert_eq!(first_revision, revision + 1);
    assert_eq!(second_revision, first_revision);
    assert_eq!(second_baseline[0].city, "First city");
    assert_eq!(second_baseline[0].stats.messages_read, 103);
    assert_eq!(first_user.language, "en");
    {
        let live = bounded(board.lock()).await;
        assert_eq!(live.user_revision, first_revision);
        assert_eq!(encoded(&live.users), encoded(&second_baseline));
        assert_eq!(encoded(&disk_users(&dir)), encoded(&live.users));
    }
    pending(second.as_mut()).await;
    second_barrier.release();
    let (second_user, final_revision) = bounded(second).await.unwrap();
    assert_eq!(final_revision, first_revision + 1);
    assert_eq!(second_user.city, "First city");
    assert_eq!(second_user.language, "de");
    assert_eq!(second_user.stats.messages_read, 110);
    assert_eq!(second_user.stats.today_num_downloads, 16);
    let live = bounded(board.lock()).await;
    assert_eq!(live.user_revision, final_revision);
    assert_eq!(encoded(&disk_users(&dir)), encoded(&live.users));
    assert_eq!(live.users[0].stats.messages_read, 110);
    assert_eq!(live.users[0].stats.today_num_downloads, 16);
    assert_eq!(original[0].city, "Original city");
    assert_eq!(original[0].stats.messages_read, 100);
    assert_eq!(second_baseline[0].language, "en");
    assert_eq!(second_baseline[0].stats.today_num_downloads, 12);
}

#[tokio::test(flavor = "current_thread")]
async fn io_failure_does_not_publish_and_already_queued_job_continues() {
    let (dir, board) = fixture();
    let (original, revision) = {
        let board = board.lock().await;
        (board.users.clone(), board.user_revision)
    };
    let path = dir.path().join("users.toml");
    let backup = dir.path().join("saved-users.toml");
    let bytes = std::fs::read(&path).unwrap();
    // A directory cannot be replaced by the serializer's file rename, even as root.
    std::fs::rename(&path, &backup).unwrap();
    std::fs::create_dir(&path).unwrap();
    let (mut failed_barrier, failed_wait) = WorkerBarrier::new();
    let (mut next_barrier, next_wait) = WorkerBarrier::new();
    let mut failed = pin!(IcyBoard::write_users(&board, move |staging| {
        staging.edit_users(|users| {
            users[0].city = "Must not publish".into();
            failed_wait();
            Ok(())
        })
    }));
    pending(failed.as_mut()).await;
    failed_barrier.entered().await;
    let mut next = pin!(IcyBoard::write_users(&board, move |staging| {
        assert_eq!(staging.user_revision, revision);
        assert_eq!(staging.users[0].city, "Original city");
        next_wait();
        staging.edit_users(|users| {
            users[0].language = "de".into();
            Ok(())
        })
    }));
    pending(next.as_mut()).await;
    assert_one_queued(&board).await;
    failed_barrier.release();
    let error = bounded(failed).await.unwrap_err();
    assert!(matches!(
        error.downcast_ref::<crate::icy_board::IcyError>(),
        Some(crate::icy_board::IcyError::ErrorGeneratingToml(_, _))
    ));
    next_barrier.entered().await;
    {
        let live = bounded(board.lock()).await;
        assert_eq!(live.user_revision, revision);
        assert_eq!(encoded(&live.users), encoded(&original));
    }
    assert!(path.is_dir());
    assert_eq!(std::fs::read(&backup).unwrap(), bytes);
    pending(next.as_mut()).await;
    std::fs::remove_dir(&path).unwrap();
    std::fs::rename(&backup, &path).unwrap();
    next_barrier.release();
    bounded(next).await.unwrap();
    let live = bounded(board.lock()).await;
    assert_eq!(live.user_revision, revision + 1);
    assert_eq!(live.users[0].city, "Original city");
    assert_eq!(live.users[0].language, "de");
    assert_eq!(encoded(&disk_users(&dir)), encoded(&live.users));
    assert_eq!(original[0].language, "en");
}

#[tokio::test(flavor = "current_thread")]
async fn cancelled_accepted_request_commits_before_queued_observer() {
    let (dir, board) = fixture();
    let (original, revision) = {
        let board = board.lock().await;
        (board.users.clone(), board.user_revision)
    };
    let (mut barrier, wait) = WorkerBarrier::new();
    let mut cancelled = Box::pin(IcyBoard::write_users(&board, move |staging| {
        staging.edit_users(|users| {
            users[0].city = "Cancelled waiter, committed edit".into();
            wait();
            Ok(())
        })
    }));
    pending(cancelled.as_mut()).await;
    barrier.entered().await;
    let observed_board = board.clone();
    let mut observer = pin!(IcyBoard::write_users(&board, move |staging| {
        let live = observed_board.blocking_lock();
        assert_eq!(live.user_revision, staging.user_revision);
        assert_eq!(encoded(&live.users), encoded(&staging.users));
        Ok((staging.users.clone(), staging.user_revision))
    }));
    pending(observer.as_mut()).await;
    assert_one_queued(&board).await;
    // Drop the actual future, not just a Pin<&mut _>, after acceptance is proven.
    drop(cancelled);
    assert_eq!(bounded(board.lock()).await.user_revision, revision);
    barrier.release();
    let (observed, observed_revision) = bounded(observer).await.unwrap();
    assert_eq!(observed_revision, revision + 1);
    assert_eq!(observed[0].city, "Cancelled waiter, committed edit");
    assert_eq!(encoded(&disk_users(&dir)), encoded(&observed));
    assert_eq!(original[0].city, "Original city");
}

#[tokio::test(flavor = "current_thread")]
async fn panic_before_durable_commit_preserves_state_and_writer_survives() {
    let (dir, board) = fixture();
    let (original, revision) = {
        let board = board.lock().await;
        (board.users.clone(), board.user_revision)
    };
    let bytes = std::fs::read(dir.path().join("users.toml")).unwrap();
    let (mut panic_barrier, panic_wait) = WorkerBarrier::new();
    let (mut next_barrier, next_wait) = WorkerBarrier::new();
    let mut panicking = pin!(IcyBoard::write_users(&board, move |staging| {
        staging.edit_users::<()>(|users| {
            users[0].city = "Panicked mutation".into();
            panic_wait();
            panic!("intentional pre-save callback panic");
        })
    }));
    pending(panicking.as_mut()).await;
    panic_barrier.entered().await;
    let mut next = pin!(IcyBoard::write_users(&board, move |staging| {
        assert_eq!(staging.user_revision, revision);
        assert_eq!(staging.users[0].city, "Original city");
        next_wait();
        staging.edit_users(|users| {
            users[0].city = "After panic".into();
            Ok(())
        })
    }));
    pending(next.as_mut()).await;
    assert_one_queued(&board).await;
    panic_barrier.release();
    assert_eq!(
        bounded(panicking).await.unwrap_err().downcast_ref::<UserUpdateError>(),
        Some(&UserUpdateError::WriterPanicked)
    );
    next_barrier.entered().await;
    {
        let live = bounded(board.lock()).await;
        assert_eq!(live.user_revision, revision);
        assert_eq!(encoded(&live.users), encoded(&original));
    }
    assert_eq!(std::fs::read(dir.path().join("users.toml")).unwrap(), bytes);
    pending(next.as_mut()).await;
    next_barrier.release();
    bounded(next).await.unwrap();
    let live = bounded(board.lock()).await;
    assert_eq!(live.user_revision, revision + 1);
    assert_eq!(live.users[0].city, "After panic");
    assert_eq!(encoded(&disk_users(&dir)), encoded(&live.users));
    assert_eq!(original[0].city, "Original city");
}

#[tokio::test(flavor = "current_thread")]
async fn synchronous_offline_edit_is_rejected_while_writer_is_busy() {
    let (dir, board) = fixture();
    let (original, revision) = {
        let board = board.lock().await;
        (board.users.clone(), board.user_revision)
    };
    let (mut barrier, wait) = WorkerBarrier::new();
    let mut request = pin!(IcyBoard::write_users(&board, move |staging| {
        staging.edit_users(|users| {
            users[0].city = "Writer edit".into();
            wait();
            Ok(())
        })
    }));
    pending(request.as_mut()).await;
    barrier.entered().await;
    let offline_board = board.clone();
    let called = Arc::new(AtomicBool::new(false));
    let callback_called = called.clone();
    let (result_tx, result_rx) = sync_mpsc::channel();
    // A regressed lock inversion must time out, not prevent Tokio runtime shutdown.
    let offline = std::thread::spawn(move || {
        let result = offline_board.blocking_lock().edit_users(|users| {
            callback_called.store(true, Ordering::SeqCst);
            users[0].language = "Offline overwrite".into();
            Ok(())
        });
        let _ = result_tx.send(result);
    });
    let error = receive(result_rx).await.unwrap_err();
    offline.join().unwrap();
    assert_eq!(error.downcast_ref::<UserUpdateError>(), Some(&UserUpdateError::WriterBusy));
    assert!(!called.load(Ordering::SeqCst));
    {
        let live = bounded(board.lock()).await;
        assert_eq!(live.user_revision, revision);
        assert_eq!(encoded(&live.users), encoded(&original));
    }
    assert_eq!(encoded(&disk_users(&dir)), encoded(&original));
    pending(request.as_mut()).await;
    barrier.release();
    bounded(request).await.unwrap();
    let live = bounded(board.lock()).await;
    assert_eq!(live.user_revision, revision + 1);
    assert_eq!(live.users[0].city, "Writer edit");
    assert_eq!(live.users[0].language, "en");
    assert_eq!(encoded(&disk_users(&dir)), encoded(&live.users));
    assert_eq!(original[0].city, "Original city");
}

#[tokio::test(flavor = "current_thread")]
async fn idle_offline_edits_coexist_with_runtime_writes() {
    let (dir, board) = fixture();
    let revision = {
        let mut live = board.lock().await;
        live.edit_users(|users| {
            users[0].city = "Offline before worker".into();
            Ok(())
        })
        .unwrap();
        live.user_revision
    };
    bounded(IcyBoard::write_users(&board, |context| {
        assert_eq!(context.users[0].city, "Offline before worker");
        context.edit_users(|users| {
            users[0].language = "de".into();
            Ok(())
        })
    }))
    .await
    .unwrap();
    bounded(IcyBoard::flush_persistence(&board)).await.unwrap();
    {
        // No producers remain; direct offline initialization is safe only while quiescent.
        let mut live = board.lock().await;
        let baseline = live.users[0].clone();
        let mut edited = baseline.clone();
        edited.city = "Offline after worker".into();
        live.update_user(&baseline, &edited, UserUpdateMode::Edit).unwrap();
        live.users[0].user_comment = "Offline direct edit".into();
        live.save_userbase().unwrap();
        assert_eq!(live.user_revision, revision + 3);
        assert_eq!(encoded(&disk_users(&dir)), encoded(&live.users));
    }
    bounded(IcyBoard::write_users(&board, move |context| {
        assert_eq!(context.user_revision, revision + 3);
        assert_eq!(context.users[0].city, "Offline after worker");
        assert_eq!(context.users[0].language, "de");
        assert_eq!(context.users[0].user_comment, "Offline direct edit");
        context.edit_users(|users| {
            users[0].city = "Runtime resumed".into();
            Ok(())
        })
    }))
    .await
    .unwrap();
    let live = board.lock().await;
    assert_eq!(live.user_revision, revision + 4);
    assert_eq!(live.users[0].city, "Runtime resumed");
    assert_eq!(live.users[0].language, "de");
    assert_eq!(live.users[0].user_comment, "Offline direct edit");
    assert_eq!(encoded(&disk_users(&dir)), encoded(&live.users));
}

#[tokio::test(flavor = "current_thread")]
async fn poisoned_transaction_gate_rejects_offline_and_queued_writes_without_callbacks() {
    let (dir, board) = fixture();
    let stats_path = dir.path().join("statistics.toml");
    let (original, revision, writer) = {
        let mut live = board.lock().await;
        live.config.paths.statistics_file = stats_path.clone();
        live.statistics.save(&stats_path).unwrap();
        (live.users.clone(), live.user_revision, live.persistence_writer.clone())
    };
    let user_bytes = std::fs::read(dir.path().join("users.toml")).unwrap();
    let stats_bytes = std::fs::read(&stats_path).unwrap();
    let offline_board = board.clone();
    bounded(tokio::task::spawn_blocking(move || {
        let mut live = offline_board.blocking_lock();
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = live.edit_users::<()>(|users| {
                    users[0].city = "Interrupted offline edit".into();
                    panic!("intentional offline transaction panic");
                });
            }))
            .is_err()
        );
    }))
    .await
    .unwrap();
    assert!(writer.gate.is_poisoned());
    let assert_poisoned = |error: Box<dyn std::error::Error + Send + Sync>| {
        assert_eq!(error.downcast_ref::<UserUpdateError>(), Some(&UserUpdateError::WriterPoisoned));
    };
    {
        let mut live = board.lock().await;
        assert_poisoned(live.edit_users::<()>(|_| panic!("offline callback must not run")).unwrap_err());
        assert_poisoned(live.update_user(&original[0], &original[0], UserUpdateMode::Edit).err().unwrap());
        assert_poisoned(live.save_userbase().unwrap_err());
    }
    assert_poisoned(
        bounded(IcyBoard::write_users::<()>(&board, |_| panic!("user callback must not run")))
            .await
            .unwrap_err(),
    );
    assert_poisoned(
        bounded(IcyBoard::write_statistics(&board, |_| panic!("statistics callback must not run")))
            .await
            .unwrap_err(),
    );
    assert_poisoned(
        bounded(IcyBoard::ordered_persistence::<()>(&board, || panic!("ordered callback must not run")))
            .await
            .unwrap_err(),
    );
    assert_poisoned(bounded(IcyBoard::flush_persistence(&board)).await.unwrap_err());
    assert!(writer.gate.is_poisoned());
    let live = board.lock().await;
    assert_eq!(live.user_revision, revision);
    assert_eq!(encoded(&live.users), encoded(&original));
    assert_eq!(std::fs::read(dir.path().join("users.toml")).unwrap(), user_bytes);
    assert_eq!(std::fs::read(&stats_path).unwrap(), stats_bytes);
    assert_eq!(toml::to_string(&live.statistics).unwrap(), String::from_utf8(stats_bytes).unwrap());
}

#[tokio::test(flavor = "current_thread")]
async fn flush_waits_for_cancelled_sync_then_allows_more_writes() {
    let (dir, board) = fixture();
    let revision = board.lock().await.user_revision;
    let old_disk = std::fs::read(dir.path().join("users.toml")).unwrap();
    let (mut barrier, wait) = WorkerBarrier::new();
    let worker_board = board.clone();
    let waiter = tokio::spawn(async move {
        IcyBoard::write_users(&worker_board, move |staging| {
            super::super::BEFORE_FILE_SYNC.with(|hook| *hook.borrow_mut() = Some(Box::new(wait)));
            staging.edit_users(|users| {
                users[0].city = "Flushed cancelled write".into();
                Ok(())
            })
        })
        .await
    });
    barrier.entered().await;
    waiter.abort();
    assert!(bounded(waiter).await.unwrap_err().is_cancelled());

    let mut flush = pin!(IcyBoard::flush_persistence(&board));
    pending(flush.as_mut()).await;
    assert_one_queued(&board).await;
    assert!(tokio::time::timeout(Duration::from_millis(10), flush.as_mut()).await.is_err());
    assert_eq!(bounded(board.lock()).await.user_revision, revision);
    assert_eq!(std::fs::read(dir.path().join("users.toml")).unwrap(), old_disk);
    barrier.release();
    bounded(flush).await.unwrap();
    {
        let live = bounded(board.lock()).await;
        assert_eq!(live.user_revision, revision + 1);
        assert_eq!(live.users[0].city, "Flushed cancelled write");
        assert_eq!(encoded(&disk_users(&dir)), encoded(&live.users));
    }

    bounded(IcyBoard::write_users(&board, |staging| {
        staging.edit_users(|users| {
            users[0].language = "de".into();
            Ok(())
        })
    }))
    .await
    .unwrap();
    bounded(IcyBoard::flush_persistence(&board)).await.unwrap();
    let live = bounded(board.lock()).await;
    assert_eq!(live.user_revision, revision + 2);
    assert_eq!(live.users[0].city, "Flushed cancelled write");
    assert_eq!(live.users[0].language, "de");
    assert_eq!(encoded(&disk_users(&dir)), encoded(&live.users));
}
