use std::{ffi::OsString, path::PathBuf};

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
            UserActivity,
        },
    },
    vm::TerminalTarget,
};
use dizbase::file_base::{file_header::FileHeader, metadata::MetadaType, FileBase};
use humanize_bytes::humanize_bytes_decimal;

impl IcyBoardState {
    pub async fn find_files_cmd(&mut self, help: &str) -> Res<()> {
        self.set_activity(UserActivity::BrowseFiles).await;

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
                    self.search_file_area(help, area, search_pattern.clone()).await?;
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
                    self.search_file_area(help, number as usize - 1, search_pattern).await?;
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

    async fn search_file_area(&mut self, help: &str, area: usize, search_pattern: String) -> Res<()> {
        let file_base_path = self.resolve_path(&self.session.current_conference.directories[area].file_base);
        let Ok(mut base) = FileBase::open(&file_base_path) else {
            log::error!("Could not open file base: {}", file_base_path.display());
            self.session.op_text = self.session.current_conference.directories[area].file_base.to_str().unwrap().to_string();
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
        base.load_headers()?;
        let files = base.find_files(search_pattern.as_str())?;

        let mut list = FileList { base: &base, files, help };
        list.display_file_list(self).await
    }
}

pub struct FileList<'a> {
    pub base: &'a FileBase,
    pub files: Vec<&'a FileHeader>,
    pub help: &'a str,
}

impl<'a> FileList<'a> {
    pub async fn display_file_list(&mut self, cmd: &mut IcyBoardState) -> Res<()> {
        cmd.session.is_non_stop = true;
        let short_header = if let Some(user) = &cmd.session.current_user {
            user.flags.use_short_filedescr
        } else {
            false
        };
        let colors = cmd.get_board().await.config.color_configuration.clone();

        for header in &self.files {
            let metadata = self.base.read_metadata(header)?;

            let size = header.size();
            let date = header.file_date().unwrap();
            let name = header.name();
            cmd.set_color(TerminalTarget::Both, colors.file_name.clone()).await?;
            cmd.print(TerminalTarget::Both, &format!("{:<12} ", name)).await?;
            if name.len() > 12 {
                cmd.new_line().await?;
            }
            cmd.set_color(TerminalTarget::Both, colors.file_size.clone()).await?;
            let mut exists = false;
            for m in &metadata {
                if m.metadata_type == MetadaType::FilePath {
                    let file_name = unsafe { OsString::from_encoded_bytes_unchecked(m.data.clone()) };
                    let file_path = PathBuf::from(file_name);
                    exists = file_path.exists();
                    break;
                }
            }
            if exists {
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
            for m in metadata {
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
                        if cmd.session.more_requested && !self.filebase_more(cmd).await? {
                            cmd.session.cancel_batch = true;
                            return Ok(());
                        }
                        if short_header {
                            break;
                        }
                        cmd.set_color(TerminalTarget::Both, colors.file_description_low.clone()).await?;
                    }
                }
            }
            if !printed_lines {
                cmd.new_line().await?;
            }
        }
        Ok(())
    }

    pub async fn filebase_more(&self, cmd: &mut IcyBoardState) -> Res<bool> {
        loop {
            let input = cmd
                .input_field(
                    IceText::FilesMorePrompt,
                    40,
                    MASK_COMMAND,
                    &self.help,
                    None,
                    display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFAFTER | display_flags::HIGHASCII,
                )
                .await?;
            cmd.session.more_requested = false;
            cmd.session.num_lines_printed = 0;

            match input.as_str() {
                "F" => {
                    // flag
                    let input = cmd
                        .input_field(
                            IceText::FlagForDownload,
                            60,
                            &MASK_ASCII,
                            &"hlpflag",
                            None,
                            display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFAFTER | display_flags::HIGHASCII,
                        )
                        .await?;
                    if !input.is_empty() {
                        for f in &self.files {
                            if f.name().eq_ignore_ascii_case(&input) {
                                let metadata = self.base.read_metadata(f)?;
                                for m in &metadata {
                                    if m.metadata_type == MetadaType::FilePath {
                                        cmd.display_text(IceText::CheckingFileTransfer, display_flags::NEWLINE).await?;

                                        let file_name = unsafe { OsString::from_encoded_bytes_unchecked(m.data.clone()) };
                                        let file_path = PathBuf::from(file_name);
                                        if !file_path.exists() {
                                            cmd.session.op_text = input.clone();
                                            cmd.display_text(IceText::NotFoundOnDisk, display_flags::NEWLINE | display_flags::LFBEFORE)
                                                .await?;
                                        } else {
                                            cmd.session.flag_for_download(file_path.clone());
                                            cmd.set_color(TerminalTarget::Both, IcbColor::Dos(10)).await?;
                                            cmd.println(
                                                TerminalTarget::Both,
                                                &format!(
                                                    "{:<12} {}",
                                                    file_path.file_name().unwrap().to_string_lossy(),
                                                    humanize_bytes_decimal!(f.size).to_string()
                                                ),
                                            )
                                            .await?;
                                        }
                                        break;
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
                "V" => {
                    // view: TODO
                    cmd.println(TerminalTarget::Both, "TODO").await?;
                }
                "S" => {
                    // show: TODO
                    cmd.println(TerminalTarget::Both, "TODO").await?;
                }
                "G" => {
                    cmd.goodbye_cmd().await?;
                }
                _ => return Ok(input.to_ascii_uppercase() != cmd.session.no_char.to_string()),
            }
        }
    }
}
