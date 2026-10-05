use std::{
    fs::{File, Metadata},
    io::{BufReader, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use tempfile::NamedTempFile;
use unarc_rs::{
    error::ArchiveError,
    unified::{ArchiveEntry, ArchiveEntryKind, ArchiveFormat, ArchiveOptions, UnifiedArchive},
};

use super::ppl_error::{ERR_DENIED, ERR_FORMAT, ERR_INVALID, ERR_IO, ERR_KIND_FILE, ERR_LIMIT, ERR_UNAVAILABLE, ERR_UNSUPPORTED, PplError};
use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::{GenericVariableData, VariableType, VariableValue, temporal::TemporalValue},
    parser::{ARCHIVE_ENTRY_ID, ARCHIVE_ENTRY_KIND_ENUM_ID, ARCHIVE_ID, ARCHIVE_OPTIONS_ID, ARCHIVE_READER_ID},
};

type ArchiveResult<T> = Result<T, PplError>;
fn error(code: i32, message: impl Into<String>) -> PplError {
    PplError::new(ERR_KIND_FILE, code, message)
}
fn io_error(e: std::io::Error) -> PplError {
    error(
        match e.kind() {
            std::io::ErrorKind::NotFound => ERR_UNAVAILABLE,
            std::io::ErrorKind::PermissionDenied => ERR_DENIED,
            _ => ERR_IO,
        },
        e.to_string(),
    )
}
fn backend_error(e: ArchiveError) -> PplError {
    let code = match e {
        ArchiveError::Io(e) if matches!(e.kind(), std::io::ErrorKind::InvalidData | std::io::ErrorKind::UnexpectedEof) => {
            return error(ERR_FORMAT, "Invalid or damaged archive");
        }
        ArchiveError::Io(e) => {
            return match e.kind() {
                std::io::ErrorKind::NotFound => error(ERR_UNAVAILABLE, "Archive data is unavailable"),
                std::io::ErrorKind::PermissionDenied => error(ERR_DENIED, "Archive data access was denied"),
                _ => error(ERR_IO, "Archive data could not be read"),
            };
        }
        ArchiveError::PasswordRequired { .. } | ArchiveError::InvalidPassword { .. } | ArchiveError::EncryptionRequired { .. } => ERR_DENIED,
        ArchiveError::UnsupportedMethod { .. } | ArchiveError::UnsupportedFormat(_) => ERR_UNSUPPORTED,
        ArchiveError::SizeLimitExceeded { .. } => ERR_LIMIT,
        _ => ERR_FORMAT,
    };
    // Backend messages may contain password-related input; expose only a stable category.
    error(
        code,
        match code {
            ERR_DENIED => "Archive password is missing or incorrect",
            ERR_UNSUPPORTED => "Archive operation is unsupported",
            ERR_LIMIT => "Archive resource limit exceeded",
            _ => "Invalid or damaged archive",
        },
    )
}
fn lock<T>(value: &Mutex<T>) -> ArchiveResult<std::sync::MutexGuard<'_, T>> {
    value.lock().map_err(|_| error(ERR_IO, "Archive handle lock is poisoned"))
}

#[derive(Clone, Debug)]
struct Options {
    password: icy_board_ppl::password::Password,
    format: String,
    entry_bytes: u64,
    total_bytes: u64,
    entries: u64,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            password: icy_board_ppl::password::Password::Protected(String::new()),
            format: String::new(),
            entry_bytes: 16 * 1024 * 1024,
            total_bytes: 64 * 1024 * 1024,
            entries: 10_000,
        }
    }
}
impl Options {
    fn backend(&self, remaining: u64) -> ArchiveOptions {
        let options = ArchiveOptions::new()
            .with_max_entry_size(Some(self.entry_bytes))
            .with_max_total_size(Some(remaining));
        match &self.password {
            icy_board_ppl::password::Password::PlainText(p) | icy_board_ppl::password::Password::Protected(p) if !p.is_empty() => {
                options.with_password(p.clone())
            }
            _ => options,
        }
    }
}
fn format_named(name: &str) -> Option<ArchiveFormat> {
    ArchiveFormat::ALL.iter().copied().find(|format| format.name().eq_ignore_ascii_case(name))
}
fn detect(file: &mut File, path: &Path) -> ArchiveResult<ArchiveFormat> {
    let detected = ArchiveFormat::detect(file, Some(path)).map_err(io_error)?;
    // unarc-rs 0.7.2 reports compressed TAR as plain gzip/bzip2/compress; newer releases map these themselves.
    match (detected, ArchiveFormat::from_path(path)) {
        (Some(ArchiveFormat::Gz), Some(ArchiveFormat::Tgz)) => Ok(ArchiveFormat::Tgz),
        (Some(ArchiveFormat::Bz2), Some(ArchiveFormat::Tbz)) => Ok(ArchiveFormat::Tbz),
        (Some(ArchiveFormat::Z), Some(ArchiveFormat::TarZ)) => Ok(ArchiveFormat::TarZ),
        (Some(format), _) => Ok(format),
        (None, _) => Err(error(ERR_FORMAT, "Archive format could not be detected")),
    }
}

struct Reader {
    source: File,
    source_path: PathBuf,
    identity: Metadata,
    archive: Option<UnifiedArchive<BufReader<File>>>,
    format: ArchiveFormat,
    options: Options,
    single_name: Option<String>,
    ace_replay_history: bool,
    ace_needs_replay: bool,
    current: Option<ArchiveEntry>,
    consumed: bool,
    index: i64,
    visited: u64,
    bytes: u64,
}
impl Reader {
    fn open(path: PathBuf, options: Options) -> ArchiveResult<Self> {
        reject_link(&path)?;
        if !std::fs::metadata(&path).map_err(io_error)?.is_file() {
            return Err(error(ERR_INVALID, "Archive source must be a regular file"));
        }
        let mut source = File::open(&path).map_err(io_error)?;
        let identity = source.metadata().map_err(io_error)?;
        let source_path = path.canonicalize().map_err(io_error)?;
        let format = if options.format.is_empty() {
            detect(&mut source, &path)?
        } else {
            format_named(&options.format).ok_or_else(|| error(ERR_INVALID, "Unknown archive format"))?
        };
        source.seek(SeekFrom::Start(0)).map_err(io_error)?;
        let ace_replay_history = if format == ArchiveFormat::Ace {
            let archive = unarc_rs::ace::AceArchive::new(BufReader::new(source.try_clone().map_err(io_error)?)).map_err(backend_error)?;
            archive.main_header().header_flags & 0x8000 != 0
        } else {
            false
        };
        source.seek(SeekFrom::Start(0)).map_err(io_error)?;
        let mut archive = format
            .open_with_options(BufReader::new(source.try_clone().map_err(io_error)?), options.backend(options.total_bytes))
            .map_err(backend_error)?;
        let single_name = matches!(format, ArchiveFormat::Z | ArchiveFormat::Gz | ArchiveFormat::Bz2)
            .then(|| path.file_stem().and_then(|s| s.to_str()).map(str::to_string))
            .flatten();
        if let Some(name) = &single_name {
            archive.set_single_file_name(name.clone());
        }
        Ok(Self {
            source,
            source_path,
            identity,
            archive: Some(archive),
            format,
            options,
            single_name,
            ace_replay_history,
            ace_needs_replay: false,
            current: None,
            consumed: false,
            index: -1,
            visited: 0,
            bytes: 0,
        })
    }
    fn ensure_open(&self) -> ArchiveResult<()> {
        if self.archive.is_none() {
            return Err(error(ERR_INVALID, "Archive reader is closed or invalid"));
        }
        let now = self.source.metadata().map_err(io_error)?;
        if now.len() != self.identity.len() || now.modified().ok() != self.identity.modified().ok() {
            return Err(error(ERR_IO, "Archive source changed"));
        }
        Ok(())
    }
    fn invalidate<T>(&mut self, result: ArchiveResult<T>) -> ArchiveResult<T> {
        if result.is_err() {
            self.archive.take();
            self.current.take();
        }
        result
    }
    fn read(&mut self) -> ArchiveResult<Vec<u8>> {
        self.ensure_open()?;
        let entry = self.current.clone().ok_or_else(|| error(ERR_INVALID, "No current archive entry"))?;
        if self.consumed {
            return Err(error(ERR_INVALID, "Archive entry was already consumed"));
        }
        match entry.kind() {
            ArchiveEntryKind::File => {}
            ArchiveEntryKind::Directory => return Err(error(ERR_INVALID, "Archive directories cannot be read")),
            _ => return Err(error(ERR_UNSUPPORTED, "Only regular archive files can be read")),
        }
        self.consumed = true;
        let result = (|| {
            if self.ace_needs_replay {
                self.replay_ace()?;
            }
            let entry = self
                .current
                .clone()
                .ok_or_else(|| error(ERR_FORMAT, "Archive entry disappeared during replay"))?;
            self.decode(&entry)
        })();
        self.invalidate(result)
    }
    fn decode(&mut self, entry: &ArchiveEntry) -> ArchiveResult<Vec<u8>> {
        let remaining = self.options.total_bytes.saturating_sub(self.bytes);
        self.archive.as_mut().unwrap().set_options(
            self.options
                .backend(self.options.total_bytes)
                .with_max_entry_size(Some(self.options.entry_bytes.min(remaining))),
        );
        self.archive.as_mut().unwrap().read(entry).map_err(backend_error).and_then(|bytes| {
            self.ensure_open()?;
            if bytes.len() as u64 > self.options.entry_bytes || bytes.len() as u64 > remaining {
                Err(error(ERR_LIMIT, "Archive byte limit exceeded"))
            } else {
                self.bytes += bytes.len() as u64;
                Ok(bytes)
            }
        })
    }
    fn replay_ace(&mut self) -> ArchiveResult<()> {
        self.archive.take();
        self.source.seek(SeekFrom::Start(0)).map_err(io_error)?;
        self.archive = Some(
            self.format
                .open_with_options(
                    BufReader::new(self.source.try_clone().map_err(io_error)?),
                    self.options.backend(self.options.total_bytes.saturating_sub(self.bytes)),
                )
                .map_err(backend_error)?,
        );
        for position in 0..=self.index {
            let entry = self
                .archive
                .as_mut()
                .unwrap()
                .next_entry()
                .map_err(backend_error)?
                .ok_or_else(|| error(ERR_FORMAT, "Archive ended during solid ACE replay"))?;
            if position == self.index {
                self.current = Some(entry);
                self.ace_needs_replay = false;
                return Ok(());
            }
            // Solid ACE skips do not update the dictionary; replayed payloads consume the same decode budget.
            if entry.kind() == ArchiveEntryKind::File {
                self.decode(&entry)?;
            } else {
                self.archive.as_mut().unwrap().skip(&entry).map_err(backend_error)?;
            }
        }
        Err(error(ERR_FORMAT, "Invalid solid ACE replay position"))
    }
    fn next(&mut self) -> ArchiveResult<bool> {
        self.ensure_open()?;
        let result = (|| {
            if let Some(entry) = self.current.clone() {
                if !self.consumed {
                    self.archive.as_mut().unwrap().skip(&entry).map_err(backend_error)?;
                    if self.ace_replay_history && entry.kind() == ArchiveEntryKind::File {
                        self.ace_needs_replay = true;
                    }
                }
            }
            self.current = None;
            let entry = self.archive.as_mut().unwrap().next_entry().map_err(backend_error)?;
            let Some(entry) = entry else { return Ok(false) };
            if self.visited >= self.options.entries {
                return Err(error(ERR_LIMIT, "Archive entry limit exceeded"));
            }
            i64::try_from(entry.original_size()).map_err(|_| error(ERR_LIMIT, "Archive size exceeds LONG"))?;
            i64::try_from(entry.compressed_size()).map_err(|_| error(ERR_LIMIT, "Archive compressed size exceeds LONG"))?;
            self.visited += 1;
            self.index += 1;
            self.current = Some(entry);
            self.consumed = false;
            Ok(true)
        })();
        self.invalidate(result)
    }
    fn rewind(&mut self) -> ArchiveResult<()> {
        self.ensure_open()?;
        self.archive.take();
        self.current.take();
        self.source.seek(SeekFrom::Start(0)).map_err(io_error)?;
        let mut archive = self
            .format
            .open_with_options(
                BufReader::new(self.source.try_clone().map_err(io_error)?),
                self.options.backend(self.options.total_bytes.saturating_sub(self.bytes)),
            )
            .map_err(backend_error)?;
        if let Some(name) = &self.single_name {
            archive.set_single_file_name(name.clone());
        }
        self.archive = Some(archive);
        self.index = -1;
        self.consumed = false;
        self.ace_needs_replay = false;
        Ok(())
    }
}
/// Only the file itself may not be a link. Linked directories on the way, such as a
/// board below a symlinked home or macOS's `/tmp`, are followed like any PPL file access.
fn reject_link(path: &Path) -> ArchiveResult<()> {
    if std::fs::symlink_metadata(path).map_err(io_error)?.file_type().is_symlink() {
        return Err(error(ERR_DENIED, "Archive files must not be symbolic links"));
    }
    Ok(())
}
fn same_file(left: &Metadata, right: &Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        left.dev() == right.dev() && left.ino() == right.ino()
    }
    #[cfg(not(unix))]
    {
        left.created().ok() == right.created().ok() && left.len() == right.len() && left.modified().ok() == right.modified().ok()
    }
}
fn destination(path: &Path, source_path: &Path, source: &Metadata, overwrite: bool) -> ArchiveResult<PathBuf> {
    if path.file_name().is_none() || path.components().any(|p| matches!(p, std::path::Component::ParentDir)) {
        return Err(error(ERR_INVALID, "Archive extraction needs a file destination"));
    }
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let path = parent.canonicalize().map_err(io_error)?.join(path.file_name().unwrap());
    if path == source_path {
        return Err(error(ERR_DENIED, "Archive extraction cannot overwrite its source"));
    }
    match std::fs::symlink_metadata(&path) {
        Ok(metadata) => {
            if !metadata.is_file() || metadata.file_type().is_symlink() || same_file(source, &metadata) {
                return Err(error(ERR_DENIED, "Unsafe archive extraction destination"));
            }
            if !overwrite {
                return Err(error(ERR_INVALID, "Archive extraction destination already exists"));
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(io_error(e)),
    }
    Ok(path)
}

#[derive(Debug)]
pub struct PplArchive;
#[derive(Debug, Default)]
pub struct PplArchiveOptions {
    options: Arc<Mutex<Options>>,
}
#[derive(Default)]
pub struct PplArchiveReader {
    reader: Arc<Mutex<Option<Reader>>>,
}
impl std::fmt::Debug for PplArchiveReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ArchiveReader")
    }
}
#[derive(Debug, Default)]
pub struct PplArchiveEntry {
    entry: Option<ArchiveEntry>,
    index: i64,
}
macro_rules! data_type {
    ($type:ty, $name:literal, $id:ident) => {
        impl UserData for $type {
            const TYPE_NAME: &'static str = $name;
            const EMPTY_VALUE: Option<fn() -> VariableValue> = Some(|| user_data_value(Self::default(), $id));
            fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
                crate::parser::board_catalog::register_members($id, registry);
            }
        }
    };
}
data_type!(PplArchiveOptions, "ArchiveOptions", ARCHIVE_OPTIONS_ID);
data_type!(PplArchiveReader, "ArchiveReader", ARCHIVE_READER_ID);
data_type!(PplArchiveEntry, "ArchiveEntry", ARCHIVE_ENTRY_ID);
impl UserData for PplArchive {
    const TYPE_NAME: &'static str = "Archive";
    const STATIC_RECEIVER: Option<fn() -> VariableValue> = Some(|| user_data_value(Self, ARCHIVE_ID));
    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        crate::parser::board_catalog::register_members(ARCHIVE_ID, registry);
    }
}
fn publish(vm: &mut crate::vm::VirtualMachine<'_>, result: ArchiveResult<VariableValue>, fallback: VariableValue) -> VariableValue {
    match result {
        Ok(value) => {
            vm.operation_succeeded();
            value
        }
        Err(e) => {
            vm.set_error(e);
            fallback
        }
    }
}
#[async_trait(?Send)]
impl UserDataValue for PplArchive {
    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _value: VariableValue) -> crate::Res<()> {
        Err(format!("Archive property {name} is read-only").into())
    }
    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown archive method {name}").into())
    }
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        Err(format!("Unknown Archive property {name}").into())
    }
    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        match name.as_str().to_ascii_lowercase().as_str() {
            "options" => {
                vm.operation_succeeded();
                Ok(user_data_value(PplArchiveOptions::default(), ARCHIVE_OPTIONS_ID))
            }
            "formats" => {
                vm.operation_succeeded();
                Ok(VariableValue {
                    vtype: VariableType::UnboundedString,
                    data: Default::default(),
                    generic_data: GenericVariableData::Dim1(Arc::new(
                        ArchiveFormat::ALL
                            .iter()
                            .map(|f| VariableValue::new_unbounded_string(f.name().to_string()))
                            .collect(),
                    )),
                })
            }
            "open" => {
                let options = if let Some(value) = arguments.get(1) {
                    match &value.generic_data {
                        GenericVariableData::UserData(value) => {
                            let data = crate::compiler::user_data::runtime_object(value.as_ref(), ARCHIVE_OPTIONS_ID as u32)?;
                            let options = (data as &dyn std::any::Any)
                                .downcast_ref::<PplArchiveOptions>()
                                .ok_or("ArchiveOptions required")?;
                            lock(&options.options).map(|v| v.clone())
                        }
                        _ => Err(error(ERR_INVALID, "ArchiveOptions required")),
                    }
                } else {
                    Ok(Options::default())
                };
                let path = vm.resolve_file(&arguments[0].as_string()).await;
                let result = match options {
                    Ok(options) => tokio::task::spawn_blocking(move || Reader::open(path, options))
                        .await
                        .map_err(|_| error(ERR_IO, "Archive worker failed"))
                        .and_then(|r| r),
                    Err(e) => Err(e),
                };
                let reader = match result {
                    Ok(reader) => {
                        vm.operation_succeeded();
                        Some(reader)
                    }
                    Err(e) => {
                        vm.set_error(e);
                        None
                    }
                };
                Ok(user_data_value(
                    PplArchiveReader {
                        reader: Arc::new(Mutex::new(reader)),
                    },
                    ARCHIVE_READER_ID,
                ))
            }
            _ => Err(format!("Unknown Archive function {name}").into()),
        }
    }
}
#[async_trait(?Send)]
impl UserDataValue for PplArchiveOptions {
    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown archive method {name}").into())
    }
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        let options = lock(&self.options).map_err(|e| e.message)?;
        Ok(match name.as_str().to_ascii_lowercase().as_str() {
            "password" => VariableValue::new_password(options.password.protected()),
            "format" => VariableValue::new_unbounded_string(options.format.clone()),
            "maxentrybytes" => VariableValue::new_long(options.entry_bytes as i64),
            "maxtotalbytes" => VariableValue::new_long(options.total_bytes as i64),
            "maxentries" => VariableValue::new_int(options.entries as i32),
            _ => return Err(format!("Unknown ArchiveOptions property {name}").into()),
        })
    }
    async fn set_property_value(&self, vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, value: VariableValue) -> crate::Res<()> {
        let result = (|| {
            let mut options = lock(&self.options)?;
            match name.as_str().to_ascii_lowercase().as_str() {
                "password" => match value.generic_data {
                    GenericVariableData::Password(p @ (icy_board_ppl::password::Password::PlainText(_) | icy_board_ppl::password::Password::Protected(_))) => {
                        options.password = p
                    }
                    GenericVariableData::String(p) => options.password = icy_board_ppl::password::Password::Protected(p.as_ref().clone()),
                    _ => return Err(error(ERR_INVALID, "Archive password must contain a native plaintext secret")),
                },
                "format" => {
                    let name = value.as_string();
                    if !name.is_empty() && format_named(&name).is_none() {
                        return Err(error(ERR_INVALID, "Unknown archive format"));
                    }
                    options.format = name;
                }
                key @ ("maxentrybytes" | "maxtotalbytes" | "maxentries") => {
                    let number = value.as_long();
                    let cap = match key {
                        "maxentrybytes" => 64 * 1024 * 1024,
                        "maxtotalbytes" => 256 * 1024 * 1024,
                        _ => 100_000,
                    };
                    if number <= 0 {
                        return Err(error(ERR_INVALID, "Archive limit must be positive"));
                    }
                    if number > cap {
                        return Err(error(ERR_LIMIT, "Archive limit exceeds the host hard cap"));
                    }
                    match key {
                        "maxentrybytes" => options.entry_bytes = number as u64,
                        "maxtotalbytes" => options.total_bytes = number as u64,
                        _ => options.entries = number as u64,
                    }
                }
                _ => return Err(error(ERR_INVALID, "Unknown ArchiveOptions property")),
            }
            Ok(VariableValue::new_bool(true))
        })();
        publish(vm, result, VariableValue::new_bool(false));
        Ok(())
    }
    async fn call_function(
        &self,
        _vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        _arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        Err(format!("Unknown ArchiveOptions function {name}").into())
    }
}
enum Operation {
    Value(VariableValue),
    Extract(NamedTempFile, PathBuf, PathBuf, Metadata, bool),
}
#[async_trait(?Send)]
impl UserDataValue for PplArchiveReader {
    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _value: VariableValue) -> crate::Res<()> {
        Err(format!("Archive property {name} is read-only").into())
    }
    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown archive method {name}").into())
    }
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        let reader = lock(&self.reader).map_err(|e| e.message)?;
        Ok(match name.as_str().to_ascii_lowercase().as_str() {
            "valid" => VariableValue::new_bool(reader.as_ref().is_some_and(|r| r.ensure_open().is_ok())),
            "format" => VariableValue::new_unbounded_string(reader.as_ref().map_or(String::new(), |r| r.format.name().to_string())),
            "entry" => user_data_value(
                PplArchiveEntry {
                    entry: reader.as_ref().and_then(|r| r.current.clone()),
                    index: reader.as_ref().map_or(0, |r| r.index),
                },
                ARCHIVE_ENTRY_ID,
            ),
            _ => return Err(format!("Unknown ArchiveReader property {name}").into()),
        })
    }
    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        let method = name.as_str().to_ascii_lowercase();
        let fallback = match method.as_str() {
            "readbytes" => VariableValue::new_bytes(Vec::new()),
            "readtext" => VariableValue::new_unbounded_string(String::new()),
            _ => VariableValue::new_bool(false),
        };
        let destination_path = if method == "extract" {
            Some(vm.resolve_file(&arguments[0].as_string()).await)
        } else {
            None
        };
        let overwrite = arguments.get(1).is_some_and(VariableValue::as_bool);
        let handle = self.reader.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut handle = lock(&handle)?;
            if method == "close" {
                handle.take();
                return Ok(Operation::Value(VariableValue::new_bool(true)));
            }
            let reader = handle.as_mut().ok_or_else(|| error(ERR_INVALID, "Archive reader is empty"))?;
            match method.as_str() {
                "next" => reader.next().map(|v| Operation::Value(VariableValue::new_bool(v))),
                "rewind" => reader.rewind().map(|_| Operation::Value(VariableValue::new_bool(true))),
                "readbytes" => reader.read().map(|v| Operation::Value(VariableValue::new_bytes(v))),
                "readtext" => reader
                    .read()
                    .and_then(|v| {
                        let end = v.iter().position(|b| matches!(b, 0 | 0x1a)).unwrap_or(v.len());
                        icy_board_ppl::io::read_data_with_utf8_detection(&v[..end]).map_err(|_| error(ERR_FORMAT, "Invalid archive text"))
                    })
                    .map(|v| Operation::Value(VariableValue::new_unbounded_string(v))),
                "extract" => {
                    reader.ensure_open()?;
                    let path = destination(destination_path.as_ref().unwrap(), &reader.source_path, &reader.identity, overwrite)?;
                    let mut temporary = NamedTempFile::new_in(path.parent().unwrap()).map_err(io_error)?;
                    temporary.write_all(&reader.read()?).map_err(io_error)?;
                    temporary.as_file().sync_all().map_err(io_error)?;
                    Ok(Operation::Extract(
                        temporary,
                        path,
                        reader.source_path.clone(),
                        reader.identity.clone(),
                        overwrite,
                    ))
                }
                _ => Err(error(ERR_INVALID, "Unknown ArchiveReader function")),
            }
        })
        .await
        .map_err(|_| error(ERR_IO, "Archive worker failed"))
        .and_then(|r| r)
        .and_then(|operation| match operation {
            Operation::Value(value) => Ok(value),
            Operation::Extract(temporary, path, source_path, source, overwrite) => {
                // Commit only in the awaited caller: a cancelled task can never publish its temporary file.
                let path = destination(&path, &source_path, &source, overwrite)?;
                if overwrite {
                    temporary.persist(path).map_err(|e| io_error(e.error))?;
                } else {
                    temporary.persist_noclobber(path).map_err(|e| io_error(e.error))?;
                }
                Ok(VariableValue::new_bool(true))
            }
        });
        Ok(publish(vm, result, fallback))
    }
}
#[async_trait(?Send)]
impl UserDataValue for PplArchiveEntry {
    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _value: VariableValue) -> crate::Res<()> {
        Err(format!("Archive property {name} is read-only").into())
    }
    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown archive method {name}").into())
    }
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        let entry = self.entry.as_ref();
        let kind = entry.map_or(ArchiveEntryKind::Unknown, ArchiveEntry::kind);
        let time = entry.and_then(ArchiveEntry::modified_time);
        let kind_number = match kind {
            ArchiveEntryKind::File => 0,
            ArchiveEntryKind::Directory => 1,
            ArchiveEntryKind::SymbolicLink => 2,
            ArchiveEntryKind::HardLink => 3,
            ArchiveEntryKind::Special => 4,
            ArchiveEntryKind::Unknown => 5,
        };
        Ok(match name.as_str().to_ascii_lowercase().as_str() {
            "valid" => VariableValue::new_bool(entry.is_some()),
            "index" => VariableValue::new_long(if entry.is_some() { self.index } else { 0 }),
            "name" => VariableValue::new_unbounded_string(entry.map_or("", ArchiveEntry::name).to_string()),
            "filename" => VariableValue::new_unbounded_string(entry.map_or("", ArchiveEntry::file_name).to_string()),
            "size" => VariableValue::new_long(entry.map_or(0, |e| e.original_size() as i64)),
            "compressedsize" => VariableValue::new_long(entry.map_or(0, |e| e.compressed_size() as i64)),
            "method" => VariableValue::new_unbounded_string(entry.map_or("", ArchiveEntry::compression_method).to_string()),
            "kind" => VariableValue::new_enum(VariableType::UserData(ARCHIVE_ENTRY_KIND_ENUM_ID as u32), kind_number, 5),
            "linktarget" => VariableValue::new_unbounded_string(entry.and_then(ArchiveEntry::link_target).unwrap_or("").to_string()),
            "haslinktarget" => VariableValue::new_bool(entry.and_then(ArchiveEntry::link_target).is_some()),
            "isdirectory" => VariableValue::new_bool(kind == ArchiveEntryKind::Directory),
            "islink" => VariableValue::new_bool(matches!(kind, ArchiveEntryKind::SymbolicLink | ArchiveEntryKind::HardLink)),
            "isencrypted" => VariableValue::new_bool(entry.is_some_and(ArchiveEntry::is_encrypted)),
            "date" => VariableValue::new_temporal(TemporalValue::Date(
                time.and_then(|t| chrono::NaiveDate::from_ymd_opt(t.year() as i32, t.month() as u32, t.day() as u32)),
            )),
            "time" => VariableValue::new_temporal(TemporalValue::Time(
                time.and_then(|t| chrono::NaiveTime::from_hms_opt(t.hour() as u32, t.minute() as u32, t.second() as u32)),
            )),
            _ => return Err(format!("Unknown ArchiveEntry property {name}").into()),
        })
    }
    async fn call_function(
        &self,
        _vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        _arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        Err(format!("Unknown ArchiveEntry function {name}").into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detected(name: &str, data: &[u8]) -> ArchiveResult<ArchiveFormat> {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(name);
        std::fs::write(&path, data).unwrap();
        detect(&mut File::open(&path).unwrap(), &path)
    }

    #[test]
    fn detection_uses_content_then_extension_and_maps_compressed_tar_names() {
        let gz = [0x1F, 0x8B, 0x08, 0x00];
        assert_eq!(detected("magicless.ice", b"no magic").unwrap(), ArchiveFormat::Ice);
        assert_eq!(detected("archive.tgz", &gz).unwrap(), ArchiveFormat::Tgz);
        assert_eq!(detected("archive.tar.bz2", b"BZh9data").unwrap(), ArchiveFormat::Tbz);
        assert_eq!(detected("archive.tar.Z", &[0x1F, 0x9D, 0x90]).unwrap(), ArchiveFormat::TarZ);
        assert_eq!(detected("plain.gz", &gz).unwrap(), ArchiveFormat::Gz);
        assert_eq!(detected("misnamed.zip", &gz).unwrap(), ArchiveFormat::Gz);
        assert_eq!(detected("notes.txt", b"no magic").unwrap_err().code, ERR_FORMAT);
    }
}
