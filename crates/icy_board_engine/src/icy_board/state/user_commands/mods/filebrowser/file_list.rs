use std::path::PathBuf;

use dizbase::file_base::{metadata::MetadaType, FileEntry};
use humanize_bytes::humanize_bytes_decimal;

use crate::{icy_board::state::IcyBoardState, vm::TerminalTarget, Res};

pub struct FileList<'a> {
    pub path: PathBuf,
    pub files: Vec<&'a mut FileEntry>,
}

impl<'a> FileList<'a> {
    pub fn new(path: PathBuf, files: Vec<&'a mut FileEntry>) -> Self {
        Self { path, files }
    }

    pub async fn display_file_list(&mut self, cmd: &mut IcyBoardState) -> Res<()> {
        let short_header = if let Some(user) = &cmd.session.current_user {
            user.flags.use_short_filedescr
        } else {
            false
        };
        cmd.session.disp_options.in_file_list = Some(self.path.clone());
        let colors = cmd.get_board().await.config.color_configuration.clone();
        for entry in &mut self.files {
            let date = entry.date();
            let size = entry.size();
            let name = &entry.file_name;
            cmd.set_color(TerminalTarget::Both, colors.file_name.clone()).await?;
            cmd.print(TerminalTarget::Both, &format!("{:<12} ", name)).await?;
            if name.len() > 12 {
                cmd.new_line().await?;
            }

            if entry.full_path.exists() {
                cmd.set_color(TerminalTarget::Both, colors.file_size.clone()).await?;
                cmd.print(TerminalTarget::Both, &format!("{:>8}  ", humanize_bytes_decimal!(size).to_string()))
                    .await?;
            } else {
                cmd.set_color(TerminalTarget::Both, colors.file_offline.clone()).await?;
                cmd.print(TerminalTarget::Both, &format!("{:>8}  ", "Offline".to_string())).await?;
            }

            cmd.set_color(TerminalTarget::Both, colors.file_date.clone()).await?;
            cmd.print(TerminalTarget::Both, &format!("{}", date.format("%m/%d/%y"))).await?;
            if false {
                cmd.set_color(TerminalTarget::Both, colors.file_new_file.clone()).await?;
                cmd.print(TerminalTarget::Both, "*").await?;
                cmd.reset_color(TerminalTarget::Both).await?;
                cmd.print(TerminalTarget::Both, " ").await?;
            } else {
                cmd.print(TerminalTarget::Both, "  ").await?;
            }

            let mut printed_lines = false;
            match entry.get_metadata() {
                Ok(data) => {
                    for m in data {
                        if m.get_type() == MetadaType::FileID {
                            let description = std::str::from_utf8(&m.data)?;
                            cmd.set_color(TerminalTarget::Both, colors.file_description.clone()).await?;
                            for (i, line) in description.lines().enumerate() {
                                if i > 0 {
                                    cmd.print(TerminalTarget::Both, &format!("{:33}", " ")).await?;
                                }
                                cmd.print(TerminalTarget::Both, line).await?;
                                cmd.new_line().await?;
                                printed_lines = true;
                                if short_header {
                                    break;
                                }
                                cmd.set_color(TerminalTarget::Both, colors.file_description_low.clone()).await?;
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("Error reading metadata: {} for {}", e, entry.full_path.display());
                }
            }
            if !printed_lines {
                cmd.new_line().await?;
            }
            if cmd.session.disp_options.abort_printout {
                break;
            }
        }
        cmd.session.disp_options.in_file_list = None;
        Ok(())
    }
}
