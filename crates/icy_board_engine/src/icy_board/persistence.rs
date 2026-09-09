//! Ordered persistence off the async executor, with short snapshot/publication locks.

use std::{
    ops::Deref,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard, TryLockError},
};

use tokio::sync::{mpsc, oneshot};

use crate::Res;

use super::{
    IcyBoard, IcyBoardSerializer,
    icb_config::IcbConfig,
    password_recovery::RecoveryService,
    resolve_file_from_root,
    snapshot::Snapshot,
    statistics::Statistics,
    user_base::{User, UserBase},
    user_store::{UserUpdateError, UserUpdateMode, UserWriteCore},
};

#[cfg(test)]
#[path = "persistence_tests.rs"]
mod persistence_tests;

type Job = Box<dyn FnOnce() + Send>;
type SharedBoard = Arc<tokio::sync::Mutex<IcyBoard>>;

/// Read-only callback state; the context deliberately does not implement `DerefMut`.
pub struct UserWriteSnapshot {
    pub users: UserBase,
    pub config: Snapshot<IcbConfig>,
    pub root_path: PathBuf,
    pub password_recovery_service: Arc<RecoveryService>,
    pub user_revision: u64,
}

/// User mutations must go through staged, synchronous persistence methods.
///
/// ```
/// use icy_board_engine::{Res, icy_board::{persistence::UserWriteContext, user_store::UserUpdateMode}};
/// fn edit(context: &mut UserWriteContext) -> Res<()> {
///     let baseline = context.users[0].clone();
///     let _enabled = context.config.password_recovery.enabled;
///     let _revision = context.user_revision;
///     let _path = context.resolve_file(&context.config.paths.user_file);
///     let mut edited = baseline.clone();
///     edited.city.clear();
///     context.update_user(&baseline, &edited, UserUpdateMode::Edit)?;
///     context.edit_users(|users| { users[0].city.clear(); Ok(()) })
/// }
/// ```
///
/// ```compile_fail
/// use icy_board_engine::icy_board::persistence::UserWriteContext;
/// fn bypass(context: &mut UserWriteContext) {
///     context.users[0].city.clear();
/// }
/// ```
///
/// ```compile_fail
/// use icy_board_engine::icy_board::persistence::UserWriteContext;
/// fn bypass(context: &mut UserWriteContext) {
///     context.config.password_recovery.enabled = false;
/// }
/// ```
///
/// ```compile_fail
/// use icy_board_engine::icy_board::persistence::UserWriteContext;
/// fn bypass(context: &mut UserWriteContext) {
///     context.user_revision += 1;
/// }
/// ```
pub struct UserWriteContext {
    snapshot: UserWriteSnapshot,
}

impl Deref for UserWriteContext {
    type Target = UserWriteSnapshot;

    fn deref(&self) -> &Self::Target {
        &self.snapshot
    }
}

impl UserWriteContext {
    pub fn resolve_file<P: AsRef<Path>>(&self, file: &P) -> PathBuf {
        resolve_file_from_root(&self.root_path, file.as_ref())
    }

    pub fn edit_users<R>(&mut self, edit: impl FnOnce(&mut UserBase) -> Res<R>) -> Res<R> {
        self.user_write_core().edit_users(edit)
    }

    /// Return the normalized saved record, not the pre-save merge result.
    pub fn update_user(&mut self, baseline: &User, edited: &User, mode: UserUpdateMode) -> Res<User> {
        self.user_write_core().update_user(baseline, edited, mode)
    }

    fn user_write_core(&mut self) -> UserWriteCore<'_> {
        UserWriteCore {
            users: &mut self.snapshot.users,
            revision: &mut self.snapshot.user_revision,
            config: &self.snapshot.config,
            root_path: &self.snapshot.root_path,
            recovery: &self.snapshot.password_recovery_service,
        }
    }
}

#[derive(Default)]
pub struct PersistenceWriter {
    sender: Mutex<Option<mpsc::Sender<Job>>>,
    gate: Arc<Mutex<()>>,
}

impl PersistenceWriter {
    fn sender(&self) -> Res<mpsc::Sender<Job>> {
        let mut sender = self.sender.lock().map_err(|_| UserUpdateError::WriterStopped)?;
        if let Some(sender) = sender.as_ref() {
            return Ok(sender.clone());
        }
        let (tx, mut rx) = mpsc::channel::<Job>(32);
        std::thread::Builder::new().name("board-persistence".into()).spawn(move || {
            while let Some(job) = rx.blocking_recv() {
                // A failed request must not strand the remaining acknowledgements.
                if std::panic::catch_unwind(std::panic::AssertUnwindSafe(job)).is_err() {
                    log::error!("Board persistence request panicked");
                }
            }
        })?;
        *sender = Some(tx.clone());
        Ok(tx)
    }

    pub(super) fn offline_guard(&self) -> Res<MutexGuard<'_, ()>> {
        self.gate.try_lock().map_err(|error| match error {
            TryLockError::WouldBlock => UserUpdateError::WriterBusy.into(),
            TryLockError::Poisoned(_) => UserUpdateError::WriterPoisoned.into(),
        })
    }

    async fn submit<R: Send + 'static>(&self, operation: impl FnOnce() -> Res<R> + Send + 'static) -> Res<R> {
        let (reply, result) = oneshot::channel();
        self.sender()?
            .send(Box::new(move || {
                let _ = reply.send(operation());
            }))
            .await
            .map_err(|_| UserUpdateError::WriterStopped)?;
        // Once accepted, the job also publishes when its waiting session is cancelled.
        result.await.map_err(|_| UserUpdateError::WriterStopped)?
    }
}

impl IcyBoard {
    /// Wait for jobs accepted before this barrier to finish, including publication.
    /// Stop producers first for shutdown/reload; later jobs are not covered. This
    /// neither closes the writer nor reports earlier jobs' individual failures.
    pub async fn flush_persistence(board: &SharedBoard) -> Res<()> {
        Self::ordered_persistence(board, || ()).await
    }

    /// Run a blocking transaction in the same queue and gate as user/statistics writes.
    /// Admin backups deliberately delay all later writers, but not snapshot readers.
    /// The operation must release any board lock before performing persistence I/O.
    pub async fn ordered_persistence<R: Send + 'static>(board: &SharedBoard, operation: impl FnOnce() -> R + Send + 'static) -> Res<R> {
        let writer = board.lock().await.persistence_writer.clone();
        let gate = writer.gate.clone();
        writer
            .submit(move || {
                let _ordered = gate.lock().map_err(|_| UserUpdateError::WriterPoisoned)?;
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation)).map_err(|_| UserUpdateError::WriterPanicked.into())
            })
            .await
    }

    /// Run user checks and mutations against the latest committed snapshot in writer order.
    /// Successful saves are published even if a later callback step fails or panics.
    pub async fn write_users<R: Send + 'static>(board: &SharedBoard, edit: impl FnOnce(&mut UserWriteContext) -> Res<R> + Send + 'static) -> Res<R> {
        let writer = board.lock().await.persistence_writer.clone();
        let gate = writer.gate.clone();
        let board = board.clone();
        writer
            .submit(move || {
                let _ordered = gate.lock().map_err(|_| UserUpdateError::WriterPoisoned)?;
                let snapshot = {
                    let board = board.blocking_lock();
                    UserWriteSnapshot {
                        users: board.users.clone(),
                        user_revision: board.user_revision,
                        config: board.configuration_snapshot(),
                        root_path: board.root_path.clone(),
                        password_recovery_service: board.password_recovery_service.clone(),
                    }
                };
                let revision = snapshot.user_revision;
                let mut staging = UserWriteContext { snapshot };
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| edit(&mut staging)))
                    .unwrap_or_else(|_| Err(UserUpdateError::WriterPanicked.into()));
                if staging.user_revision != revision {
                    let old_users = {
                        let mut board = board.blocking_lock();
                        board.user_revision = staging.user_revision;
                        std::mem::replace(&mut board.users, staging.snapshot.users)
                    };
                    drop(old_users);
                }
                result
            })
            .await
    }

    /// Statistics have their own payload and retain their historical best-effort disk policy.
    pub async fn write_statistics(board: &SharedBoard, edit: impl FnOnce(&mut Statistics) + Send + 'static) -> Res<()> {
        let writer = board.lock().await.persistence_writer.clone();
        let gate = writer.gate.clone();
        let board = board.clone();
        writer
            .submit(move || {
                let _ordered = gate.lock().map_err(|_| UserUpdateError::WriterPoisoned)?;
                let (mut statistics, path) = {
                    let board = board.blocking_lock();
                    (board.statistics.clone(), board.config.paths.statistics_file.clone())
                };
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| edit(&mut statistics))).map_err(|_| UserUpdateError::WriterPanicked)?;
                if let Err(error) = statistics.save(&path) {
                    log::error!("Error saving statistics to {} : {error}", path.display());
                }
                board.blocking_lock().statistics = statistics;
                Ok(())
            })
            .await
    }
}
