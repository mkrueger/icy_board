use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use async_trait::async_trait;
use tempfile::NamedTempFile;
use zip::{CompressionMethod, DateTime, ZipWriter, write::SimpleFileOptions};

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::VariableValue,
    parser::{ZIP_ID, ZIP_WRITER_ID},
};

use super::ppl_error::{ERR_INVALID, ERR_IO, ERR_KIND_FILE, ERR_LIMIT, PplError};

type ZipResult<T> = Result<T, PplError>;

fn invalid(message: impl Into<String>) -> PplError {
    PplError::new(ERR_KIND_FILE, ERR_INVALID, message)
}

fn io_error(error: impl std::fmt::Display) -> PplError {
    PplError::new(ERR_KIND_FILE, ERR_IO, error.to_string())
}

#[derive(Debug)]
struct Archive {
    writer: Option<ZipWriter<File>>,
    temporary: Option<NamedTempFile>,
    destination: PathBuf,
    overwrite: bool,
    names: BTreeMap<String, bool>,
    method: CompressionMethod,
    level: Option<i64>,
    timestamp: Option<DateTime>,
    permissions: Option<u32>,
    zip64: bool,
    failure: Option<PplError>,
    cancelled: Arc<AtomicBool>,
}

impl Archive {
    fn create(destination: PathBuf, overwrite: bool) -> ZipResult<Self> {
        if destination.file_name().is_none() || (!overwrite && destination.try_exists().map_err(io_error)?) {
            return Err(invalid("ZIP destination already exists or has no filename"));
        }
        if std::fs::symlink_metadata(&destination).is_ok_and(|metadata| !metadata.is_file()) {
            return Err(invalid("ZIP destination must be a regular file"));
        }
        let temporary = NamedTempFile::new_in(destination.parent().unwrap_or(Path::new("."))).map_err(io_error)?;
        let writer = ZipWriter::new(temporary.reopen().map_err(io_error)?);
        Ok(Self {
            writer: Some(writer),
            temporary: Some(temporary),
            destination,
            overwrite,
            names: BTreeMap::new(),
            method: CompressionMethod::Deflated,
            level: Some(6),
            timestamp: None,
            permissions: None,
            zip64: true,
            failure: None,
            cancelled: Arc::new(AtomicBool::new(false)),
        })
    }

    fn open(&self) -> ZipResult<()> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(invalid("ZIP operation was cancelled"));
        }
        if let Some(failure) = &self.failure {
            return Err(failure.clone());
        }
        if self.writer.is_none() {
            return Err(invalid("ZIP writer is closed"));
        }
        Ok(())
    }

    fn abort(&mut self) {
        self.writer.take();
        self.temporary.take();
    }

    fn added(&mut self, result: ZipResult<()>) -> ZipResult<()> {
        if let Err(error) = &result {
            if self.failure.is_none() {
                self.failure = Some(error.clone());
            }
            self.abort();
        }
        result
    }

    fn name(&mut self, name: &str, directory: bool) -> ZipResult<String> {
        self.open()?;
        let name = if directory { name.strip_suffix('/').unwrap_or(name) } else { name };
        if name.is_empty()
            || name.len() + usize::from(directory) > u16::MAX as usize
            || name.contains(['\\', ':', '\0'])
            || name.split('/').any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(invalid("ZIP entry must have a safe relative archive path"));
        }
        if self.names.contains_key(name)
            || name.match_indices('/').any(|(index, _)| self.names.get(&name[..index]) == Some(&false))
            || (!directory && self.names.keys().any(|existing| existing.starts_with(&format!("{name}/"))))
        {
            return Err(invalid("Duplicate ZIP entry or file/directory collision"));
        }
        if self.names.len() >= 100_000 || (!self.zip64 && self.names.len() >= u16::MAX as usize - 1) {
            return Err(PplError::new(ERR_KIND_FILE, ERR_LIMIT, "ZIP entry limit exceeded"));
        }
        self.names.insert(name.to_string(), directory);
        Ok(if directory { format!("{name}/") } else { name.to_string() })
    }

    fn options(&self, size: u64, directory: bool, modified: Option<DateTime>) -> ZipResult<SimpleFileOptions> {
        if !self.zip64 && size >= u32::MAX as u64 {
            return Err(PplError::new(ERR_KIND_FILE, ERR_LIMIT, "ZIP64 is disabled"));
        }
        Ok(SimpleFileOptions::default()
            .compression_method(self.method)
            .compression_level(self.level)
            .large_file(size >= u32::MAX as u64)
            .last_modified_time(self.timestamp.or(modified).unwrap_or_else(now))
            .unix_permissions(self.permissions.unwrap_or(if directory { 0o755 } else { 0o644 })))
    }

    fn add_bytes(&mut self, data: &[u8], name: &str) -> ZipResult<()> {
        let result = (|| {
            let name = self.name(name, false)?;
            let options = self.options(data.len() as u64, false, None)?;
            let writer = self.writer.as_mut().unwrap();
            writer.start_file(name, options).map_err(io_error)?;
            writer.write_all(data).map_err(io_error)
        })();
        self.added(result)
    }

    fn add_directory(&mut self, name: &str) -> ZipResult<()> {
        let result = (|| {
            let name = self.name(name, true)?;
            let options = self.options(0, true, None)?;
            self.writer.as_mut().unwrap().add_directory(name, options).map_err(io_error)
        })();
        self.added(result)
    }

    fn add_file(&mut self, source: &Path, name: &str) -> ZipResult<()> {
        let result = (|| {
            self.open()?;
            regular_source(source)?;
            let canonical = source.canonicalize().map_err(io_error)?;
            if self.destination.canonicalize().is_ok_and(|destination| destination == canonical)
                || self.temporary.as_ref().is_some_and(|temporary| temporary.path() == canonical)
            {
                return Err(invalid("ZIP cannot include its own output"));
            }
            let mut file = File::open(source).map_err(io_error)?;
            let metadata = file.metadata().map_err(io_error)?;
            let name = self.name(name, false)?;
            let modified = metadata.modified().ok().map(|time| timestamp(chrono::DateTime::<chrono::Utc>::from(time)));
            let options = self.options(metadata.len(), false, modified)?;
            let writer = self.writer.as_mut().unwrap();
            writer.start_file(name, options).map_err(io_error)?;
            let mut copied = 0;
            let mut buffer = [0; 65536];
            loop {
                if self.cancelled.load(Ordering::Acquire) {
                    return Err(invalid("ZIP operation was cancelled"));
                }
                let count = file.read(&mut buffer).map_err(io_error)?;
                if count == 0 {
                    break;
                }
                copied += count as u64;
                if copied > metadata.len() {
                    return Err(io_error("ZIP source grew while being read"));
                }
                writer.write_all(&buffer[..count]).map_err(io_error)?;
            }
            if copied != metadata.len() || file.metadata().map_err(io_error)?.modified().ok() != metadata.modified().ok() {
                return Err(io_error("ZIP source changed while being read"));
            }
            Ok(())
        })();
        self.added(result)
    }

    fn add_tree(&mut self, source: &Path, prefix: &str, recursive: bool) -> ZipResult<()> {
        let result = (|| {
            self.open()?;
            let source = source.canonicalize().map_err(io_error)?;
            let parent = self.destination.parent().unwrap().canonicalize().map_err(io_error)?;
            if parent.starts_with(&source) {
                return Err(invalid("ZIP output must be outside the source tree"));
            }
            if !prefix.is_empty() {
                self.add_directory(prefix)?;
            }
            self.walk_tree(&source, prefix.trim_end_matches('/'), recursive, 0)
        })();
        self.added(result)
    }

    fn walk_tree(&mut self, source: &Path, prefix: &str, recursive: bool, depth: usize) -> ZipResult<()> {
        self.open()?;
        if depth > 128 {
            return Err(PplError::new(ERR_KIND_FILE, ERR_LIMIT, "ZIP directory depth exceeded"));
        }
        reject_links(source)?;
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(source).map_err(io_error)? {
            if entries.len() >= 100_000 {
                return Err(PplError::new(ERR_KIND_FILE, ERR_LIMIT, "ZIP directory entry limit exceeded"));
            }
            entries.push(entry.map_err(io_error)?);
        }
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            self.open()?;
            let filename = entry.file_name().into_string().map_err(|_| invalid("ZIP filename is not Unicode"))?;
            let name = if prefix.is_empty() { filename } else { format!("{prefix}/{filename}") };
            let kind = entry.file_type().map_err(io_error)?;
            if kind.is_file() {
                self.add_file(&entry.path(), &name)?;
            } else if kind.is_dir() {
                if recursive {
                    self.add_directory(&name)?;
                    self.walk_tree(&entry.path(), &name, true, depth + 1)?;
                }
            } else {
                return Err(invalid("ZIP tree contains a symbolic link or special file"));
            }
        }
        Ok(())
    }

    fn finish(&mut self) -> ZipResult<()> {
        let result = (|| {
            self.open()?;
            let file = self.writer.take().unwrap().finish().map_err(io_error)?;
            if !self.zip64 && file.metadata().map_err(io_error)?.len() >= u32::MAX as u64 {
                return Err(PplError::new(ERR_KIND_FILE, ERR_LIMIT, "ZIP64 is disabled"));
            }
            file.sync_all().map_err(io_error)?;
            drop(file);
            if self.cancelled.load(Ordering::Acquire) {
                return Err(invalid("ZIP operation was cancelled"));
            }
            let temporary = self.temporary.take().unwrap();
            if self.overwrite {
                temporary.persist(&self.destination).map_err(io_error)?;
            } else {
                temporary.persist_noclobber(&self.destination).map_err(io_error)?;
            }
            Ok(())
        })();
        self.added(result)
    }
}

fn regular_source(path: &Path) -> ZipResult<()> {
    reject_links(path)?;
    if !std::fs::symlink_metadata(path).map_err(io_error)?.is_file() {
        return Err(invalid("ZIP source is not a regular file"));
    }
    Ok(())
}

fn reject_links(path: &Path) -> ZipResult<()> {
    for ancestor in path.ancestors() {
        if !ancestor.as_os_str().is_empty() && std::fs::symlink_metadata(ancestor).map_err(io_error)?.file_type().is_symlink() {
            return Err(invalid("ZIP sources must not contain symbolic links"));
        }
    }
    Ok(())
}

fn timestamp(time: chrono::DateTime<chrono::Utc>) -> DateTime {
    use chrono::{Datelike, Timelike};
    DateTime::from_date_and_time(
        time.year() as u16,
        time.month() as u8,
        time.day() as u8,
        time.hour() as u8,
        time.minute() as u8,
        time.second() as u8,
    )
    .unwrap_or_default()
}

fn now() -> DateTime {
    timestamp(chrono::Utc::now())
}

#[derive(Debug)]
pub struct PplZip;

#[derive(Debug, Default)]
pub struct PplZipWriter {
    archive: Arc<Mutex<Option<Archive>>>,
}

impl UserData for PplZip {
    const TYPE_NAME: &'static str = "Zip";
    const STATIC_RECEIVER: Option<fn() -> VariableValue> = Some(|| user_data_value(Self, ZIP_ID));
    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        crate::parser::board_catalog::register_members(ZIP_ID, registry);
    }
}

impl UserData for PplZipWriter {
    const TYPE_NAME: &'static str = "ZipWriter";
    const EMPTY_VALUE: Option<fn() -> VariableValue> = Some(|| user_data_value(Self::default(), ZIP_WRITER_ID));
    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        crate::parser::board_catalog::register_members(ZIP_WRITER_ID, registry);
    }
}

fn publish(vm: &mut crate::vm::VirtualMachine<'_>, result: ZipResult<()>) -> VariableValue {
    match result {
        Ok(()) => {
            vm.operation_succeeded();
            VariableValue::new_bool(true)
        }
        Err(error) => {
            vm.set_error(error);
            VariableValue::new_bool(false)
        }
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplZip {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        Err(format!("Unknown ZIP property {name}").into())
    }
    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _value: VariableValue) -> crate::Res<()> {
        Err(format!("ZIP property {name} is read-only").into())
    }
    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        if !name.as_str().eq_ignore_ascii_case("Create") {
            return Err(format!("Unknown ZIP function {name}").into());
        }
        let path = vm.resolve_file(&arguments[0].as_string()).await;
        let overwrite = arguments.get(1).is_some_and(VariableValue::as_bool);
        let result = tokio::task::spawn_blocking(move || Archive::create(path, overwrite))
            .await
            .map_err(io_error)
            .and_then(|result| result);
        let archive = match result {
            Ok(archive) => {
                vm.operation_succeeded();
                Some(archive)
            }
            Err(error) => {
                vm.set_error(error);
                None
            }
        };
        Ok(user_data_value(
            PplZipWriter {
                archive: Arc::new(Mutex::new(archive)),
            },
            ZIP_WRITER_ID,
        ))
    }
    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown ZIP method {name}").into())
    }
}

struct CancelOperation(Option<Arc<AtomicBool>>);
impl Drop for CancelOperation {
    fn drop(&mut self) {
        if let Some(cancelled) = &self.0 {
            cancelled.store(true, Ordering::Release);
        }
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplZipWriter {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        let archive = self.archive.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        Ok(match name.as_str().to_ascii_lowercase().as_str() {
            "valid" => VariableValue::new_bool(archive.as_ref().is_some_and(|archive| archive.open().is_ok())),
            "method" => VariableValue::new_int(if archive.as_ref().is_some_and(|archive| archive.method == CompressionMethod::Stored) {
                0
            } else {
                8
            }),
            "level" => VariableValue::new_int(archive.as_ref().map_or(6, |archive| archive.level.unwrap_or(0) as i32)),
            _ => return Err(format!("Unknown ZIPWRITER property {name}").into()),
        })
    }
    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _value: VariableValue) -> crate::Res<()> {
        Err(format!("ZIPWRITER property {name} is read-only").into())
    }
    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        let method = name.as_str().to_ascii_lowercase();
        let source = if matches!(method.as_str(), "addfile" | "addtree") {
            Some(vm.resolve_file(&arguments[0].as_string()).await)
        } else {
            None
        };
        let cancelled = self
            .archive
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .map(|archive| archive.cancelled.clone());
        let mut cancellation = CancelOperation(cancelled);
        let shared = self.archive.clone();
        let arguments = arguments.to_vec();
        let result = tokio::task::spawn_blocking(move || {
            let mut state = shared.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if method == "abort" {
                if let Some(archive) = state.as_mut() {
                    archive.abort();
                }
                return Ok(());
            }
            let archive = state.as_mut().ok_or_else(|| invalid("Uninitialized ZIP writer"))?;
            archive.open()?;
            match method.as_str() {
                "setcompression" => {
                    let (method, level) = match arguments[0].as_int() {
                        8 => {
                            let level = arguments.get(1).map_or(6, VariableValue::as_int);
                            if !(0..=9).contains(&level) {
                                return Err(invalid("Deflate level must be in 0..9"));
                            }
                            (CompressionMethod::Deflated, Some(i64::from(level)))
                        }
                        0 if arguments.len() == 1 => (CompressionMethod::Stored, None),
                        _ => return Err(invalid("Unsupported ZIP method or level")),
                    };
                    archive.method = method;
                    archive.level = level;
                    Ok(())
                }
                "setcomment" => {
                    let text = arguments[0].as_string();
                    let bytes = match arguments.get(1).map_or(0, VariableValue::as_int) {
                        0 => text.into_bytes(),
                        1 => text
                            .chars()
                            .map(|character| {
                                codepages::tables::CP437_TO_UNICODE
                                    .iter()
                                    .position(|candidate| *candidate == character)
                                    .map(|value| value as u8)
                                    .ok_or_else(|| invalid("Comment is not representable in CP437"))
                            })
                            .collect::<ZipResult<Vec<_>>>()?,
                        _ => return Err(invalid("Unsupported ZIP comment encoding")),
                    };
                    if bytes.len() > u16::MAX as usize {
                        return Err(PplError::new(ERR_KIND_FILE, ERR_LIMIT, "ZIP comment exceeds 65535 bytes"));
                    }
                    archive.writer.as_mut().unwrap().set_raw_comment(bytes.into_boxed_slice()).map_err(io_error)
                }
                "setzip64" => {
                    if !archive.names.is_empty() {
                        return Err(invalid("SetZip64 must precede the first entry"));
                    }
                    archive.zip64 = match arguments[0].as_int() {
                        0 => true,
                        1 => false,
                        _ => return Err(invalid("Unsupported ZIP64 mode")),
                    };
                    Ok(())
                }
                "settimestamputc" => {
                    use crate::executable::temporal::TemporalValue;
                    use chrono::{Datelike, Timelike};
                    let value = match arguments.first().and_then(VariableValue::temporal) {
                        Some(TemporalValue::Timestamp(value)) => value,
                        _ => return Err(invalid("SetTimestampUtc requires a TIMESTAMP")),
                    };
                    archive.timestamp = value
                        .map(|value| {
                            DateTime::from_date_and_time(
                                u16::try_from(value.year()).map_err(|_| invalid("ZIP timestamp must be in 1980..2107"))?,
                                value.month() as u8,
                                value.day() as u8,
                                value.hour() as u8,
                                value.minute() as u8,
                                value.second() as u8,
                            )
                            .map_err(|_| invalid("ZIP timestamp must be in 1980..2107"))
                        })
                        .transpose()?;
                    Ok(())
                }
                "settimestamp" => {
                    use crate::executable::temporal::TemporalValue;
                    use chrono::{Datelike, Timelike};
                    archive.timestamp = match arguments.len() {
                        0 => None,
                        2 => {
                            let Some(TemporalValue::Date(Some(date))) = arguments[0].temporal() else {
                                return Err(invalid("SetTimestamp requires a nonempty DATE"));
                            };
                            let Some(TemporalValue::Time(Some(time))) = arguments[1].temporal() else {
                                return Err(invalid("SetTimestamp requires a nonempty TIME"));
                            };
                            Some(
                                DateTime::from_date_and_time(
                                    u16::try_from(date.year()).map_err(|_| invalid("ZIP timestamp must be in 1980..2107"))?,
                                    date.month() as u8,
                                    date.day() as u8,
                                    time.hour() as u8,
                                    time.minute() as u8,
                                    time.second() as u8,
                                )
                                .map_err(|_| invalid("ZIP timestamp must be in 1980..2107"))?,
                            )
                        }
                        _ => return Err(invalid("SetTimestamp requires either date and time or no arguments")),
                    };
                    Ok(())
                }
                "setpermissions" => {
                    let mode = arguments.first().map(VariableValue::as_int);
                    if mode.is_some_and(|mode| !(0..=0o777).contains(&mode)) {
                        return Err(invalid("ZIP permissions must be in 0..511"));
                    }
                    archive.permissions = mode.map(|mode| mode as u32);
                    Ok(())
                }
                "addfile" => {
                    let source = source.unwrap();
                    let name = arguments
                        .get(1)
                        .map(VariableValue::as_string)
                        .or_else(|| source.file_name().and_then(|name| name.to_str()).map(str::to_string))
                        .unwrap_or_default();
                    archive.add_file(&source, &name)
                }
                "adddirectory" => archive.add_directory(&arguments[0].as_string()),
                "addbytes" => archive.add_bytes(arguments[0].as_byte_slice(), &arguments[1].as_string()),
                "addtree" => {
                    let source = source.unwrap();
                    let result = reject_links(&source);
                    archive.added(result)?;
                    archive.add_tree(
                        &source,
                        &arguments.get(1).map(VariableValue::as_string).unwrap_or_default(),
                        arguments.get(2).is_none_or(VariableValue::as_bool),
                    )
                }
                "finish" => archive.finish(),
                _ => Err(invalid(format!("Unknown ZIPWRITER function {method}"))),
            }
        })
        .await
        .map_err(io_error)
        .and_then(|result| result);
        cancellation.0 = None;
        Ok(publish(vm, result))
    }
    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown ZIPWRITER method {name}").into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zip_archive_publication_and_failure() {
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join("test.zip");
        std::fs::write(&destination, b"previous").unwrap();
        let mut archive = Archive::create(destination.clone(), true).unwrap();
        archive.add_directory("docs").unwrap();
        archive.add_bytes(b"hello", "docs/readme.txt").unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), b"previous");
        archive.finish().unwrap();
        let bytes = std::fs::read(&destination).unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
        assert_eq!(zip.len(), 2);
        let mut entry = zip.by_name("docs/readme.txt").unwrap();
        assert_eq!(entry.compression(), CompressionMethod::Deflated);
        let mut text = String::new();
        entry.read_to_string(&mut text).unwrap();
        assert_eq!(text, "hello");
        let mut failed = Archive::create(destination.clone(), true).unwrap();
        failed.add_bytes(b"first", "same.txt").unwrap();
        assert!(failed.add_bytes(b"second", "same.txt").is_err());
        assert!(failed.finish().is_err());
        assert_eq!(std::fs::read(&destination).unwrap(), bytes);
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    }
}
