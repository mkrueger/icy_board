use dizbase::file_base::metadata::MetadataType;

use crate::{
    Res,
    datetime::IcbDate,
    icy_board::{
        commands::CommandType,
        state::{IcyBoardState, functions::MASK_COMMAND, functions::MASK_NUM},
    },
    tables::import_cp437_string,
};
use crate::{
    icy_board::{
        icb_config::IcbColor,
        icb_text::IceText,
        state::functions::{MASK_ASCII, display_flags},
    },
    vm::TerminalTarget,
};

#[derive(Default)]
pub struct DirNumbers {
    pub flag_files: bool,
    pub date_time: Option<chrono::prelude::DateTime<chrono::prelude::Local>>,
    pub numbers: Vec<(usize, String, std::path::PathBuf, std::path::PathBuf)>,
}

/// What a lone `N` or `S` on the command line asked for.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NewScanKind {
    /// `N` - ask which date to scan from.
    AskDate,
    /// `S` - take the stored date without asking.
    SinceLastScan,
}

impl IcyBoardState {
    pub async fn zippy_directory_scan(&mut self) -> Res<()> {
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
                self.show_dir_menu().await?;
            } else {
                self.session.push_tokens(&search_area);
                let mut dir_numbers = self.get_dir_numbers().await?;
                if scan_date.is_some() {
                    dir_numbers.date_time = scan_date;
                }
                self.displaycmdfile("prefile").await?;
                self.new_line().await?;
                self.session.disp_options.no_change();
                let r = self.session.search_pattern.as_ref().unwrap().clone();

                for (num, desc, path, metadata) in dir_numbers.numbers {
                    self.display_text(IceText::ScanningDirectory, display_flags::DEFAULT).await?;
                    self.print(TerminalTarget::Both, &format!(" {}", num)).await?;
                    if !desc.is_empty() {
                        self.set_color(TerminalTarget::Both, IcbColor::dos_light_green()).await?;
                        self.print(TerminalTarget::Both, &format!(" ({})", desc)).await?;
                    }
                    self.new_line().await?;
                    self.reset_color(TerminalTarget::Both).await?;
                    let r = r.clone();
                    let date_time = dir_numbers.date_time.clone();
                    self.display_file_area(
                        &path,
                        &metadata,
                        Box::new(move |p, md| {
                            if let Some(date) = date_time {
                                if p.date() < date {
                                    return false;
                                }
                            }

                            if r.is_match(p.name()) {
                                return true;
                            }

                            for d in md {
                                if d.metadata_type != MetadataType::FileID {
                                    continue;
                                }
                                let desc = import_cp437_string(&d.data, true);
                                if r.is_match(&desc) {
                                    return true;
                                }
                            }
                            false
                        }),
                    )
                    .await?;
                    if self.session.disp_options.abort_printout {
                        break;
                    }
                }
            }
        }
        self.stop_search();
        Ok(())
    }

    /// PCBoard treats a lone N or S on the command line as a new file scan and
    /// then asks for a date before it asks for the search text. The tokens are
    /// taken out of the queue so the later directory parse does not see them twice.
    /// S wins over N, the way PCBoard's date buffer keeps the last thing written to it.
    pub fn tokens_request_new_scan(&mut self) -> Option<NewScanKind> {
        let mut kind = None;
        self.session.tokens.retain(|token| {
            if token.eq_ignore_ascii_case("S") {
                kind = Some(NewScanKind::SinceLastScan);
                false
            } else if token.eq_ignore_ascii_case("N") {
                kind.get_or_insert(NewScanKind::AskDate);
                false
            } else {
                true
            }
        });
        kind
    }

    /// An empty answer keeps the default, a
    /// date of all zeroes turns the filter off and anything that is not six
    /// digits is asked again.
    pub async fn ask_scan_date(&mut self, kind: NewScanKind) -> Res<Option<chrono::prelude::DateTime<chrono::prelude::Local>>> {
        let default = self.session.current_user.as_ref().map(|user| user.stats.last_on.format("%m%d%y").to_string());
        if kind == NewScanKind::SinceLastScan {
            let Some(default) = &default else {
                return Ok(None);
            };
            return Ok(Self::parse_scan_date(default));
        }
        loop {
            let mut answer = self
                .input_field(
                    IceText::DateToSearch,
                    6,
                    &MASK_NUM,
                    "",
                    default.clone(),
                    display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::FIELDLEN | display_flags::GUIDE,
                )
                .await?;
            if answer.is_empty() {
                let Some(default) = &default else {
                    return Ok(None);
                };
                answer = default.clone();
            }
            if answer == "000000" {
                return Ok(None);
            }
            if answer.len() != 6 || !answer.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            return Ok(Self::parse_scan_date(&answer));
        }
    }

    fn parse_scan_date(answer: &str) -> Option<chrono::prelude::DateTime<chrono::prelude::Local>> {
        if answer.len() != 6 || !answer.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let month = answer[0..2].parse::<u8>().unwrap_or(0);
        let day = answer[2..4].parse::<u8>().unwrap_or(0);
        let year = answer[4..6].parse::<u16>().unwrap_or(0);
        Some(IcbDate::new(month, day, year).to_local_date_time())
    }

    pub async fn get_dir_numbers(&mut self) -> Res<DirNumbers> {
        let mut res = DirNumbers::default();
        let mut private_upload = false;
        let mut public_upload = false;
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
                "D" => {
                    res.flag_files = true;
                }
                "0" | "P" => {
                    if !self
                        .board
                        .lock()
                        .await
                        .config
                        .sysop_command_level
                        .view_private_uploads
                        .session_can_access(&self.session)
                    {
                        continue;
                    }
                    private_upload = true;
                }
                "U" => {
                    if self.session.current_conference.private_uploads && !self.session.current_conference.pub_upload_location.is_dir() {
                        self.display_text(IceText::UploadsArePrivate, display_flags::NEWLINE | display_flags::LFBEFORE)
                            .await?;
                        continue;
                    }
                    public_upload = true;
                }
                "N" => {
                    read_date = true;
                }
                "S" => {
                    // Scan from the stored date without asking for one.
                    if let Some(user) = self.session.current_user.as_ref() {
                        let stored = user.stats.last_on.format("%m%d%y").to_string();
                        res.date_time = Self::parse_scan_date(&stored);
                    }
                }
                t => {
                    self.add_numbers(&mut numbers, t).await?;
                }
            }
        }

        if private_upload {
            res.numbers.push((
                0,
                self.get_display_text(IceText::RecentUploads)?,
                self.session.current_conference.private_upload_location.clone(),
                self.session.current_conference.private_upload_metadata.clone(),
            ))
        }

        for p in numbers {
            let desc = self.session.current_conference.directories.as_ref().unwrap()[p - 1].name.clone();
            res.numbers.push((
                p,
                desc,
                self.session.current_conference.directories.as_ref().unwrap()[p - 1].path.clone(),
                self.session.current_conference.directories.as_ref().unwrap()[p - 1].metadata_path.clone(),
            ));
        }

        if public_upload {
            res.numbers.push((
                max_dirs + 1,
                self.get_display_text(IceText::RecentUploads)?,
                self.session.current_conference.pub_upload_location.clone(),
                self.session.current_conference.private_upload_metadata.clone(),
            ))
        }

        Ok(res)
    }

    async fn add_numbers(&mut self, numbers: &mut Vec<usize>, token: &str) -> Res<()> {
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
            || beg > self.session.current_conference.directories.as_ref().unwrap().len()
            || parse_end && (end < beg || end > self.session.current_conference.directories.as_ref().unwrap().len())
        {
            self.display_text(IceText::InvalidFileNumber, display_flags::NEWLINE | display_flags::LFBEFORE)
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
