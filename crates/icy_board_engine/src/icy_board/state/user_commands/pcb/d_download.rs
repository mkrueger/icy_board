use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use async_recursion::async_recursion;
use humanize_bytes::humanize_bytes_decimal;

use crate::icy_board::icb_config::IcbColor;
use crate::icy_board::limits::{self, BatchSoFar, LimitVerdict, TransferHistory};
use crate::icy_board::state::functions::{MASK_NUM, transfer_cps};
use crate::{Res, icy_board::state::IcyBoardState};

use super::u_upload_file::create_protocol;
use crate::{
    icy_board::{icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};

impl IcyBoardState {
    pub async fn download(&mut self, ask_flagged_files: bool) -> Res<()> {
        if ask_flagged_files {
            if !self.session.flagged_files.is_empty() {
                let download_tagged = self
                    .input_field(
                        IceText::DownloadTagged,
                        1,
                        "",
                        &"",
                        Some(self.session.yes_char.to_string()),
                        display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::YESNO | display_flags::FIELDLEN,
                    )
                    .await?;

                if download_tagged == self.session.no_char.to_uppercase().to_string() {
                    return Ok(());
                }
            }

            // PCBoard asks for another name until the caller answers nothing or the
            // batch limit is reached, and outside a batch that limit is a single file.
            let had_token = !self.session.tokens.is_empty();
            let limit = if self.promotes_to_batch(had_token).await {
                self.session.batch_limit.max(1)
            } else {
                1
            };
            while self.session.flagged_files.len() < limit {
                if !self.flag_files_cmd(true).await? {
                    break;
                }
            }

            if self.session.flagged_files.is_empty() {
                return Ok(());
            }
        } else {
            self.new_line().await?;
        }

        let mut protocol_str: String = self.session.current_user.as_ref().unwrap().protocol.clone();
        let mut protocol;
        let mut p_descr;

        let mut goodbye_after_dl = false;
        let mut do_dl = true;
        loop {
            protocol = None;
            p_descr = "None".to_string();
            for p in self.get_board().await.protocols.iter() {
                if p.is_enabled && p.char_code == protocol_str {
                    p_descr = p.description.clone();
                    protocol = Some(p.send_command.clone());
                    break;
                }
            }

            // PCBoard asks which protocol to use instead of starting a transfer
            // the caller has no protocol for.
            if protocol.is_none() {
                let answer = self.ask_transfer_protocol(&protocol_str).await?;
                if answer.is_empty() || answer.eq_ignore_ascii_case("N") {
                    return Ok(());
                }
                protocol_str = answer;
                continue;
            }

            let mut total_size = 0;
            for path in &self.session.flagged_files {
                if let Ok(data) = path.metadata() {
                    total_size += data.len();
                }
            }
            self.display_text(IceText::BatchDownloadSize, display_flags::DEFAULT).await?;
            self.set_color(TerminalTarget::Both, IcbColor::dos_light_green()).await?;
            self.println(TerminalTarget::Both, &format!(" {}", humanize_bytes_decimal!(total_size).to_string()))
                .await?;

            self.display_text(IceText::BatchProtocol, display_flags::DEFAULT).await?;
            self.set_color(TerminalTarget::Both, IcbColor::dos_light_cyan()).await?;
            self.println(TerminalTarget::Both, &p_descr).await?;
            self.display_text(IceText::ReadyToSendBatch, display_flags::NEWLINE | display_flags::LFAFTER)
                .await?;

            let input = self
                .input_field(
                    IceText::GoodbyeAfterDownload,
                    1,
                    &DL_LISTMASK,
                    &"",
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
                let files: Vec<PathBuf> = self.session.flagged_files.drain(..).collect();
                for f in &files {
                    if !f.exists() {
                        log::error!("File not found: {:?}", f);
                        self.session.op_text = f.file_name().unwrap().to_string_lossy().to_string();
                        self.display_text(IceText::NotFoundOnDisk, display_flags::NEWLINE).await?;
                        return Ok(());
                    }
                }
                let files = self.screen_transfer_limits(files).await?;
                if files.is_empty() {
                    return Ok(());
                }
                match prot.initiate_send(&mut *self.connection, &files).await {
                    Ok(mut state) => {
                        let started = Instant::now();
                        while !state.is_finished {
                            if let Err(e) = prot.update_transfer(&mut *self.connection, &mut state).await {
                                log::error!("Error while updating file transfer with {:?} : {}", protocol, e);
                                self.display_text(IceText::TransferAborted, display_flags::NEWLINE).await?;
                                break;
                            }
                        }
                        self.display_text(IceText::BatchTransferEnded, display_flags::LFBEFORE).await?;
                        self.transfer_statistics.downloaded_bytes = state.send_state.total_bytes_transfered as usize;
                        self.transfer_statistics.downloaded_files = state.send_state.finished_files.len();
                        self.display_text(IceText::BatchSend, display_flags::LFBEFORE).await?;

                        let sent: Vec<String> = state.send_state.finished_files.iter().map(|(name, _)| name.clone()).collect();
                        let cps = transfer_cps(state.send_state.total_bytes_transfered, started);
                        self.log_transfer(false, &sent, &protocol_str, state.send_state.errors, cps).await?;

                        self.count_downloads(&files, &sent).await;
                        let bytes = state.send_state.total_bytes_transfered;
                        if let Some(user) = &mut self.session.current_user {
                            user.stats.num_downloads += sent.len() as u64;
                            user.stats.today_num_downloads += sent.len() as u64;
                            user.stats.total_dnld_bytes += bytes;
                            user.stats.today_dnld_bytes += bytes as i64;
                        }
                        self.board.lock().await.statistics.add_download(&state);
                        self.board.lock().await.save_statistics()?;
                    }
                    Err(e) => {
                        log::error!("Error while initiating file transfer with {:?} : {}", protocol, e);
                        self.println(TerminalTarget::Both, &format!("Error: {}", e)).await?;
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

    /// Raises the per file download counter, the way PCBoard reports how popular a file is.
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
    /// PCBoard judges every file on its own against what the batch has already taken, so
    /// one refusal does not cost the caller the rest of their batch. There is no sysop
    /// exemption in the original either - a sysop simply holds a level with no limits.
    async fn screen_transfer_limits(&mut self, files: Vec<PathBuf>) -> Res<Vec<PathBuf>> {
        if !self.get_board().await.config.system_control.enforce_transfer_limits {
            return Ok(files);
        }
        let Some(user) = &self.session.current_user else {
            return Ok(files);
        };
        let history = TransferHistory {
            num_uploads: user.stats.num_uploads,
            num_downloads: user.stats.num_downloads,
            total_upld_bytes: user.stats.total_upld_bytes,
            total_dnld_bytes: user.stats.total_dnld_bytes,
        };
        let mut limits = self.session.transfer_limits.clone();
        // PPL can move the allowance around during the session, so take the live figure.
        limits.bytes_remaining = (self.session.bytes_remaining >= 0).then_some(self.session.bytes_remaining);

        let free_areas = self.free_download_areas().await;
        let bps = self.get_bps().max(0) as u32;
        let minutes_left = self.minutes_left();
        let mut seconds_so_far = 0i64;
        let mut so_far = BatchSoFar::default();
        let mut allowed = Vec::new();
        for path in files {
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            let free = path.parent().map_or(false, |dir| free_areas.iter().any(|area| area == dir));
            let verdict = limits.check_file(&history, so_far, size, free);
            if !verdict.is_allowed() {
                if let Some(user) = &mut self.session.current_user {
                    user.stats.num_reach_dnld_lim += 1;
                }
                self.report_limit(&path, verdict).await?;
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
            seconds_so_far += seconds;
            so_far.accept(size, free);
            allowed.push(path);
        }
        Ok(allowed)
    }

    /// Directories the sysop marked free, which PCBoard's FSEC file did with a password.
    async fn free_download_areas(&mut self) -> Vec<PathBuf> {
        let Some(directories) = &self.session.current_conference.directories else {
            return Vec::new();
        };
        directories.iter().filter(|area| area.is_free).map(|area| area.path.clone()).collect()
    }

    /// Tells the caller which limit stopped the file, in the order PCBoard prints it:
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
                    &DL_EDITMASK,
                    &"",
                    None,
                    display_flags::NEWLINE | display_flags::UPCASE | display_flags::FIELDLEN,
                )
                .await?;

            match input.as_str() {
                "A" => {
                    self.new_line().await?;
                    self.flag_files_cmd(true).await?;
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
                &"",
                None,
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE,
            )
            .await?;
        self.session.push_tokens(&input);

        let mut remove = Vec::new();
        while let Some(token) = self.session.tokens.pop_front() {
            if let Ok(num) = token.parse::<usize>() {
                if num == 0 {
                    continue;
                }
                if let Some(path) = &self.session.flagged_files.get(num - 1) {
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

/// Ratios are held in tenths, and PCBoard shows them with the one decimal back.
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
