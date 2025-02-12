use std::path::Path;

use crate::icy_board::commands::CommandType;
use crate::icy_board::state::functions::MASK_COMMAND;
use crate::icy_board::state::user_commands::mods::filebrowser::FileList;
use crate::icy_board::state::IcyBoardState;
use crate::Res;
use crate::{
    icy_board::{
        icb_text::IceText,
        state::{functions::display_flags, NodeStatus},
    },
    vm::TerminalTarget,
};

impl IcyBoardState {
    pub async fn show_file_directories(&mut self) -> Res<()> {
        self.set_activity(NodeStatus::Available).await;

        self.session.non_stop_off();
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
                    let mnu = self.session.current_conference.dir_menu.clone();
                    let mnu = self.resolve_path(&mnu);
                    self.display_menu(&mnu).await?;
                    self.new_line().await?;
                }

                let input = self
                    .input_field(
                        if self.session.expert_mode {
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
            if let Ok(number) = directory_number.parse::<i32>() {
                if 1 <= number && (number as usize) <= self.session.current_conference.directories.as_ref().unwrap().len() {
                    let area = &self.session.current_conference.directories.as_ref().unwrap()[number as usize - 1];
                    if area.list_security.user_can_access(&self.session) {
                        self.display_file_area(&area.path.to_path_buf()).await?;
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
                        self.display_file_area(&self.session.current_conference.pub_upload_location.clone()).await?;
                    }
                    "P" => {
                        if self.session.is_sysop {
                            self.display_file_area(&self.session.current_conference.private_upload_location.clone()).await?;
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
                        self.find_new_files(self.session.current_user.as_ref().unwrap().stats.last_on.into()).await?;
                    }
                    "D" => {
                        self.download().await?;
                    }
                    "V" => {
                        self.view_file().await?;
                    }
                    "G" => {
                        self.goodbye_cmd().await?;
                        return Ok(());
                    }
                    "NS" => {
                        self.session.non_stop_on();
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
        Ok(())
    }

    async fn display_file_area(&mut self, path: &Path) -> Res<()> {
        let colors = self.get_board().await.config.color_configuration.clone();
        let file_base_path = self.resolve_path(&path);
        let Ok(base) = self.get_filebase(&file_base_path).await else {
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

        let files = {
            let mut base = base.lock().await;
            base.file_headers
                .iter_mut()
                .map(|f| {
                    let _ = f.get_metadata();
                    f.clone()
                })
                .collect::<Vec<_>>()
        };
        let mut list = FileList::new(file_base_path.clone(), files);
        list.display_file_list(self).await?;

        if self.session.num_lines_printed > 0 {
            self.filebase_more().await?;
        }
        self.session.non_stop_off();
        self.session.more_requested = false;
        Ok(())
    }
}
