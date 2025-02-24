use crate::{
    Res,
    icy_board::state::{IcyBoardState, functions::MASK_COMMAND, user_commands::mods::filebrowser::FileList},
};
use crate::{
    icy_board::{
        icb_config::IcbColor,
        icb_text::IceText,
        state::functions::{MASK_ASCII, display_flags},
    },
    vm::TerminalTarget,
};

impl IcyBoardState {
    pub async fn zippy_directory_scan(&mut self) -> Res<()> {
        if self.session.current_conference.directories.as_ref().unwrap().is_empty() {
            self.display_text(IceText::NoDirectoriesAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(());
        }
        let search_pattern = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            self.input_field(
                IceText::TextToScanFor,
                40,
                &MASK_ASCII,
                "hlpts", // Help text scan
                None,
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::HIGHASCII,
            )
            .await?
        };
        if search_pattern.is_empty() {
            return Ok(());
        }

        let search_area = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            self.input_field(
                if self.session.expert_mode() {
                    IceText::FileNumberExpertmode
                } else {
                    IceText::FileNumberNovice
                },
                40,
                MASK_COMMAND,
                "",
                None,
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::HIGHASCII,
            )
            .await?
        };
        if search_area.is_empty() {
            return Ok(());
        }

        let mut joined = false;
        if search_area == "A" {
            self.session.cancel_batch = false;
            for area in 0..self.session.current_conference.directories.as_ref().unwrap().len() {
                if self.session.current_conference.directories.as_ref().unwrap()[area]
                    .list_security
                    .session_can_access(&self.session)
                {
                    self.pattern_search_file_area(area, search_pattern.clone()).await?;
                }
                if self.session.cancel_batch {
                    break;
                }
            }
            joined = true;
        } else if let Ok(number) = search_area.parse::<i32>() {
            if 1 <= number && (number as usize) <= self.session.current_conference.directories.as_ref().unwrap().len() {
                let area = &self.session.current_conference.directories.as_ref().unwrap()[number as usize - 1];

                if area.list_security.session_can_access(&self.session) {
                    self.pattern_search_file_area(number as usize - 1, search_pattern).await?;
                }

                joined = true;
            }
        }

        if !joined {
            self.session.op_text = search_area;
            self.display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::NOTBLANK)
                .await?;
        }
        Ok(())
    }

    async fn pattern_search_file_area(&mut self, area: usize, search_pattern: String) -> Res<()> {
        let file_base_path = self.resolve_path(&self.session.current_conference.directories.as_ref().unwrap()[area].path);

        self.display_text(IceText::ScanningDirectory, display_flags::DEFAULT).await?;
        self.print(TerminalTarget::Both, &format!(" {} ", area + 1)).await?;
        self.set_color(TerminalTarget::Both, IcbColor::dos_light_green()).await?;
        self.print(
            TerminalTarget::Both,
            &format!("({})", self.session.current_conference.directories.as_ref().unwrap()[area].name),
        )
        .await?;
        self.new_line().await?;

        let files = {
            let Ok(base) = self.get_filebase(&file_base_path).await else {
                return Ok(());
            };
            let mut base = base.lock().await;
            base.find_files_with_pattern(search_pattern.as_str())?
                .iter_mut()
                .map(|f| {
                    let _ = f.get_metadata();
                    f.clone()
                })
                .collect::<Vec<_>>()
        };

        let mut list = FileList::new(file_base_path, files);
        list.display_file_list(self).await?;

        self.session.more_requested = false;
        Ok(())
    }
}
