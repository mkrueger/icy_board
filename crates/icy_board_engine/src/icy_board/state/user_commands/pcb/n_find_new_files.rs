use crate::{
    Res,
    datetime::IcbDate,
    icy_board::{
        commands::CommandType,
        state::{
            IcyBoardState,
            functions::{MASK_COMMAND, MASK_NUM},
        },
    },
};
use crate::{
    icy_board::{icb_config::IcbColor, icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};

impl IcyBoardState {
    pub async fn find_new_files(&mut self) -> Res<()> {
        if self.session.current_conference.directories.is_none() || self.session.current_conference.directories.as_ref().unwrap().is_empty() {
            self.display_text(IceText::NoDirectoriesAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(());
        }
        let search_pattern = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            let date = if let Some(user) = &self.session.current_user {
                Some(user.stats.last_on.format("%m%d%y").to_string())
            } else {
                None
            };

            self.input_field(
                IceText::DateToSearch,
                6,
                &MASK_NUM,
                CommandType::LocateFile.get_help(),
                date,
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::FIELDLEN | display_flags::GUIDE,
            )
            .await?
        };
        if search_pattern.is_empty() {
            return Ok(());
        }
        let month = search_pattern[0..2].parse::<u8>().unwrap_or(0);
        let day = search_pattern[2..4].parse::<u8>().unwrap_or(0);
        let year = search_pattern[4..6].parse::<u16>().unwrap_or(0);
        let search_date = IcbDate::new(month, day, year).to_local_date_time();

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

                for (num, desc, path) in dir_numbers.numbers {
                    self.display_text(IceText::ScanningDirectory, display_flags::DEFAULT).await?;
                    self.print(TerminalTarget::Both, &format!(" {}", num)).await?;
                    if !desc.is_empty() {
                        self.set_color(TerminalTarget::Both, IcbColor::dos_light_green()).await?;
                        self.print(TerminalTarget::Both, &format!(" ({})", desc)).await?;
                    }
                    self.new_line().await?;
                    self.reset_color(TerminalTarget::Both).await?;
                    let r = search_date.clone();
                    self.display_file_area(&path, Box::new(move |p| p.date() >= r)).await?;
                    if self.session.disp_options.abort_printout {
                        break;
                    }
                }
            }
        }
    }
}
