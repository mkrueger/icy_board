use dizbase::file_base::metadata::MetadataType;

use crate::{
    Res,
    datetime::IcbDate,
    icy_board::{
        commands::CommandType,
        state::{IcyBoardState, functions::MASK_COMMAND},
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
struct DirNumbers {
    pub numbers: Vec<usize>,
    pub flag_files: bool,
    pub _date: Option<IcbDate>,
    pub private_upload: bool,
    pub public_upload: bool,
}

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
                let r = self.session.search_pattern.as_ref().unwrap().clone();

                for p in dir_numbers.numbers {
                    self.display_text(IceText::ScanningDirectory, display_flags::DEFAULT).await?;
                    self.print(TerminalTarget::Both, &format!(" {}", p)).await?;
                    let desc = self.session.current_conference.directories.as_ref().unwrap()[p - 1].name.clone();
                    if !desc.is_empty() {
                        self.set_color(TerminalTarget::Both, IcbColor::dos_light_green()).await?;
                        self.print(TerminalTarget::Both, &format!(" ({})", desc)).await?;
                    }
                    self.new_line().await?;
                    self.reset_color(TerminalTarget::Both).await?;
                    let r = r.clone();
                    let path = self.session.current_conference.directories.as_ref().unwrap()[p - 1].path.clone();
                    self.display_file_area(
                        &path,
                        Box::new(move |p| {
                            if r.is_match(p.name()) {
                                return true;
                            }
                            if let Ok(md) = p.get_metadata() {
                                for d in md {
                                    if d.metadata_type != MetadataType::FileID {
                                        continue;
                                    }
                                    let desc = import_cp437_string(&d.data, true);
                                    if r.is_match(&desc) {
                                        return true;
                                    }
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
    }

    async fn get_dir_numbers(&mut self) -> Res<DirNumbers> {
        let mut res = DirNumbers::default();
        while let Some(token) = self.session.tokens.pop_front() {
            match token.as_str() {
                "A" => {
                    for num in 1..=self.session.current_conference.directories.as_ref().unwrap().len() {
                        res.numbers.push(num);
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
                    res.private_upload = true;
                }
                "U" => {
                    if self.session.current_conference.private_uploads && !self.session.current_conference.pub_upload_location.is_dir() {
                        self.display_text(IceText::UploadsArePrivate, display_flags::NEWLINE | display_flags::LFBEFORE)
                            .await?;
                        continue;
                    }
                    res.public_upload = true;
                }
                "N" => {
                    // TODO
                }
                "S" => {
                    // TODO
                }
                t => {
                    self.add_numbers(&mut res.numbers, t).await?;
                }
            }
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
