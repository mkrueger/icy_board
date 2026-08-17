use crate::icy_board::state::user_commands::mods::filebrowser::FileFilter;
use dizbase::file_base::pattern::{MatchOptions, Pattern};

use crate::{
    Res,
    icy_board::{
        commands::CommandType,
        state::{IcyBoardState, functions::MASK_COMMAND},
    },
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
    pub async fn find_files_cmd(&mut self) -> Res<()> {
        if self.session.current_conference.directories.is_none() || self.session.current_conference.directories.as_ref().unwrap().is_empty() {
            self.display_text(IceText::NoDirectoriesAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(());
        }
        let scan_date = if let Some(kind) = self.tokens_request_new_scan() {
            self.ask_scan_date(kind).await?
        } else {
            None
        };
        let search_pattern = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            self.input_field(
                IceText::SearchFileName,
                40,
                &MASK_ASCII,
                CommandType::LocateFile.get_help(),
                None,
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::HIGHASCII,
            )
            .await?
        };
        if search_pattern.is_empty() {
            return Ok(());
        }

        // The prompt promises DOS wildcards, so match like PCBoard did: '*'/'?' globbing
        // against the whole name, case insensitively.
        let Ok(search_pattern) = Pattern::new(&search_pattern) else {
            self.display_text(IceText::PunctuationError, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(());
        };
        let search_pattern = std::sync::Arc::new(search_pattern);
        let match_options = MatchOptions {
            case_sensitive: false,
            ..MatchOptions::new()
        };

        loop {
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

            if search_area == "L" {
                self.show_dir_menu().await?;
            } else {
                self.session.push_tokens(&search_area);
                let dir_numbers = self.get_dir_numbers().await?;
                self.displaycmdfile("prefile").await?;
                self.new_line().await?;
                self.session.disp_options.no_change();

                for (num, desc, path, metadata) in dir_numbers.numbers {
                    self.display_text(IceText::ScanningDirectory, display_flags::DEFAULT).await?;
                    self.print(TerminalTarget::Both, &format!(" {}", num)).await?;
                    if !desc.is_empty() {
                        self.set_color(TerminalTarget::Both, IcbColor::dos_light_green()).await?;
                        self.print(TerminalTarget::Both, &format!(" ({})", desc)).await?;
                    }
                    self.new_line().await?;
                    self.reset_color(TerminalTarget::Both).await?;
                    let r = search_pattern.clone();
                    self.display_file_area(
                        &path,
                        &metadata,
                        FileFilter::header(move |p| {
                            if let Some(date) = scan_date {
                                if p.date() < date {
                                    return false;
                                }
                            }
                            r.matches_with(p.name(), &match_options)
                        }),
                    )
                    .await?;
                    if self.session.disp_options.abort_printout {
                        break;
                    }
                }
            }
        }
    }
}
