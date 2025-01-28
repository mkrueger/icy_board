use dizbase::file_base::pattern::{MatchOptions, Pattern};
use humanize_bytes::humanize_bytes_decimal;

use crate::icy_board::icb_config::IcbColor;
use crate::icy_board::state::functions::MASK_ASCII;
use crate::icy_board::{icb_text::IceText, state::functions::display_flags};
use crate::vm::TerminalTarget;
use crate::{icy_board::state::IcyBoardState, Res};

impl IcyBoardState {
    pub async fn view_file(&mut self) -> Res<()> {
        let Some(cur_dir_path) = self.session.disp_options.in_file_list.clone() else {
            return Ok(());
        };

        let input = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            self.input_field(
                IceText::ArchiveViewFileName,
                60,
                &MASK_ASCII,
                &"",
                None,
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFAFTER | display_flags::HIGHASCII,
            )
            .await?
        };

        if !input.is_empty() {
            // let saved_list = self.session.disp_options.in_file_list.take();
            let mut flagged = Vec::new();
            self.display_text(IceText::CheckingArchiveView, display_flags::NEWLINE | display_flags::LFAFTER)
                .await?;

            let files = self.get_filebase(&cur_dir_path).await?;
            let mut options = MatchOptions::new();
            options.case_sensitive = false;
            if let Ok(pattern) = Pattern::new(&input) {
                for f in &mut files.lock().await.file_headers {
                    if pattern.matches_with(&f.name(), &options) {
                        flagged.push(f.full_path.clone());
                        break;
                    }
                }
            }

            if flagged.is_empty() || !flagged.first().unwrap().exists() {
                self.session.op_text = input.clone();
                self.display_text(IceText::NotFoundOnDisk, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                return Ok(());
            }

            let file = flagged.first().unwrap();
            if !file.exists() {
                self.session.op_text = file.file_name().unwrap().to_string_lossy().to_string();
                self.display_text(IceText::NotFoundOnDisk, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                return Ok(());
            }
            match dizbase::scan_file_contents(&file) {
                Ok(file_content) => {
                    let sav = self.session.disp_options.in_file_list.take();
                    self.session.disp_options.abort_printout = false;
                    let mut len = 0;
                    let colors = self.get_board().await.config.color_configuration.clone();
                    self.set_color(TerminalTarget::Both, colors.file_head.clone()).await?;
                    self.println(TerminalTarget::Both, &format!(" Archive: {}", file.file_name().unwrap().to_string_lossy()))
                        .await?;
                    self.println(TerminalTarget::Both, "  Length      Date    Time   Name").await?;
                    self.println(TerminalTarget::Both, " ========  ========== ===== ======").await?;
                    self.set_color(TerminalTarget::Both, IcbColor::Dos(11)).await?;
                    for info in &file_content {
                        if self.session.disp_options.abort_printout {
                            break;
                        }
                        self.set_color(TerminalTarget::Both, colors.file_size.clone()).await?;
                        self.print(TerminalTarget::Both, &format!("{:>9}  ", humanize_bytes_decimal!(info.size).to_string()))
                            .await?;
                        self.set_color(TerminalTarget::Both, colors.file_date.clone()).await?;
                        self.print(
                            TerminalTarget::Both,
                            &format!("{:04}-{:02}-{:02} ", info.date.year() % 10000, info.date.month(), info.date.day()),
                        )
                        .await?;
                        self.print(TerminalTarget::Both, &format!("{:02}:{:02} ", info.date.hour(), info.date.minute()))
                            .await?;
                        self.set_color(TerminalTarget::Both, colors.file_name.clone()).await?;
                        self.println(TerminalTarget::Both, &info.name).await?;
                        len += info.size;
                    }
                    self.set_color(TerminalTarget::Both, IcbColor::Dos(14)).await?;
                    self.set_color(TerminalTarget::Both, colors.file_head.clone()).await?;
                    self.println(TerminalTarget::Both, "---------                   ------").await?;
                    self.set_color(TerminalTarget::Both, colors.file_size).await?;
                    self.set_color(TerminalTarget::Both, IcbColor::Dos(15)).await?;
                    self.println(
                        TerminalTarget::Both,
                        &format!("{:>9}                   {} files", humanize_bytes_decimal!(len).to_string(), file_content.len()),
                    )
                    .await?;
                    self.reset_color(TerminalTarget::Both).await?;
                    self.new_line().await?;
                    self.session.disp_options.in_file_list = sav;
                }
                Err(_) => {
                    self.session.op_text = file.file_name().unwrap().to_string_lossy().to_string();
                    self.display_text(IceText::ErrorViewingFile, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                }
            }
        }
        Ok(())
    }
}
