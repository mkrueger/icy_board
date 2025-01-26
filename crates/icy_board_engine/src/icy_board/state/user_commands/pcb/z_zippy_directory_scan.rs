use crate::{
    icy_board::state::{functions::MASK_COMMAND, IcyBoardState},
    Res,
};
use crate::{
    icy_board::{
        icb_config::IcbColor,
        icb_text::IceText,
        state::functions::{display_flags, MASK_ASCII},
    },
    vm::TerminalTarget,
};
use dizbase::file_base::FileBase;

use super::l_find_files::FileList;

impl IcyBoardState {
    pub async fn zippy_directory_scan(&mut self, help: &str) -> Res<()> {
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
                IceText::TextToScanFor,
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
                    self.pattern_search_file_area(help, area, search_pattern.clone()).await?;
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
                    self.pattern_search_file_area(help, number as usize - 1, search_pattern).await?;
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

    async fn pattern_search_file_area(&mut self, help: &str, area: usize, search_pattern: String) -> Res<()> {
        let file_base_path = self.resolve_path(&self.session.current_conference.directories[area].path);
        let Ok(base) = FileBase::open(&file_base_path) else {
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
        let files = base.find_files_with_pattern(search_pattern.as_str())?;

        let mut list = FileList::new(file_base_path, files, help);
        list.display_file_list(self).await?;

        self.session.non_stop_off();
        self.session.more_requested = false;
        Ok(())
    }
}
