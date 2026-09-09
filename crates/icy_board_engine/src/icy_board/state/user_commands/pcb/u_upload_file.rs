use crate::icy_board::commands::CommandType;
use crate::icy_board::icb_config::{IcbColor, UploadPublishPolicy};
use crate::icy_board::lookup_case_insensitive;
use crate::icy_board::state::local_transfer::{LocalFilePickerKind, stage_local_upload};
use crate::icy_board::upload_processor::UploadProcessor;
use crate::icy_board::upload_publish::publish_quarantine_record;
use crate::icy_board::upload_quarantine::{QuarantineStatus, UploadQuarantine};
use crate::{Res, icy_board::state::IcyBoardState};
use crate::{
    icy_board::{
        IcyBoard,
        icb_text::IceText,
        state::{
            NodeStatus,
            functions::{MASK_ASCII, display_flags, transfer_cps},
        },
    },
    vm::TerminalTarget,
};
use bstr::BString;
use chrono::Utc;
use dizbase::file_base::{
    FileBase,
    metadata::{MetadataHeader, MetadataType},
};
use dizbase::file_base_scanner::scan_file;
use fs4::available_space;
use icy_net::protocol::{Protocol, TransferProtocolType, TransferState, XYModemVariant, XYmodem, Zmodem};
use jamjam::jam::{JamMessage, attributes};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tempfile::{NamedTempFile, TempPath};
use tokio::time::{Duration, timeout};

struct UploadRequest {
    name: String,
    description: Vec<String>,
    private: bool,
    local_source: Option<TempPath>,
    local_cps: u64,
}

struct CompletedUpload {
    name: String,
    source: TempPath,
    cps: u64,
}

#[derive(Default)]
struct UploadReceipt {
    files: Vec<CompletedUpload>,
    failed: bool,
    errors: usize,
}

// Protocols keep completed temporaries in their state, even across awaits.
struct UploadReceiveGuard(TransferState);

impl Drop for UploadReceiveGuard {
    fn drop(&mut self) {
        for (_, path) in self.0.recieve_state.finished_files.drain(..) {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn normalize_upload_name(name: &str) -> Option<String> {
    // Reject DOS separators and traversal on Linux too; never silently redirect
    // an advertised path to a different, previously described file.
    if name.is_empty() || name.len() > 255 || name.chars().any(|ch| ch.is_control() || "/\\:*?[]\"<>|".contains(ch)) {
        return None;
    }
    let name = name.trim_end_matches('.');
    if name.is_empty() || name.trim() != name {
        return None;
    }
    Some(name.to_string())
}

fn upload_credits(bytes: u64, cps: u64, byte_rate: u32, time_rate: u32) -> (u64, u64) {
    // TRANSFER.C successful(): division by CPS precedes multiplication.
    (
        bytes.saturating_mul(u64::from(byte_rate)) / 10,
        if cps == 0 {
            0
        } else {
            (bytes / cps).saturating_mul(u64::from(time_rate)) / 10
        },
    )
}

fn has_upload_space(path: &std::path::Path, minimum_kib: u32) -> std::io::Result<bool> {
    Ok(enough_upload_space(available_space(path)?, minimum_kib))
}

fn enough_upload_space(available_bytes: u64, minimum_kib: u32) -> bool {
    minimum_kib == 0 || available_bytes / 1024 >= u64::from(minimum_kib)
}

fn upload_name_exists(base: &FileBase, location: &std::path::Path, name: &str) -> bool {
    base.contains_name(name) || lookup_case_insensitive(&location.join(name)).exists()
}

fn scan_upload_description(source: &Path, name: &str) -> Res<Option<Vec<String>>> {
    let name = normalize_upload_name(name).ok_or("invalid upload name")?;
    let directory = tempfile::tempdir()?;
    let path = directory.path().join(name);
    std::fs::copy(source, &path)?;
    Ok(scan_file(&path)?
        .into_iter()
        .find(|header| header.metadata_type == MetadataType::FileID && !header.data.is_empty())
        .map(|header| String::from_utf8_lossy(&header.data).lines().map(str::to_string).collect()))
}

impl IcyBoardState {
    async fn notify_sysop_about_upload(&mut self, name: &str, uploader: &str, status: &str, description: &[String]) -> Res<()> {
        let sysop = self.get_board().await.config.sysop.name.clone();
        let message = upload_notification(&sysop, name, uploader, status, description);
        let mut message_base = self.get_email_msgbase(&sysop).await?;
        message_base.write_message(&message)?;
        message_base.write_jhr_header()?;
        Ok(())
    }
}

fn upload_notification(sysop: &str, name: &str, uploader: &str, status: &str, description: &[String]) -> JamMessage {
    let subject_name: String = name.chars().filter(|character| !character.is_control()).collect();
    let description = description.join("\n");
    let description: String = description
        .chars()
        .filter(|character| !character.is_control() || matches!(character, '\n' | '\r' | '\t'))
        .collect();
    let body = format!("File: {subject_name}\nUploader: {uploader}\nStatus: {status}\n\n{description}");
    JamMessage::default()
        .with_from(BString::from("IcyBoard"))
        .with_to(BString::from(sysop))
        .with_subject(BString::from(format!("New upload: {subject_name}")))
        .with_date_time(Utc::now())
        .with_attributes(attributes::MSG_LOCAL | attributes::MSG_PRIVATE)
        .with_text(BString::from(body))
}

impl IcyBoardState {
    async fn publish_uploaded_file(
        &mut self,
        source: &Path,
        name: &str,
        upload_location: &PathBuf,
        upload_metadata: &PathBuf,
        description: &[String],
    ) -> Res<bool> {
        let Some(name) = normalize_upload_name(name) else { return Ok(false) };
        let dest = upload_location.join(&name);
        let file_base = self.get_filebase(upload_location, upload_metadata).await?;
        let mut base = file_base.lock().await;
        let duplicate = upload_name_exists(&base, upload_location, &name);
        if duplicate {
            drop(base);
            self.session.op_text = name.to_string();
            self.display_text(IceText::DuplicateFile, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(false);
        }

        let staged = NamedTempFile::new_in(upload_location)?;
        std::fs::copy(source, staged.path())?;
        staged.as_file().sync_all()?;
        if let Err(error) = staged.persist_noclobber(&dest) {
            if error.error.kind() == std::io::ErrorKind::AlreadyExists {
                return Ok(false);
            }
            return Err(error.error.into());
        }
        let metadata = match scan_file(&dest) {
            Ok(mut metadata) => {
                metadata.push(MetadataHeader {
                    data: self.session.get_username_or_alias().as_bytes().to_vec(),
                    metadata_type: MetadataType::Uploader,
                });
                if !description.is_empty() && !metadata.iter().any(|item| item.metadata_type == MetadataType::FileID) {
                    metadata.push(MetadataHeader {
                        data: description.join("\n").as_bytes().to_vec(),
                        metadata_type: MetadataType::FileID,
                    });
                }
                metadata
            }
            Err(error) => {
                let _ = std::fs::remove_file(&dest);
                return Err(error);
            }
        };
        if let Err(error) = base.add_file(&dest, metadata) {
            let _ = std::fs::remove_file(&dest);
            return Err(error);
        }
        // Source cleanup belongs to the caller's TempPath, never the local original.
        Ok(true)
    }

    async fn upload_name_exists_on_system(&mut self, name: &str) -> Res<bool> {
        let conference = self.session.current_conference.clone();
        for (location, metadata) in [
            (&conference.pub_upload_location, &conference.pub_upload_metadata),
            (&conference.private_upload_location, &conference.private_upload_metadata),
        ] {
            if lookup_case_insensitive(&location.join(name)).exists() {
                return Ok(true);
            }
            // Upload locations need not also appear in the downloadable directory
            // list. Check their indexes before quarantine intake/credit as well.
            if location.is_dir() && !metadata.as_os_str().is_empty() {
                let base = self.get_filebase(location, metadata).await?;
                if base.lock().await.contains_name(name) {
                    return Ok(true);
                }
            }
        }
        if let Some(directories) = conference.directories {
            for directory in directories.iter() {
                let base = self.get_filebase(&directory.path, &directory.metadata_path).await?;
                let exists = {
                    let base = base.lock().await;
                    upload_name_exists(&base, &directory.path, name)
                };
                if exists {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// An upload throws the flag list
    /// away, so `PCBoard` asks first - and this is the very first prompt of the
    /// U command, ahead of the file name.
    pub async fn proceed_with_upload(&mut self) -> Res<bool> {
        if self.session.flagged_files.is_empty() {
            return Ok(true);
        }
        self.display_text(IceText::FilesAreFlagged, display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::BELL)
            .await?;
        let answer = self
            .input_field(
                IceText::ContinueUpload,
                1,
                "",
                "",
                Some(self.session.no_char.to_uppercase().to_string()),
                display_flags::YESNO | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE | display_flags::FIELDLEN,
            )
            .await?;
        if answer != self.session.yes_char.to_uppercase().to_string() {
            return Ok(false);
        }
        self.session.flagged_files.clear();
        Ok(true)
    }

    /// `PCBoard` has the caller describe the file before anything is transferred, and an
    /// empty first line abandons an upload that has not started. A leading `/` or `\` on the
    /// first line asks for the upload to be screened.
    pub async fn ask_upload_description(&mut self, file_name: &str) -> Res<Option<(Vec<String>, bool)>> {
        self.read_upload_description(file_name, false).await
    }

    async fn upload_description_input_available(&mut self) -> bool {
        !self.session.request_logoff
            && (self.session.is_local
                || matches!(
                    timeout(Duration::from_secs(2), self.connection.poll()).await,
                    Ok(Ok(icy_net::ConnectionState::Connected))
                ))
    }

    async fn read_upload_description(&mut self, file_name: &str, transferred: bool) -> Res<Option<(Vec<String>, bool)>> {
        let max_lines = self.get_board().await.config.file_transfer.upload_descr_lines.max(1) as usize;
        let mut private = self.session.current_conference.private_uploads;

        if !self.upload_description_input_available().await {
            return Ok(None);
        }
        self.session.op_text = file_name.to_string();
        self.display_text(IceText::EnterDescription, display_flags::NEWLINE | display_flags::LFBEFORE)
            .await?;
        self.display_text(IceText::SlashForPrivate, display_flags::NEWLINE).await?;
        self.session.op_text = max_lines.to_string();
        self.display_text(IceText::MessageEnterText, display_flags::DEFAULT).await?;
        self.display_text(IceText::Columns45, display_flags::NEWLINE).await?;

        let mut lines: Vec<String> = Vec::new();
        while lines.len() < max_lines {
            if !self.upload_description_input_available().await {
                return Ok(None);
            }
            let line = self
                .input_string(
                    IcbColor::None,
                    String::new(),
                    45,
                    &MASK_ASCII,
                    "",
                    None,
                    display_flags::NEWLINE | display_flags::FIELDLEN | display_flags::HIGHASCII,
                )
                .await?;

            if self.session.request_logoff {
                return Ok(None);
            }
            if lines.is_empty() {
                // TRANSFER.C getdescription: a blank first line only abandons
                // a size-zero (not yet received) file. Otherwise ask for more.
                if line.is_empty() && !transferred {
                    return Ok(None);
                }
                if line.chars().count() < 5 {
                    self.display_text(IceText::LongerDescription, display_flags::NEWLINE).await?;
                    continue;
                }
                if line.starts_with(['/', '\\']) {
                    private = true;
                }
            } else if line.is_empty() {
                break;
            }
            lines.push(line);
        }
        Ok(Some((lines, private)))
    }

    pub async fn upload_file(&mut self) -> Res<()> {
        self.upload_files(false).await
    }

    pub(crate) async fn upload_files(&mut self, explicit_batch: bool) -> Res<()> {
        self.transfer_statistics.uploaded_files = 0;
        self.transfer_statistics.uploaded_bytes = 0;
        self.transfer_statistics.uploaded_cps = 0;
        if self.session.request_logoff || !self.session.user_command_level.cmd_u.session_can_access(&self.session) {
            return Ok(());
        }
        if let Some(window) = self.event_window().await
            && window.uploads_blocked(&chrono::Local::now())
        {
            self.display_text(IceText::UploadsDisabled, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(());
        }
        if !self.proceed_with_upload().await? {
            return Ok(());
        }
        self.set_activity(NodeStatus::Transfer).await;
        // Isolate stacked names: description/protocol input must not consume them.
        let stacked = std::mem::take(&mut self.session.tokens);
        let local = self.session.is_local;
        let batch = (explicit_batch && self.session.user_command_level.batch_file_transfer.session_can_access(&self.session))
            || (!local && self.promotes_to_batch(!stacked.is_empty()).await);
        let file_transfer = self.get_board().await.config.file_transfer.clone();
        let limit = if file_transfer.disallow_batch_uploads { 1 } else { 32000 };
        let mut protocol_str = self.session.current_user.as_ref().map(|user| user.protocol.clone()).unwrap_or_default();
        let mut names = std::collections::VecDeque::new();
        let mut goodbye_after_upload = false;
        for token in stacked {
            if token.len() == 1
                && self
                    .get_board()
                    .await
                    .protocols
                    .iter()
                    .any(|p| p.is_enabled && p.char_code.eq_ignore_ascii_case(&token))
            {
                protocol_str = token.to_ascii_uppercase();
            } else if token.eq_ignore_ascii_case("GB") || token.eq_ignore_ascii_case("BYE") {
                goodbye_after_upload = true;
            } else if !local {
                names.push_back(token);
            }
        }
        let mut requests: Vec<UploadRequest> = Vec::new();
        // receive()/scanfornames scans ALL stacked names even for normal U.
        // Only the interactive name loop stops after one non-batch request.
        while requests.len() < limit && !self.session.request_logoff && (batch || requests.is_empty() || !names.is_empty()) {
            // No local source path is ever accepted from a terminal token.
            let selected = if local {
                let Some(path) = self.request_local_path(LocalFilePickerKind::UploadFile).await? else {
                    break;
                };
                Some(path)
            } else {
                None
            };
            let name = if let Some(path) = &selected {
                path.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_string()
            } else if let Some(name) = names.pop_front() {
                name
            } else {
                if goodbye_after_upload && !requests.is_empty() {
                    break;
                }
                let prompt = self
                    .display_text
                    .get_display_text(if batch { IceText::FileNameToUploadBatch } else { IceText::FileNameToUpload })?;
                // FNUM normally refers to download flags. Do not pollute that list
                // just to number upload prompts (or leave flags after cancellation).
                self.input_string(
                    prompt.style.to_color(),
                    prompt.text.replace("@FNUM@", &(requests.len() + 1).to_string()),
                    60,
                    &MASK_ASCII,
                    CommandType::UploadFile.get_help(),
                    None,
                    display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?
            };
            if name.is_empty() && !local {
                break;
            }
            let Some(name) = normalize_upload_name(&name) else {
                self.display_text(IceText::InvalidFileName, display_flags::NEWLINE).await?;
                if local {
                    break;
                }
                continue;
            };
            if requests.iter().any(|request| request.name.eq_ignore_ascii_case(&name)) || self.upload_name_exists_on_system(&name).await? {
                self.session.op_text = name;
                self.display_text(IceText::DuplicateFile, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                if local {
                    break;
                }
                continue;
            }
            let Some((description, private)) = self.ask_upload_description(&name).await? else {
                if local {
                    break;
                }
                continue;
            };
            if !self.upload_destination_available(private).await? {
                return Ok(());
            }
            self.display_text(IceText::UploadStatus, display_flags::DEFAULT).await?;
            self.display_text(if private { IceText::ScreenEditor } else { IceText::PostedImmediately }, display_flags::NEWLINE)
                .await?;
            // Stage once, before collecting another file, and retain the guard all
            // the way through publication/quarantine, including cancellation.
            let started = Instant::now();
            let local_source = selected.as_deref().map(stage_local_upload).transpose()?;
            let local_cps = if let Some(source) = &local_source {
                transfer_cps(source.metadata()?.len(), started) as u64
            } else {
                0
            };
            requests.push(UploadRequest {
                name,
                description,
                private,
                local_source,
                local_cps,
            });
        }
        if requests.is_empty() || self.session.request_logoff {
            return Ok(());
        }
        // Local-only bridge bypasses protocols even for a caller with no default.
        if !local {
            loop {
                while self.get_upload_protocol(&protocol_str, batch).await.is_none() {
                    let answer = self.ask_upload_protocol(batch).await?;
                    if answer.is_empty() || answer.eq_ignore_ascii_case("N") {
                        return Ok(());
                    }
                    protocol_str = answer;
                }
                if !batch || goodbye_after_upload {
                    break;
                }
                let input = self
                    .input_field(
                        IceText::GoodbyeAfterUpload,
                        1,
                        "AGP",
                        "",
                        None,
                        display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE | display_flags::FIELDLEN,
                    )
                    .await?;
                match input.as_str() {
                    "A" => return Ok(()),
                    "G" => {
                        goodbye_after_upload = true;
                        break;
                    }
                    "P" => {
                        let answer = self.ask_upload_protocol(batch).await?;
                        if answer.is_empty() || answer.eq_ignore_ascii_case("N") {
                            return Ok(());
                        }
                        protocol_str = answer;
                    }
                    "" => break,
                    _ => {}
                }
            }
        }
        // A single U request (or NoBatchUp) retains the requested name even
        // when the selected transport carries a batch filename header. An
        // enabled batch, including multiple explicitly stacked U names, may
        // append unannounced files and describe them after transfer.
        let single_file = file_transfer.disallow_batch_uploads || (!batch && requests.len() == 1);
        let receipt = if local {
            protocol_str = "Local".into();
            UploadReceipt {
                files: requests
                    .iter_mut()
                    .filter_map(|request| {
                        request.local_source.take().map(|source| CompletedUpload {
                            name: request.name.clone(),
                            source,
                            cps: request.local_cps,
                        })
                    })
                    .collect(),
                ..Default::default()
            }
        } else {
            let Some(protocol) = self.get_upload_protocol(&protocol_str, batch).await else {
                return Ok(());
            };
            self.receive_uploads(&protocol, if single_file { 1 } else { 32000 }).await?
        };
        let retain_requested_name = single_file
            || (!local
                && self.get_upload_protocol(&protocol_str, batch).await.is_some_and(|p| {
                    matches!(
                        p,
                        TransferProtocolType::XModem | TransferProtocolType::XModemCRC | TransferProtocolType::XModem1k | TransferProtocolType::XModem1kG
                    )
                }));
        self.finish_uploads(receipt, &requests, retain_requested_name, &protocol_str).await?;
        if goodbye_after_upload {
            self.goodbye().await?;
        }
        Ok(())
    }

    fn upload_destination(&self, private: bool) -> (PathBuf, PathBuf) {
        let conference = &self.session.current_conference;
        if private {
            (conference.private_upload_location.clone(), conference.private_upload_metadata.clone())
        } else {
            (conference.pub_upload_location.clone(), conference.pub_upload_metadata.clone())
        }
    }

    async fn upload_destination_available(&mut self, private: bool) -> Res<bool> {
        let (location, _) = self.upload_destination(private);
        if !location.is_dir() {
            self.display_text(
                IceText::NoDirectoriesAvailable,
                display_flags::NEWLINE | display_flags::BELL | display_flags::LFBEFORE,
            )
            .await?;
            return Ok(false);
        }
        let config = self.get_board().await.config.file_transfer.clone();
        if !config.disable_drive_size_check && !has_upload_space(&location, config.stop_uploads_free_space)? {
            self.display_text(IceText::InsufficientUploadSpace, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(false);
        }
        Ok(true)
    }

    // Keep get_protocol's send-direction contract for existing callers.
    async fn get_upload_protocol(&self, code: &str, batch: bool) -> Option<TransferProtocolType> {
        self.get_board()
            .await
            .protocols
            .iter()
            .find(|p| {
                p.is_enabled
                    && p.char_code.eq_ignore_ascii_case(code)
                    && !code.eq_ignore_ascii_case("N")
                    && p.recv_command != TransferProtocolType::None
                    && (!batch || p.is_batch)
            })
            .map(|p| p.recv_command.clone())
    }

    async fn ask_upload_protocol(&mut self, batch: bool) -> Res<String> {
        let lines: Vec<_> = self
            .get_board()
            .await
            .protocols
            .iter()
            .filter(|p| p.is_enabled && (!batch || p.is_batch || p.recv_command == TransferProtocolType::None))
            .map(|p| format!("   ({}) {}", p.char_code, p.description))
            .collect();
        self.new_line().await?;
        self.set_color(TerminalTarget::Both, IcbColor::dos_light_cyan()).await?;
        for line in lines {
            self.println(TerminalTarget::Both, &line).await?;
        }
        self.input_field(
            IceText::ProtocolForTransfer,
            1,
            &MASK_ASCII,
            "",
            Some("N".into()),
            display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE | display_flags::FIELDLEN,
        )
        .await
    }

    async fn receive_uploads(&mut self, kind: &TransferProtocolType, limit: usize) -> Res<UploadReceipt> {
        let mut receipt = UploadReceipt::default();
        // Defence in depth: no native protocol bytes may reach the local terminal.
        if self.session.is_local || limit == 0 {
            receipt.failed = true;
            return Ok(receipt);
        }
        let Some(mut protocol) = create_protocol(kind) else {
            receipt.failed = true;
            return Ok(receipt);
        };
        let mut transfer = match timeout(Duration::from_secs(30), protocol.initiate_recv(&mut *self.connection)).await {
            Ok(Ok(state)) => UploadReceiveGuard(state),
            _ => {
                let _ = timeout(Duration::from_secs(2), protocol.cancel_transfer(&mut *self.connection)).await;
                receipt.failed = true;
                return Ok(receipt);
            }
        };
        let started = Instant::now();
        let mut file_started = started;
        loop {
            // No awaits between drain and adopting all completed temporary paths.
            for (name, path) in transfer.0.recieve_state.finished_files.drain(..) {
                match TempPath::try_from_path(&path) {
                    Ok(source) => {
                        let bytes = source.metadata().map(|m| m.len()).unwrap_or(0);
                        receipt.files.push(CompletedUpload {
                            name,
                            source,
                            cps: transfer_cps(bytes, file_started) as u64,
                        });
                        file_started = Instant::now();
                    }
                    Err(_) => {
                        let _ = std::fs::remove_file(path);
                        receipt.failed = true;
                    }
                }
            }
            receipt.errors = transfer.0.recieve_state.errors;
            receipt.failed |= transfer.0.request_cancel
                || self.session.request_logoff
                || crate::icy_board::limits::session_expired(self.session.time_limit, (Utc::now() - self.session.login_date).num_minutes())
                || started.elapsed() > Duration::from_secs(3600);
            if receipt.failed || transfer.0.is_finished {
                break;
            }
            if receipt.files.len() >= limit {
                // Native API has no receive-count setter. Stop at the completion
                // boundary, before polling a second header/payload (NoBatchUp).
                let _ = timeout(Duration::from_secs(2), protocol.cancel_transfer(&mut *self.connection)).await;
                break;
            }
            let previous_errors = transfer.0.recieve_state.errors;
            match timeout(Duration::from_secs(30), protocol.update_transfer(&mut *self.connection, &mut transfer.0)).await {
                Ok(Ok(())) => {
                    // Native Zmodem CAN/ZABORT/retry exhaustion can finish with
                    // Ok(()) plus a new logged error, rather than returning Err.
                    // Earlier recovered CRC errors alone do not fail the session.
                    receipt.failed |= transfer.0.is_finished && transfer.0.recieve_state.errors > previous_errors;
                }
                result => {
                    log::warn!("Upload receive failed: {result:?}");
                    receipt.failed = true;
                }
            }
        }
        if receipt.failed {
            let _ = timeout(Duration::from_secs(2), protocol.cancel_transfer(&mut *self.connection)).await;
        }
        Ok(receipt)
    }

    async fn finish_uploads(&mut self, receipt: UploadReceipt, requests: &[UploadRequest], retain_requested_name: bool, protocol: &str) -> Res<()> {
        let mut accepted = Vec::new();
        let mut failed = receipt.failed;
        let mut cps = 0;
        let mut pending = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        let mut can_describe = !receipt.failed && self.upload_description_input_available().await;
        // TRANSFER.C scanforuploads: describe unannounced files FIRST, before
        // processing/publication of any completed file. Never borrow another
        // file's description or privacy flag.
        for (index, completed) in receipt.files.into_iter().enumerate() {
            if retain_requested_name && index != 0 {
                failed = true;
                continue;
            }
            let name = if retain_requested_name && index == 0 {
                requests.first().and_then(|r| normalize_upload_name(&r.name))
            } else {
                normalize_upload_name(&completed.name)
            };
            let Some(name) = name else {
                failed = true;
                continue;
            };
            if completed.source.metadata()?.len() == 0 {
                failed = true;
                continue;
            }
            if seen.iter().any(|n| n.eq_ignore_ascii_case(&name)) || self.upload_name_exists_on_system(&name).await? {
                self.session.op_text = name;
                self.display_text(IceText::DuplicateFile, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                failed = true;
                continue;
            }
            seen.push(name.clone());
            let request = if let Some(request) = requests.iter().find(|r| r.name.eq_ignore_ascii_case(&name)) {
                UploadRequest {
                    name: request.name.clone(),
                    description: request.description.clone(),
                    private: request.private
                        || self.session.current_conference.private_uploads
                        || request.description.first().is_some_and(|line| line.starts_with(['/', '\\'])),
                    local_source: None,
                    local_cps: 0,
                }
            } else {
                // Deliberate safety exception to PCBoard's carrier-loss text:
                // if description entry is unavailable, skip the undescribed
                // payload (TempPath removes it), with no publication or credit.
                // Do not wait on a failed transfer, logoff, or lost carrier.
                can_describe = can_describe && self.upload_description_input_available().await;
                if !can_describe {
                    log::warn!("Skipping unannounced upload {name:?}: description entry unavailable");
                    failed = true;
                    continue;
                }
                // Native temporaries have no archive extension. Scan a guarded,
                // correctly named copy so scan_file can discover FILE_ID.DIZ.
                let description = match scan_upload_description(&completed.source, &name) {
                    Ok(description) => description,
                    Err(error) => {
                        log::warn!("Unable to scan upload {name:?} for a description: {error}");
                        None
                    }
                };
                let described = if let Some(description) = description {
                    let private = self.session.current_conference.private_uploads || description.first().is_some_and(|line| line.starts_with(['/', '\\']));
                    Some((description, private))
                } else {
                    // Bounded even for connections whose poll cannot detect EOF.
                    match timeout(Duration::from_secs(120), self.read_upload_description(&name, true)).await {
                        Ok(Ok(description)) => description,
                        result => {
                            log::warn!("Upload description interrupted for {name:?}: {result:?}");
                            None
                        }
                    }
                };
                let Some((description, private)) = described else {
                    can_describe = false;
                    failed = true;
                    continue;
                };
                UploadRequest {
                    name,
                    description,
                    private,
                    local_source: None,
                    local_cps: 0,
                }
            };
            pending.push((completed, request));
        }
        for (completed, request) in pending {
            let name = request.name.clone();
            let bytes = completed.source.metadata()?.len();
            if !self.upload_destination_available(request.private).await? {
                failed = true;
                continue;
            }
            if accepted.iter().any(|n: &String| n.eq_ignore_ascii_case(&name)) || self.upload_name_exists_on_system(&name).await? {
                self.session.op_text = name;
                self.display_text(IceText::DuplicateFile, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                failed = true;
                continue;
            }
            match self.process_completed_upload(&completed.source, &request).await {
                Ok(true) => {
                    self.credit_completed_upload(&request.name, bytes, completed.cps).await?;
                    cps = completed.cps;
                    accepted.push(request.name.clone());
                }
                Ok(false) => failed = true,
                Err(error) => {
                    log::warn!("Upload '{}' was not accepted: {error}", request.name);
                    failed = true;
                }
            }
        }
        self.log_transfer(true, &accepted, protocol, receipt.errors, cps.min(usize::MAX as u64) as usize)
            .await?;
        self.display_text(
            if failed || accepted.is_empty() {
                IceText::TransferAborted
            } else {
                IceText::TransferSuccessful
            },
            display_flags::NEWLINE | display_flags::LFBEFORE,
        )
        .await?;
        if !accepted.is_empty() {
            self.display_text(IceText::ThanksForTheFiles, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
        }
        Ok(())
    }

    async fn process_completed_upload(&mut self, source: &Path, request: &UploadRequest) -> Res<bool> {
        let (location, metadata) = self.upload_destination(request.private);
        let config = self.get_board().await.config.upload_processing.clone();
        let uploader = self.session.get_username_or_alias().to_string();
        let status;
        let accepted;
        if config.publish_policy == UploadPublishPolicy::Immediate {
            accepted = self
                .publish_uploaded_file(source, &request.name, &location, &metadata, &request.description)
                .await?;
            status = "published".to_string();
        } else {
            self.display_text(IceText::BeginUploadTest, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            let quarantine = UploadQuarantine::new(config.quarantine_path.clone());
            let record = quarantine.enqueue(
                source,
                request.name.clone(),
                location.clone(),
                metadata,
                uploader.clone(),
                request.description.clone(),
            )?;
            let mut processed = UploadProcessor::new(config.clone()).process(&record.id).await?;
            if processed.status == QuarantineStatus::ReadyToPublish {
                match publish_quarantine_record(&quarantine, &processed.id, "system", "published after processing") {
                    Ok(published) => {
                        self.file_bases.remove(&published.destination);
                        processed = published;
                    }
                    Err(error) => {
                        log::warn!("Unable to publish upload '{}': {error}", processed.original_name);
                        processed = quarantine.load(&processed.id)?;
                    }
                }
            }
            // A failed scan/publication is retained for review, not credited as
            // accepted. Manual approval is a successful intake; later administrative
            // rejection/reversal is outside this synchronous receive command.
            accepted = matches!(processed.status, QuarantineStatus::Published | QuarantineStatus::AwaitingApproval)
                && (processed.status == QuarantineStatus::Published || !lookup_case_insensitive(&location.join(&processed.original_name)).exists());
            status = format!("{:?}", processed.status);
        }
        if config.notify_sysop
            && (accepted || config.publish_policy != UploadPublishPolicy::Immediate)
            && let Err(error) = self.notify_sysop_about_upload(&request.name, &uploader, &status, &request.description).await
        {
            log::warn!("Unable to notify the SysOp about upload '{}': {error}", request.name);
        }
        Ok(accepted)
    }

    /// Accepted file or attachment intake, not text uploaded into an editor.
    pub(crate) fn accounting_record_upload(&mut self, name: &str, bytes: u64) -> Res<()> {
        let rates = self.accounting_rates();
        self.accounting_record(14, "UPLD FILE", name, rates.pay_back_for_upload_file, 1)?;
        self.accounting_record(15, "UPLD BYTES", name, rates.pay_back_for_upload_bytes, (bytes / 1024) as i64)?;
        Ok(())
    }

    async fn credit_completed_upload(&mut self, name: &str, bytes: u64, cps: u64) -> Res<()> {
        // Accepted intake only. Credit fields take positive paybacks; the
        // separate byte/time ratio allowances below are not monetary credits.
        self.accounting_record_upload(name, bytes)?;
        let config = self.get_board().await.configuration_snapshot();
        let (credit, seconds) = upload_credits(bytes, cps, config.file_transfer.upload_credit_bytes, config.file_transfer.upload_credit_time);
        let credit = credit.min(i64::MAX as u64) as i64;
        if let Some(user) = &mut self.session.current_user {
            user.stats.num_uploads = user.stats.num_uploads.saturating_add(1);
            user.stats.today_num_uploads = user.stats.today_num_uploads.saturating_add(1);
            user.stats.total_upld_bytes = user.stats.total_upld_bytes.saturating_add(bytes);
            user.stats.today_upld_bytes = user.stats.today_upld_bytes.saturating_add(bytes);
            user.stats.today_dnld_bytes = user.stats.today_dnld_bytes.saturating_sub(credit);
        }
        crate::icy_board::limits::adjust_bytes_remaining(&mut self.session.bytes_remaining, -credit);
        // Session clocks currently store minutes. Carry fractional minutes between
        // uploads without falsifying login_date or overriding an event adjustment.
        if self.session.time_limit != 0 && !self.session.time_adjusted_for_event {
            let seconds = self.session.upload_credit_seconds.saturating_add(seconds);
            self.session.time_limit = self.session.time_limit.saturating_add((seconds / 60).min(i32::MAX as u64) as i32);
            self.session.upload_credit_seconds = seconds % 60;
        }
        self.transfer_statistics.uploaded_files = self.transfer_statistics.uploaded_files.saturating_add(1);
        self.transfer_statistics.uploaded_bytes = self.transfer_statistics.uploaded_bytes.saturating_add(bytes.min(usize::MAX as u64) as usize);
        self.transfer_statistics.uploaded_cps = cps.min(usize::MAX as u64) as usize;
        IcyBoard::write_statistics(&self.board, move |statistics| statistics.add_upload_totals(1, bytes)).await?;
        self.accounting_check_balance().await?;
        Ok(())
    }

    pub async fn get_protocol(&mut self, protocol_str: String) -> Option<TransferProtocolType> {
        let mut protocol = None;
        for p in self.get_board().await.protocols.iter() {
            if p.is_enabled && p.char_code == protocol_str {
                protocol = Some(p.send_command.clone());
                break;
            }
        }
        protocol
    }

    pub async fn is_batch_protocol(&mut self, protocol_str: &str) -> bool {
        self.get_board()
            .await
            .protocols
            .iter()
            .any(|p| p.is_enabled && p.is_batch && p.char_code == protocol_str)
    }

    /// `PCBoard` promotes a transfer to a batch only when nothing was stacked on the
    /// command line, the caller's protocol is a batch one and the sysop allows it.
    pub async fn promotes_to_batch(&mut self, had_token: bool) -> bool {
        if had_token || !self.get_board().await.config.file_transfer.promote_to_batch_transfers {
            return false;
        }
        if !self.session.user_command_level.batch_file_transfer.session_can_access(&self.session) {
            return false;
        }
        let protocol = self.session.current_user.as_ref().map(|user| user.protocol.clone()).unwrap_or_default();
        if protocol.eq_ignore_ascii_case("N") {
            return false;
        }
        self.is_batch_protocol(&protocol).await
    }
}

pub fn create_protocol(protocol: &TransferProtocolType) -> Option<Box<dyn Protocol>> {
    match protocol {
        // No native handler runs a DOS external protocol, and ASCII/None have no
        // framing to drive, so the caller aborts rather than claim a transfer.
        TransferProtocolType::None | TransferProtocolType::ASCII | TransferProtocolType::External(_) => None,
        TransferProtocolType::XModem => Some(Box::new(XYmodem::new(XYModemVariant::XModem))),
        TransferProtocolType::XModemCRC => Some(Box::new(XYmodem::new(XYModemVariant::XModemCRC))),
        TransferProtocolType::XModem1k => Some(Box::new(XYmodem::new(XYModemVariant::XModem1k))),
        TransferProtocolType::XModem1kG => Some(Box::new(XYmodem::new(XYModemVariant::XModem1kG))),
        TransferProtocolType::YModem | TransferProtocolType::YModemG => Some(Box::new(XYmodem::new(XYModemVariant::YModem))),
        TransferProtocolType::ZModem => Some(Box::new(Zmodem::new(1024))),
        TransferProtocolType::ZModem8k => Some(Box::new(Zmodem::new(8 * 1024))),
    }
}

#[cfg(test)]
#[path = "upload_tests.rs"]
mod upload_tests;

#[cfg(test)]
mod option_tests {
    use bstr::ByteSlice;
    use jamjam::jam::attributes;
    use tempfile::TempDir;

    use super::{FileBase, enough_upload_space, upload_name_exists, upload_notification};

    #[test]
    fn upload_space_limit_is_in_kib_and_zero_disables_it() {
        assert!(!enough_upload_space(1023, 1));
        assert!(enough_upload_space(1024, 1));
        assert!(enough_upload_space(0, 0));
    }

    #[test]
    fn upload_duplicate_checks_ignore_ascii_case_in_the_index_and_on_disk() {
        let indexed_dir = TempDir::new().unwrap();
        let indexed_file = indexed_dir.path().join("Existing.ZIP");
        std::fs::write(&indexed_file, b"old").unwrap();
        let indexed = FileBase::open(indexed_dir.path(), indexed_dir.path().join("dir")).unwrap();
        std::fs::remove_file(indexed_file).unwrap();
        assert!(upload_name_exists(&indexed, indexed_dir.path(), "EXISTING.ZIP"));

        let disk_dir = TempDir::new().unwrap();
        let unindexed = FileBase::open(disk_dir.path(), disk_dir.path().join("dir")).unwrap();
        std::fs::write(disk_dir.path().join("OnDisk.ZIP"), b"old").unwrap();
        assert!(upload_name_exists(&unindexed, disk_dir.path(), "ondisk.zip"));
    }

    #[test]
    fn upload_notification_is_private_local_mail_with_sanitized_text() {
        let message = upload_notification(
            "SYSOP",
            "BAD\rNAME.ZIP",
            "ALICE",
            "AwaitingApproval",
            &["First line".to_string(), "Second\u{1b} line".to_string()],
        );
        assert_eq!(Some("SYSOP"), message.to().map(|value| value.to_str_lossy()).as_deref());
        assert_eq!(
            Some("New upload: BADNAME.ZIP"),
            message.header().subject().map(|value| value.to_str_lossy()).as_deref()
        );
        assert_ne!(0, message.header().attributes & attributes::MSG_LOCAL);
        assert_ne!(0, message.header().attributes & attributes::MSG_PRIVATE);
        let text = message.text().to_str_lossy();
        assert!(text.contains("Uploader: ALICE"));
        assert!(text.contains("First line\nSecond line"));
        assert!(!text.contains('\u{1b}'));
    }
}
