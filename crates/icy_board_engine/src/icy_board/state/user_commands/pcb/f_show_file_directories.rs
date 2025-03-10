use std::path::PathBuf;

use dizbase::file_base::file_header::FileHeader;
use dizbase::file_base::metadata::MetadataHeader;

use crate::Res;
use crate::icy_board::commands::CommandType;
use crate::icy_board::state::IcyBoardState;
use crate::icy_board::state::functions::MASK_COMMAND;
use crate::icy_board::state::user_commands::mods::filebrowser::FileList;
use crate::{
    icy_board::{
        icb_text::IceText,
        state::{NodeStatus, functions::display_flags},
    },
    vm::TerminalTarget,
};

impl IcyBoardState {
    pub async fn show_file_directories_cmd(&mut self) -> Res<()> {
        self.set_activity(NodeStatus::Available).await;

        self.session.disp_options.no_change();
        self.session.more_requested = false;

        if self.session.current_conference.directories.is_none() || self.session.current_conference.directories.as_ref().unwrap().is_empty() {
            return Ok(());
        }
        let mut redisplay_menu = true;

        loop {
            let directory_number = if let Some(token) = self.session.tokens.pop_front() {
                token
            } else {
                if redisplay_menu {
                    redisplay_menu = false;
                    self.show_dir_menu().await?;
                    self.new_line().await?;
                }

                let input = self
                    .input_field(
                        if self.session.expert_mode() {
                            IceText::FileListCommandExpert
                        } else {
                            IceText::FileListCommand
                        },
                        40,
                        MASK_COMMAND,
                        CommandType::FileDirectory.get_help(),
                        None,
                        display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
                    )
                    .await?;
                self.session.push_tokens(&input);
                self.session.tokens.pop_front().unwrap_or_default()
            };

            if directory_number.is_empty() {
                break;
            }
            let mut joined = false;
            self.session.disp_options.no_change();

            if let Ok(number) = directory_number.parse::<i32>() {
                if 1 <= number && (number as usize) <= self.session.current_conference.directories.as_ref().unwrap().len() {
                    let area = &self.session.current_conference.directories.as_ref().unwrap()[number as usize - 1];
                    if area.list_security.session_can_access(&self.session) {
                        self.display_file_area(&area.path.to_path_buf(), &area.metadata_path.clone(), Box::new(|_f, _| true))
                            .await?;
                        self.new_line().await?;
                        continue;
                    }
                    joined = true;
                }
                if !joined {
                    self.session.op_text = directory_number;
                    self.display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::NOTBLANK)
                        .await?;
                }
            } else {
                match directory_number.to_ascii_uppercase().as_str() {
                    "U" => {
                        self.display_file_area(
                            &self.session.current_conference.pub_upload_location.clone(),
                            &self.session.current_conference.private_upload_metadata.clone(),
                            Box::new(|_f, _| true),
                        )
                        .await?;
                    }
                    "P" => {
                        if self.session.is_sysop {
                            self.display_file_area(
                                &self.session.current_conference.private_upload_location.clone(),
                                &self.session.current_conference.private_upload_metadata.clone(),
                                Box::new(|_f, _| true),
                            )
                            .await?;
                        }
                    }
                    "F" | "FL" | "FLA" | "FLAG" => {
                        self.flag_files_cmd(false).await?;
                    }
                    "L" => {
                        self.find_files_cmd().await?;
                    }
                    "Z" => {
                        self.zippy_directory_scan().await?;
                    }
                    "N" => {
                        self.find_new_files().await?;
                    }
                    "D" => {
                        self.download(true).await?;
                    }
                    "V" => {
                        self.view_file().await?;
                    }
                    "G" => {
                        self.goodbye_cmd().await?;
                        return Ok(());
                    }
                    "NS" => {
                        self.session.disp_options.force_count_lines();
                        continue;
                    }
                    "R" => {
                        redisplay_menu = true;
                        continue;
                    }
                    _ => {}
                }
            }
        }
        // no prompt after displaying bulletins
        self.session.disp_options.count_lines = false;

        Ok(())
    }

    pub async fn display_file_area(&mut self, dir: &PathBuf, meta_data_path: &PathBuf, f: Box<dyn Fn(&FileHeader, &[MetadataHeader]) -> bool>) -> Res<()> {
        let colors = self.get_board().await.config.color_configuration.clone();
        let Ok(base) = self.get_filebase(dir, meta_data_path).await else {
            return Ok(());
        };

        self.clear_screen(TerminalTarget::Both).await?;

        self.set_color(TerminalTarget::Both, colors.file_head).await?;
        self.println(TerminalTarget::Both, "Filename       Size      Date    Description of File Contents")
            .await?;
        self.println(
            TerminalTarget::Both,
            "============ ========  ========  ============================================",
        )
        .await?;

        let mut list = FileList::new(dir.to_path_buf(), base);
        let lines = self.session.disp_options.num_lines_printed;
        list.display_file_list(self, f).await?;

        if self.session.disp_options.num_lines_printed > lines {
            self.filebase_more().await?;
        }
        self.session.more_requested = false;
        // no prompt after displaying bulletins
        self.session.disp_options.count_lines = false;
        Ok(())
    }
}
