use std::{
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
};

use crate::error::{AdminError, Result};

pub use icy_board_engine::icy_board::lock::LOCK_FILE_NAME;

pub const BACKUP_DIR_NAME: &str = "backups";
pub const AUDIT_LOG_NAME: &str = "icbadmin-audit.log";

/// The board wide advisory lock, reported as a conflict the API can answer with.
pub struct BoardLock(#[allow(dead_code)] icy_board_engine::icy_board::lock::BoardLock);

impl BoardLock {
    pub fn acquire(root_path: &Path) -> Result<Self> {
        icy_board_engine::icy_board::lock::BoardLock::acquire(root_path)
            .map(Self)
            .map_err(|_| AdminError::Locked)
    }
}

/// Identifies the on-disk state of a file so concurrent edits by another tool can be detected.
pub fn fingerprint(path: &Path) -> Result<String> {
    let meta = fs::metadata(path)?;
    let modified = meta
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| AdminError::Io(std::io::Error::other(e)))?;
    Ok(format!("{}-{}.{}", meta.len(), modified.as_secs(), modified.subsec_nanos()))
}

pub fn check_fingerprint(path: &Path, expected: &str) -> Result<()> {
    if fingerprint(path)? == expected { Ok(()) } else { Err(AdminError::Conflict) }
}

/// Copies `path` into `<root>/backups/<name>.<timestamp>.bak` before it gets overwritten.
pub fn create_backup(root_path: &Path, path: &Path) -> Result<PathBuf> {
    let backup_dir = root_path.join(BACKUP_DIR_NAME);
    fs::create_dir_all(&backup_dir)?;

    let file_name = path.file_name().unwrap_or_else(|| std::ffi::OsStr::new("config")).to_string_lossy().to_string();
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S%.3f");
    let backup = backup_dir.join(format!("{file_name}.{stamp}.bak"));
    fs::copy(path, &backup)?;
    Ok(backup)
}

/// Appends a single JSON line describing an administrative change.
pub fn append_audit(root_path: &Path, entry: &serde_json::Value) {
    use std::io::Write as _;

    let path = root_path.join(AUDIT_LOG_NAME);
    let line = format!("{}\n", entry);
    let result = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| f.write_all(line.as_bytes()));
    if let Err(e) = result {
        log::error!("Could not write audit log {}: {}", path.display(), e);
    }
}
