use crate::Res;
use std::{fs, path::Path};

const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

#[derive(Debug, thiserror::Error)]
#[error("Error reading file '{}': {source}", path.display())]
pub struct FileReadError {
    pub path: std::path::PathBuf,
    #[source]
    pub source: std::io::Error,
}

pub fn read_with_encoding_detection<P: AsRef<Path>>(path: &P) -> Res<String> {
    let data = fs::read(path).map_err(|source| FileReadError {
        path: path.as_ref().to_path_buf(),
        source,
    })?;
    read_data_with_encoding_detection(&data)
}

pub fn read_data_with_encoding_detection(data: &[u8]) -> Res<String> {
    Ok(if data.starts_with(&UTF8_BOM) {
        String::from_utf8_lossy(&data[UTF8_BOM.len()..]).to_string()
    } else {
        crate::tables::import_cp437_string(data, false)
    })
}

/// Writes through a temporary file in the target directory and atomically replaces the target.
pub fn write_atomic<P: AsRef<Path>>(path: P, contents: &[u8]) -> std::io::Result<()> {
    write_atomic_with_before_sync(path, contents, || {})
}

/// The callback supports deterministic persistence fault tests in runtime consumers.
#[doc(hidden)]
pub fn write_atomic_with_before_sync<P: AsRef<Path>>(path: P, contents: &[u8], before_sync: impl FnOnce()) -> std::io::Result<()> {
    use std::io::Write as _;
    let path = path.as_ref();
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(contents)?;
    before_sync();
    tmp.as_file().sync_all()?;
    if let Ok(meta) = fs::metadata(path) {
        let _ = tmp.as_file().set_permissions(meta.permissions());
    }
    tmp.persist(path).map_err(|e| e.error)?;
    // The rename becomes durable only once the directory entry itself is flushed.
    if let Ok(handle) = fs::File::open(dir) {
        let _ = handle.sync_all();
    }
    Ok(())
}
