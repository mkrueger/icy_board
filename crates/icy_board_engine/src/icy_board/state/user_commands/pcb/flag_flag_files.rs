use dizbase::file_base::pattern::{MatchOptions, Pattern};
use dizbase::file_base::FileBase;
use humanize_bytes::humanize_bytes_decimal;

use crate::icy_board::icb_config::IcbColor;
use crate::icy_board::state::functions::MASK_ASCII;
use crate::{icy_board::state::IcyBoardState, Res};
use crate::{
    icy_board::{icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};

impl IcyBoardState {
    pub async fn flag_files(&mut self) -> Res<()> {
        // flag
        let input = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            self.input_field(
                IceText::FlagForDownload,
                60,
                &MASK_ASCII,
                &"hlpflag",
                None,
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFAFTER | display_flags::HIGHASCII,
            )
            .await?
        };
        log::info!("flag {input}");

        if !input.is_empty() {
            let saved_list = self.session.disp_options.in_file_list.take();
            let mut flagged = Vec::new();
            self.display_text(IceText::CheckingFileTransfer, display_flags::NEWLINE).await?;

            for dir in self.session.current_conference.directories.iter() {
                let mut files = FileBase::open(&dir.path)?;
                let mut options = MatchOptions::new();
                options.case_sensitive = false;
                if let Ok(pattern) = Pattern::new(&input) {
                    for f in &mut files.file_headers {
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

            for (file, size) in flagged {
                let name = file.file_name().unwrap().to_string_lossy().to_string();
                let count = self.session.flag_for_download(file);
                self.set_color(TerminalTarget::Both, IcbColor::Dos(10)).await?;
                let nr = format!("({})", count);
                self.println(
                    TerminalTarget::Both,
                    &format!("{:<6}{:<12} {}", nr, name, humanize_bytes_decimal!(size).to_string()),
                )
                .await?;
                self.reset_color(TerminalTarget::Both).await?;
            }

            self.session.disp_options.in_file_list = saved_list;
        }
        Ok(())
    }
}
