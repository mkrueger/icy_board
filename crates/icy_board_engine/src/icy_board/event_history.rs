//! Single-writer event journal, independent of the board-data lock. Never delete
//! or rotate the journal to trim logs: its occurrence keys are the replay barrier.
//! A durable pending claim is written BEFORE any process can be spawned. On open,
//! pending entries become interrupted, never eligible for automatic retry.
//!
//! Version 1 is an atomic full-file TOML snapshot: `version = 1`, `[[entries]]`.
//! UTC RFC3339 timestamps disambiguate local timezone offsets; scheduled keys are
//! `<event_id>@<scheduled_for UTC RFC3339>`, manual keys `manual@<new UUID>`.
//! `start` is an attempted start persisted before spawn, not proof a process ran.
//! `finish`, result, optional exit_code/detail and a relative unique log_file
//! record outcomes. Skips before start preparation have no start/log;
//! pending/interrupted may lack either. A planned log may not exist if its open
//! failed or the latest-start bound expired during start preparation.
//!
//! The journal is not part of IcyBoard saves/reloads. Its independent exclusive
//! lease survives maintenance releasing BoardLock. Tempfile + fsync + atomic
//! replacement preserves the old or new snapshot; Unix also syncs the directory.
//! Power-loss rename durability on non-Unix is limited by the platform. Logs and
//! history are intentionally unbounded; rotate logs separately, retain history
//! keys to preserve the no-replay guarantee. A lost journal cannot prove old runs.

use std::{
    collections::HashSet,
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{BoardEvent, EventExecution};
use crate::Res;

pub const HISTORY_FILE: &str = "event_history.toml";
pub const LOG_DIRECTORY: &str = "event_logs";
const LOCK_FILE: &str = ".event_history.lock";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventResult {
    Pending,
    Success,
    NonzeroExit,
    SpawnError,
    WaitError,
    Interrupted,
    SkippedBusy,
    Expired,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventHistoryEntry {
    /// Scheduled: `<event_id>@<UTC RFC3339>`; manual: `manual@<UUID>`.
    pub key: String,
    pub event_id: String,
    pub description: String,
    pub scheduled_for: DateTime<Utc>,
    pub start: Option<DateTime<Utc>>,
    pub finish: Option<DateTime<Utc>>,
    pub result: EventResult,
    pub exit_code: Option<i32>,
    /// Planned destination relative to the board root; None before start preparation.
    pub log_file: Option<String>,
    pub manual: bool,
    pub execution: EventExecution,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    version: u32,
    entries: Vec<EventHistoryEntry>,
}

pub struct EventHistory {
    root: PathBuf,
    journal: Journal,
    /// Separate file description: unlike BoardLock, even same-process double
    /// scheduler starts are refused. Held across maintenance's BoardLock release.
    _lock: File,
    /// Once a write fails (including a directory sync after rename), all writes
    /// fail until explicit restart/repair. Memory cannot prove durable disk state.
    failed: bool,
}

impl EventHistory {
    /// Read-only UI/API snapshot. Atomic replacement permits reading while the
    /// scheduler owns the journal; this never recovers claims or takes its lease.
    pub fn read_entries(root: &Path) -> Res<Vec<EventHistoryEntry>> {
        match std::fs::read_to_string(root.join(HISTORY_FILE)) {
            Ok(text) => {
                let journal: Journal = toml::from_str(&text)?;
                Self::validate_journal(&journal)?;
                Ok(journal.entries)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e.into()),
        }
    }

    fn validate_journal(journal: &Journal) -> Res<()> {
        let mut keys = HashSet::new();
        if journal.version != 1 {
            return Err("unsupported event history version".into());
        }
        for entry in &journal.entries {
            if !keys.insert(entry.key.clone())
                || entry.event_id.is_empty()
                || entry.event_id.len() > 128
                || !entry.event_id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                || (!entry.manual && entry.key != Self::scheduled_key(&entry.event_id, entry.scheduled_for))
                || (entry.manual && !entry.key.starts_with("manual@"))
                || (entry.result == EventResult::Pending) != entry.finish.is_none()
                || entry.start.is_some() != entry.log_file.is_some()
            {
                return Err("invalid or duplicate event history occurrence".into());
            }
            if let Some(log) = &entry.log_file {
                let Some(name) = log.strip_prefix("event_logs/") else {
                    return Err("invalid event log path".into());
                };
                if !name.ends_with(".log") || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.') || name.contains("..") {
                    return Err("invalid event log path".into());
                }
            }
        }
        Ok(())
    }

    pub fn open(root: &Path, now: DateTime<Utc>) -> Res<Self> {
        let root = root.canonicalize()?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(root.join(LOCK_FILE))?;
        lock.try_lock().map_err(|e| format!("event journal already owned or cannot be locked: {e}"))?;
        let journal = match std::fs::read_to_string(root.join(HISTORY_FILE)) {
            Ok(text) => toml::from_str::<Journal>(&text)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Journal {
                version: 1,
                entries: Vec::new(),
            },
            Err(e) => return Err(e.into()),
        };
        Self::validate_journal(&journal)?;
        let mut history = Self {
            root,
            journal,
            _lock: lock,
            failed: false,
        };
        for entry in &mut history.journal.entries {
            if entry.result == EventResult::Pending {
                entry.result = EventResult::Interrupted;
                entry.finish = Some(now);
                entry.detail = Some("Scheduler stopped with a pending claim; not retried".into());
            }
        }
        // Also probes writeability before the scheduler can admit any execution.
        history.persist()?;
        Ok(history)
    }

    pub fn entries(&self) -> &[EventHistoryEntry] {
        &self.journal.entries
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn scheduled_key(event_id: &str, scheduled_for: DateTime<Utc>) -> String {
        format!("{event_id}@{}", scheduled_for.to_rfc3339())
    }

    pub fn contains(&self, event_id: &str, scheduled_for: DateTime<Utc>) -> bool {
        let key = Self::scheduled_key(event_id, scheduled_for);
        self.journal.entries.iter().any(|entry| entry.key == key)
    }

    /// None means already claimed, including interrupted/skipped occurrences.
    pub fn claim(&mut self, event: &BoardEvent, scheduled_for: DateTime<Utc>, manual: bool) -> Res<Option<EventHistoryEntry>> {
        event.validate()?;
        let key = if manual {
            format!("manual@{}", BoardEvent::new_id())
        } else {
            Self::scheduled_key(&event.id, scheduled_for)
        };
        if self.failed {
            return Err("event journal has a latched persistence failure".into());
        }
        if self.journal.entries.iter().any(|entry| entry.key == key) {
            return Ok(None);
        }
        let entry = EventHistoryEntry {
            key,
            event_id: event.id.clone(),
            description: event.description.clone(),
            scheduled_for,
            start: None,
            finish: None,
            result: EventResult::Pending,
            exit_code: None,
            log_file: None,
            manual,
            execution: event.execution,
            detail: None,
        };
        self.journal.entries.push(entry.clone());
        self.persist()?;
        Ok(Some(entry))
    }

    /// Persist attempted start/log destination before spawning. The file itself
    /// is opened create_new, so neither history nor logs are ever overwritten.
    pub fn start(&mut self, key: &str, now: DateTime<Utc>) -> Res<PathBuf> {
        let log = format!("{LOG_DIRECTORY}/{}.log", BoardEvent::new_id());
        let entry = self.pending_mut(key)?;
        if entry.start.is_some() {
            return Err("event claim has already started".into());
        }
        entry.start = Some(now);
        entry.log_file = Some(log.clone());
        self.persist()?;
        Ok(self.root.join(log))
    }

    pub fn finish(&mut self, key: &str, now: DateTime<Utc>, result: EventResult, exit_code: Option<i32>, detail: Option<String>) -> Res<()> {
        if result == EventResult::Pending {
            return Err("cannot finish an event as pending".into());
        }
        let entry = self.pending_mut(key)?;
        entry.finish = Some(now);
        entry.result = result;
        entry.exit_code = exit_code;
        entry.detail = detail;
        self.persist()
    }

    fn pending_mut(&mut self, key: &str) -> Res<&mut EventHistoryEntry> {
        if self.failed {
            return Err("event journal has a latched persistence failure".into());
        }
        self.journal
            .entries
            .iter_mut()
            .find(|e| e.key == key && e.result == EventResult::Pending)
            .ok_or_else(|| "missing or finished event claim".into())
    }

    fn persist(&mut self) -> Res<()> {
        if self.failed {
            return Err("event journal has a latched persistence failure".into());
        }
        let result = self.write_atomic();
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    fn write_atomic(&self) -> Res<()> {
        let text = toml::to_string_pretty(&self.journal)?;
        let mut temp = tempfile::NamedTempFile::new_in(&self.root)?;
        temp.write_all(text.as_bytes())?;
        temp.as_file().sync_all()?;
        temp.persist(self.root.join(HISTORY_FILE))?;
        // Linux/Unix: make the rename durable as well as the file contents.
        #[cfg(unix)]
        File::open(&self.root)?.sync_all()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        "2024-06-03T03:00:00Z".parse().unwrap()
    }

    #[test]
    fn durable_claim_deduplicates_and_restart_interrupts_pending() {
        let dir = tempfile::tempdir().unwrap();
        let event = BoardEvent::default();
        let mut journal = EventHistory::open(dir.path(), now()).unwrap();
        let entry = journal.claim(&event, now(), false).unwrap().unwrap();
        assert!(journal.claim(&event, now(), false).unwrap().is_none());
        assert!(EventHistory::open(dir.path(), now()).is_err());
        let log = journal.start(&entry.key, now()).unwrap();
        assert!(log.starts_with(dir.path()));
        drop(journal);
        let mut journal = EventHistory::open(dir.path(), now()).unwrap();
        assert_eq!(journal.entries()[0].result, EventResult::Interrupted);
        assert!(journal.claim(&event, now(), false).unwrap().is_none());
        assert!(journal.claim(&event, now(), true).unwrap().is_some());
        assert!(journal.claim(&event, now(), true).unwrap().is_some());
    }

    #[test]
    fn successful_and_skipped_entries_survive_restart() {
        let dir = tempfile::tempdir().unwrap();
        let event = BoardEvent::default();
        let mut journal = EventHistory::open(dir.path(), now()).unwrap();
        let entry = journal.claim(&event, now(), false).unwrap().unwrap();
        journal.finish(&entry.key, now(), EventResult::SkippedBusy, None, None).unwrap();
        let manual = journal.claim(&event, now(), true).unwrap().unwrap();
        journal.start(&manual.key, now()).unwrap();
        journal.finish(&manual.key, now(), EventResult::Success, Some(0), None).unwrap();
        assert!(journal.finish(&manual.key, now(), EventResult::Success, Some(0), None).is_err());
        drop(journal);
        let journal = EventHistory::open(dir.path(), now()).unwrap();
        assert_eq!(journal.entries()[0].result, EventResult::SkippedBusy);
        assert_eq!(journal.entries()[1].result, EventResult::Success);
    }

    #[test]
    fn corrupted_history_and_failed_replacement_fail_closed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(HISTORY_FILE), "broken = [").unwrap();
        assert!(EventHistory::open(dir.path(), now()).is_err());
        std::fs::remove_file(dir.path().join(HISTORY_FILE)).unwrap();
        let mut journal = EventHistory::open(dir.path(), now()).unwrap();
        std::fs::remove_file(dir.path().join(HISTORY_FILE)).unwrap();
        std::fs::create_dir(dir.path().join(HISTORY_FILE)).unwrap();
        assert!(journal.claim(&BoardEvent::default(), now(), false).is_err());
        std::fs::remove_dir(dir.path().join(HISTORY_FILE)).unwrap();
        assert!(journal.claim(&BoardEvent::default(), now(), false).is_err(), "failure stays latched");
    }
}
