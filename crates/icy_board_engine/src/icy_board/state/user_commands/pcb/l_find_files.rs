use std::path::PathBuf;

use crate::{
    icy_board::state::{functions::MASK_COMMAND, IcyBoardState},
    Res,
};
use crate::{
    icy_board::{
        icb_config::IcbColor,
        icb_text::IceText,
        state::{
            functions::{display_flags, MASK_ASCII},
            NodeStatus,
        },
    },
    vm::TerminalTarget,
};
use dizbase::file_base::{metadata::MetadaType, FileBase, FileEntry};
use humanize_bytes::humanize_bytes_decimal;

impl IcyBoardState {
    pub async fn find_files_cmd(&mut self, help: &str) -> Res<()> {
        self.set_activity(NodeStatus::Available).await;

        if self.session.current_conference.directories.is_empty() {
            self.display_text(IceText::NoDirectoriesAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            self.press_enter().await?;
            return Ok(());
        }
        let search_pattern = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            self.input_field(
                IceText::SearchFileName,
                40,
                &MASK_ASCII,
                help,
                None,
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::HIGHASCII,
            )
            .await?
        };
        if search_pattern.is_empty() {
            self.press_enter().await?;
            self.display_current_menu = true;
            return Ok(());
        }

        let search_area = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            self.input_field(
                if self.session.expert_mode {
                    IceText::FileNumberExpertmode
                } else {
                    IceText::FileNumberNovice
                },
                40,
                MASK_COMMAND,
                help,
                None,
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::HIGHASCII,
            )
            .await?
        };
        if search_area.is_empty() {
            self.press_enter().await?;
            self.display_current_menu = true;
            return Ok(());
        }

        let mut joined = false;
        if search_area == "A" {
            self.session.cancel_batch = false;
            for area in 0..self.session.current_conference.directories.len() {
                if self.session.current_conference.directories[area].list_security.user_can_access(&self.session) {
                    self.search_file_area(area, search_pattern.clone()).await?;
                }
                if self.session.cancel_batch {
                    break;
                }
            }
            joined = true;
        } else if let Ok(number) = search_area.parse::<i32>() {
            if 1 <= number && (number as usize) <= self.session.current_conference.directories.len() {
                let area = &self.session.current_conference.directories[number as usize - 1];
                if area.list_security.user_can_access(&self.session) {
                    self.search_file_area(number as usize - 1, search_pattern).await?;
                }

                joined = true;
            }
        }

        if !joined {
            self.session.op_text = search_area;
            self.display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::NOTBLANK)
                .await?;
        }

        self.new_line().await?;
        self.press_enter().await?;
        self.display_current_menu = true;
        Ok(())
    }

    async fn search_file_area(&mut self, area: usize, search_pattern: String) -> Res<()> {
        let file_base_path = self.resolve_path(&self.session.current_conference.directories[area].path);
        let Ok(mut base) = FileBase::open(&file_base_path) else {
            log::error!("Could not open file base: {}", file_base_path.display());
            self.session.op_text = file_base_path.display().to_string();
            self.display_text(IceText::NotFoundOnDisk, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(());
        };

        self.display_text(IceText::ScanningDirectory, display_flags::DEFAULT).await?;
        self.print(TerminalTarget::Both, &format!(" {} ", area + 1)).await?;
        self.set_color(TerminalTarget::Both, IcbColor::Dos(10)).await?;
        self.print(TerminalTarget::Both, &format!("({})", self.session.current_conference.directories[area].name))
            .await?;
        self.new_line().await?;
        let files = base.find_files(search_pattern.as_str())?;

        let mut list = FileList::new(file_base_path.clone(), files);
        list.display_file_list(self).await
    }
}

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
            cmd.set_color(TerminalTarget::Both, colors.file_size.clone()).await?;
            cmd.print(TerminalTarget::Both, &format!("{:>8}  ", humanize_bytes_decimal!(size).to_string()))
                .await?;

            /* Filebase no longer supports offline files
                cmd.set_color(TerminalTarget::Both, colors.file_offline.clone()).await?;
                cmd.print(TerminalTarget::Both, &format!("{:>8}  ", "Offline".to_string())).await?;
            */
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
        }
        cmd.session.disp_options.in_file_list = None;
        Ok(())
    }
}
