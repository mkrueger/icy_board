use std::{path::PathBuf, sync::Arc};

use dizbase::file_base::{
    FileBase,
    file_header::FileHeader,
    metadata::{MetadataHeader, MetadataType},
};
use humanize_bytes::humanize_bytes_decimal;
use tokio::sync::Mutex;

use crate::{
    Res,
    icy_board::{icb_text::IceText, state::IcyBoardState},
    vm::TerminalTarget,
};

pub struct FileList {
    pub path: PathBuf,
    pub files: Arc<Mutex<FileBase>>,
}

impl FileList {
    pub fn new(path: PathBuf, files: Arc<Mutex<FileBase>>) -> Self {
        Self { path, files }
    }

    pub async fn display_file_list(&mut self, cmd: &mut IcyBoardState, f: Box<dyn Fn(&FileHeader, &[MetadataHeader]) -> bool>) -> Res<()> {
        let short_header = if let Some(user) = &cmd.session.current_user {
            user.flags.use_short_filedescr
        } else {
            false
        };
        cmd.session.disp_options.in_file_list = Some(self.path.clone());
        let colors = cmd.get_board().await.config.color_configuration.clone();
        let dir = self.files.lock().await.dir().to_path_buf();
        let show_uploader = cmd.board.lock().await.config.file_transfer.display_uploader;
        let sysop_name = cmd.board.lock().await.config.sysop.name.clone();
        let headers = self.files.lock().await.clone();
        for entry in headers.iter() {
            let full_path = dir.join(entry.name());
            let meta_data = self.files.lock().await.read_metadata(&full_path)?;
            if !f(entry, &meta_data) {
                continue;
            }
            if cmd.session.request_logoff {
                break;
            }
            if cmd.session.disp_options.abort_printout {
                break;
            }
            let date = entry.date();
            let size = entry.size();
            let name = entry.name();
            cmd.set_color(TerminalTarget::Both, colors.file_name.clone()).await?;
            if cmd.session.search_pattern.is_some() {
                cmd.print_found_text(TerminalTarget::Both, &format!("{:<12} ", name)).await?;
            } else {
                cmd.print(TerminalTarget::Both, &format!("{:<12} ", name)).await?;
            }
            if name.len() > 12 {
                cmd.new_line().await?;
            }

            if dir.join(entry.name()).exists() {
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
            let mut first_line = true;
            for m in &meta_data {
                if m.get_type() == MetadataType::FileID {
                    let description = std::str::from_utf8(&m.data)?;
                    cmd.set_color(TerminalTarget::Both, colors.file_description.clone()).await?;
                    for line in description.lines() {
                        if cmd.session.disp_options.abort_printout {
                            break;
                        }
                        if first_line {
                            first_line = false;
                        } else {
                            cmd.print(TerminalTarget::Both, &format!("{:33}", " ")).await?;
                        }
                        if cmd.session.search_pattern.is_some() {
                            cmd.print_found_text(TerminalTarget::Both, line).await?;
                        } else {
                            cmd.print(TerminalTarget::Both, line).await?;
                        }
                        cmd.new_line().await?;
                        printed_lines = true;
                        if short_header {
                            break;
                        }
                        cmd.set_color(TerminalTarget::Both, colors.file_description_low.clone()).await?;
                    }
                }
            }
            if show_uploader {
                if !first_line {
                    cmd.print(TerminalTarget::Both, &format!("{:33}", " ")).await?;
                }
                let mut uploader = None;
                for m in &meta_data {
                    if m.get_type() == MetadataType::Uploader {
                        uploader = Some(std::str::from_utf8(&m.data)?.to_string());
                        break;
                    }
                }

                if let Ok(line) = cmd.get_display_text(IceText::UploadedBy) {
                    cmd.set_color(TerminalTarget::Both, colors.file_description.clone()).await?;
                    cmd.session.op_text = uploader.unwrap_or_else(|| sysop_name.clone());
                    if cmd.session.search_pattern.is_some() {
                        cmd.print_found_text(TerminalTarget::Both, &line).await?;
                    } else {
                        cmd.print(TerminalTarget::Both, &line).await?;
                    }
                    cmd.new_line().await?;
                }
            }
            if !printed_lines {
                cmd.new_line().await?;
            }
        }
        cmd.session.disp_options.in_file_list = None;
        Ok(())
    }
}
