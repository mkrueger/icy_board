use dizbase::file_base::{file_header::FileHeader, metadata::MetadaType, FileBase};
use humanize_bytes::humanize_bytes_decimal;
use icy_board_engine::{
    icy_board::{
        commands::Command,
        icb_config::IcbColor,
        icb_text::IceText,
        state::{
            functions::{display_flags, MASK_ASCII},
            UserActivity,
        },
    },
    vm::TerminalTarget,
};
use icy_ppe::Res;

use super::{PcbBoardCommand, MASK_COMMAND};

impl PcbBoardCommand {
    pub fn find_files(&mut self, action: &Command) -> Res<()> {
        self.state.node_state.lock().unwrap().user_activity = UserActivity::BrowseFiles;

        if self.state.session.current_file_areas.is_empty() {
            self.state
                .display_text(IceText::NoDirectoriesAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)?;
            self.state.press_enter()?;
            return Ok(());
        }
        let search_pattern = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            self.state.input_field(
                IceText::SearchFileName,
                40,
                &MASK_ASCII,
                &action.help,
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::HIGHASCII,
            )?
        };
        if search_pattern.is_empty() {
            self.state.press_enter()?;
            self.display_menu = true;
            return Ok(());
        }

        let search_area = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            self.state.input_field(
                if self.state.session.expert_mode {
                    IceText::FileNumberExpertmode
                } else {
                    IceText::FileNumberNovice
                },
                40,
                MASK_COMMAND,
                &action.help,
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::HIGHASCII,
            )?
        };
        if search_area.is_empty() {
            self.state.press_enter()?;
            self.display_menu = true;
            return Ok(());
        }

        let mut joined = false;
        if search_area == "A" {
            self.state.session.cancel_batch = false;
            for area in 0..self.state.session.current_file_areas.len() {
                if self.state.session.current_file_areas[area].list_security.user_can_access(&self.state.session) {
                    self.search_file_area(action, area, search_pattern.clone())?;
                }
                if self.state.session.cancel_batch {
                    break;
                }
            }
            joined = true;
        } else if let Ok(number) = search_area.parse::<i32>() {
            if 1 <= number && (number as usize) <= self.state.session.current_file_areas.len() {
                let area = &self.state.session.current_file_areas[number as usize - 1];

                if area.list_security.user_can_access(&self.state.session) {
                    self.search_file_area(action, number as usize - 1, search_pattern)?;
                }

                joined = true;
            }
        }

        if !joined {
            self.state.session.op_text = search_area;
            self.state
                .display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::NOTBLANK)?;
        }

        self.state.new_line()?;
        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }

    fn search_file_area(&mut self, action: &Command, area: usize, search_pattern: String) -> Res<()> {
        let file_base_path = self.state.resolve_path(&self.state.session.current_file_areas[area].file_base);
        let Ok(mut base) = FileBase::open(&file_base_path) else {
            log::error!("Could not open file base: {}", file_base_path.display());
            self.state.session.op_text = self.state.session.current_file_areas[area].file_base.to_str().unwrap().to_string();
            self.state
                .display_text(IceText::NotFoundOnDisk, display_flags::NEWLINE | display_flags::LFBEFORE)?;
            return Ok(());
        };

        self.state.display_text(IceText::ScanningDirectory, display_flags::DEFAULT)?;
        self.state.print(TerminalTarget::Both, &format!(" {} ", area + 1))?;
        self.state.set_color(TerminalTarget::Both, IcbColor::Dos(10))?;
        self.state
            .print(TerminalTarget::Both, &format!("({})", self.state.session.current_file_areas[area].name))?;
        self.state.new_line()?;
        base.load_headers()?;
        let files = base.find_files(search_pattern.as_str())?;
        self.display_files(action, &base, files)
    }

    pub fn find_new_files(&mut self, action: &Command, time_stamp: u64) -> Res<()> {
        for area in 0..self.state.session.current_file_areas.len() {
            if self.state.session.current_file_areas[area].list_security.user_can_access(&self.state.session) {
                self.find_newer_files(action, area, time_stamp)?;
            }
            if self.state.session.cancel_batch {
                break;
            }
        }

        Ok(())
    }

    fn find_newer_files(&mut self, action: &Command, area: usize, time_stamp: u64) -> Res<()> {
        let file_base_path = self.state.resolve_path(&self.state.session.current_file_areas[area].file_base);
        let Ok(mut base) = FileBase::open(&file_base_path) else {
            log::error!("Could not open file base: {}", file_base_path.display());
            return Ok(());
        };
        base.load_headers()?;
        let files = base.find_newer_files(time_stamp)?;
        self.display_files(action, &base, files)
    }

    pub fn zippy_directory_scan(&mut self, action: &Command) -> Res<()> {
        if self.state.session.current_file_areas.is_empty() {
            self.state
                .display_text(IceText::NoDirectoriesAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)?;
            self.state.press_enter()?;
            return Ok(());
        }
        let search_pattern = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            self.state.input_field(
                IceText::TextToScanFor,
                40,
                &MASK_ASCII,
                &action.help,
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::HIGHASCII,
            )?
        };
        if search_pattern.is_empty() {
            self.state.press_enter()?;
            self.display_menu = true;
            return Ok(());
        }

        let search_area = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            self.state.input_field(
                if self.state.session.expert_mode {
                    IceText::FileNumberExpertmode
                } else {
                    IceText::FileNumberNovice
                },
                40,
                MASK_COMMAND,
                &action.help,
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::HIGHASCII,
            )?
        };
        if search_area.is_empty() {
            self.state.press_enter()?;
            self.display_menu = true;
            return Ok(());
        }

        let mut joined = false;
        if search_area == "A" {
            self.state.session.cancel_batch = false;
            for area in 0..self.state.session.current_file_areas.len() {
                if self.state.session.current_file_areas[area].list_security.user_can_access(&self.state.session) {
                    self.pattern_search_file_area(action, area, search_pattern.clone())?;
                }
                if self.state.session.cancel_batch {
                    break;
                }
            }
            joined = true;
        } else if let Ok(number) = search_area.parse::<i32>() {
            if 1 <= number && (number as usize) <= self.state.session.current_file_areas.len() {
                let area = &self.state.session.current_file_areas[number as usize - 1];

                if area.list_security.user_can_access(&self.state.session) {
                    self.pattern_search_file_area(action, number as usize - 1, search_pattern)?;
                }

                joined = true;
            }
        }

        if !joined {
            self.state.session.op_text = search_area;
            self.state
                .display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::NOTBLANK)?;
        }

        self.state.new_line()?;
        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }

    fn pattern_search_file_area(&mut self, action: &Command, area: usize, search_pattern: String) -> Res<()> {
        let file_base_path = self.state.resolve_path(&self.state.session.current_file_areas[area].file_base);
        let Ok(mut base) = FileBase::open(&file_base_path) else {
            log::error!("Could not open file base: {}", file_base_path.display());
            self.state.session.op_text = self.state.session.current_file_areas[area].file_base.to_str().unwrap().to_string();
            self.state
                .display_text(IceText::NotFoundOnDisk, display_flags::NEWLINE | display_flags::LFBEFORE)?;
            return Ok(());
        };

        self.state.display_text(IceText::ScanningDirectory, display_flags::DEFAULT)?;
        self.state.print(TerminalTarget::Both, &format!(" {} ", area + 1))?;
        self.state.set_color(TerminalTarget::Both, IcbColor::Dos(10))?;
        self.state
            .print(TerminalTarget::Both, &format!("({})", self.state.session.current_file_areas[area].name))?;
        self.state.new_line()?;
        base.load_headers()?;
        let files = base.find_files_with_pattern(search_pattern.as_str())?;
        self.display_files(action, &base, files)
    }

    fn display_files(&mut self, action: &Command, base: &FileBase, files: Vec<&FileHeader>) -> Res<()> {
        self.state.session.disable_auto_more = true;
        for header in &files {
            let metadata = base.read_metadata(header)?;

            let size = header.size();
            let date = header.file_date().unwrap();
            let name = header.name();
            self.state.set_color(TerminalTarget::Both, IcbColor::Dos(14))?;
            self.state.print(TerminalTarget::Both, &format!("{:<12} ", name))?;
            if name.len() > 12 {
                self.state.new_line()?;
            }
            self.state.set_color(TerminalTarget::Both, IcbColor::Dos(2))?;
            self.state
                .print(TerminalTarget::Both, &format!("{:>8}  ", humanize_bytes_decimal!(size).to_string()))?;
            self.state.set_color(TerminalTarget::Both, IcbColor::Dos(4))?;
            self.state.print(TerminalTarget::Both, &format!("{}", date.format("%m/%d/%C")))?;
            self.state.set_color(TerminalTarget::Both, IcbColor::Dos(3))?;

            self.state.print(TerminalTarget::Both, "  ")?;

            let mut printed_lines = false;
            for m in metadata {
                if m.get_type() == MetadaType::FileID {
                    let description = std::str::from_utf8(&m.data)?;
                    self.state.set_color(TerminalTarget::Both, IcbColor::Dos(11))?;
                    for (i, line) in description.lines().enumerate() {
                        if i > 0 {
                            self.state.print(TerminalTarget::Both, &format!("{:33}", " "))?;
                        }
                        self.state.print(TerminalTarget::Both, line)?;
                        self.state.new_line()?;
                        printed_lines = true;
                        if self.state.session.more_requested && !self.filebase_more(action)? {
                            self.state.session.cancel_batch = true;
                            return Ok(());
                        }
                        self.state.set_color(TerminalTarget::Both, IcbColor::Dos(3))?;
                    }
                }
            }
            if !printed_lines {
                self.state.new_line()?;
            }
        }

        self.state.session.disable_auto_more = false;
        self.state.session.more_requested = false;
        Ok(())
    }
}
