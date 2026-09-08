use std::{
    collections::{BTreeSet, HashSet},
    fs, io,
    path::{Component, Path, PathBuf},
};

use icy_board_engine::{Res, icy_board::write_atomic};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const VERSION: u32 = 1;
const LEDGER: &str = "help-generation.toml";
const PENDING: &str = "help-generation.pending.toml";
const BACKUPS: &str = "help-generation.backups";

pub struct Artifact {
    pub name: String,
    pub bytes: Vec<u8>,
    pub source_hash: String,
    pub settings_hash: String,
}

#[derive(Default)]
pub struct InstallOptions {
    pub dry_run: bool,
    pub adopt: bool,
    pub replace_modified: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Entry {
    output: String,
    name: String,
    hash: String,
    source_hash: String,
    settings_hash: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Ledger {
    schema_version: u32,
    entries: Vec<Entry>,
}

impl Default for Ledger {
    fn default() -> Self {
        Self {
            schema_version: VERSION,
            entries: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Permissions {
    readonly: bool,
    unix_mode: Option<u32>,
}

impl Permissions {
    fn capture(value: &fs::Permissions) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            Self {
                readonly: value.readonly(),
                unix_mode: Some(value.mode() & 0o7777),
            }
        }
        #[cfg(not(unix))]
        {
            Self {
                readonly: value.readonly(),
                unix_mode: None,
            }
        }
    }

    fn validate(&self) -> Res<()> {
        #[cfg(unix)]
        if !self.unix_mode.is_some_and(|mode| mode <= 0o7777 && self.readonly == (mode & 0o222 == 0)) {
            return Err("Invalid Unix permissions in help transaction".into());
        }
        #[cfg(not(unix))]
        if self.unix_mode.is_some() {
            return Err("Help transaction permissions belong to another platform".into());
        }
        Ok(())
    }

    fn apply(&self, path: &Path) -> Res<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(self.unix_mode.ok_or("Missing Unix permissions")?))?;
        }
        #[cfg(not(unix))]
        {
            let mut permissions = fs::metadata(path)?.permissions();
            permissions.set_readonly(self.readonly);
            fs::set_permissions(path, permissions)?;
        }
        Ok(())
    }
}

#[derive(Clone)]
struct Snapshot {
    bytes: Vec<u8>,
    permissions: Permissions,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Before {
    hash: String,
    permissions: Permissions,
}

impl Before {
    fn from_snapshot(snapshot: &Snapshot) -> Self {
        Self {
            hash: hash(&snapshot.bytes),
            permissions: snapshot.permissions.clone(),
        }
    }

    fn validate(&self) -> Res<()> {
        validate_hash(&self.hash)?;
        self.permissions.validate()
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Change {
    name: String,
    before: Option<Before>,
    after_hash: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    schema_version: u32,
    output: String,
    backup_dir: String,
    ledger_before: Option<Before>,
    ledger_after_hash: String,
    changes: Vec<Change>,
}

struct Planned<'a> {
    artifact: &'a Artifact,
    before: Option<Snapshot>,
    write: bool,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn validate_hash(value: &str) -> Res<()> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
        return Err(format!("Invalid SHA-256 hash: {value:?}").into());
    }
    Ok(())
}

fn validate_name(name: &str) -> Res<()> {
    let parts: Vec<_> = name.split('.').collect();
    let stem = parts[0];
    let symbol = stem.find(['!', '@']).unwrap_or(stem.len());
    let (base, suffix) = stem.split_at(symbol);
    // One optional language suffix precedes the single display-format extension.
    let valid = name.len() <= 128
        && (parts.len() == 2 || parts.len() == 3)
        && parts.last() == Some(&"pcb")
        && !base.is_empty()
        && base.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        && matches!(suffix, "" | "!" | "@" | "@w")
        && (parts.len() == 2 || (!parts[1].is_empty() && parts[1].bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())));
    if !valid {
        return Err(format!("Unsafe help filename: {name:?}; expected lowercase stem[.language].pcb").into());
    }
    Ok(())
}

fn path_text(path: &Path) -> Res<String> {
    Ok(path.to_str().ok_or("Help installation paths must be UTF-8")?.to_owned())
}

fn validate_stored_output(value: &str) -> Res<()> {
    let path = Path::new(value);
    if value.contains('\0') || !path.is_absolute() || path.components().any(|part| matches!(part, Component::CurDir | Component::ParentDir)) {
        return Err(format!("Invalid absolute help output path: {value:?}").into());
    }
    let rebuilt: PathBuf = path.components().collect();
    if path_text(&rebuilt)? != value {
        return Err(format!("Noncanonical help output path: {value:?}").into());
    }
    Ok(())
}

fn metadata(path: &Path) -> Res<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(format!("Symlink denied: {}", path.display()).into()),
        Ok(meta) => Ok(Some(meta)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Cannot inspect {}: {error}", path.display()).into()),
    }
}

// Resolve missing directories without creating them, checking every existing component.
fn resolve_directory(base: &Path, path: &Path) -> Res<PathBuf> {
    let absolute = if path.is_absolute() { path.to_path_buf() } else { base.join(path) };
    let mut result = PathBuf::new();
    for part in absolute.components() {
        match part {
            Component::CurDir => continue,
            Component::ParentDir => {
                result.pop();
            }
            _ => result.push(part.as_os_str()),
        }
        if let Some(meta) = metadata(&result)? {
            if !meta.is_dir() {
                return Err(format!("Not a directory: {}", result.display()).into());
            }
        }
    }
    if !result.is_absolute() {
        return Err("Help installation requires an absolute resolved directory".into());
    }
    validate_stored_output(&path_text(&result)?)?;
    Ok(result)
}

fn read_snapshot(path: &Path) -> Res<Option<Snapshot>> {
    let parent = path.parent().ok_or("Missing file parent")?;
    resolve_directory(parent, parent)?;
    let Some(meta) = metadata(path)? else { return Ok(None) };
    if !meta.is_file() {
        return Err(format!("Not a regular file: {}", path.display()).into());
    }
    let bytes = fs::read(path).map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
    Ok(Some(Snapshot {
        bytes,
        permissions: Permissions::capture(&meta.permissions()),
    }))
}

fn parse_ledger(bytes: &[u8]) -> Res<Ledger> {
    let ledger: Ledger = toml::from_str(std::str::from_utf8(bytes)?)?;
    if ledger.schema_version != VERSION {
        return Err(format!("Unsupported help ledger schema: {}", ledger.schema_version).into());
    }
    let mut seen = HashSet::new();
    for entry in &ledger.entries {
        validate_stored_output(&entry.output)?;
        validate_name(&entry.name)?;
        validate_hash(&entry.hash)?;
        validate_hash(&entry.source_hash)?;
        validate_hash(&entry.settings_hash)?;
        if !seen.insert((&entry.output, &entry.name)) {
            return Err(format!("Duplicate help ledger entry: {} / {}", entry.output, entry.name).into());
        }
    }
    Ok(ledger)
}

fn sync_directory(path: &Path) -> Res<()> {
    #[cfg(unix)]
    fs::File::open(path)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

fn ensure_directory(path: &Path) -> Res<()> {
    resolve_directory(path, path)?;
    if metadata(path)?.is_some() {
        return Ok(());
    }
    let parent = path.parent().ok_or("Missing directory parent")?;
    ensure_directory(parent)?;
    fs::create_dir(path)?;
    sync_directory(parent)
}

fn durable_write(path: &Path, bytes: &[u8], permissions: Option<&Permissions>) -> Res<()> {
    // Recheck immediately before replacing, even though the caller holds BoardLock.
    read_snapshot(path)?;
    write_atomic(path, bytes).map_err(|error| format!("Cannot atomically write {}: {error}", path.display()))?;
    if let Some(permissions) = permissions {
        permissions.apply(path)?;
    }
    fs::File::open(path)?.sync_all()?;
    sync_directory(path.parent().ok_or("Missing file parent")?)
}

fn durable_remove(path: &Path) -> Res<()> {
    if read_snapshot(path)?.is_some() {
        fs::remove_file(path)?;
        sync_directory(path.parent().ok_or("Missing file parent")?)?;
    }
    Ok(())
}

fn snapshot_hash(snapshot: &Option<Snapshot>) -> Option<String> {
    snapshot.as_ref().map(|snapshot| hash(&snapshot.bytes))
}

fn check_expected(path: &Path, expected: &Option<Snapshot>) -> Res<()> {
    let current = read_snapshot(path)?;
    if snapshot_hash(&current) != snapshot_hash(expected) || current.as_ref().map(|s| &s.permissions) != expected.as_ref().map(|s| &s.permissions) {
        return Err(format!("Conflict: {} changed since preflight", path.display()).into());
    }
    Ok(())
}

fn shadow_reports(output: &Path, artifacts: &[Artifact]) -> Res<Vec<String>> {
    if metadata(output)?.is_none() {
        return Ok(Vec::new());
    }
    let selected: HashSet<_> = artifacts.iter().map(|a| a.name.as_str()).collect();
    let mut reports = BTreeSet::new();
    for entry in fs::read_dir(output)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if selected.contains(name.as_ref()) {
            continue;
        }
        let lower = name.to_ascii_lowercase();
        let candidate = lower.split('.').next().unwrap_or_default();
        for artifact in artifacts {
            let base = artifact.name.split('.').next().unwrap_or_default();
            let Some(mut suffix) = candidate.strip_prefix(base) else { continue };
            suffix = suffix.trim_start_matches(|c: char| c.is_ascii_digit());
            if suffix.is_empty() || matches!(suffix, "g" | "r" | "v") {
                reports.insert(format!(
                    "Shadow warning: {} may take precedence over {} for some display modes, security levels or languages; left untouched",
                    entry.path().display(),
                    artifact.name
                ));
            }
        }
    }
    Ok(reports.into_iter().collect())
}

fn journal_backup_path(main: &Path, journal: &Journal) -> Res<PathBuf> {
    let Some(suffix) = journal.backup_dir.strip_prefix("transaction-") else {
        return Err("Invalid help transaction backup directory".into());
    };
    if suffix.is_empty() || suffix.len() > 64 || !suffix.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err("Invalid help transaction backup directory".into());
    }
    let path = main.join(BACKUPS).join(&journal.backup_dir);
    resolve_directory(main, &path)?;
    if metadata(&path)?.is_none() {
        return Err(format!("Missing help transaction backups: {}", path.display()).into());
    }
    Ok(path)
}

fn parse_journal(bytes: &[u8], output: &Path) -> Res<Journal> {
    let journal: Journal = toml::from_str(std::str::from_utf8(bytes)?)?;
    if journal.schema_version != VERSION {
        return Err(format!("Unsupported help transaction schema: {}", journal.schema_version).into());
    }
    validate_stored_output(&journal.output)?;
    if journal.output != path_text(output)? {
        return Err(format!(
            "Pending help transaction belongs to {}; configured output is {}. Recover using the original output directory",
            journal.output,
            output.display()
        )
        .into());
    }
    validate_hash(&journal.ledger_after_hash)?;
    if let Some(before) = &journal.ledger_before {
        before.validate()?;
    }
    let mut seen = HashSet::new();
    for change in &journal.changes {
        validate_name(&change.name)?;
        validate_hash(&change.after_hash)?;
        if let Some(before) = &change.before {
            before.validate()?;
        }
        if !seen.insert(&change.name) {
            return Err(format!("Duplicate help transaction destination: {}", change.name).into());
        }
    }
    Ok(journal)
}

fn load_backup(path: &Path, before: Option<&Before>) -> Res<Option<Snapshot>> {
    let Some(before) = before else { return Ok(None) };
    let snapshot = read_snapshot(path)?.ok_or_else(|| format!("Missing backup: {}", path.display()))?;
    if hash(&snapshot.bytes) != before.hash || snapshot.permissions != before.permissions {
        return Err(format!("Backup hash or permissions mismatch: {}", path.display()).into());
    }
    Ok(Some(snapshot))
}

fn recovery_target(path: &Path, before: Option<&Before>, after_hash: &str) -> Res<()> {
    let current = snapshot_hash(&read_snapshot(path)?);
    if current.as_deref() != before.map(|before| before.hash.as_str()) && current.as_deref() != Some(after_hash) {
        return Err(format!(
            "Conflict: {} is neither the before nor the intended after hash; refusing recovery over an external edit",
            path.display()
        )
        .into());
    }
    Ok(())
}

struct Recovery {
    journal: Journal,
    files: Vec<Option<Snapshot>>,
    ledger: Option<Snapshot>,
}

fn prepare_recovery(main: &Path, output: &Path, bytes: &[u8]) -> Res<Recovery> {
    let journal = parse_journal(bytes, output)?;
    let backups = journal_backup_path(main, &journal)?;
    let mut problems = Vec::new();
    let mut files = Vec::new();
    for (index, change) in journal.changes.iter().enumerate() {
        match load_backup(&backups.join(format!("{index}.before")), change.before.as_ref()) {
            Ok(snapshot) => files.push(snapshot),
            Err(error) => {
                problems.push(error.to_string());
                files.push(None);
            }
        }
        if let Err(error) = recovery_target(&output.join(&change.name), change.before.as_ref(), &change.after_hash) {
            problems.push(error.to_string());
        }
    }
    let ledger = match load_backup(&backups.join("ledger.before"), journal.ledger_before.as_ref()) {
        Ok(snapshot) => {
            if let Some(snapshot) = &snapshot {
                if let Err(error) = parse_ledger(&snapshot.bytes) {
                    problems.push(format!("Invalid old ledger backup: {error}"));
                }
            }
            snapshot
        }
        Err(error) => {
            problems.push(error.to_string());
            None
        }
    };
    if let Err(error) = recovery_target(&main.join(LEDGER), journal.ledger_before.as_ref(), &journal.ledger_after_hash) {
        problems.push(error.to_string());
    }
    match read_snapshot(&main.join(LEDGER)) {
        Ok(Some(current)) => {
            if let Err(error) = parse_ledger(&current.bytes) {
                problems.push(format!("Invalid current help ledger: {error}"));
            }
        }
        Ok(None) => {}
        Err(error) => problems.push(error.to_string()),
    }
    if !problems.is_empty() {
        return Err(format!("Help transaction recovery blocked; nothing changed:\n{}", problems.join("\n")).into());
    }
    Ok(Recovery { journal, files, ledger })
}

fn restore(path: &Path, snapshot: &Option<Snapshot>) -> Res<()> {
    if let Some(snapshot) = snapshot {
        let current = read_snapshot(path)?;
        if current
            .as_ref()
            .is_some_and(|current| current.bytes == snapshot.bytes && current.permissions == snapshot.permissions)
        {
            return Ok(());
        }
        durable_write(path, &snapshot.bytes, Some(&snapshot.permissions))
    } else {
        durable_remove(path)
    }
}

fn rollback(main: &Path, output: &Path, pending: &Snapshot) -> Res<()> {
    let recovery = prepare_recovery(main, output, &pending.bytes)?;
    check_expected(&main.join(PENDING), &Some(pending.clone()))?;
    // Recovery is idempotent: each restored path remains a valid before state.
    for (change, snapshot) in recovery.journal.changes.iter().zip(&recovery.files).rev() {
        let path = output.join(&change.name);
        recovery_target(&path, change.before.as_ref(), &change.after_hash)?;
        restore(&path, snapshot)?;
    }
    recovery_target(&main.join(LEDGER), recovery.journal.ledger_before.as_ref(), &recovery.journal.ledger_after_hash)?;
    restore(&main.join(LEDGER), &recovery.ledger)?;
    check_expected(&main.join(PENDING), &Some(pending.clone()))?;
    durable_remove(&main.join(PENDING))
}

fn save_backup(path: &Path, snapshot: &Snapshot) -> Res<()> {
    durable_write(path, &snapshot.bytes, Some(&snapshot.permissions))
}

fn stage_transaction(main: &Path, output: &Path, plans: &[Planned<'_>], old_ledger: &Option<Snapshot>, ledger_bytes: &[u8]) -> Res<Snapshot> {
    let backups = main.join(BACKUPS);
    ensure_directory(&backups)?;
    // keep() is intentional: successful replacements retain reusable original bytes.
    let directory = tempfile::Builder::new().prefix("transaction-").tempdir_in(&backups)?.keep();
    sync_directory(&backups)?;
    let mut changes = Vec::new();
    for plan in plans.iter().filter(|plan| plan.write) {
        if let Some(before) = &plan.before {
            save_backup(&directory.join(format!("{}.before", changes.len())), before)?;
        }
        changes.push(Change {
            name: plan.artifact.name.clone(),
            before: plan.before.as_ref().map(Before::from_snapshot),
            after_hash: hash(&plan.artifact.bytes),
        });
    }
    if let Some(old) = old_ledger {
        save_backup(&directory.join("ledger.before"), old)?;
    }
    let journal = Journal {
        schema_version: VERSION,
        output: path_text(output)?,
        backup_dir: directory
            .file_name()
            .ok_or("Missing backup directory name")?
            .to_str()
            .ok_or("Non-UTF-8 backup directory")?
            .to_owned(),
        ledger_before: old_ledger.as_ref().map(Before::from_snapshot),
        ledger_after_hash: hash(ledger_bytes),
        changes,
    };
    let bytes = toml::to_string(&journal)?.into_bytes();
    // Keep the manifest with its backups after the active journal is removed.
    durable_write(&directory.join("transaction.toml"), &bytes, None)?;
    for plan in plans {
        check_expected(&output.join(&plan.artifact.name), &plan.before)?;
    }
    check_expected(&main.join(LEDGER), old_ledger)?;
    check_expected(&main.join(PENDING), &None)?;
    durable_write(&main.join(PENDING), &bytes, None)?;
    read_snapshot(&main.join(PENDING))?.ok_or_else(|| "Help transaction journal disappeared".into())
}

/// Installs a precompiled batch. The caller must hold BoardLock on apply; dry runs never lock or write.
pub fn install(root: &Path, output: &Path, artifacts: &[Artifact], options: &InstallOptions) -> Res<Vec<String>> {
    install_with_hook(root, output, artifacts, options, |_| Ok(()))
}

fn install_with_hook(
    root: &Path,
    output: &Path,
    artifacts: &[Artifact],
    options: &InstallOptions,
    mut after_write: impl FnMut(usize) -> Res<()>,
) -> Res<Vec<String>> {
    let root = resolve_directory(&std::env::current_dir()?, root)?;
    let output = resolve_directory(&root, output)?;
    let output_text = path_text(&output)?;
    let main = resolve_directory(&root, &root.join("main"))?;
    resolve_directory(&main, &main.join(BACKUPS))?;
    let mut reports = Vec::new();
    if let Some(pending) = read_snapshot(&main.join(PENDING))? {
        let recovery = prepare_recovery(&main, &output, &pending.bytes)?;
        if options.dry_run {
            reports.push(format!(
                "Pending transaction: would roll back {} output changes and restore the old ledger; run apply with this output directory before planning a new batch. No files changed",
                recovery.journal.changes.len()
            ));
            for change in &recovery.journal.changes {
                reports.push(format!(
                    "Would {}: {}",
                    if change.before.is_some() {
                        "restore from backup"
                    } else {
                        "remove transaction-created file if present"
                    },
                    output.join(&change.name).display()
                ));
            }
            return Ok(reports);
        }
        rollback(&main, &output, &pending)?;
        reports.push("Recovered pending help transaction; restored previous files and ledger; backups retained".to_owned());
    }
    let old_ledger = read_snapshot(&main.join(LEDGER))?;
    let mut ledger = old_ledger.as_ref().map_or_else(|| Ok(Ledger::default()), |old| parse_ledger(&old.bytes))?;
    let initial_entries = ledger.entries.clone();
    let mut plans = Vec::new();
    let mut problems = Vec::new();
    let mut names = HashSet::new();
    for artifact in artifacts {
        let validation = validate_name(&artifact.name)
            .and_then(|()| validate_hash(&artifact.source_hash))
            .and_then(|()| validate_hash(&artifact.settings_hash));
        if let Err(error) = validation {
            problems.push(format!("Conflict: {}: {error}", artifact.name));
            continue;
        }
        if !names.insert(&artifact.name) {
            problems.push(format!("Conflict: duplicate destination {}", artifact.name));
            continue;
        }
        let path = output.join(&artifact.name);
        let before = match read_snapshot(&path) {
            Ok(before) => before,
            Err(error) => {
                problems.push(format!("Conflict: {}: {error}", path.display()));
                continue;
            }
        };
        let managed = ledger
            .entries
            .iter()
            .position(|entry| entry.output == output_text && entry.name == artifact.name);
        let current_hash = snapshot_hash(&before);
        let desired_hash = hash(&artifact.bytes);
        let identical = current_hash.as_ref() == Some(&desired_hash);
        let mut claim = true;
        let action = if before.is_none() {
            "Create"
        } else if let Some(index) = managed {
            if current_hash.as_ref() != Some(&ledger.entries[index].hash) && !options.replace_modified {
                problems.push(format!(
                    "Conflict: {} is a modified managed file; requires --replace-modified (--adopt does not apply)",
                    path.display()
                ));
                continue;
            }
            if identical { "Unchanged managed" } else { "Update managed (backup retained)" }
        } else if identical {
            if options.adopt {
                "Adopt identical (no output write)"
            } else {
                claim = false;
                "Unchanged unmanaged (not claimed; use --adopt to manage)"
            }
        } else if options.adopt {
            "Update unmanaged with explicit adoption (backup retained)"
        } else {
            problems.push(format!("Conflict: {} is unmanaged with different bytes; requires --adopt", path.display()));
            continue;
        };
        reports.push(format!("{}{action}: {}", if options.dry_run { "Would " } else { "" }, path.display()));
        if claim {
            let entry = Entry {
                output: output_text.clone(),
                name: artifact.name.clone(),
                hash: desired_hash,
                source_hash: artifact.source_hash.clone(),
                settings_hash: artifact.settings_hash.clone(),
            };
            if let Some(index) = managed {
                ledger.entries[index] = entry;
            } else {
                ledger.entries.push(entry);
            }
        }
        plans.push(Planned {
            artifact,
            before,
            write: !identical,
        });
    }
    match shadow_reports(&output, artifacts) {
        Ok(warnings) => reports.extend(warnings),
        Err(error) => problems.push(format!("Cannot inspect shadowing help files: {error}")),
    }
    if !problems.is_empty() {
        let details = reports
            .into_iter()
            .map(|report| format!("Preflight only: {report}"))
            .chain(problems)
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!("Help installation preflight failed; selected batch not written:\n{details}").into());
    }
    let ledger_changed = ledger.entries != initial_entries;
    if options.dry_run || (!ledger_changed && plans.iter().all(|plan| !plan.write)) {
        return Ok(reports);
    }
    let ledger_bytes = if ledger_changed {
        toml::to_string(&ledger)?.into_bytes()
    } else {
        old_ledger.as_ref().ok_or("Missing unchanged ledger")?.bytes.clone()
    };
    ensure_directory(&main)?;
    ensure_directory(&output)?;
    let pending = stage_transaction(&main, &output, &plans, &old_ledger, &ledger_bytes)?;
    let result = (|| -> Res<()> {
        for (index, plan) in plans.iter().filter(|plan| plan.write).enumerate() {
            let path = output.join(&plan.artifact.name);
            check_expected(&path, &plan.before)?;
            durable_write(&path, &plan.artifact.bytes, plan.before.as_ref().map(|snapshot| &snapshot.permissions))?;
            after_write(index)?;
        }
        for plan in &plans {
            let current = snapshot_hash(&read_snapshot(&output.join(&plan.artifact.name))?);
            if current.as_deref() != Some(hash(&plan.artifact.bytes).as_str()) {
                return Err(format!("Conflict: {} changed during installation", plan.artifact.name).into());
            }
        }
        check_expected(&main.join(LEDGER), &old_ledger)?;
        check_expected(&main.join(PENDING), &Some(pending.clone()))?;
        if ledger_changed {
            durable_write(&main.join(LEDGER), &ledger_bytes, old_ledger.as_ref().map(|snapshot| &snapshot.permissions))?;
        }
        after_write(usize::MAX)?;
        check_expected(&main.join(PENDING), &Some(pending.clone()))?;
        durable_remove(&main.join(PENDING))
    })();
    if let Err(error) = result {
        // A directory-sync failure can occur after unlinking the journal.
        let retain_journal = (|| -> Res<()> {
            if read_snapshot(&main.join(PENDING))?.is_none() {
                durable_write(&main.join(PENDING), &pending.bytes, Some(&pending.permissions))?;
            }
            Ok(())
        })();
        if let Err(retain_error) = retain_journal {
            return Err(format!(
                "Help installation failed: {error}; could not ensure a pending recovery journal: {retain_error}; retained backups require manual recovery under {}",
                main.join(BACKUPS).display()
            )
            .into());
        }
        return match rollback(&main, &output, &pending) {
            Ok(()) => Err(format!("Help installation failed: {error}; previous files and ledger restored; backups retained").into()),
            Err(recovery) => {
                Err(format!("Help installation failed: {error}; rollback incomplete: {recovery}; pending transaction and backups retained").into())
            }
        };
    }
    let journal = parse_journal(&pending.bytes, &output)?;
    reports.push(format!("Backups retained: {}", main.join(BACKUPS).join(journal.backup_dir).display()));
    Ok(reports)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> InstallOptions {
        InstallOptions {
            dry_run: false,
            adopt: false,
            replace_modified: false,
        }
    }

    fn artifact(name: &str, bytes: &[u8]) -> Artifact {
        Artifact {
            name: name.to_owned(),
            bytes: bytes.to_vec(),
            source_hash: hash(b"source"),
            settings_hash: hash(b"settings"),
        }
    }

    fn apply(root: &Path, artifacts: &[Artifact], options: &InstallOptions) -> Res<Vec<String>> {
        install(root, Path::new("help"), artifacts, options)
    }

    fn ledger(root: &Path) -> Ledger {
        parse_ledger(&fs::read(root.join("main").join(LEDGER)).unwrap()).unwrap()
    }

    fn assert_no_pending(root: &Path) {
        assert!(!root.join("main").join(PENDING).exists());
    }

    #[test]
    fn create_update_noop_metadata_only_and_keep_unselected() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        apply(root, &[artifact("hlpa.pcb", b"a"), artifact("hlpb.de.pcb", b"b")], &options()).unwrap();
        let path = root.join("help/hlpa.pcb");
        let stamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
        for file in [&path, &root.join("main").join(LEDGER)] {
            fs::File::open(file).unwrap().set_times(fs::FileTimes::new().set_modified(stamp)).unwrap();
        }
        let modified = fs::metadata(&path).unwrap().modified().unwrap();
        let old_ledger = fs::read(root.join("main").join(LEDGER)).unwrap();
        let ledger_mtime = fs::metadata(root.join("main").join(LEDGER)).unwrap().modified().unwrap();
        apply(root, &[artifact("hlpa.pcb", b"a")], &options()).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
        assert_eq!(fs::metadata(root.join("main").join(LEDGER)).unwrap().modified().unwrap(), ledger_mtime);
        assert_eq!(fs::read(root.join("main").join(LEDGER)).unwrap(), old_ledger);
        let mut changed_settings = artifact("hlpa.pcb", b"a");
        changed_settings.settings_hash = hash(b"new settings");
        apply(root, &[changed_settings], &options()).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
        apply(root, &[artifact("hlpa.pcb", b"updated")], &options()).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"updated");
        assert_eq!(fs::read(root.join("help/hlpb.de.pcb")).unwrap(), b"b");
        assert_eq!(ledger(root).entries.len(), 2);
        assert_no_pending(root);
    }

    #[test]
    fn missing_managed_output_is_recreated() {
        let dir = tempfile::tempdir().unwrap();
        let artifacts = [artifact("hlpa.pcb", b"a")];
        apply(dir.path(), &artifacts, &options()).unwrap();
        fs::remove_file(dir.path().join("help/hlpa.pcb")).unwrap();
        apply(dir.path(), &artifacts, &options()).unwrap();
        assert_eq!(fs::read(dir.path().join("help/hlpa.pcb")).unwrap(), b"a");
        assert_no_pending(dir.path());
    }

    #[test]
    fn dry_run_of_updates_and_adoption_preserves_existing_files_and_bookkeeping() {
        let dir = tempfile::tempdir().unwrap();
        apply(dir.path(), &[artifact("hlpa.pcb", b"old")], &options()).unwrap();
        fs::write(dir.path().join("help/hlpb.pcb"), b"custom").unwrap();
        let main = dir.path().join("main");
        let old_ledger = fs::read(main.join(LEDGER)).unwrap();
        let backup_count = fs::read_dir(main.join(BACKUPS)).unwrap().count();
        let mut dry = options();
        dry.dry_run = true;
        dry.adopt = true;
        apply(dir.path(), &[artifact("hlpa.pcb", b"new"), artifact("hlpb.pcb", b"adopted")], &dry).unwrap();
        assert_eq!(fs::read(dir.path().join("help/hlpa.pcb")).unwrap(), b"old");
        assert_eq!(fs::read(dir.path().join("help/hlpb.pcb")).unwrap(), b"custom");
        assert_eq!(fs::read(main.join(LEDGER)).unwrap(), old_ledger);
        assert_eq!(fs::read_dir(main.join(BACKUPS)).unwrap().count(), backup_count);
        assert_no_pending(dir.path());
    }

    #[test]
    fn dry_run_and_empty_batch_create_nothing_even_when_root_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("missing-board");
        let mut dry = options();
        dry.dry_run = true;
        let report = apply(&root, &[artifact("hlpa.pcb", b"a")], &dry).unwrap();
        assert!(report.iter().any(|line| line.contains("Would Create")));
        assert!(!root.exists());
        apply(&root, &[], &options()).unwrap();
        assert!(!root.exists());
    }

    #[test]
    fn unmanaged_identical_is_not_claimed_without_adoption() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("help")).unwrap();
        let path = dir.path().join("help/hlpa.pcb");
        fs::write(&path, b"a").unwrap();
        let modified = fs::metadata(&path).unwrap().modified().unwrap();
        let artifacts = [artifact("hlpa.pcb", b"a")];
        apply(dir.path(), &artifacts, &options()).unwrap();
        assert!(!dir.path().join("main").exists());
        let mut adopt = options();
        adopt.adopt = true;
        apply(dir.path(), &artifacts, &adopt).unwrap();
        assert_eq!(ledger(dir.path()).entries.len(), 1);
        assert_eq!(fs::metadata(path).unwrap().modified().unwrap(), modified);
    }

    #[test]
    fn both_conflicts_are_reported_and_whole_batch_stays_untouched() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("help")).unwrap();
        fs::write(dir.path().join("help/hlpa.pcb"), b"custom a").unwrap();
        fs::write(dir.path().join("help/hlpb.pcb"), b"custom b").unwrap();
        let artifacts = [artifact("hlpnew.pcb", b"new"), artifact("hlpa.pcb", b"a"), artifact("hlpb.pcb", b"b")];
        for dry_run in [false, true] {
            let mut opts = options();
            opts.dry_run = dry_run;
            opts.replace_modified = true;
            let error = apply(dir.path(), &artifacts, &opts).unwrap_err().to_string();
            assert!(error.contains("hlpa.pcb") && error.contains("hlpb.pcb"));
            assert!(!dir.path().join("help/hlpnew.pcb").exists());
            assert!(!dir.path().join("main").exists());
        }
        assert_eq!(fs::read(dir.path().join("help/hlpa.pcb")).unwrap(), b"custom a");
        assert_eq!(fs::read(dir.path().join("help/hlpb.pcb")).unwrap(), b"custom b");
    }

    #[test]
    fn adoption_does_not_force_managed_edits_even_if_desired_matches() {
        let dir = tempfile::tempdir().unwrap();
        apply(dir.path(), &[artifact("hlpa.pcb", b"a")], &options()).unwrap();
        fs::write(dir.path().join("help/hlpa.pcb"), b"edited").unwrap();
        let mut opts = options();
        opts.adopt = true;
        assert!(
            apply(dir.path(), &[artifact("hlpa.pcb", b"edited")], &opts)
                .unwrap_err()
                .to_string()
                .contains("--replace-modified")
        );
        assert!(apply(dir.path(), &[artifact("hlpa.pcb", b"new")], &opts).is_err());
        opts.replace_modified = true;
        apply(dir.path(), &[artifact("hlpa.pcb", b"new")], &opts).unwrap();
        assert_eq!(fs::read(dir.path().join("help/hlpa.pcb")).unwrap(), b"new");
    }

    #[test]
    fn explicit_adoption_replaces_with_durable_unique_backups_and_reports_shadows() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("help")).unwrap();
        fs::write(dir.path().join("help/hlpa.pcb"), b"custom").unwrap();
        fs::write(dir.path().join("help/hlpa10g.ans"), b"shadow").unwrap();
        fs::write(dir.path().join("help/unknown.data"), b"unknown").unwrap();
        let mut opts = options();
        opts.adopt = true;
        let report = apply(dir.path(), &[artifact("hlpa.pcb", b"a")], &opts).unwrap();
        assert!(report.iter().any(|line| line.contains("Shadow warning") && line.contains("hlpa10g.ans")));
        apply(dir.path(), &[artifact("hlpa.pcb", b"new")], &options()).unwrap();
        let backups: Vec<_> = fs::read_dir(dir.path().join("main").join(BACKUPS))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(backups.len(), 2);
        let contents: Vec<_> = backups.iter().map(|backup| fs::read(backup.join("0.before")).unwrap()).collect();
        assert!(contents.contains(&b"custom".to_vec()) && contents.contains(&b"a".to_vec()));
        assert!(backups.iter().all(|backup| backup.join("transaction.toml").is_file()));
        assert_eq!(fs::read(dir.path().join("help/hlpa10g.ans")).unwrap(), b"shadow");
        assert_eq!(fs::read(dir.path().join("help/unknown.data")).unwrap(), b"unknown");
    }

    #[test]
    fn absolute_and_distinct_outputs_do_not_reuse_ownership_hashes() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        apply(dir.path(), &[artifact("hlpa.pcb", b"a")], &options()).unwrap();
        fs::write(outside.path().join("hlpa.pcb"), b"a").unwrap();
        install(dir.path(), outside.path(), &[artifact("hlpa.pcb", b"a")], &options()).unwrap();
        assert_eq!(ledger(dir.path()).entries.len(), 1);
        assert!(
            install(dir.path(), outside.path(), &[artifact("hlpa.pcb", b"new")], &options())
                .unwrap_err()
                .to_string()
                .contains("unmanaged")
        );
        let mut adopt = options();
        adopt.adopt = true;
        install(dir.path(), outside.path(), &[artifact("hlpa.pcb", b"new")], &adopt).unwrap();
        assert_eq!(ledger(dir.path()).entries.len(), 2);
        apply(dir.path(), &[artifact("hlpa.pcb", b"updated original")], &options()).unwrap();
        assert_eq!(fs::read(outside.path().join("hlpa.pcb")).unwrap(), b"new");
    }

    #[test]
    fn rejects_traversal_duplicate_names_and_invalid_hashes_without_writes() {
        let dir = tempfile::tempdir().unwrap();
        for name in [
            "../a.pcb",
            "/a.pcb",
            "a/b.pcb",
            "a\\b.pcb",
            "A.pcb",
            ".pcb",
            "a..pcb",
            "a.de.extra.pcb",
            "a.pcb\0",
            "a.de/evil.pcb",
            "!a.pcb",
            "a@@.pcb",
        ] {
            assert!(apply(dir.path(), &[artifact(name, b"a")], &options()).is_err(), "{name:?}");
        }
        assert!(apply(dir.path(), &[artifact("a.pcb", b"a"), artifact("a.pcb", b"b")], &options()).is_err());
        let mut invalid = artifact("a.pcb", b"a");
        invalid.source_hash = "not a hash".to_owned();
        assert!(apply(dir.path(), &[invalid], &options()).is_err());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
        for name in ["hlp!.pcb", "hlp@.pcb", "hlp@w.de.pcb", "hlpa.pcb"] {
            validate_name(name).unwrap();
        }
    }

    #[test]
    fn rejects_unknown_ledger_schema_fields_and_untrusted_entries() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("main")).unwrap();
        for text in [
            "schema_version = 99\nentries = []\n",
            "schema_version = 1\nentries = []\nevil = true\n",
            "entries = []\n",
        ] {
            fs::write(dir.path().join("main").join(LEDGER), text).unwrap();
            assert!(apply(dir.path(), &[artifact("a.pcb", b"a")], &options()).is_err());
            assert!(!dir.path().join("help").exists());
        }
        let mut value = Ledger {
            schema_version: VERSION,
            entries: vec![Entry {
                output: path_text(dir.path()).unwrap(),
                name: "../a.pcb".to_owned(),
                hash: hash(b"a"),
                source_hash: hash(b"s"),
                settings_hash: hash(b"t"),
            }],
        };
        assert!(parse_ledger(toml::to_string(&value).unwrap().as_bytes()).is_err());
        value.entries[0].name = "a.pcb".to_owned();
        value.entries[0].output = "../outside".to_owned();
        assert!(parse_ledger(toml::to_string(&value).unwrap().as_bytes()).is_err());
        value.entries[0].output = path_text(dir.path()).unwrap();
        value.entries.push(value.entries[0].clone());
        assert!(parse_ledger(toml::to_string(&value).unwrap().as_bytes()).is_err());
    }

    #[test]
    fn injected_failure_restores_replacements_creations_and_old_ledger() {
        for fail_at in [0, 1, usize::MAX] {
            let dir = tempfile::tempdir().unwrap();
            apply(dir.path(), &[artifact("hlpa.pcb", b"old")], &options()).unwrap();
            let old_ledger = fs::read(dir.path().join("main").join(LEDGER)).unwrap();
            let error = install_with_hook(
                dir.path(),
                Path::new("help"),
                &[artifact("hlpa.pcb", b"new"), artifact("hlpb.pcb", b"created")],
                &options(),
                |index| if index == fail_at { Err("injected disk failure".into()) } else { Ok(()) },
            )
            .unwrap_err()
            .to_string();
            assert!(error.contains("previous files and ledger restored"), "{error}");
            assert_eq!(fs::read(dir.path().join("help/hlpa.pcb")).unwrap(), b"old");
            assert!(!dir.path().join("help/hlpb.pcb").exists());
            assert_eq!(fs::read(dir.path().join("main").join(LEDGER)).unwrap(), old_ledger);
            assert_no_pending(dir.path());
        }
    }

    #[test]
    fn first_install_failure_after_ledger_commit_removes_new_files_and_ledger() {
        let dir = tempfile::tempdir().unwrap();
        let result = install_with_hook(dir.path(), Path::new("help"), &[artifact("hlpa.pcb", b"new")], &options(), |index| {
            if index == usize::MAX { Err("injected commit failure".into()) } else { Ok(()) }
        });
        assert!(result.unwrap_err().to_string().contains("previous files and ledger restored"));
        assert!(!dir.path().join("help/hlpa.pcb").exists());
        assert!(!dir.path().join("main").join(LEDGER).exists());
        assert_no_pending(dir.path());
    }

    #[test]
    fn external_edit_during_apply_blocks_rollback_without_overwriting_it() {
        let dir = tempfile::tempdir().unwrap();
        apply(dir.path(), &[artifact("hlpa.pcb", b"old"), artifact("hlpb.pcb", b"old b")], &options()).unwrap();
        let result = install_with_hook(
            dir.path(),
            Path::new("help"),
            &[artifact("hlpa.pcb", b"new"), artifact("hlpb.pcb", b"new b")],
            &options(),
            |_| {
                fs::write(dir.path().join("help/hlpb.pcb"), b"external edit")?;
                Err("injected failure".into())
            },
        );
        assert!(result.unwrap_err().to_string().contains("rollback incomplete"));
        assert_eq!(fs::read(dir.path().join("help/hlpa.pcb")).unwrap(), b"new");
        assert_eq!(fs::read(dir.path().join("help/hlpb.pcb")).unwrap(), b"external edit");
        assert!(dir.path().join("main").join(PENDING).exists());
        fs::write(dir.path().join("help/hlpb.pcb"), b"old b").unwrap();
        apply(dir.path(), &[], &options()).unwrap();
        assert_eq!(fs::read(dir.path().join("help/hlpa.pcb")).unwrap(), b"old");
        assert_no_pending(dir.path());
    }

    #[test]
    fn recovery_after_ledger_commit_restores_old_ledger_and_removes_creations() {
        let dir = tempfile::tempdir().unwrap();
        apply(dir.path(), &[artifact("hlpa.pcb", b"old")], &options()).unwrap();
        let old_ledger = fs::read(dir.path().join("main").join(LEDGER)).unwrap();
        let crash = std::panic::catch_unwind(|| {
            let _ = install_with_hook(
                dir.path(),
                Path::new("help"),
                &[artifact("hlpa.pcb", b"new"), artifact("hlpb.pcb", b"created")],
                &options(),
                |index| {
                    assert_ne!(index, usize::MAX, "simulated crash after ledger commit");
                    Ok(())
                },
            );
        });
        assert!(crash.is_err());
        assert_eq!(ledger(dir.path()).entries.len(), 2);
        // Simulate a second crash partway through rollback, after removing a creation.
        fs::remove_file(dir.path().join("help/hlpb.pcb")).unwrap();
        apply(dir.path(), &[], &options()).unwrap();
        assert_eq!(fs::read(dir.path().join("help/hlpa.pcb")).unwrap(), b"old");
        assert!(!dir.path().join("help/hlpb.pcb").exists());
        assert_eq!(fs::read(dir.path().join("main").join(LEDGER)).unwrap(), old_ledger);
        assert_no_pending(dir.path());
    }

    fn leave_pending(root: &Path) {
        apply(root, &[artifact("hlpa.pcb", b"old"), artifact("hlpb.pcb", b"old b")], &options()).unwrap();
        let crash = std::panic::catch_unwind(|| {
            let _ = install_with_hook(
                root,
                Path::new("help"),
                &[artifact("hlpa.pcb", b"new"), artifact("hlpb.pcb", b"new b")],
                &options(),
                |_| panic!("simulated process death after a durable output write"),
            );
        });
        assert!(crash.is_err());
        assert!(root.join("main").join(PENDING).is_file());
    }

    #[test]
    fn crash_recovery_dry_run_is_readonly_and_apply_restores_before_new_batch() {
        let dir = tempfile::tempdir().unwrap();
        leave_pending(dir.path());
        let pending_path = dir.path().join("main").join(PENDING);
        let pending = fs::read(&pending_path).unwrap();
        let mut dry = options();
        dry.dry_run = true;
        assert!(apply(dir.path(), &[], &dry).unwrap().iter().any(|line| line.contains("Pending transaction")));
        assert_eq!(fs::read(&pending_path).unwrap(), pending);
        assert_eq!(fs::read(dir.path().join("help/hlpa.pcb")).unwrap(), b"new");
        apply(dir.path(), &[artifact("hlpc.pcb", b"next batch")], &options()).unwrap();
        assert_eq!(fs::read(dir.path().join("help/hlpa.pcb")).unwrap(), b"old");
        assert_eq!(fs::read(dir.path().join("help/hlpb.pcb")).unwrap(), b"old b");
        assert_eq!(fs::read(dir.path().join("help/hlpc.pcb")).unwrap(), b"next batch");
        assert_no_pending(dir.path());
    }

    #[test]
    fn recovery_fails_closed_for_all_external_edits_and_wrong_output() {
        let dir = tempfile::tempdir().unwrap();
        leave_pending(dir.path());
        assert!(
            install(dir.path(), Path::new("different"), &[], &options())
                .unwrap_err()
                .to_string()
                .contains("original output directory")
        );
        assert!(!dir.path().join("different").exists());
        fs::write(dir.path().join("help/hlpa.pcb"), b"external a").unwrap();
        fs::write(dir.path().join("help/hlpb.pcb"), b"external b").unwrap();
        let error = apply(dir.path(), &[], &options()).unwrap_err().to_string();
        assert!(error.contains("hlpa.pcb") && error.contains("hlpb.pcb"));
        assert_eq!(fs::read(dir.path().join("help/hlpa.pcb")).unwrap(), b"external a");
        assert_eq!(fs::read(dir.path().join("help/hlpb.pcb")).unwrap(), b"external b");
        assert!(dir.path().join("main").join(PENDING).exists());
    }

    #[test]
    fn invalid_journal_names_backup_paths_and_schemas_do_not_mutate_files() {
        let dir = tempfile::tempdir().unwrap();
        leave_pending(dir.path());
        let path = dir.path().join("main").join(PENDING);
        let original = fs::read_to_string(&path).unwrap();
        let output = dir.path().join("help");
        for case in 0..5 {
            let mut journal = parse_journal(original.as_bytes(), &output).unwrap();
            match case {
                0 => journal.backup_dir = "../outside".to_owned(),
                1 => journal.changes[0].name = "../outside.pcb".to_owned(),
                2 => journal.schema_version = 99,
                3 => journal.ledger_after_hash = "bad".to_owned(),
                _ => journal.output = "/other/output".to_owned(),
            }
            fs::write(&path, toml::to_string(&journal).unwrap()).unwrap();
            assert!(apply(dir.path(), &[], &options()).is_err());
            assert_eq!(fs::read(dir.path().join("help/hlpa.pcb")).unwrap(), b"new");
        }
        fs::write(&path, format!("unknown_field = true\n{original}")).unwrap();
        assert!(apply(dir.path(), &[], &options()).is_err());
    }

    #[test]
    fn tampered_backup_or_ledger_blocks_recovery_before_any_restore() {
        let dir = tempfile::tempdir().unwrap();
        leave_pending(dir.path());
        let main = dir.path().join("main");
        let journal = parse_journal(&fs::read(main.join(PENDING)).unwrap(), &dir.path().join("help")).unwrap();
        let backup = main.join(BACKUPS).join(journal.backup_dir).join("0.before");
        fs::write(&backup, b"tampered").unwrap();
        assert!(apply(dir.path(), &[], &options()).unwrap_err().to_string().contains("Backup hash"));
        assert_eq!(fs::read(dir.path().join("help/hlpa.pcb")).unwrap(), b"new");
        fs::write(&backup, b"old").unwrap();
        fs::write(main.join(LEDGER), b"external ledger").unwrap();
        assert!(apply(dir.path(), &[], &options()).unwrap_err().to_string().contains(LEDGER));
        assert_eq!(fs::read(dir.path().join("help/hlpa.pcb")).unwrap(), b"new");
    }

    #[cfg(unix)]
    #[test]
    fn refuses_target_ancestor_metadata_and_backup_symlinks_including_dangling_links() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("help")).unwrap();
        symlink(outside.path().join("missing"), dir.path().join("help/hlpa.pcb")).unwrap();
        assert!(apply(dir.path(), &[artifact("hlpa.pcb", b"a")], &options()).is_err());
        assert!(!outside.path().join("missing").exists());
        assert!(!dir.path().join("main").exists());
        symlink(outside.path(), dir.path().join("linked")).unwrap();
        assert!(install(dir.path(), Path::new("linked/subdir"), &[], &options()).is_err());
        fs::create_dir(dir.path().join("main")).unwrap();
        for name in [LEDGER, PENDING, BACKUPS] {
            symlink(outside.path().join("missing"), dir.path().join("main").join(name)).unwrap();
            assert!(apply(dir.path(), &[], &options()).is_err());
            fs::remove_file(dir.path().join("main").join(name)).unwrap();
        }
    }

    #[cfg(unix)]
    #[test]
    fn recovery_rejects_symlinked_backup_contents() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        leave_pending(dir.path());
        let main = dir.path().join("main");
        let journal = parse_journal(&fs::read(main.join(PENDING)).unwrap(), &dir.path().join("help")).unwrap();
        let backup = main.join(BACKUPS).join(journal.backup_dir).join("0.before");
        let outside = tempfile::NamedTempFile::new().unwrap();
        fs::write(outside.path(), b"old").unwrap();
        fs::remove_file(&backup).unwrap();
        symlink(outside.path(), backup).unwrap();
        assert!(apply(dir.path(), &[], &options()).unwrap_err().to_string().contains("Symlink denied"));
        assert_eq!(fs::read(dir.path().join("help/hlpa.pcb")).unwrap(), b"new");
        assert_eq!(fs::read(outside.path()).unwrap(), b"old");
    }

    #[cfg(unix)]
    #[test]
    fn replacement_and_rollback_preserve_original_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        apply(dir.path(), &[artifact("hlpa.pcb", b"old")], &options()).unwrap();
        let path = dir.path().join("help/hlpa.pcb");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        apply(dir.path(), &[artifact("hlpa.pcb", b"new")], &options()).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o7777, 0o640);
        assert!(
            install_with_hook(dir.path(), Path::new("help"), &[artifact("hlpa.pcb", b"failed")], &options(), |_| Err(
                "injected".into()
            ))
            .is_err()
        );
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o7777, 0o640);
        assert_eq!(fs::read(path).unwrap(), b"new");
    }
}
