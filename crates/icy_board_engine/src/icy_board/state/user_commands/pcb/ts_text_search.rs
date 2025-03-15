use std::path::PathBuf;

use crate::{
    datetime::IcbDate,
    icy_board::{
        commands::CommandType,
        icb_config::IcbColor,
        icb_text::IceText,
        state::{
            IcyBoardState, NodeStatus,
            functions::{MASK_ASCII, MASK_COMMAND, display_flags},
            user_commands::mods::messagereader::MessageViewer,
        },
    },
    vm::TerminalTarget,
};
use jamjam::jam::JamMessageBase;

use crate::Res;

use super::z_zippy_directory_scan::DirNumbers;

impl IcyBoardState {
    pub async fn text_search(&mut self) -> Res<()> {
        if self.session.current_conference.areas.is_none() || self.session.current_conference.areas.as_ref().unwrap().is_empty() {
            self.display_text(IceText::NoAreasAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(());
        }
        self.set_activity(NodeStatus::HandlingMail).await;

        let search_pattern = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            self.input_field(
                IceText::TextToScanFor,
                40,
                &MASK_ASCII,
                CommandType::ZippyDirectoryScan.get_help(),
                None,
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::HIGHASCII,
            )
            .await?
        };
        if search_pattern.is_empty() {
            return Ok(());
        }
        if !self.search_init(search_pattern, false) {
            self.display_text(IceText::PunctuationError, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(());
        }
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
                break;
            }

            if search_area == "L" {
                self.show_area_menu().await?;
            } else {
                self.session.push_tokens(&search_area);
                let area_numbers = self.get_area_numbers().await?;
                self.displaycmdfile("premsg").await?;
                self.new_line().await?;
                self.session.disp_options.no_change();
                // let r = self.session.search_pattern.as_ref().unwrap().clone();

                for (num, desc, path, _) in area_numbers.numbers {
                    self.display_text(IceText::ScanningArea, display_flags::DEFAULT).await?;
                    self.print(TerminalTarget::Both, &format!(" {}", num)).await?;
                    if !desc.is_empty() {
                        self.set_color(TerminalTarget::Both, IcbColor::dos_light_green()).await?;
                        self.print(TerminalTarget::Both, &format!(" ({})", desc)).await?;
                    }
                    self.new_line().await?;
                    self.reset_color(TerminalTarget::Both).await?;
                    let viewer = MessageViewer::load(&self.display_text)?;
                    match JamMessageBase::open(path) {
                        Ok(mut message_base) => {
                            self.read_message_number(&mut message_base, &viewer, 1, 1, false, Box::new(|_, _| true)).await?;
                            return Ok(());
                        }
                        Err(_err) => {
                            self.display_text(IceText::PathErrorInSystemConfiguration, display_flags::NEWLINE | display_flags::LFAFTER)
                                .await?;
                            break;
                        }
                    }
                }
            }
        }
        self.stop_search();
        Ok(())
    }

    pub async fn get_area_numbers(&mut self) -> Res<DirNumbers> {
        let mut res = DirNumbers::default();
        let mut read_date = false;
        let mut numbers = Vec::new();
        let max_dirs = self.session.current_conference.directories.as_ref().unwrap().len();
        while let Some(token) = self.session.tokens.pop_front() {
            if read_date {
                let month = token[0..2].parse::<u8>().unwrap_or(0);
                let day = token[2..4].parse::<u8>().unwrap_or(0);
                let year = token[4..6].parse::<u16>().unwrap_or(0);
                res.date_time = Some(IcbDate::new(month, day, year).to_local_date_time());
                continue;
            }
            match token.as_str() {
                "A" => {
                    for num in 1..=max_dirs {
                        numbers.push(num);
                    }
                }
                "N" => {
                    read_date = true;
                }
                "S" => {
                    // TODO
                }
                t => {
                    self.add_area_numbers(&mut numbers, t).await?;
                }
            }
        }

        for p in numbers {
            let desc = self.session.current_conference.areas.as_ref().unwrap()[p - 1].name.clone();
            res.numbers.push((
                p,
                desc,
                self.session.current_conference.areas.as_ref().unwrap()[p - 1].path.clone(),
                PathBuf::new(),
            ));
        }

        Ok(res)
    }

    async fn add_area_numbers(&mut self, numbers: &mut Vec<usize>, token: &str) -> Res<()> {
        let mut beg = 0;
        let mut end = 0;
        let mut parse_end = false;

        for c in token.chars() {
            if c.is_ascii_digit() {
                if parse_end {
                    end = end * 10 + c.to_digit(10).unwrap() as usize;
                } else {
                    beg = beg * 10 + c.to_digit(10).unwrap() as usize;
                }
            } else if c == '-' {
                parse_end = true;
            }
        }
        if beg < 1
            || beg > self.session.current_conference.areas.as_ref().unwrap().len()
            || parse_end && (end < beg || end > self.session.current_conference.areas.as_ref().unwrap().len())
        {
            self.session.op_text = token.to_string();
            self.display_text(IceText::InvalidAreaNumber, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(());
        }

        if parse_end {
            numbers.extend(beg..=end);
        } else {
            numbers.push(beg);
        }
        Ok(())
    }
}
