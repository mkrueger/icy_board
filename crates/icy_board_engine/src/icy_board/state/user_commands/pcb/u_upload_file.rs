use crate::icy_board::commands::CommandType;
use crate::icy_board::icb_config::{IcbColor, UploadPublishPolicy};
use crate::icy_board::lookup_case_insensitive;
use crate::icy_board::upload_processor::UploadProcessor;
use crate::icy_board::upload_publish::publish_quarantine_record;
use crate::icy_board::upload_quarantine::{QuarantineStatus, UploadQuarantine};
use crate::{Res, icy_board::state::IcyBoardState};
use crate::{
    icy_board::{
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
use icy_net::protocol::{Protocol, TransferProtocolType, XYModemVariant, XYmodem, Zmodem};
use jamjam::jam::{JamMessage, attributes};
use std::path::{Path, PathBuf};
use std::time::Instant;

fn has_upload_space(path: &std::path::Path, minimum_kib: u32) -> std::io::Result<bool> {
    Ok(enough_upload_space(available_space(path)?, minimum_kib))
}

fn enough_upload_space(available_bytes: u64, minimum_kib: u32) -> bool {
    minimum_kib == 0 || available_bytes / 1024 >= u64::from(minimum_kib)
}

fn upload_name_exists(base: &FileBase, location: &std::path::Path, name: &str) -> bool {
    base.contains_name(name) || lookup_case_insensitive(&location.join(name)).exists()
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
        let dest = upload_location.join(name);
        let file_base = self.get_filebase(upload_location, upload_metadata).await?;
        let duplicate = {
            let base = file_base.lock().await;
            upload_name_exists(&base, upload_location, name)
        };
        if duplicate {
            self.session.op_text = name.to_string();
            self.display_text(IceText::DuplicateFile, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(false);
        }

        std::fs::copy(source, &dest)?;
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
        if let Err(error) = file_base.lock().await.add_file(&dest, metadata) {
            let _ = std::fs::remove_file(&dest);
            return Err(error);
        }
        std::fs::remove_file(source)?;
        Ok(true)
    }

    async fn upload_name_exists_on_system(&mut self, name: &str) -> Res<bool> {
        let conference = self.session.current_conference.clone();
        for location in [&conference.pub_upload_location, &conference.private_upload_location] {
            if lookup_case_insensitive(&location.join(name)).exists() {
                return Ok(true);
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
    /// empty first line abandons an upload that has not started. A leading `/` on the
    /// first line asks for the upload to be screened.
    pub async fn ask_upload_description(&mut self, file_name: &str) -> Res<Option<(Vec<String>, bool)>> {
        let max_lines = self.get_board().await.config.file_transfer.upload_descr_lines.max(1) as usize;
        let mut private = self.session.current_conference.private_uploads;

        self.session.op_text = file_name.to_string();
        self.display_text(IceText::EnterDescription, display_flags::NEWLINE | display_flags::LFBEFORE)
            .await?;
        self.display_text(IceText::SlashForPrivate, display_flags::NEWLINE).await?;
        self.session.op_text = max_lines.to_string();
        self.display_text(IceText::MessageEnterText, display_flags::DEFAULT).await?;
        self.display_text(IceText::Columns45, display_flags::NEWLINE).await?;

        let mut lines: Vec<String> = Vec::new();
        while lines.len() < max_lines {
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

            if lines.is_empty() {
                if line.is_empty() {
                    return Ok(None);
                }
                if line.chars().count() < 5 {
                    self.display_text(IceText::LongerDescription, display_flags::NEWLINE).await?;
                    continue;
                }
                if line.starts_with('/') {
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
        let upload_location = self.session.current_conference.pub_upload_location.clone();
        let upload_metadata = self.session.current_conference.pub_upload_metadata.clone();
        if !upload_location.exists() {
            self.display_text(
                IceText::NoDirectoriesAvailable,
                display_flags::NEWLINE | display_flags::BELL | display_flags::LFBEFORE,
            )
            .await?;
            return Ok(());
        }

        // PCBoard asks for a name until the answer is empty or a file is accepted, so an
        // abandoned description comes back here rather than ending the command.
        let (description, private_upload, had_token) = loop {
            let stacked_name = self.session.tokens.pop_front();
            let had_token = stacked_name.is_some();
            let file_name = if let Some(token) = stacked_name {
                token
            } else {
                self.input_field(
                    IceText::FileNameToUpload,
                    60,
                    &MASK_ASCII,
                    CommandType::UploadFile.get_help(),
                    None,
                    display_flags::NEWLINE | display_flags::LFBEFORE,
                )
                .await?
            };

            if file_name.is_empty() {
                return Ok(());
            }

            if self.upload_name_exists_on_system(&file_name).await? {
                self.session.op_text = file_name;
                self.display_text(IceText::DuplicateFile, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                continue;
            }

            if let Some((description, private_upload)) = self.ask_upload_description(&file_name).await? {
                break (description, private_upload, had_token);
            }
        };

        // PCBoard receives into the private location and moves the file to the public
        // one afterwards unless it is to be screened.
        let (upload_location, upload_metadata) = if private_upload {
            (
                self.session.current_conference.private_upload_location.clone(),
                self.session.current_conference.private_upload_metadata.clone(),
            )
        } else {
            (upload_location, upload_metadata)
        };
        if !upload_location.exists() {
            self.display_text(
                IceText::NoDirectoriesAvailable,
                display_flags::NEWLINE | display_flags::BELL | display_flags::LFBEFORE,
            )
            .await?;
            return Ok(());
        }

        let file_transfer = self.get_board().await.config.file_transfer.clone();
        if !file_transfer.disable_drive_size_check && !has_upload_space(&upload_location, file_transfer.stop_uploads_free_space)? {
            self.display_text(IceText::InsufficientUploadSpace, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(());
        }

        self.display_text(IceText::UploadStatus, display_flags::DEFAULT).await?;
        self.display_text(
            if private_upload { IceText::ScreenEditor } else { IceText::PostedImmediately },
            display_flags::NEWLINE,
        )
        .await?;

        // PCBoard settles the protocol before it offers the goodbye question, and
        // asks for one rather than starting a transfer the caller has none for.
        let mut protocol_str: String = self.session.current_user.as_ref().unwrap().protocol.clone();
        loop {
            if self.get_protocol(protocol_str.clone()).await.is_some() {
                break;
            }
            let answer = self.ask_transfer_protocol(&protocol_str).await?;
            if answer.is_empty() || answer.eq_ignore_ascii_case("N") {
                return Ok(());
            }
            protocol_str = answer;
        }

        // PCBoard only offers the goodbye question in a batch upload.
        let batch_upload = !had_token
            && self.get_board().await.config.file_transfer.promote_to_batch_transfers
            && self.session.user_command_level.batch_file_transfer.session_can_access(&self.session)
            && self.is_batch_protocol(&protocol_str).await;

        let mut goodbye_after_upload = false;

        if batch_upload {
            loop {
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
                    "A" => {
                        return Ok(());
                    }
                    "G" => {
                        goodbye_after_upload = true;
                        break;
                    }
                    "P" => {
                        self.set_transfer_protocol().await?;
                        protocol_str = self.session.current_user.as_ref().unwrap().protocol.clone();
                    }
                    "" => {
                        break;
                    }
                    _ => {}
                }
            }
        }

        let protocol = self.get_protocol(protocol_str.clone()).await;

        if let Some(protocol) = protocol {
            let Some(mut prot) = create_protocol(&protocol) else {
                self.display_text(IceText::TransferAborted, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                return Ok(());
            };

            match prot.initiate_recv(&mut *self.connection).await {
                Ok(mut state) => {
                    let started = Instant::now();
                    while !state.is_finished {
                        if let Err(e) = prot.update_transfer(&mut *self.connection, &mut state).await {
                            log::error!("Error while updating file transfer with {protocol:?} : {e}");
                            self.display_text(IceText::TransferAborted, display_flags::NEWLINE).await?;
                            break;
                        }
                    }
                    self.display_text(IceText::TransferSuccessful, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                    self.display_text(IceText::ThanksForTheFiles, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;

                    let received: Vec<String> = state.recieve_state.finished_files.iter().map(|(name, _)| name.clone()).collect();
                    let cps = transfer_cps(state.recieve_state.total_bytes_transfered, started);
                    self.log_transfer(true, &received, &protocol_str, state.recieve_state.errors, cps).await?;

                    let bytes = state.recieve_state.total_bytes_transfered;
                    let credit_rate = self.get_board().await.config.file_transfer.upload_credit_bytes as u64;
                    let credit = bytes.saturating_mul(credit_rate) / 10;
                    if let Some(user) = &mut self.session.current_user {
                        user.stats.num_uploads = user.stats.num_uploads.saturating_add(received.len() as u64);
                        user.stats.today_num_uploads = user.stats.today_num_uploads.saturating_add(received.len() as u64);
                        user.stats.total_upld_bytes = user.stats.total_upld_bytes.saturating_add(bytes);
                        user.stats.today_upld_bytes = user.stats.today_upld_bytes.saturating_add(bytes);
                        user.stats.today_dnld_bytes = user.stats.today_dnld_bytes.saturating_sub(credit.min(i64::MAX as u64) as i64);
                    }
                    crate::icy_board::limits::adjust_bytes_remaining(&mut self.session.bytes_remaining, -(credit.min(i64::MAX as u64) as i64));
                    self.board.lock().await.statistics.add_upload(&state);
                    self.board.lock().await.save_statistics()?;

                    let upload_processing = self.get_board().await.config.upload_processing.clone();

                    for (x, path) in state.recieve_state.finished_files {
                        if upload_processing.publish_policy == UploadPublishPolicy::Immediate {
                            if self.publish_uploaded_file(&path, &x, &upload_location, &upload_metadata, &description).await? {
                                if upload_processing.notify_sysop
                                    && let Err(error) = self
                                        .notify_sysop_about_upload(&x, &self.session.get_username_or_alias().to_string(), "published", &description)
                                        .await
                                {
                                    log::error!("Unable to notify the SysOp about upload '{x}': {error}");
                                }
                            } else {
                                std::fs::remove_file(&path)?;
                            }
                            continue;
                        }

                        self.display_text(IceText::BeginUploadTest, display_flags::NEWLINE | display_flags::LFBEFORE)
                            .await?;
                        let quarantine = UploadQuarantine::new(upload_processing.quarantine_path.clone());
                        let record = quarantine.enqueue(
                            &path,
                            x,
                            upload_location.clone(),
                            upload_metadata.clone(),
                            self.session.get_username_or_alias().to_string(),
                            description.clone(),
                        )?;
                        let mut processed = UploadProcessor::new(upload_processing.clone()).process(&record.id).await?;
                        log::info!(
                            "Upload '{}' entered quarantine as {} with status {:?}",
                            processed.original_name,
                            processed.id,
                            processed.status
                        );
                        if processed.status == QuarantineStatus::ReadyToPublish {
                            match publish_quarantine_record(&quarantine, &processed.id, "system", "published after processing") {
                                Ok(published) => {
                                    self.file_bases.remove(&published.destination);
                                    processed = published;
                                }
                                Err(error) => {
                                    log::warn!("Unable to publish upload '{}': {error}", processed.original_name);
                                    // Another actor may have rejected/claimed it. Never
                                    // overwrite that decision with a stale local status.
                                    processed = quarantine.load(&processed.id)?;
                                }
                            }
                        }
                        if upload_processing.notify_sysop
                            && let Err(error) = self
                                .notify_sysop_about_upload(
                                    &processed.original_name,
                                    &processed.uploader,
                                    &format!("{:?}", processed.status),
                                    &processed.description,
                                )
                                .await
                        {
                            log::error!("Unable to notify the SysOp about upload '{}': {error}", processed.original_name);
                        }
                    }
                }
                Err(e) => {
                    log::error!("Error while initiating file transfer with {protocol:?} : {e}");
                    self.println(TerminalTarget::Both, &format!("Error: {e}")).await?;
                }
            }
        }
        if goodbye_after_upload {
            self.goodbye().await?;
        }
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
