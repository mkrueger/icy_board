use crate::{
    icy_board::state::{functions::MASK_COMMAND, IcyBoardState},
    Res,
};
use crate::{
    icy_board::{
        icb_config::IcbColor,
        icb_text::IceText,
        state::{
            functions::{display_flags, MASK_ASCII},
            NodeStatus,
        },
    },
    vm::TerminalTarget,
};
use dizbase::file_base::{metadata::MetadaType, FileBase, FileEntry};
use humanize_bytes::humanize_bytes_decimal;

impl IcyBoardState {
    pub async fn find_files_cmd(&mut self, help: &str) -> Res<()> {
        self.set_activity(NodeStatus::Available).await;

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
                IceText::SearchFileName,
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
                    self.search_file_area(help, area, search_pattern.clone()).await?;
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
                    self.search_file_area(help, number as usize - 1, search_pattern).await?;
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

    async fn search_file_area(&mut self, help: &str, area: usize, search_pattern: String) -> Res<()> {
        let file_base_path = self.resolve_path(&self.session.current_conference.directories[area].path);
        let Ok(mut base) = FileBase::open(&file_base_path) else {
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
        let files = base.find_files(search_pattern.as_str())?;

        let mut list = FileList::new(files, help);
        list.display_file_list(self).await
    }
}

pub struct FileList<'a> {
    pub files: Vec<&'a mut FileEntry>,
    pub help: &'a str,
}

impl<'a> FileList<'a> {
    pub fn new(files: Vec<&'a mut FileEntry>, help: &'a str) -> Self {
        Self { files, help }
    }

    pub async fn display_file_list(&mut self, cmd: &mut IcyBoardState) -> Res<()> {
        cmd.session.non_stop_on();
        let short_header = if let Some(user) = &cmd.session.current_user {
            user.flags.use_short_filedescr
        } else {
            false
        };
        cmd.session.num_lines_printed = 0;
        let colors = cmd.get_board().await.config.color_configuration.clone();
        for entry in &mut self.files {
            let date = entry.date();
            let size = entry.size();
            let name = &entry.file_name;
            cmd.set_color(TerminalTarget::Both, colors.file_name.clone()).await?;
            cmd.print(TerminalTarget::Both, &format!("{:<12} ", name)).await?;
            if name.len() > 12 {
                cmd.new_line().await?;
            }
            cmd.set_color(TerminalTarget::Both, colors.file_size.clone()).await?;
            cmd.print(TerminalTarget::Both, &format!("{:>8}  ", humanize_bytes_decimal!(size).to_string()))
                .await?;

            /* Filebase no longer supports offline files
                cmd.set_color(TerminalTarget::Both, colors.file_offline.clone()).await?;
                cmd.print(TerminalTarget::Both, &format!("{:>8}  ", "Offline".to_string())).await?;
            */
            cmd.set_color(TerminalTarget::Both, colors.file_date.clone()).await?;
            cmd.print(TerminalTarget::Both, &format!("{}", date.format("%m/%d/%y"))).await?;
            if false {
                cmd.set_color(TerminalTarget::Both, colors.file_new_file.clone()).await?;
                cmd.print(TerminalTarget::Both, "*").await?;
                cmd.reset_color(TerminalTarget::Both).await?;
                cmd.print(TerminalTarget::Both, " ").await?;
            } else {
                cmd.print(TerminalTarget::Both, "  ").await?;
            }

            let mut printed_lines = false;
            match entry.get_metadata() {
                Ok(data) => {
                    for m in data {
                        if m.get_type() == MetadaType::FileID {
                            let description = std::str::from_utf8(&m.data)?;
                            cmd.set_color(TerminalTarget::Both, colors.file_description.clone()).await?;
                            for (i, line) in description.lines().enumerate() {
                                if i > 0 {
                                    cmd.print(TerminalTarget::Both, &format!("{:33}", " ")).await?;
                                }
                                cmd.print(TerminalTarget::Both, line).await?;
                                cmd.new_line().await?;
                                printed_lines = true;
                                if cmd.session.num_lines_printed >= cmd.session.page_len as usize {
                                    cmd.session.num_lines_printed = 0;
                                    Self::filebase_more(cmd, &self.help).await?;
                                    if cmd.session.disp_options.abort_printout {
                                        cmd.session.cancel_batch = cmd.session.disp_options.abort_printout;
                                        return Ok(());
                                    }
                                }
                                if short_header {
                                    break;
                                }
                                cmd.set_color(TerminalTarget::Both, colors.file_description_low.clone()).await?;
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("Error reading metadata: {} for {}", e, entry.full_path.display());
                }
            }
            if !printed_lines {
                cmd.new_line().await?;
            }
        }
        Ok(())
    }

    pub async fn filebase_more(cmd: &mut IcyBoardState, help: &str) -> Res<()> {
        loop {
            let input = cmd
                .input_field(
                    IceText::FilesMorePrompt,
                    40,
                    MASK_COMMAND,
                    help,
                    None,
                    display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFAFTER | display_flags::HIGHASCII,
                )
                .await?;
            cmd.session.more_requested = false;
            cmd.session.num_lines_printed = 0;

            match input.as_str() {
                "F" => {
                    // flag
                    let _input = cmd
                        .input_field(
                            IceText::FlagForDownload,
                            60,
                            &MASK_ASCII,
                            &"hlpflag",
                            None,
                            display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFAFTER | display_flags::HIGHASCII,
                        )
                        .await?;
                    /*
                    if !input.is_empty() {
                        for f in files {
                            if f.name().eq_ignore_ascii_case(&input) {
                                let metadata = f.get_metadata()?;
                                for m in metadata {
                                    if m.metadata_type == MetadaType::FilePath {
                                        cmd.display_text(IceText::CheckingFileTransfer, display_flags::NEWLINE).await?;

                                        let file_name = unsafe { OsString::from_encoded_bytes_unchecked(m.data.clone()) };
                                        let file_path = PathBuf::from(file_name);
                                        if !file_path.exists() {
                                            cmd.session.op_text = input.clone();
                                            cmd.display_text(IceText::NotFoundOnDisk, display_flags::NEWLINE | display_flags::LFBEFORE)
                                                .await?;
                                        } else {
                                            cmd.session.flag_for_download(file_path.clone());
                                            cmd.set_color(TerminalTarget::Both, IcbColor::Dos(10)).await?;
                                            cmd.println(
                                                TerminalTarget::Both,
                                                &format!(
                                                    "{:<12} {}",
                                                    file_path.file_name().unwrap().to_string_lossy(),
                                                    humanize_bytes_decimal!(f.size()).to_string()
                                                ),
                                            )
                                            .await?;
                                        }
                                        break;
                                    }
                                }
                                break;
                            }
                        }
                    }*/
                }
                "V" => {
                    // view: TODO
                    cmd.println(TerminalTarget::Both, "TODO").await?;
                }
                "S" => {
                    // show: TODO
                    cmd.println(TerminalTarget::Both, "TODO").await?;
                }
                "G" => {
                    cmd.goodbye_cmd().await?;
                }
                _ => {
                    if input.to_ascii_uppercase() == cmd.session.no_char.to_string() {
                        cmd.session.disp_options.abort_printout = true;
                    }
                    return Ok(());
                }
            }
        }
    }
}
