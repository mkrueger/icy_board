use crate::{
    menu_runner::{PcbBoardCommand, MASK_COMMAND},
    Res,
};
use dizbase::file_base::FileBase;
use icy_board_engine::{
    icy_board::{
        icb_config::IcbColor,
        icb_text::IceText,
        state::functions::{display_flags, MASK_ASCII},
    },
    vm::TerminalTarget,
};

use super::l_find_files::FileList;

impl PcbBoardCommand {
    pub async fn zippy_directory_scan(&mut self, help: &str) -> Res<()> {
        if self.state.session.current_conference.directories.is_empty() {
            self.state
                .display_text(IceText::NoDirectoriesAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            self.state.press_enter().await?;
            return Ok(());
        }
        let search_pattern = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            self.state
                .input_field(
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
            self.state.press_enter().await?;
            self.display_menu = true;
            return Ok(());
        }

        let search_area = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            self.state
                .input_field(
                    if self.state.session.expert_mode {
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
            self.state.press_enter().await?;
            self.display_menu = true;
            return Ok(());
        }

        let mut joined = false;
        if search_area == "A" {
            self.state.session.cancel_batch = false;
            for area in 0..self.state.session.current_conference.directories.len() {
                if self.state.session.current_conference.directories[area]
                    .list_security
                    .user_can_access(&self.state.session)
                {
                    self.pattern_search_file_area(help, area, search_pattern.clone()).await?;
                }
                if self.state.session.cancel_batch {
                    break;
                }
            }
            joined = true;
        } else if let Ok(number) = search_area.parse::<i32>() {
            if 1 <= number && (number as usize) <= self.state.session.current_conference.directories.len() {
                let area = &self.state.session.current_conference.directories[number as usize - 1];

                if area.list_security.user_can_access(&self.state.session) {
                    self.pattern_search_file_area(help, number as usize - 1, search_pattern).await?;
                }

                joined = true;
            }
        }

        if !joined {
            self.state.session.op_text = search_area;
            self.state
                .display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::NOTBLANK)
                .await?;
        }

        self.state.new_line().await?;
        self.state.press_enter().await?;
        self.display_menu = true;
        Ok(())
    }

    async fn pattern_search_file_area(&mut self, help: &str, area: usize, search_pattern: String) -> Res<()> {
        let file_base_path = self.state.resolve_path(&self.state.session.current_conference.directories[area].file_base);
        let Ok(mut base) = FileBase::open(&file_base_path) else {
            log::error!("Could not open file base: {}", file_base_path.display());
            self.state.session.op_text = self.state.session.current_conference.directories[area].file_base.to_str().unwrap().to_string();
            self.state
                .display_text(IceText::NotFoundOnDisk, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(());
        };

        self.state.display_text(IceText::ScanningDirectory, display_flags::DEFAULT).await?;
        self.state.print(TerminalTarget::Both, &format!(" {} ", area + 1)).await?;
        self.state.set_color(TerminalTarget::Both, IcbColor::Dos(10)).await?;
        self.state
            .print(
                TerminalTarget::Both,
                &format!("({})", self.state.session.current_conference.directories[area].name),
            )
            .await?;
        self.state.new_line().await?;
        base.load_headers()?;
        let files = base.find_files_with_pattern(search_pattern.as_str())?;

        let mut list = FileList { base: &base, files, help };
        list.display_file_list(self).await?;

        self.state.session.disable_auto_more = false;
        self.state.session.more_requested = false;
        Ok(())
    }
}
