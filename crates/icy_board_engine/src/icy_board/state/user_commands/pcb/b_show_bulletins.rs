use chrono::{DateTime, Utc};

use crate::icy_board::{
    bulletins::MASK_BULLETINS,
    commands::CommandType,
    icb_text::IceText,
    state::{
        functions::{display_flags, MASK_ALNUM},
        NodeStatus,
    },
};
use crate::{icy_board::state::IcyBoardState, Res};

impl IcyBoardState {
    pub async fn show_bulletins(&mut self) -> Res<()> {
        self.set_activity(NodeStatus::ReadBulletins).await;
        let bulletins = self.session.current_conference.bulletins.clone().unwrap_or_default();
        if bulletins.is_empty() {
            self.display_text(
                IceText::NoBulletinsAvailable,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::BELL,
            )
            .await?;
            return Ok(());
        }
        let mut display_current_menu = self.session.tokens.is_empty();
        loop {
            self.session.disp_options.force_count_lines();
            if display_current_menu {
                let file = self.session.current_conference.blt_menu.clone();
                self.session.disp_options.no_change();
                self.display_file(&file).await?;
                display_current_menu = false;
            }
            if self.session.tokens.is_empty() {
                let input = self
                    .input_field(
                        if self.session.expert_mode() {
                            IceText::BulletinListCommandExpertmode
                        } else {
                            IceText::BulletinListCommand
                        },
                        12,
                        MASK_BULLETINS,
                        CommandType::BulletinList.get_help(),
                        None,
                        display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE | display_flags::STACKED,
                    )
                    .await?;
                self.session.push_tokens(&input);
            };
            if self.session.tokens.is_empty() {
                break;
            }
            let mut files = Vec::new();
            let mut download_blt = false;
            let mut search_blt = false;
            let mut new_files = false;
            while let Some(text) = self.session.tokens.pop_front() {
                match text.as_str() {
                    "G" => {
                        self.goodbye().await?;
                        return Ok(());
                    }
                    "R" | "L" => {
                        display_current_menu = true;
                    }
                    "A" => {
                        files.extend(0..bulletins.len() as i32);
                    }
                    "D" => {
                        // Download Bulletins
                        download_blt = true;
                    }
                    "S" => {
                        search_blt = true;
                    }
                    "N" => {
                        new_files = true;
                        files.extend(0..bulletins.len() as i32);
                    }
                    "NS" => {
                        self.session.disp_options.force_count_lines();
                    }
                    _ => {
                        if let Ok(number) = text.parse::<i32>() {
                            files.push(number - 1);
                        }
                    }
                }
            }

            if search_blt {
                let search_text = self
                    .input_field(
                        IceText::TextToScanFor,
                        79,
                        &MASK_ALNUM,
                        "",
                        None,
                        display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE,
                    )
                    .await?;
                let search = search_text.to_lowercase();
                let Some(search_text) = self.session.parse_search_text(search) else {
                    self.display_text(IceText::PunctuationError, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                    return Ok(());
                };
                self.session.search_text = search_text;
                if files.is_empty() {
                    files.extend(0..bulletins.len() as i32);
                }
            }
            self.session.disp_options.no_change();
            for i in files {
                if let Some(b) = bulletins.get(i as usize) {
                    if new_files {
                        if let Ok(md) = b.file.metadata() {
                            let mod_time: DateTime<Utc> = md.modified().unwrap().into();
                            if mod_time < self.session.current_user.as_ref().unwrap().stats.last_on {
                                continue;
                            }
                        }
                    }
                    if download_blt {
                        self.add_flagged_file(b.file.clone(), false, false).await?;
                    } else {
                        self.display_file(&b.file).await?;
                    }
                } else {
                    self.display_text(
                        IceText::InvalidBulletinNumber,
                        display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
                    )
                    .await?;
                }
            }
            self.session.search_text.clear();
            // no prompt after displaying bulletins
            self.session.disp_options.count_lines = false;

            if download_blt {
                self.download(false).await?;
            }
        }
        Ok(())
    }
}
