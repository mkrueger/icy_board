use std::{process::Stdio, time::Duration};

use dizbase::file_base_scanner::{
    bbstro_fingerprint::FingerprintData,
    repack::{RepackOptions, Repacked, repack_file},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
};

use crate::Res;

use super::{
    icb_config::{UploadProcessingConfig, UploadPublishPolicy, UploadScannerConfig},
    upload_quarantine::{QuarantineRecord, QuarantineStatus, UploadQuarantine},
};

pub struct UploadProcessor {
    quarantine: UploadQuarantine,
    config: UploadProcessingConfig,
}

impl UploadProcessor {
    pub fn new(config: UploadProcessingConfig) -> Self {
        Self {
            quarantine: UploadQuarantine::new(config.quarantine_path.clone()),
            config,
        }
    }

    pub async fn process(&self, id: &str) -> Res<QuarantineRecord> {
        // Keep the submitted name for logging, even if conversion changes the
        // public extension or a later record write/reload fails.
        let original_name = self
            .quarantine
            .load(id)
            .inspect_err(|error| {
                log::warn!("{}", upload_log_message("<unavailable>", id, &format!("unable to load upload: {error}")));
            })?
            .original_name;
        let result = async {
            self.quarantine.transition(
                id,
                &[QuarantineStatus::Pending, QuarantineStatus::NeedsReview],
                QuarantineStatus::Processing,
                "system",
                "processing started",
            )?;
            match self.process_inner(id, &original_name).await {
                Ok(record) => Ok(record),
                Err(error) => {
                    let message = error.to_string();
                    log::warn!("{}", upload_log_message(&original_name, id, &format!("processing failed: {message}")));
                    let mut record = self.quarantine.load(id)?;
                    record.processing_report.push(format!("error: {message}"));
                    self.quarantine.save(&record)?;
                    self.quarantine
                        .transition(id, &[QuarantineStatus::Processing], QuarantineStatus::NeedsReview, "system", &message)
                }
            }
        }
        .await;
        if let Err(error) = &result {
            log::warn!(
                "{}",
                upload_log_message(&original_name, id, &format!("processing state update failed: {error}"))
            );
        }
        result
    }

    async fn process_inner(&self, id: &str, original_name: &str) -> Res<QuarantineRecord> {
        let mut record = self.quarantine.load(id)?;
        let mut payload = self.quarantine.payload_path(&record);
        if self.archive_processing_enabled() {
            let rules = if self.config.remove_advertisements {
                FingerprintData::load(&self.config.advertisement_rules)?
            } else {
                FingerprintData::default()
            };
            let additions =
                super::upload_advertisement::load_advertisement(&self.config.advertisement_file, original_name, self.config.max_member_size).await?;
            let options = RepackOptions {
                remove_advertisements: self.config.remove_advertisements,
                recompress: self.config.repack_to_zip,
                compression_level: self.config.compression_level,
                max_members: self.config.max_members,
                max_member_size: self.config.max_member_size,
                max_expanded_size: self.config.max_expanded_size,
                max_compression_ratio: self.config.max_compression_ratio,
                additions,
                replacement_archive_comment: (!self.config.replacement_archive_comment.is_empty())
                    .then(|| self.config.replacement_archive_comment.as_bytes().to_vec()),
                ..Default::default()
            };
            let source = payload.clone();
            // Repack a working copy. A crash or failed record write must leave
            // the currently recorded payload usable, even when the format changes.
            let (result, converted_payload) = tokio::task::spawn_blocking(move || -> Res<_> {
                let parent = source.parent().ok_or("quarantine payload has no parent")?;
                let work = tempfile::tempdir_in(parent)?;
                let work_source = work.path().join(source.file_name().ok_or("quarantine payload has no file name")?);
                std::fs::copy(&source, &work_source)?;
                let result = repack_file(&work_source, &rules, &options)?;
                let converted = if let Repacked::Converted { name, .. } = &result {
                    let staged = tempfile::Builder::new().prefix("processed-").suffix(".zip").tempfile_in(parent)?;
                    std::fs::copy(work.path().join(name), staged.path())?;
                    staged.as_file().sync_all()?;
                    Some(staged.into_temp_path().keep()?)
                } else {
                    None
                };
                Ok((result, converted))
            })
            .await??;
            match result {
                Repacked::Skipped(reason) => record.processing_report.push(format!("archive skipped: {reason}")),
                Repacked::Unchanged => record.processing_report.push("archive unchanged".to_string()),
                Repacked::NeedsReview { reason } => {
                    log::warn!("{}", upload_log_message(original_name, id, &format!("archive needs review: {reason}")));
                    record.processing_report.push(format!("archive needs review: {reason}"));
                    self.quarantine.save(&record)?;
                    return self
                        .quarantine
                        .transition(id, &[QuarantineStatus::Processing], QuarantineStatus::NeedsReview, "system", &reason);
                }
                Repacked::Converted {
                    removed,
                    added,
                    cleaned_descriptions,
                    archive_comment_rules,
                    ..
                } => {
                    let converted = converted_payload.ok_or("repacked upload has no payload")?;
                    record
                        .payload_file
                        .set_file_name(converted.file_name().ok_or("repacked upload has no file name")?);
                    if self.config.repack_to_zip || !record.original_name.to_ascii_lowercase().ends_with(".zip") {
                        record.original_name = std::path::Path::new(&record.original_name)
                            .with_extension("zip")
                            .file_name()
                            .and_then(|file_name| file_name.to_str())
                            .ok_or("processed upload has no usable public file name")?
                            .to_string();
                    }
                    let mut committed_changes = Vec::new();
                    for member in removed {
                        let message = format!("removed member: {member}");
                        record.processing_report.push(message.clone());
                        committed_changes.push(message);
                    }
                    for member in added {
                        record.processing_report.push(format!("added member: {member}"));
                    }
                    for description in cleaned_descriptions {
                        for change in description.changes {
                            let message = format!(
                                "cleaned {} lines {}-{} with {}",
                                description.name, change.first_line, change.last_line, change.rule_id
                            );
                            record.processing_report.push(message.clone());
                            committed_changes.push(message);
                        }
                    }
                    for rule in archive_comment_rules {
                        let message = format!("cleaned archive comment with {rule}");
                        record.processing_report.push(message.clone());
                        committed_changes.push(message);
                    }
                    // Commit the new path before scanning (including timeout and
                    // process-start errors), and only then retire the old payload.
                    self.quarantine.save(&record)?;
                    for message in committed_changes {
                        log::info!("{}", upload_log_message(original_name, id, &message));
                    }
                    if let Err(error) = std::fs::remove_file(&payload) {
                        log::warn!(
                            "{}",
                            upload_log_message(original_name, id, &format!("unable to remove superseded upload {payload:?}: {error}"))
                        );
                    }
                    payload = converted;
                }
            }
        }

        if self.config.scanner.enabled {
            match run_scanner(&self.config.scanner, &payload).await? {
                ScannerResult::Clean => record.processing_report.push("virus scanner: clean".to_string()),
                ScannerResult::Infected(output) => {
                    log::warn!(
                        "{}",
                        upload_log_message(original_name, id, &format!("virus scanner reported an infection: {output}"))
                    );
                    record.processing_report.push("virus scanner: infected".to_string());
                    record.processing_report.push(format!("virus scanner output: {output}"));
                    self.quarantine.save(&record)?;
                    return self.quarantine.transition(
                        id,
                        &[QuarantineStatus::Processing],
                        QuarantineStatus::NeedsReview,
                        "system",
                        "virus scanner reported an infection",
                    );
                }
            }
        }

        self.quarantine.save(&record)?;
        let status = match self.config.publish_policy {
            UploadPublishPolicy::Immediate | UploadPublishPolicy::AfterProcessing => QuarantineStatus::ReadyToPublish,
            UploadPublishPolicy::ManualApproval => QuarantineStatus::AwaitingApproval,
        };
        self.quarantine
            .transition(id, &[QuarantineStatus::Processing], status, "system", "processing completed")
    }

    fn archive_processing_enabled(&self) -> bool {
        self.config.remove_advertisements
            || self.config.repack_to_zip
            || !self.config.advertisement_file.as_os_str().is_empty()
            || !self.config.replacement_archive_comment.is_empty()
    }
}

// Debug formatting escapes untrusted filenames, rule IDs, scanner output and
// errors, keeping every event on one physical line in the board's log facade.
fn upload_log_message(original_name: &str, id: &str, message: &str) -> String {
    format!("Upload filename={original_name:?} quarantine_id={id:?}: {message:?}")
}

#[derive(Debug)]
enum ScannerResult {
    Clean,
    Infected(String),
}

const SCANNER_OUTPUT_LIMIT: usize = 16 * 1024;

#[derive(Default)]
struct ScannerOutput {
    bytes: Vec<u8>,
    truncated: bool,
}

impl ScannerOutput {
    fn report(&self) -> String {
        format!("{:?}{}", String::from_utf8_lossy(&self.bytes), if self.truncated { " [truncated]" } else { "" })
    }
}

async fn drain_scanner_output(mut reader: impl AsyncRead + Unpin, output: &mut ScannerOutput) -> std::io::Result<()> {
    let mut buffer = [0; 4096];
    loop {
        let count = reader.read(&mut buffer).await?;
        if count == 0 {
            return Ok(());
        }
        let retained = count.min(SCANNER_OUTPUT_LIMIT - output.bytes.len());
        output.bytes.extend_from_slice(&buffer[..retained]);
        output.truncated |= retained < count;
        // Continue draining after the cap so a verbose scanner cannot block
        // on a full pipe. Both streams are drained concurrently with wait().
    }
}

async fn run_scanner(config: &UploadScannerConfig, file: &std::path::Path) -> Res<ScannerResult> {
    if !config.arguments.iter().any(|argument| argument == "{file}") {
        return Err("virus scanner arguments must contain a standalone {file} argument".into());
    }
    let arguments: Vec<String> = config
        .arguments
        .iter()
        .map(|argument| {
            if argument == "{file}" {
                file.to_string_lossy().to_string()
            } else {
                argument.clone()
            }
        })
        .collect();
    let mut command = Command::new(&config.executable);
    command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command.spawn().map_err(|error| format!("unable to start virus scanner: {error}"))?;
    let stdout = child.stdout.take().ok_or("virus scanner stdout pipe unavailable")?;
    let stderr = child.stderr.take().ok_or("virus scanner stderr pipe unavailable")?;
    let mut stdout_output = ScannerOutput::default();
    let mut stderr_output = ScannerOutput::default();
    // Include the drains in the timeout: descendants may keep pipes open even
    // after the scanner exits. No detached drain tasks survive cancellation,
    // and dropping child still kills a running scanner on every error path.
    let result = tokio::time::timeout(Duration::from_secs(config.timeout_seconds), async {
        tokio::try_join!(
            child.wait(),
            drain_scanner_output(stdout, &mut stdout_output),
            drain_scanner_output(stderr, &mut stderr_output),
        )
    })
    .await;
    let output = format!("stdout={} stderr={}", stdout_output.report(), stderr_output.report());
    let status = match result {
        Ok(Ok((status, (), ()))) => status,
        Ok(Err(error)) => return Err(format!("virus scanner failed: {error}; {output}").into()),
        Err(_) => return Err(format!("virus scanner timed out after {} seconds; {output}", config.timeout_seconds).into()),
    };
    match status.code() {
        Some(code) if code == config.clean_exit_code => Ok(ScannerResult::Clean),
        Some(code) if code == config.infected_exit_code => Ok(ScannerResult::Infected(output)),
        Some(code) => Err(format!("virus scanner failed with exit code {code}; {output}").into()),
        None => Err(format!("virus scanner terminated without an exit code; {output}").into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[cfg(unix)]
    use std::path::PathBuf;
    use zip::unstable::write::FileOptionsExt;

    fn queued_zip(directory: &std::path::Path, config: &UploadProcessingConfig, encrypted: bool) -> (UploadQuarantine, QuarantineRecord) {
        let source = tempfile::NamedTempFile::new_in(directory).unwrap();
        let mut zip = zip::ZipWriter::new(source.as_file());
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let options = if encrypted {
            options.with_deprecated_encryption(b"test password").unwrap()
        } else {
            options
        };
        zip.start_file("PAYLOAD.TXT", options).unwrap();
        zip.write_all(b"payload").unwrap();
        zip.finish().unwrap();
        let quarantine = UploadQuarantine::new(config.quarantine_path.clone());
        let record = quarantine
            .enqueue(
                source.path(),
                "UPLOAD.ZIP".into(),
                directory.join("public"),
                directory.join("metadata"),
                "ALICE".into(),
                vec![],
            )
            .unwrap();
        (quarantine, record)
    }

    #[test]
    fn upload_log_message_escapes_all_untrusted_fields() {
        let message = upload_log_message("UPLOAD\n\r\x1b[31m.ZIP", "id\nforged", "removed member: ad\n.txt\r\0\x1b[2J");
        assert_eq!(
            message,
            "Upload filename=\"UPLOAD\\n\\r\\u{1b}[31m.ZIP\" quarantine_id=\"id\\nforged\": \"removed member: ad\\n.txt\\r\\0\\u{1b}[2J\""
        );
        assert!(!message.chars().any(char::is_control));
    }

    #[tokio::test]
    async fn scanner_capture_is_bounded_and_drains_past_the_limit() {
        for size in [0, SCANNER_OUTPUT_LIMIT, SCANNER_OUTPUT_LIMIT + 8192] {
            let input = vec![b'x'; size];
            let mut reader = input.as_slice();
            let mut output = ScannerOutput::default();
            drain_scanner_output(&mut reader, &mut output).await.unwrap();
            assert!(reader.is_empty(), "the entire stream must be drained");
            assert_eq!(output.bytes, input[..size.min(SCANNER_OUTPUT_LIMIT)]);
            assert_eq!(output.truncated, size > SCANNER_OUTPUT_LIMIT);
            assert_eq!(output.report().contains("[truncated]"), size > SCANNER_OUTPUT_LIMIT);
        }
    }

    #[tokio::test]
    async fn scanner_capture_sanitizes_controls_and_invalid_utf8() {
        let mut output = ScannerOutput::default();
        drain_scanner_output(b"Eicar-Test-Signature FOUND\n\r\x1b[2J\0\xff".as_slice(), &mut output)
            .await
            .unwrap();
        let report = output.report();
        assert!(report.contains("Eicar-Test-Signature FOUND"));
        assert!(report.contains("\\n\\r\\u{1b}[2J\\0"));
        assert!(report.contains('\u{fffd}'));
        assert!(!report.chars().any(char::is_control));
        assert!(!upload_log_message("original.zip", "quarantine-id", &report).chars().any(char::is_control));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn verbose_infected_scanner_drains_both_pipes_with_bounded_reports() {
        let config = UploadScannerConfig {
            enabled: true,
            executable: PathBuf::from("/bin/sh"),
            arguments: vec![
                "-c".into(),
                "printf 'Eicar-Test-Signature FOUND\\n'; printf 'scanner diagnostic\\n' >&2; \
                 i=0; while [ \"$i\" -lt 8192 ]; do \
                 printf '0123456789abcdef0123456789abcdef\\n'; \
                 printf '0123456789abcdef0123456789abcdef\\n' >&2; \
                 i=$((i + 1)); done; exit 1"
                    .into(),
                "{file}".into(),
            ],
            timeout_seconds: 10,
            ..Default::default()
        };
        let ScannerResult::Infected(output) = run_scanner(&config, std::path::Path::new("unused.zip")).await.unwrap() else {
            panic!("expected an infection report");
        };
        assert!(output.contains("stdout=\"Eicar-Test-Signature FOUND\\n"));
        assert!(output.contains("stderr=\"scanner diagnostic\\n"));
        assert_eq!(output.matches("[truncated]").count(), 2);
        assert!(output.len() < 4 * SCANNER_OUTPUT_LIMIT);
        assert!(!output.chars().any(char::is_control));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn scanner_error_output_is_recorded_and_fail_closed() {
        let directory = tempfile::tempdir().unwrap();
        let config = UploadProcessingConfig {
            publish_policy: UploadPublishPolicy::AfterProcessing,
            quarantine_path: directory.path().join("quarantine"),
            scanner: UploadScannerConfig {
                enabled: true,
                executable: PathBuf::from("/bin/sh"),
                arguments: vec!["-c".into(), "printf 'scan failed\\n\\033[2J' >&2; exit 2".into(), "{file}".into()],
                ..Default::default()
            },
            ..Default::default()
        };
        let (quarantine, record) = queued_zip(directory.path(), &config, false);
        let processed = UploadProcessor::new(config).process(&record.id).await.unwrap();
        assert_eq!(processed.status, QuarantineStatus::NeedsReview);
        let error = processed.processing_report.iter().find(|line| line.starts_with("error:")).unwrap();
        assert!(error.contains("exit code 2"));
        assert!(error.contains("stderr=\"scan failed\\n\\u{1b}[2J\""));
        assert!(!error.chars().any(char::is_control));
        assert_eq!(processed.processing_report, quarantine.load(&record.id).unwrap().processing_report);
    }

    #[tokio::test]
    async fn description_as_advertisement_requires_review() {
        let directory = tempfile::tempdir().unwrap();
        let description = directory.path().join("file_id.diz");
        std::fs::write(&description, b"Board advertisement").unwrap();
        let config = UploadProcessingConfig {
            publish_policy: UploadPublishPolicy::AfterProcessing,
            quarantine_path: directory.path().join("quarantine"),
            advertisement_file: description,
            ..Default::default()
        };
        let (quarantine, record) = queued_zip(directory.path(), &config, false);
        let original = std::fs::read(quarantine.payload_path(&record)).unwrap();
        let result = UploadProcessor::new(config).process(&record.id).await.unwrap();
        assert_eq!(QuarantineStatus::NeedsReview, result.status);
        assert!(result.processing_report.iter().any(|line| line.contains("protected description file")));
        assert_eq!(original, std::fs::read(quarantine.payload_path(&result)).unwrap());
    }

    #[tokio::test]
    async fn protocol_tempfile_is_actually_processed_as_a_zip() {
        let directory = tempfile::tempdir().unwrap();
        let config = UploadProcessingConfig {
            publish_policy: UploadPublishPolicy::AfterProcessing,
            quarantine_path: directory.path().join("quarantine"),
            repack_to_zip: true,
            replacement_archive_comment: "Own BBS".into(),
            ..Default::default()
        };
        let (quarantine, record) = queued_zip(directory.path(), &config, false);
        let result = UploadProcessor::new(config).process(&record.id).await.unwrap();
        assert_eq!(QuarantineStatus::ReadyToPublish, result.status);
        let archive = zip::ZipArchive::new(std::fs::File::open(quarantine.payload_path(&result)).unwrap()).unwrap();
        assert_eq!(b"Own BBS", archive.comment());
        assert!(!result.processing_report.iter().any(|line| line.starts_with("archive skipped")));
    }

    #[tokio::test]
    async fn scanner_start_failure_after_conversion_keeps_a_reprocessable_payload() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = UploadProcessingConfig {
            publish_policy: UploadPublishPolicy::AfterProcessing,
            quarantine_path: directory.path().join("quarantine"),
            repack_to_zip: true,
            replacement_archive_comment: "Own BBS".into(),
            scanner: UploadScannerConfig {
                enabled: true,
                executable: directory.path().join("missing-scanner"),
                ..Default::default()
            },
            ..Default::default()
        };
        let (quarantine, record) = queued_zip(directory.path(), &config, false);
        let result = UploadProcessor::new(config.clone()).process(&record.id).await.unwrap();
        assert_eq!(QuarantineStatus::NeedsReview, result.status);
        assert_eq!(result.payload_file, quarantine.load(&record.id).unwrap().payload_file);
        assert!(quarantine.payload_path(&result).exists());
        assert_eq!(
            b"Own BBS",
            zip::ZipArchive::new(std::fs::File::open(quarantine.payload_path(&result)).unwrap())
                .unwrap()
                .comment()
        );
        config.scanner.enabled = false;
        let retried = UploadProcessor::new(config).process(&record.id).await.unwrap();
        assert_eq!(QuarantineStatus::ReadyToPublish, retried.status);
        assert!(quarantine.payload_path(&retried).exists());
    }

    #[tokio::test]
    async fn encrypted_upload_requires_review_without_a_virus_scanner() {
        let directory = tempfile::tempdir().unwrap();
        let config = UploadProcessingConfig {
            publish_policy: UploadPublishPolicy::AfterProcessing,
            quarantine_path: directory.path().join("quarantine"),
            repack_to_zip: true,
            ..Default::default()
        };
        let (quarantine, record) = queued_zip(directory.path(), &config, true);
        let original = std::fs::read(quarantine.payload_path(&record)).unwrap();
        let result = UploadProcessor::new(config).process(&record.id).await.unwrap();
        assert_eq!(QuarantineStatus::NeedsReview, result.status);
        assert!(result.processing_report.iter().any(|line| line.contains("password")));
        assert_eq!(original, std::fs::read(quarantine.payload_path(&result)).unwrap());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn scanner_timeout_after_conversion_keeps_the_committed_payload() {
        let directory = tempfile::tempdir().unwrap();
        let config = UploadProcessingConfig {
            publish_policy: UploadPublishPolicy::AfterProcessing,
            quarantine_path: directory.path().join("quarantine"),
            repack_to_zip: true,
            replacement_archive_comment: "Own BBS".into(),
            scanner: UploadScannerConfig {
                enabled: true,
                executable: PathBuf::from("/bin/sh"),
                arguments: vec!["-c".into(), "while :; do :; done".into(), "{file}".into()],
                timeout_seconds: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        let (quarantine, record) = queued_zip(directory.path(), &config, false);
        let result = UploadProcessor::new(config).process(&record.id).await.unwrap();
        assert_eq!(QuarantineStatus::NeedsReview, result.status);
        assert!(result.processing_report.iter().any(|line| line.contains("timed out")));
        assert!(quarantine.payload_path(&result).exists());
        assert_eq!(
            b"Own BBS",
            zip::ZipArchive::new(std::fs::File::open(quarantine.payload_path(&result)).unwrap())
                .unwrap()
                .comment()
        );
    }

    #[tokio::test]
    async fn processing_without_steps_reaches_manual_approval() {
        let directory = tempfile::tempdir().unwrap();
        let config = UploadProcessingConfig {
            publish_policy: UploadPublishPolicy::ManualApproval,
            quarantine_path: directory.path().join("quarantine"),
            ..Default::default()
        };
        let quarantine = UploadQuarantine::new(config.quarantine_path.clone());
        let source = directory.path().join("upload.bin");
        std::fs::write(&source, b"payload").unwrap();
        let record = quarantine
            .enqueue(
                &source,
                "UPLOAD.BIN".to_string(),
                directory.path().join("files"),
                directory.path().join("metadata"),
                "SYSOP".to_string(),
                Vec::new(),
            )
            .unwrap();

        let processed = UploadProcessor::new(config).process(&record.id).await.unwrap();
        assert_eq!(QuarantineStatus::AwaitingApproval, processed.status);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn infected_scanner_result_is_fail_closed() {
        let directory = tempfile::tempdir().unwrap();
        let config = UploadProcessingConfig {
            publish_policy: UploadPublishPolicy::AfterProcessing,
            quarantine_path: directory.path().join("quarantine"),
            scanner: UploadScannerConfig {
                enabled: true,
                executable: PathBuf::from("/bin/sh"),
                arguments: vec![
                    "-c".to_string(),
                    "printf 'Eicar-Test-Signature FOUND\\n'; printf 'infected diagnostic\\r\\033[2J' >&2; exit 1".to_string(),
                    "{file}".to_string(),
                ],
                ..Default::default()
            },
            ..Default::default()
        };
        let quarantine = UploadQuarantine::new(config.quarantine_path.clone());
        let source = directory.path().join("upload.bin");
        std::fs::write(&source, b"payload").unwrap();
        let record = quarantine
            .enqueue(
                &source,
                "UPLOAD.BIN".to_string(),
                directory.path().join("files"),
                directory.path().join("metadata"),
                "SYSOP".to_string(),
                Vec::new(),
            )
            .unwrap();

        let processed = UploadProcessor::new(config).process(&record.id).await.unwrap();
        assert_eq!(QuarantineStatus::NeedsReview, processed.status);
        assert!(processed.processing_report.iter().any(|line| line == "virus scanner: infected"));
        let output = processed
            .processing_report
            .iter()
            .find(|line| line.starts_with("virus scanner output:"))
            .unwrap();
        assert!(output.contains("stdout=\"Eicar-Test-Signature FOUND\\n\""));
        assert!(output.contains("stderr=\"infected diagnostic\\r\\u{1b}[2J\""));
        assert!(!output.chars().any(char::is_control));
        assert_eq!(processed.processing_report, quarantine.load(&record.id).unwrap().processing_report);
    }
}
