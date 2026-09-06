use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::Res;

use super::write_atomic;

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuarantineStatus {
    Pending,
    Processing,
    NeedsReview,
    ReadyToPublish,
    AwaitingApproval,
    Publishing,
    Published,
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QuarantineDecision {
    pub at: DateTime<Utc>,
    pub actor: String,
    pub from: QuarantineStatus,
    pub to: QuarantineStatus,
    pub note: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QuarantineRecord {
    pub id: String,
    pub original_name: String,
    pub payload_file: PathBuf,
    pub destination: PathBuf,
    pub metadata_path: PathBuf,
    pub uploader: String,
    pub description: Vec<String>,
    pub uploaded_at: DateTime<Utc>,
    pub status: QuarantineStatus,
    #[serde(default)]
    pub processing_report: Vec<String>,
    #[serde(default)]
    pub decisions: Vec<QuarantineDecision>,
}

pub struct UploadQuarantine {
    root: PathBuf,
}

impl UploadQuarantine {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn enqueue(
        &self,
        source: &Path,
        original_name: String,
        destination: PathBuf,
        metadata_path: PathBuf,
        uploader: String,
        description: Vec<String>,
    ) -> Res<QuarantineRecord> {
        self.create_directories()?;
        let id = next_id();
        // Protocol receivers use extensionless temporary files. The advertised
        // name supplies the format, but never any part of the quarantine path.
        let extension = Path::new(&original_name)
            .extension()
            .and_then(|extension| extension.to_str())
            .filter(|extension| extension.len() <= 16 && extension.chars().all(|character| character.is_ascii_alphanumeric()));
        let payload_name = match extension {
            Some(extension) => format!("{id}.{extension}"),
            None => id.clone(),
        };
        let relative_payload = PathBuf::from("files").join(payload_name);
        let payload = self.root.join(&relative_payload);
        let temporary = tempfile::NamedTempFile::new_in(self.root.join("files"))?;
        fs::copy(source, temporary.path())?;
        temporary.persist(&payload).map_err(|error| error.error)?;

        let record = QuarantineRecord {
            id,
            original_name,
            payload_file: relative_payload,
            destination,
            metadata_path,
            uploader,
            description,
            uploaded_at: Utc::now(),
            status: QuarantineStatus::Pending,
            processing_report: Vec::new(),
            decisions: Vec::new(),
        };
        if let Err(error) = self.save(&record) {
            let _ = fs::remove_file(&payload);
            return Err(error);
        }
        fs::remove_file(source)?;
        Ok(record)
    }

    pub fn payload_path(&self, record: &QuarantineRecord) -> PathBuf {
        self.root.join(&record.payload_file)
    }

    pub fn load(&self, id: &str) -> Res<QuarantineRecord> {
        let text = fs::read_to_string(self.record_path(id))?;
        Ok(toml::from_str(&text)?)
    }

    pub fn list(&self) -> Res<Vec<QuarantineRecord>> {
        self.create_directories()?;
        let mut records = Vec::new();
        for entry in fs::read_dir(self.root.join("records"))? {
            let entry = entry?;
            if entry.path().extension().is_some_and(|extension| extension.eq_ignore_ascii_case("toml")) {
                let text = fs::read_to_string(entry.path())?;
                records.push(toml::from_str(&text)?);
            }
        }
        records.sort_by_key(|record: &QuarantineRecord| record.uploaded_at);
        Ok(records)
    }

    pub fn transition(&self, id: &str, expected: &[QuarantineStatus], status: QuarantineStatus, actor: &str, note: &str) -> Res<QuarantineRecord> {
        let _lock = self.lock()?;
        let mut record = self.load(id)?;
        if !expected.contains(&record.status) {
            return Err(format!("quarantine item {id} is {:?}, expected one of {expected:?}", record.status).into());
        }
        let from = record.status;
        record.status = status;
        record.decisions.push(QuarantineDecision {
            at: Utc::now(),
            actor: actor.to_string(),
            from,
            to: status,
            note: note.to_string(),
        });
        self.save_unlocked(&record)?;
        Ok(record)
    }

    /// Call once on startup while holding the board's exclusive process lock,
    /// before starting sessions or the web admin. Never recover live jobs.
    pub fn recover_interrupted(&self) -> Res<usize> {
        if !self.root.exists() {
            return Ok(0);
        }
        let _lock = self.lock()?;
        let mut recovered = 0;
        for mut record in self.list()? {
            let note = match record.status {
                QuarantineStatus::Processing => "processing was interrupted",
                QuarantineStatus::Publishing => "publication was interrupted; inspect the destination before retrying",
                _ => continue,
            };
            record.decisions.push(QuarantineDecision {
                at: Utc::now(),
                actor: "system".to_string(),
                from: record.status,
                to: QuarantineStatus::NeedsReview,
                note: note.to_string(),
            });
            record.status = QuarantineStatus::NeedsReview;
            self.save_unlocked(&record)?;
            recovered += 1;
        }
        Ok(recovered)
    }

    pub fn save(&self, record: &QuarantineRecord) -> Res<()> {
        let _lock = self.lock()?;
        self.save_unlocked(record)
    }

    fn save_unlocked(&self, record: &QuarantineRecord) -> Res<()> {
        self.create_directories()?;
        write_atomic(self.record_path(&record.id), toml::to_string_pretty(record)?.as_bytes())?;
        Ok(())
    }

    fn create_directories(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.root.join("files"))?;
        fs::create_dir_all(self.root.join("records"))
    }

    fn record_path(&self, id: &str) -> PathBuf {
        self.root.join("records").join(format!("{id}.toml"))
    }

    fn lock(&self) -> Res<File> {
        self.create_directories()?;
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(self.root.join(".quarantine.lock"))?;
        file.lock()?;
        Ok(file)
    }
}

fn next_id() -> String {
    let timestamp = Utc::now().timestamp_nanos_opt().unwrap_or_default();
    let sequence = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    format!("{timestamp:016x}-{sequence:08x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enqueue(quarantine: &UploadQuarantine, directory: &Path) -> QuarantineRecord {
        let source = directory.join("upload.zip");
        fs::write(&source, b"payload").unwrap();
        let record = quarantine
            .enqueue(
                &source,
                "UPLOAD.ZIP".to_string(),
                directory.join("files"),
                directory.join("metadata"),
                "SYSOP".to_string(),
                vec!["Description".to_string()],
            )
            .unwrap();
        assert!(!source.exists());
        record
    }

    #[test]
    fn enqueue_uses_an_opaque_payload_name_and_persists_the_record() {
        let directory = tempfile::tempdir().unwrap();
        let quarantine = UploadQuarantine::new(directory.path().join("quarantine"));
        let record = enqueue(&quarantine, directory.path());

        assert_eq!(QuarantineStatus::Pending, record.status);
        assert_ne!(record.original_name, record.payload_file.file_name().unwrap().to_string_lossy());
        assert_eq!(b"payload", fs::read(quarantine.payload_path(&record)).unwrap().as_slice());
        assert_eq!(record, quarantine.load(&record.id).unwrap());
    }

    #[test]
    fn enqueue_preserves_the_upload_extension_not_the_temporary_extension() {
        let directory = tempfile::tempdir().unwrap();
        let quarantine = UploadQuarantine::new(directory.path().join("quarantine"));
        for temporary_name in [".tmpABC123", "wrong.bin"] {
            let source = directory.path().join(temporary_name);
            fs::write(&source, b"payload").unwrap();
            let record = quarantine
                .enqueue(
                    &source,
                    "UPLOAD.ZIP".into(),
                    directory.path().join("public"),
                    directory.path().join("metadata"),
                    "ALICE".into(),
                    vec![],
                )
                .unwrap();
            assert_eq!(Some(std::ffi::OsStr::new("ZIP")), record.payload_file.extension());
            assert_eq!(b"payload", fs::read(quarantine.payload_path(&record)).unwrap().as_slice());
        }
    }

    #[test]
    fn recovery_moves_interrupted_processing_to_review() {
        let directory = tempfile::tempdir().unwrap();
        let quarantine = UploadQuarantine::new(directory.path().join("quarantine"));
        let record = enqueue(&quarantine, directory.path());
        quarantine
            .transition(&record.id, &[QuarantineStatus::Pending], QuarantineStatus::Processing, "system", "started")
            .unwrap();

        assert_eq!(1, quarantine.recover_interrupted().unwrap());
        let recovered = quarantine.load(&record.id).unwrap();
        assert_eq!(QuarantineStatus::NeedsReview, recovered.status);
        assert_eq!("processing was interrupted", recovered.decisions.last().unwrap().note);
    }

    #[test]
    fn recovery_handles_publication_and_is_idempotent() {
        let directory = tempfile::tempdir().unwrap();
        let quarantine = UploadQuarantine::new(directory.path().join("quarantine"));
        let record = enqueue(&quarantine, directory.path());
        quarantine
            .transition(&record.id, &[QuarantineStatus::Pending], QuarantineStatus::Publishing, "SYSOP", "claimed")
            .unwrap();
        assert_eq!(1, quarantine.recover_interrupted().unwrap());
        assert_eq!(0, quarantine.recover_interrupted().unwrap());
        let recovered = quarantine.load(&record.id).unwrap();
        assert_eq!(QuarantineStatus::NeedsReview, recovered.status);
        assert_eq!(QuarantineStatus::Publishing, recovered.decisions.last().unwrap().from);
        assert!(quarantine.payload_path(&recovered).exists());
        // Recovery releases the state so the operator can decide what to do.
        quarantine
            .transition(&record.id, &[QuarantineStatus::NeedsReview], QuarantineStatus::Rejected, "SYSOP", "reviewed")
            .unwrap();
        assert_eq!(0, quarantine.recover_interrupted().unwrap());
    }

    #[test]
    fn transition_rejects_a_stale_expected_status() {
        let directory = tempfile::tempdir().unwrap();
        let quarantine = UploadQuarantine::new(directory.path().join("quarantine"));
        let record = enqueue(&quarantine, directory.path());

        assert!(
            quarantine
                .transition(&record.id, &[QuarantineStatus::Processing], QuarantineStatus::Published, "SYSOP", "approve")
                .is_err()
        );
        assert_eq!(QuarantineStatus::Pending, quarantine.load(&record.id).unwrap().status);
    }
}
