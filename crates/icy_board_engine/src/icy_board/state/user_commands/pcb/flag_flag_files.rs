use std::path::PathBuf;

use dizbase::file_base::pattern::{MatchOptions, Pattern};
use humanize_bytes::humanize_bytes_decimal;

use crate::icy_board::commands::CommandType;
use crate::icy_board::icb_config::IcbColor;
use crate::icy_board::state::functions::MASK_ASCII;
use crate::{icy_board::state::IcyBoardState, Res};
use crate::{
    icy_board::{icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};

impl IcyBoardState {
    pub async fn flag_files_cmd(&mut self, show_flagged: bool) -> Res<()> {
        // flag
        let input = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            self.input_field(
                if show_flagged {
                    IceText::FileNameToDownloadBatch
                } else {
                    IceText::FlagForDownload
                },
                60,
                &MASK_ASCII,
                &CommandType::FlagFiles.get_help(),
                None,
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFAFTER | display_flags::LFBEFORE | display_flags::HIGHASCII,
            )
            .await?
        };

        if !input.is_empty() {
            let saved_list = self.session.disp_options.in_file_list.take();
            let mut flagged = Vec::new();
            self.display_text(IceText::CheckingFileTransfer, display_flags::NEWLINE).await?;

            for dir in self.session.current_conference.directories.as_ref().unwrap().clone().iter() {
                let files = self.get_filebase(&dir.path).await?;
                let mut options = MatchOptions::new();
                options.case_sensitive = false;
                if let Ok(pattern) = Pattern::new(&input) {
                    for f in &mut files.lock().await.file_headers {
                        if pattern.matches_with(&f.name(), &options) {
                            let size = f.size();
                            flagged.push((f.full_path.clone(), size));
                        }
                    }
                }
            }

            if flagged.is_empty() {
                self.session.op_text = input.clone();
                self.display_text(IceText::NotFoundOnDisk, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                self.session.disp_options.in_file_list = saved_list;
                return Ok(());
            }

            for (file, _size) in flagged {
                if self.session.disp_options.abort_printout {
                    break;
                }
                if !file.exists() {
                    self.session.op_text = file.file_name().unwrap().to_string_lossy().to_string();
                    self.display_text(IceText::NotFoundOnDisk, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                    continue;
                }
                self.add_flagged_file(file, false, true).await?;
            }

            self.session.disp_options.in_file_list = saved_list;
        }
        Ok(())
    }

    pub async fn add_flagged_file(&mut self, file: impl Into<PathBuf>, force: bool, show_duplicates: bool) -> Res<()> {
        let file = file.into();
        let name = file.file_name().unwrap().to_string_lossy().to_string();

        if !force && self.session.flagged_files.len() >= self.session.batch_limit {
            self.session.op_text = name;
            self.display_text(IceText::BatchLimitReached, display_flags::NEWLINE).await?;
            return Ok(());
        }

        if self.session.flagged_files.contains(&file) {
            if show_duplicates {
                self.session.op_text = name;
                self.display_text(IceText::DuplicateBatchFile, display_flags::NEWLINE).await?;
            }
            return Ok(());
        }
        let size = if let Ok(md) = file.metadata() { md.len() } else { 0 };
        self.session.flagged_files.push(file);

        let count = self.session.flagged_files.len();
        self.set_color(TerminalTarget::Both, IcbColor::dos_light_green()).await?;
        let nr: String = format!("({})", count);
        self.println(
            TerminalTarget::Both,
            &format!("{:<6}{:<12} {}", nr, name, humanize_bytes_decimal!(size).to_string()),
        )
        .await?;
        self.reset_color(TerminalTarget::Both).await?;

        Ok(())
    }
}
