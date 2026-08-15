use std::{
    collections::HashMap,
    fs::{File, OpenOptions, TryLockError},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use super::IcyBoardError;
use crate::Res;

/// One lock file for every tool that writes board data.
pub const LOCK_FILE_NAME: &str = ".icboard.lock";

/// The locks this process holds, so the board server and the web admin it hosts
/// share one file lock instead of shutting each other out.
fn held_locks() -> &'static Mutex<HashMap<PathBuf, (File, usize)>> {
    static HELD: OnceLock<Mutex<HashMap<PathBuf, (File, usize)>>> = OnceLock::new();
    HELD.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Advisory cross-process lock on a board directory, released once the last
/// handle in this process is dropped.
///
/// Tools take it before they write so two of them cannot overwrite each other's
/// idea of the user base.
pub struct BoardLock {
    key: PathBuf,
}

impl BoardLock {
    pub fn acquire(root_path: &Path) -> Res<Self> {
        let path = root_path.join(LOCK_FILE_NAME);
        let mut held = held_locks().lock().map_err(|_| IcyBoardError::ErrorLockingBoard)?;

        let file = OpenOptions::new().create(true).read(true).write(true).truncate(false).open(&path)?;
        let key = std::fs::canonicalize(&path).unwrap_or(path);

        if let Some((_, count)) = held.get_mut(&key) {
            *count += 1;
            return Ok(Self { key });
        }

        match file.try_lock() {
            Ok(()) => {
                held.insert(key.clone(), (file, 1));
                Ok(Self { key })
            }
            Err(TryLockError::WouldBlock) => Err(IcyBoardError::BoardInUse.into()),
            Err(TryLockError::Error(e)) => Err(e.into()),
        }
    }
}

impl Drop for BoardLock {
    fn drop(&mut self) {
        let Ok(mut held) = held_locks().lock() else {
            return;
        };
        if let Some((_, count)) = held.get_mut(&self.key) {
            *count -= 1;
            if *count == 0 {
                held.remove(&self.key);
            }
        }
    }
}
