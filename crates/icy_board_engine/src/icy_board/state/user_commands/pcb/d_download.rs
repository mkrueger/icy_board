use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use async_recursion::async_recursion;
use humanize_bytes::humanize_bytes_decimal;

use crate::icy_board::icb_config::IcbColor;
use crate::icy_board::limits::{self, BatchSoFar, LimitVerdict, TransferHistory};
use crate::icy_board::state::functions::{MASK_ASCII, MASK_NUM, transfer_cps};
use crate::{Res, icy_board::state::IcyBoardState};

use super::u_upload_file::create_protocol;
use crate::{
    icy_board::{IcyBoard, icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};

impl IcyBoardState {
    /// Both the directory exemption and the file header's FREE flag apply.
    /// There is no persisted NOTIME or FSEC multiplier in the current schema.
    /// TRANSFER.C's FILETIME CREDIT additionally needs successful per-file CPS:
    /// finished_files only retains names/paths and resets its timing at finish.
    /// Batch CPS includes partial files, so it cannot safely fund that rebate.
    pub(crate) async fn accounting_download_free(&mut self, path: &Path) -> Res<bool> {
        let mut directory = self
            .session
            .current_conference
            .directories
            .as_ref()
            .and_then(|directories| directories.iter().find(|area| Some(area.path.as_path()) == path.parent()))
            .cloned();
        if directory.is_none() {
            // A caller can queue files, then join another conference.
            directory = self
                .get_board()
                .await
                .conferences
                .iter()
                .filter_map(|conference| conference.directories.as_ref())
                .flat_map(|directories| directories.iter())
                .find(|area| Some(area.path.as_path()) == path.parent())
                .cloned();
        }
        let Some(directory) = directory else {
            return Ok(false);
        };
        if directory.is_free {
            return Ok(true);
        }
        let files = self.get_filebase(&directory.path, &directory.metadata_path).await?;
        let files = files.lock().await;
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        Ok(files.iter().any(|file| file.name().eq_ignore_ascii_case(&name) && file.is_free()))
    }

    /// TRANSFER.C estimates fractional KiB and normal-rate time, including
    /// earlier queued files. A free file still consumes online time.
    pub(crate) async fn accounting_download_estimate(&mut self, path: &Path, bytes: u64) -> Res<f64> {
        if !self.accounting_active() {
            return Ok(0.0);
        }
        let free = self.accounting_download_free(path).await?;
        let seconds = limits::seconds_for_transfer(bytes, self.get_bps().max(0) as u32);
        Ok(download_estimate(&self.accounting_rates(), bytes, seconds, free))
    }

    pub(crate) async fn accounting_queued_download_cost(&mut self, except: &Path) -> Res<f64> {
        let mut reserved = 0.0;
        let mut seen = std::collections::HashSet::new();
        for path in self.session.flagged_files.clone() {
            if path == except || !seen.insert(path.clone()) {
                continue;
            }
            if let Ok(metadata) = path.metadata() {
                reserved += self.accounting_download_estimate(&path, metadata.len()).await?;
            }
        }
        Ok(reserved)
    }

    /// TRANSFER.C's generated MSGCAP/QWKCAP packets are NOCOST/FreeFile.
    /// They still cost online time; StopClockOnCap is not in the live schema.
    pub(crate) fn accounting_capture_transfer_estimate(&self, bytes: u64) -> f64 {
        if !self.accounting_active() {
            return 0.0;
        }
        let seconds = limits::seconds_for_transfer(bytes, self.get_bps().max(0) as u32);
        download_estimate(&self.accounting_rates(), bytes, seconds, true)
    }

    /// Called once per confirmed file, never for partial protocol wire bytes.
    /// Legacy byte rates use whole KiB PER FILE, not a rounded batch total.
    pub(crate) fn accounting_record_download(&mut self, name: &str, bytes: u64, free: bool) -> Res<()> {
        if !free {
            let rates = self.accounting_rates();
            self.accounting_record(9, "DNLD FILE", name, rates.charge_per_download_file, 1)?;
            self.accounting_record(10, "DNLD BYTES", name, rates.charge_per_download_bytes, (bytes / 1024) as i64)?;
        }
        Ok(())
    }

    pub async fn download(&mut self, ask_flagged_files: bool) -> Res<()> {
        self.download_files(ask_flagged_files, false).await
    }

    pub(crate) async fn download_files(&mut self, ask_flagged_files: bool, explicit_batch: bool) -> Res<()> {
        self.transfer_statistics.downloaded_bytes = 0;
        self.transfer_statistics.downloaded_files = 0;
        let mut protocol_str = self.session.current_user.as_ref().map(|user| user.protocol.clone()).unwrap_or_default();
        let mut goodbye_after_dl = false;
        if ask_flagged_files {
            // TRANSFER.C scans command-line names separately from prompt answers.
            // In particular, DownloadTagged must not consume the first filename.
            let stacked = std::mem::take(&mut self.session.tokens);
            let mut batch = (explicit_batch && self.session.user_command_level.batch_file_transfer.session_can_access(&self.session))
                || self.promotes_to_batch(!stacked.is_empty()).await;
            if !self.session.flagged_files.is_empty() {
                let download_tagged = self
                    .input_field(
                        IceText::DownloadTagged,
                        1,
                        "",
                        "",
                        Some(self.session.yes_char.to_string()),
                        display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::YESNO | display_flags::FIELDLEN,
                    )
                    .await?;

                if download_tagged == self.session.no_char.to_uppercase().to_string() {
                    self.session.flagged_files.clear();
                }
            }

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
                    goodbye_after_dl = true;
                } else if !token.is_empty() {
                    self.session.tokens.push_back(token);
                    self.flag_files_cmd(true).await?;
                }
            }
            batch |= self.session.flagged_files.len() > 1;
            while self.session.flagged_files.is_empty() || (batch && !goodbye_after_dl && self.session.flagged_files.len() < self.session.batch_limit) {
                if !self.download_names(&mut goodbye_after_dl, batch).await? {
                    break;
                }
                batch |= self.session.flagged_files.len() > 1;
            }

            if self.session.flagged_files.is_empty() {
                return Ok(());
            }
        } else {
            self.new_line().await?;
        }

        if self.session.flagged_files.is_empty() {
            return Ok(());
        }
        if self.session.is_local {
            let files = self.screen_transfer_limits(self.session.flagged_files.clone()).await?;
            self.session.flagged_files.clone_from(&files);
            if files.is_empty() {
                return Ok(());
            }
            let offered: Vec<_> = files.iter().map(|path| (path.clone(), path.metadata().map_or(0, |m| m.len()))).collect();
            let started = Instant::now();
            if let Some(state) = self.local_download_files(&files).await? {
                let cps = transfer_cps(state.send_state.total_bytes_transfered, started);
                self.finish_download_batch(&offered, &state, "Local", cps).await?;
                if state.send_state.errors > 0 {
                    self.display_text(IceText::TransferAborted, display_flags::NEWLINE).await?;
                    self.println(
                        TerminalTarget::Both,
                        "Some files were not copied (existing destination or unreadable source). Unsent files remain flagged.",
                    )
                    .await?;
                }
                if goodbye_after_dl {
                    self.goodbye().await?;
                }
            }
            return Ok(());
        }
        let mut protocol;
        let mut p_descr;

        let mut do_dl = true;
        loop {
            protocol = None;
            p_descr = "None".to_string();
            for p in self.get_board().await.protocols.iter() {
                if p.is_enabled
                    && p.char_code.eq_ignore_ascii_case(&protocol_str)
                    && !protocol_str.eq_ignore_ascii_case("N")
                    && (self.session.flagged_files.len() <= 1 || p.is_batch)
                {
                    p_descr = p.description.clone();
                    protocol = Some(p.send_command.clone());
                    break;
                }
            }

            // PCBoard asks which protocol to use instead of starting a transfer
            // the caller has no protocol for.
            if protocol.is_none() {
                // getxferprotocol resets an invalid/non-batch selection to N;
                // accepting the default must abort, not loop on that selection.
                let answer = self.ask_transfer_protocol("N").await?;
                if answer.is_empty() || answer.eq_ignore_ascii_case("N") {
                    return Ok(());
                }
                protocol_str = answer;
                continue;
            }

            if goodbye_after_dl {
                break;
            }

            let mut total_size = 0;
            for path in &self.session.flagged_files {
                if let Ok(data) = path.metadata() {
                    total_size += data.len();
                }
            }
            self.display_text(IceText::BatchDownloadSize, display_flags::DEFAULT).await?;
            self.set_color(TerminalTarget::Both, IcbColor::dos_light_green()).await?;
            self.println(TerminalTarget::Both, &format!(" {}", humanize_bytes_decimal!(total_size))).await?;

            self.display_text(IceText::BatchProtocol, display_flags::DEFAULT).await?;
            self.set_color(TerminalTarget::Both, IcbColor::dos_light_cyan()).await?;
            self.println(TerminalTarget::Both, &p_descr).await?;
            self.display_text(IceText::ReadyToSendBatch, display_flags::NEWLINE | display_flags::LFAFTER)
                .await?;

            let input = self
                .input_field(
                    IceText::GoodbyeAfterDownload,
                    1,
                    DL_LISTMASK,
                    "",
                    None,
                    display_flags::NEWLINE | display_flags::UPCASE | display_flags::FIELDLEN,
                )
                .await?;

            match input.as_str() {
                "A" => {
                    do_dl = false;
                    break;
                }
                "E" => {
                    self.edit_dl_batch().await?;
                    if self.session.flagged_files.is_empty() {
                        return Ok(());
                    }
                }
                "G" => {
                    goodbye_after_dl = true;
                    break;
                }
                "L" => {
                    self.list_dl_batch().await?;
                }
                "P" => {
                    let protocol = self.ask_protocols(&protocol_str).await?;

                    if !protocol.is_empty() {
                        protocol_str = protocol;
                    }
                }
                _ => {
                    break;
                }
            }
        }
        if do_dl {
            self.display_text(IceText::SendingFiles, display_flags::NEWLINE).await?;

            if let Some(protocol) = &protocol {
                let Some(mut prot) = create_protocol(protocol) else {
                    self.display_text(IceText::TransferAborted, display_flags::NEWLINE).await?;
                    return Ok(());
                };
                let files = self.screen_transfer_limits(self.session.flagged_files.clone()).await?;
                // Keep offered files until the protocol confirms each completion.
                // PCBoard removes rejected files now, but retains failed transfers.
                self.session.flagged_files.clone_from(&files);
                if files.is_empty() {
                    return Ok(());
                }
                let offered: Vec<_> = files.iter().map(|path| (path.clone(), path.metadata().map_or(0, |m| m.len()))).collect();
                match prot.initiate_send(&mut *self.connection, &files).await {
                    Ok(mut state) => {
                        let started = Instant::now();
                        let mut failed = false;
                        while !state.is_finished {
                            if let Err(e) = prot.update_transfer(&mut *self.connection, &mut state).await {
                                log::error!("Error while updating file transfer with {protocol:?} : {e}");
                                failed = true;
                                break;
                            }
                        }
                        let cps = transfer_cps(state.send_state.total_bytes_transfered, started);
                        self.finish_download_batch(&offered, &state, &protocol_str, cps).await?;
                        if failed {
                            self.display_text(IceText::TransferAborted, display_flags::NEWLINE).await?;
                        }
                    }
                    Err(e) => {
                        log::error!("Error while initiating file transfer with {protocol:?} : {e}");
                        self.println(TerminalTarget::Both, &format!("Error: {e}")).await?;
                    }
                }
            } else {
                self.println(TerminalTarget::Both, "Protocol not found.").await?;
            }

            if goodbye_after_dl {
                self.goodbye().await?;
            }
        }
        Ok(())
    }

    /// Prompt input may contain several names, but unlike command-line input a
    /// single letter is a filename, not a protocol (TRANSFER.C scanfornames).
    async fn download_names(&mut self, goodbye: &mut bool, batch: bool) -> Res<bool> {
        let input = self
            .input_field(
                if batch {
                    IceText::FileNameToDownloadBatch
                } else {
                    IceText::FileNameToDownload
                },
                60,
                &MASK_ASCII,
                "hlpd",
                None,
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE,
            )
            .await?;
        if input.is_empty() {
            return Ok(false);
        }
        for token in crate::tokens::tokenize(&input) {
            if token.eq_ignore_ascii_case("GB") || token.eq_ignore_ascii_case("BYE") {
                *goodbye = true;
            } else if !token.is_empty() {
                self.session.tokens.push_front(token);
                self.flag_files_cmd(true).await?;
            }
        }
        Ok(true)
    }

    async fn finish_download_batch(&mut self, offered: &[(PathBuf, u64)], state: &icy_net::protocol::TransferState, protocol: &str, cps: usize) -> Res<()> {
        // A completed transfer session may include a partial or skipped file.
        // Match exact offered paths, once each; do not charge another area's
        // same-named file or include partial wire bytes in successful statistics.
        let completed: Vec<_> = offered
            .iter()
            .filter(|(path, _)| state.send_state.finished_files.iter().any(|(_, sent)| sent == path))
            .collect();
        self.session.flagged_files.retain(|path| !completed.iter().any(|(sent, _)| sent == path));
        let bytes = completed.iter().fold(0u64, |sum, (_, size)| sum.saturating_add(*size));
        self.transfer_statistics.downloaded_bytes = bytes.min(usize::MAX as u64) as usize;
        self.transfer_statistics.downloaded_files = completed.len();
        if completed.is_empty() {
            return Ok(());
        }
        let (mut charged_files, mut charged_bytes) = (0u64, 0u64);
        for (path, size) in &completed {
            let free = self.accounting_download_free(path).await?;
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            self.accounting_record_download(&name, *size, free)?;
            if !free {
                charged_files = charged_files.saturating_add(1);
                charged_bytes = charged_bytes.saturating_add(*size);
            }
        }
        if let Some(user) = &mut self.session.current_user {
            user.stats.num_downloads = user.stats.num_downloads.saturating_add(charged_files);
            user.stats.today_num_downloads = user.stats.today_num_downloads.saturating_add(charged_files);
            user.stats.total_dnld_bytes = user.stats.total_dnld_bytes.saturating_add(charged_bytes);
            user.stats.today_dnld_bytes = user.stats.today_dnld_bytes.saturating_add(charged_bytes.min(i64::MAX as u64) as i64);
        }
        limits::adjust_bytes_remaining(&mut self.session.bytes_remaining, charged_bytes.min(i64::MAX as u64) as i64);
        let paths: Vec<_> = completed.iter().map(|(path, _)| path.clone()).collect();
        let sent: Vec<_> = paths
            .iter()
            .map(|path| path.file_name().unwrap_or_default().to_string_lossy().to_string())
            .collect();
        self.count_downloads(&paths, &sent).await;
        let files = completed.len() as u64;
        IcyBoard::write_statistics(&self.board, move |statistics| statistics.add_download_totals(files, bytes)).await?;
        // Commit completed-file accounting even if the connection subsequently
        // fails while printing the summary or caller log.
        self.accounting_check_balance().await?;
        self.log_transfer(false, &sent, protocol, state.send_state.errors, cps).await?;
        self.display_text(IceText::BatchTransferEnded, display_flags::LFBEFORE).await?;
        self.display_text(IceText::BatchSend, display_flags::LFBEFORE).await?;
        Ok(())
    }

    /// Raises the per file download counter, the way `PCBoard` reports how popular a file is.
    ///
    /// The counter lives in the area the file came from, so an offered file whose area is
    /// not part of this conference is skipped.
    async fn count_downloads(&mut self, offered: &[PathBuf], sent: &[String]) {
        let Some(directories) = self.session.current_conference.directories.clone() else {
            return;
        };
        for (dir, names) in downloads_per_area(offered, sent) {
            let Some(area) = directories.iter().find(|area| area.path == dir) else {
                continue;
            };
            let (path, metadata_path) = (area.path.clone(), area.metadata_path.clone());
            let Ok(base) = self.get_filebase(&path, &metadata_path).await else {
                continue;
            };
            let mut base = base.lock().await;
            for name in names {
                if let Some(header) = base.iter_mut().find(|header| header.name().eq_ignore_ascii_case(&name)) {
                    header.dl_counter += 1;
                }
            }
            if let Err(err) = base.save() {
                log::error!("Could not record downloads in {}: {}", dir.display(), err);
            }
        }
    }

    /// Drops the files the caller's limits will not cover.
    ///
    /// `PCBoard` judges every file on its own against what the batch has already taken, so
    /// one refusal does not cost the caller the rest of their batch. There is no sysop
    /// exemption in the original either - a sysop simply holds a level with no limits.
    async fn screen_transfer_limits(&mut self, files: Vec<PathBuf>) -> Res<Vec<PathBuf>> {
        let enforce_transfer_limits = self.get_board().await.config.system_control.enforce_transfer_limits;
        let history = self.session.current_user.as_ref().map(|user| TransferHistory {
            num_uploads: user.stats.num_uploads,
            num_downloads: user.stats.num_downloads,
            total_upld_bytes: user.stats.total_upld_bytes,
            total_dnld_bytes: user.stats.total_dnld_bytes,
        });
        let mut limits = self.session.transfer_limits.clone();
        // PPL can move the allowance around during the session, so take the live figure.
        limits.bytes_remaining = (self.session.bytes_remaining >= 0).then_some(self.session.bytes_remaining);

        let bps = self.get_bps().max(0) as u32;
        let minutes_left = self.minutes_left();
        let mut seconds_so_far = 0i64;
        let mut reserved = 0.0;
        let mut so_far = BatchSoFar::default();
        let mut allowed = Vec::new();
        let mut names = std::collections::HashSet::new();
        for path in files {
            self.session.op_text = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let Some(metadata) = std::fs::metadata(&path).ok().filter(|m| m.is_file()) else {
                self.display_text(IceText::NotFoundOnDisk, display_flags::NEWLINE).await?;
                continue;
            };
            if names.contains(&self.session.op_text.to_ascii_uppercase()) {
                self.display_text(IceText::DuplicateBatchFile, display_flags::NEWLINE).await?;
                continue;
            }
            let size = metadata.len();
            let free = self.accounting_download_free(&path).await?;
            let verdict = if enforce_transfer_limits {
                history
                    .as_ref()
                    .map_or(LimitVerdict::Allowed, |history| limits.check_file(history, so_far, size, free))
            } else {
                LimitVerdict::Allowed
            };
            if !verdict.is_allowed() {
                if let Some(user) = &mut self.session.current_user {
                    user.stats.num_reach_dnld_lim += 1;
                }
                // BYTESLEFT subtracts the flagged queue. While screening, only
                // already accepted files belong in that subtotal, not this
                // rejected file or the as-yet unchecked remainder of the batch.
                let queued = std::mem::replace(&mut self.session.flagged_files, allowed.clone());
                let report = self.report_limit(&path, verdict).await;
                self.session.flagged_files = queued;
                report?;
                continue;
            }

            // A free download still costs time; only PCBoard's NOTIME files were spared,
            // and those came from an FSEC file we do not read.
            let seconds = limits::seconds_for_transfer(size, bps);
            if let Some(minutes) = minutes_left {
                // PCBoard keeps a minute back so a transfer cannot run into the logoff.
                let seconds_left = (minutes - 1) * 60 - seconds_so_far;
                if seconds > seconds_left {
                    self.session.op_text = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    self.display_text(IceText::NoTimeForDownload, display_flags::NEWLINE | display_flags::LOGIT | display_flags::BELL)
                        .await?;
                    continue;
                }
            }
            let charge = self.accounting_download_estimate(&path, size).await?;
            if self.accounting_insufficient(charge, reserved).await? {
                continue;
            }
            reserved += charge;
            seconds_so_far += seconds;
            so_far.accept(size, free);
            names.insert(path.file_name().unwrap_or_default().to_string_lossy().to_ascii_uppercase());
            allowed.push(path);
        }
        Ok(allowed)
    }

    /// Tells the caller which limit stopped the file, in the order `PCBoard` prints it:
    /// where they stand, what the limit is, then the file that broke it.
    async fn report_limit(&mut self, path: &Path, verdict: LimitVerdict) -> Res<()> {
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let (standing, limit_text, limit_value, exceeded) = match verdict {
            LimitVerdict::Allowed => return Ok(()),
            LimitVerdict::DailyBytes { .. } => {
                self.session.op_text = name;
                self.display_text(IceText::BytesLeftAre, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                return Ok(());
            }
            LimitVerdict::FileRatio { limit_tenths, .. } => (
                IceText::FileRatio,
                IceText::RatioLimit,
                format!("{}:1", tenths(limit_tenths)),
                IceText::FileRatioExceeded,
            ),
            LimitVerdict::ByteRatio { limit_tenths, .. } => (
                IceText::ByteRatio,
                IceText::RatioLimit,
                format!("{}:1", tenths(limit_tenths)),
                IceText::ByteRatioExceeded,
            ),
            LimitVerdict::FileLimit { limit, .. } => (IceText::FilesDownloaded, IceText::DownloadLimit, limit.to_string(), IceText::FileLimitExceeded),
            LimitVerdict::ByteLimit { limit, .. } => (IceText::BytesDownloaded, IceText::DownloadLimit, limit.to_string(), IceText::ByteLimitExceeded),
        };

        self.display_text(standing, display_flags::NEWLINE | display_flags::LFBEFORE).await?;
        self.session.op_text = limit_value;
        self.display_text(limit_text, display_flags::NEWLINE).await?;
        self.session.op_text = name;
        self.display_text(exceeded, display_flags::NEWLINE | display_flags::LFAFTER).await?;
        Ok(())
    }

    #[async_recursion(?Send)]
    async fn edit_dl_batch(&mut self) -> Res<()> {
        self.new_line().await?;

        loop {
            let input = self
                .input_field(
                    IceText::EditBatch,
                    1,
                    DL_EDITMASK,
                    "",
                    None,
                    display_flags::NEWLINE | display_flags::UPCASE | display_flags::FIELDLEN,
                )
                .await?;

            match input.as_str() {
                "A" => {
                    self.new_line().await?;
                    let mut goodbye = false;
                    while self.session.flagged_files.len() < self.session.batch_limit && self.download_names(&mut goodbye, true).await? {}
                }
                "R" => {
                    self.remove_dl_batch().await?;
                }
                "L" => {
                    self.list_dl_batch().await?;
                }
                _ => {
                    break;
                }
            }
        }
        Ok(())
    }

    async fn remove_dl_batch(&mut self) -> Res<()> {
        self.session.op_text = format!("1-{}", self.session.flagged_files.len());
        let input = self
            .input_field(
                IceText::RemoveFileNumber,
                16,
                &MASK_NUM,
                "",
                None,
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::STACKED,
            )
            .await?;
        let mut remove = Vec::new();
        for token in crate::tokens::tokenize(&input) {
            if let Ok(num) = token.parse::<usize>() {
                if num == 0 {
                    continue;
                }
                if let Some(path) = &self.session.flagged_files.get(num - 1) {
                    if remove.contains(&(num - 1)) {
                        continue;
                    }
                    remove.push(num - 1);
                    self.session.op_text = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    self.display_text(IceText::RemovedFile, display_flags::NEWLINE).await?;
                }
            }
        }
        remove.sort_by(|a, b| b.cmp(a));
        for r in remove {
            self.session.flagged_files.remove(r);
        }
        self.new_line().await?;
        Ok(())
    }
    async fn list_dl_batch(&mut self) -> Res<()> {
        self.new_line().await?;
        for (i, path) in self.session.flagged_files.clone().iter().enumerate() {
            let size = if let Ok(data) = path.metadata() { data.len() } else { 0 };
            self.display_text(IceText::FileSelected, display_flags::DEFAULT).await?;
            self.set_color(TerminalTarget::Both, IcbColor::dos_light_green()).await?;

            let number = format!("({})", i + 1);
            self.print(TerminalTarget::Both, &format!("{number:<5}{:>8} ", humanize_bytes_decimal!(size).to_string()))
                .await?;
            self.println(TerminalTarget::Both, &format!("{}", path.file_name().unwrap_or_default().to_string_lossy()))
                .await?;
        }
        self.new_line().await?;

        Ok(())
    }
}

const DL_LISTMASK: &str = "AEGLP";
const DL_EDITMASK: &str = "ARL";

fn download_estimate(rates: &crate::icy_board::accounting_cfg::AccountingConfig, bytes: u64, seconds: i64, free: bool) -> f64 {
    let files = if free {
        0.0
    } else {
        rates.charge_per_download_file + rates.charge_per_download_bytes * (bytes as f64 / 1024.0)
    };
    files + rates.charge_per_time * seconds as f64 / 60.0
}

#[cfg(test)]
pub(crate) async fn enable_activity_accounting(state: &mut IcyBoardState, rates: crate::icy_board::accounting_cfg::AccountingConfig) {
    let security = state.session.cur_security;
    let password = state.session.last_password.clone();
    {
        let mut board = state.get_board().await;
        board.sec_levels.clear();
        board.sec_levels.push(crate::icy_board::sec_levels::SecurityLevel {
            security,
            password,
            is_enabled: true,
            ..Default::default()
        });
        board.config.accounting.enabled = true;
        board.config.accounting.concurrent_tracking = false;
        board.config.accounting.ignore_empty_sec_level = false;
        board.config.accounting.accounting_config = Some(rates);
        board.config.accounting.info_file = PathBuf::new();
        board.config.accounting.warning_file = PathBuf::new();
        board.config.accounting.logoff_file = PathBuf::new();
        board.config.accounting.tracking_file = PathBuf::new();
    }
    state.session.current_user.as_mut().unwrap().account = Some(crate::icy_board::pcb::user_inf::AccountUserInf {
        starting_balance: 100_000.0,
        start_this_session: 100_000.0,
        ..Default::default()
    });
    state.accounting_start().await.unwrap();
    assert!(state.accounting_active());
}

#[cfg(test)]
#[path = "download_tests.rs"]
mod download_compatibility_tests;

/// Ratios are held in tenths, and `PCBoard` shows them with the one decimal back.
fn tenths(value: u64) -> String {
    format!("{}.{}", value / 10, value % 10)
}

/// Groups the files that really went out by the directory they came from, so each area's
/// counters can be written in one go.
///
/// A batch can be aborted part way through, so only what the protocol reported as finished
/// counts - and it reports bare names, which is why they are matched the way DOS did.
fn downloads_per_area(offered: &[PathBuf], sent: &[String]) -> HashMap<PathBuf, Vec<String>> {
    let mut per_area: HashMap<PathBuf, Vec<String>> = HashMap::new();
    for path in offered {
        let (Some(dir), Some(name)) = (path.parent(), path.file_name().and_then(|name| name.to_str())) else {
            continue;
        };
        if sent.iter().any(|s| s.eq_ignore_ascii_case(name)) {
            per_area.entry(dir.to_path_buf()).or_default().push(name.to_string());
        }
    }
    per_area
}

#[cfg(test)]
mod download_counter_tests {
    use super::downloads_per_area;
    use std::path::PathBuf;

    #[test]
    fn accounting_estimates_fractional_kib_and_free_files_still_cost_time() {
        let rates = crate::icy_board::accounting_cfg::AccountingConfig {
            charge_per_download_file: 3.0,
            charge_per_download_bytes: 2.0,
            charge_per_time: 4.0,
            ..Default::default()
        };
        assert_eq!(super::download_estimate(&rates, 1536, 30, false), 8.0);
        assert_eq!(super::download_estimate(&rates, 1536, 30, true), 2.0);
        assert_eq!(super::download_estimate(&rates, 512, 0, false), 4.0);
    }

    #[test]
    fn only_finished_files_are_counted() {
        let offered = vec![PathBuf::from("/files/A.ZIP"), PathBuf::from("/files/B.ZIP")];
        let sent = vec!["A.ZIP".to_string()];
        let per_area = downloads_per_area(&offered, &sent);
        assert_eq!(per_area[&PathBuf::from("/files")], vec!["A.ZIP".to_string()]);
    }

    #[test]
    fn nothing_is_counted_when_the_transfer_failed() {
        let offered = vec![PathBuf::from("/files/A.ZIP")];
        assert!(downloads_per_area(&offered, &[]).is_empty());
    }

    /// The protocol may echo a name back in another case than the area holds it.
    #[test]
    fn names_are_matched_without_regard_to_case() {
        let offered = vec![PathBuf::from("/files/A.ZIP")];
        let sent = vec!["a.zip".to_string()];
        let per_area = downloads_per_area(&offered, &sent);
        assert_eq!(per_area[&PathBuf::from("/files")], vec!["A.ZIP".to_string()]);
    }

    /// A batch can span areas, and each one keeps its own counters.
    #[test]
    fn files_are_grouped_by_their_area() {
        let offered = vec![PathBuf::from("/one/A.ZIP"), PathBuf::from("/two/B.ZIP")];
        let sent = vec!["A.ZIP".to_string(), "B.ZIP".to_string()];
        let per_area = downloads_per_area(&offered, &sent);
        assert_eq!(per_area.len(), 2);
        assert_eq!(per_area[&PathBuf::from("/one")], vec!["A.ZIP".to_string()]);
        assert_eq!(per_area[&PathBuf::from("/two")], vec!["B.ZIP".to_string()]);
    }
}
