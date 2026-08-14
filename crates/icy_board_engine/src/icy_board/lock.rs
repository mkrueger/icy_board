use std::{
    fs::{File, OpenOptions, TryLockError},
    path::Path,
};

use super::IcyBoardError;
use crate::Res;

/// One lock file for every tool that writes board data.
pub const LOCK_FILE_NAME: &str = ".icboard.lock";

/// Advisory cross-process lock on a board directory, released when dropped.
///
/// Tools take it before they write so two of them cannot overwrite each other's
/// idea of the user base.
pub struct BoardLock {
    _file: File,
}

impl BoardLock {
    pub fn acquire(root_path: &Path) -> Res<Self> {
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(root_path.join(LOCK_FILE_NAME))?;
        match file.try_lock() {
            Ok(()) => Ok(Self { _file: file }),
            Err(TryLockError::WouldBlock) => Err(IcyBoardError::BoardInUse.into()),
            Err(TryLockError::Error(e)) => Err(e.into()),
        }
    }

    /// Tries the lock and reports whether it was free, without holding it.
    pub fn is_available(root_path: &Path) -> bool {
        Self::acquire(root_path).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_lock_on_the_same_board_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let held = BoardLock::acquire(dir.path()).unwrap();
        assert!(BoardLock::acquire(dir.path()).is_err());
        drop(held);
        assert!(BoardLock::acquire(dir.path()).is_ok());
    }
}
