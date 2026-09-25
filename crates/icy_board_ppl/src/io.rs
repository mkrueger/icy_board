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

/// Runtime 400 text reads also take UTF-8 without a BOM. Only the text before the
/// first NUL or Ctrl-Z decides, so a CP437 SAUCE record behind the EOF marker does
/// not turn a UTF-8 file into CP437.
pub fn read_data_with_utf8_detection(data: &[u8]) -> Res<String> {
    if !data.starts_with(&UTF8_BOM)
        && let Ok(text) = std::str::from_utf8(text_before_eof(data))
    {
        return Ok(text.to_string());
    }
    read_data_with_encoding_detection(data)
}

/// Decodes the bytes of a runtime 400 `FREAD` into a string. A leading BOM is dropped and
/// the text before the first Ctrl-Z is UTF-8 when it is valid UTF-8. Everything from the
/// Ctrl-Z on stays CP437, the encoding SAUCE records are defined in.
pub fn decode_utf8_or_cp437(data: &[u8]) -> String {
    let (has_bom, body) = match data.strip_prefix(&UTF8_BOM) {
        Some(rest) => (true, rest),
        None => (false, data),
    };
    let end = body.iter().position(|&b| b == 0x1A).unwrap_or(body.len());
    let (text, tail) = body.split_at(end);
    let mut result = match std::str::from_utf8(text) {
        Ok(text) => text.to_string(),
        Err(_) if has_bom => String::from_utf8_lossy(text).into_owned(),
        Err(_) => return body.iter().map(|&b| codepages::tables::CP437_TO_UNICODE[b as usize]).collect(),
    };
    result.extend(tail.iter().map(|&b| codepages::tables::CP437_TO_UNICODE[b as usize]));
    result
}

/// Strips one leading UTF-8 BOM.
pub fn strip_utf8_bom(data: &[u8]) -> &[u8] {
    data.strip_prefix(&UTF8_BOM).unwrap_or(data)
}

fn text_before_eof(data: &[u8]) -> &[u8] {
    let end = data.iter().position(|&b| b == 0 || b == 0x1A).unwrap_or(data.len());
    &data[..end]
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
