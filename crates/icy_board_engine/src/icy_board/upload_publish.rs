use std::fs;

use dizbase::{
    file_base::{
        FileBase,
        metadata::{MetadataHeader, MetadataType},
    },
    file_base_scanner::scan_file,
};

use crate::Res;

use super::{
    lookup_case_insensitive,
    upload_quarantine::{QuarantineRecord, QuarantineStatus, UploadQuarantine},
};

#[derive(Debug, thiserror::Error)]
#[error("quarantine item is no longer available for publication")]
pub struct PublicationConflict;

/// Both automatic and manual publication must acquire the same durable claim
/// before touching the destination. Rejection and other publishers then lose.
pub fn publish_quarantine_record(quarantine: &UploadQuarantine, id: &str, actor: &str, note: &str) -> Res<QuarantineRecord> {
    let record = quarantine
        .transition(
            id,
            &[QuarantineStatus::ReadyToPublish, QuarantineStatus::AwaitingApproval],
            QuarantineStatus::Publishing,
            actor,
            "claimed for publication",
        )
        .map_err(|_| PublicationConflict)?;
    match publish_claimed_record(quarantine, &record, actor, note) {
        Ok(published) => Ok(published),
        Err(error) => {
            quarantine.transition(id, &[QuarantineStatus::Publishing], QuarantineStatus::NeedsReview, "system", &error.to_string())?;
            Err(error)
        }
    }
}

fn publish_claimed_record(quarantine: &UploadQuarantine, record: &QuarantineRecord, actor: &str, note: &str) -> Res<QuarantineRecord> {
    let source = quarantine.payload_path(&record);
    let destination = record.destination.join(&record.original_name);
    fs::create_dir_all(&record.destination)?;
    let mut file_base = FileBase::open(&record.destination, &record.metadata_path)?;
    if file_base.contains_name(&record.original_name) || lookup_case_insensitive(&destination).exists() {
        return Err(format!("destination file '{}' already exists", destination.display()).into());
    }

    // Never expose a partial copy or truncate a competing publisher's file.
    let staged = tempfile::NamedTempFile::new_in(&record.destination)?;
    fs::copy(&source, staged.path())?;
    staged.as_file().sync_all()?;
    staged.persist_noclobber(&destination)?;
    let result = (|| -> Res<()> {
        let mut metadata = scan_file(&destination)?;
        metadata.push(MetadataHeader {
            data: record.uploader.as_bytes().to_vec(),
            metadata_type: MetadataType::Uploader,
        });
        if !record.description.is_empty() && !metadata.iter().any(|item| item.metadata_type == MetadataType::FileID) {
            metadata.push(MetadataHeader {
                data: record.description.join("\n").into_bytes(),
                metadata_type: MetadataType::FileID,
            });
        }
        file_base.add_file(&destination, metadata)?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&destination);
        return Err(error);
    }
    let published = quarantine.transition(&record.id, &[QuarantineStatus::Publishing], QuarantineStatus::Published, actor, note)?;
    // Keep the payload until the terminal state is durable, so interrupted
    // publication can always be sent back to review with its payload intact.
    if let Err(error) = fs::remove_file(source) {
        log::warn!("Unable to remove published quarantine payload {}: {error}", record.id);
    }
    Ok(published)
}

pub fn reject_quarantine_record(quarantine: &UploadQuarantine, id: &str, actor: &str, note: &str) -> Res<QuarantineRecord> {
    quarantine.transition(
        id,
        &[
            QuarantineStatus::Pending,
            QuarantineStatus::NeedsReview,
            QuarantineStatus::ReadyToPublish,
            QuarantineStatus::AwaitingApproval,
        ],
        QuarantineStatus::Rejected,
        actor,
        note,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queued(directory: &std::path::Path) -> (UploadQuarantine, QuarantineRecord) {
        let quarantine = UploadQuarantine::new(directory.join("quarantine"));
        let source = directory.join("upload.bin");
        fs::write(&source, b"payload").unwrap();
        let record = quarantine
            .enqueue(
                &source,
                "UPLOAD.BIN".to_string(),
                directory.join("files"),
                directory.join("metadata"),
                "ALICE".to_string(),
                vec!["Description".to_string()],
            )
            .unwrap();
        (quarantine, record)
    }

    #[test]
    fn approval_publishes_and_indexes_a_quarantined_file() {
        let directory = tempfile::tempdir().unwrap();
        let (quarantine, record) = queued(directory.path());
        quarantine
            .transition(&record.id, &[QuarantineStatus::Pending], QuarantineStatus::AwaitingApproval, "system", "ready")
            .unwrap();

        let published = publish_quarantine_record(&quarantine, &record.id, "SYSOP", "approved").unwrap();
        assert_eq!(QuarantineStatus::Published, published.status);
        assert_eq!(b"payload", fs::read(directory.path().join("files/UPLOAD.BIN")).unwrap().as_slice());
        assert!(!quarantine.payload_path(&record).exists());
    }

    #[test]
    fn rejection_keeps_the_payload_in_quarantine() {
        let directory = tempfile::tempdir().unwrap();
        let (quarantine, record) = queued(directory.path());
        let rejected = reject_quarantine_record(&quarantine, &record.id, "SYSOP", "bad file").unwrap();
        assert_eq!(QuarantineStatus::Rejected, rejected.status);
        assert!(quarantine.payload_path(&record).exists());
    }

    #[test]
    fn a_second_approval_cannot_claim_the_same_item() {
        let directory = tempfile::tempdir().unwrap();
        let (quarantine, record) = queued(directory.path());
        quarantine
            .transition(&record.id, &[QuarantineStatus::Pending], QuarantineStatus::AwaitingApproval, "system", "ready")
            .unwrap();
        quarantine
            .transition(
                &record.id,
                &[QuarantineStatus::AwaitingApproval],
                QuarantineStatus::Publishing,
                "first",
                "claimed",
            )
            .unwrap();

        let error = publish_quarantine_record(&quarantine, &record.id, "second", "approved").unwrap_err();
        assert!(error.is::<PublicationConflict>());
        assert!(!directory.path().join("files/UPLOAD.BIN").exists());
        assert_eq!(QuarantineStatus::Publishing, quarantine.load(&record.id).unwrap().status);
    }

    #[test]
    fn automatic_publication_cannot_publish_a_rejected_upload() {
        let directory = tempfile::tempdir().unwrap();
        let (quarantine, record) = queued(directory.path());
        quarantine
            .transition(
                &record.id,
                &[QuarantineStatus::Pending],
                QuarantineStatus::ReadyToPublish,
                "system",
                "processed",
            )
            .unwrap();
        reject_quarantine_record(&quarantine, &record.id, "SYSOP", "rejected before automatic publication").unwrap();
        assert!(
            publish_quarantine_record(&quarantine, &record.id, "system", "automatic")
                .unwrap_err()
                .is::<PublicationConflict>()
        );
        assert!(!directory.path().join("files/UPLOAD.BIN").exists());
        assert_eq!(QuarantineStatus::Rejected, quarantine.load(&record.id).unwrap().status);
        assert!(quarantine.payload_path(&record).exists());
    }

    #[test]
    fn automatic_and_manual_publishers_share_one_claim() {
        let directory = tempfile::tempdir().unwrap();
        let (quarantine, record) = queued(directory.path());
        quarantine
            .transition(
                &record.id,
                &[QuarantineStatus::Pending],
                QuarantineStatus::ReadyToPublish,
                "system",
                "processed",
            )
            .unwrap();
        let barrier = std::sync::Barrier::new(2);
        let results = std::thread::scope(|scope| {
            let automatic = scope.spawn(|| {
                barrier.wait();
                publish_quarantine_record(&quarantine, &record.id, "system", "automatic")
            });
            let manual = scope.spawn(|| {
                barrier.wait();
                publish_quarantine_record(&quarantine, &record.id, "SYSOP", "approved")
            });
            [automatic.join().unwrap(), manual.join().unwrap()]
        });
        assert_eq!(1, results.iter().filter(|result| result.is_ok()).count());
        assert!(
            results
                .iter()
                .filter_map(|result| result.as_ref().err())
                .all(|error| error.is::<PublicationConflict>())
        );
        let published = quarantine.load(&record.id).unwrap();
        assert_eq!(QuarantineStatus::Published, published.status);
        assert_eq!(
            1,
            published
                .decisions
                .iter()
                .filter(|decision| decision.to == QuarantineStatus::Publishing)
                .count()
        );
        assert_eq!(b"payload", fs::read(directory.path().join("files/UPLOAD.BIN")).unwrap().as_slice());
    }

    #[test]
    fn failed_publication_returns_to_review_without_losing_payload() {
        let directory = tempfile::tempdir().unwrap();
        let (quarantine, record) = queued(directory.path());
        quarantine
            .transition(
                &record.id,
                &[QuarantineStatus::Pending],
                QuarantineStatus::ReadyToPublish,
                "system",
                "processed",
            )
            .unwrap();
        fs::create_dir_all(&record.destination).unwrap();
        let destination = record.destination.join(&record.original_name);
        fs::write(&destination, b"existing file").unwrap();
        assert!(publish_quarantine_record(&quarantine, &record.id, "system", "automatic").is_err());
        assert_eq!(QuarantineStatus::NeedsReview, quarantine.load(&record.id).unwrap().status);
        assert_eq!(b"existing file", fs::read(destination).unwrap().as_slice());
        assert!(quarantine.payload_path(&record).exists());
    }
}
