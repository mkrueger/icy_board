use dizbase::file_base::{metadata::MetadaType, FileBase};
use humanize_bytes::humanize_bytes_decimal;
use icy_board_engine::{
    icy_board::{commands::Command, file_areas::FileAreaList, icb_config::IcbColor, icb_text::IceText, state::functions::display_flags, IcyBoardSerializer},
    vm::TerminalTarget,
};
use icy_ppe::Res;

use super::{PcbBoardCommand, MASK_COMMAND};

impl PcbBoardCommand {
    pub async fn show_file_directories(&mut self, action: &Command) -> Res<()> {
        let file_area_file = self.state.resolve_path(&self.state.session.current_conference.file_area_file).await;

        let file_areas = FileAreaList::load(&file_area_file)?;
        if file_areas.is_empty() {
            self.state
                .display_text(IceText::NoDirectoriesAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            self.state.press_enter().await;
            return Ok(());
        }
        let directory_number = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            let mnu = self.state.session.current_conference.file_area_menu.clone();
            let mnu = self.state.resolve_path(&mnu).await;
            self.state.display_menu(&mnu).await?;
            self.state.new_line().await?;

            self.state
                .input_field(
                    if self.state.session.expert_mode {
                        IceText::FileListCommandExpert
                    } else {
                        IceText::FileListCommand
                    },
                    40,
                    MASK_COMMAND,
                    &action.help,
                    display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
                )
                .await?
        };
        let mut joined = false;
        if let Ok(number) = directory_number.parse::<i32>() {
            if 1 <= number && (number as usize) <= file_areas.len() {
                let area = &file_areas[number as usize - 1];

                if area.list_security.user_can_access(&self.state.session) {
                    self.display_file_area(action, &area).await?;
                }

                joined = true;
            }
        }

        if !joined {
            self.state.session.op_text = directory_number;
            self.state
                .display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::NOTBLANK)
                .await?;
        }

        self.state.new_line().await?;
        self.state.press_enter().await;
        self.display_menu = true;
        Ok(())
    }

    async fn display_file_area(&mut self, action: &Command, area: &&icy_board_engine::icy_board::file_areas::FileArea) -> Res<()> {
        let file_base_path = self.state.resolve_path(&area.file_base).await;
        let Ok(base) = FileBase::open(&file_base_path) else {
            log::error!("Could not open file base: {}", file_base_path.display());
            self.state.session.op_text = area.file_base.to_str().unwrap().to_string();
            self.state
                .display_text(IceText::NotFoundOnDisk, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            return Ok(());
        };

        self.state.clear_screen().await?;

        self.state.set_color(IcbColor::Dos(6)).await?;
        self.state
            .print(TerminalTarget::Both, "Filename       Size      Date    Description of File Contents")
            .await?;
        self.state.new_line().await?;
        self.state
            .print(
                TerminalTarget::Both,
                "============ ========  ========  ============================================",
            )
            .await?;
        self.state.new_line().await?;

        let mut files = base.iter();
        self.state.session.disable_auto_more = true;
        while let Some(file) = files.next() {
            let header = file?;

            let metadata = base.read_metadata(&header)?;

            let size = header.size();
            let date = header.file_date().unwrap();
            let name = header.name();
            self.state.set_color(IcbColor::Dos(14)).await?;
            self.state.print(TerminalTarget::Both, &format!("{:<12} ", name)).await?;
            if name.len() > 12 {
                self.state.new_line().await?;
            }
            self.state.set_color(IcbColor::Dos(2)).await?;
            self.state
                .print(TerminalTarget::Both, &format!("{:>8}  ", humanize_bytes_decimal!(size).to_string()))
                .await?;
            self.state.set_color(IcbColor::Dos(4)).await?;
            self.state.print(TerminalTarget::Both, &format!("{}", date.format("%m/%d/%C"))).await?;
            self.state.set_color(IcbColor::Dos(3)).await?;

            self.state.print(TerminalTarget::Both, "  ").await?;

            let mut printed_lines = false;
            for m in metadata {
                if m.get_type() == MetadaType::FileID {
                    let description = std::str::from_utf8(&m.data)?;
                    self.state.set_color(IcbColor::Dos(11)).await?;
                    for (i, line) in description.lines().enumerate() {
                        if i > 0 {
                            self.state.print(TerminalTarget::Both, &format!("{:33}", " ")).await?;
                        }
                        self.state.print(TerminalTarget::Both, line).await?;
                        self.state.new_line().await?;
                        printed_lines = true;
                        if self.state.session.more_requested && !self.filebase_more(action).await? {
                            return Ok(());
                        }
                        self.state.set_color(IcbColor::Dos(3)).await?;
                    }
                }
            }
            if !printed_lines {
                self.state.new_line().await?;
            }
        }
        self.filebase_more(action).await?;

        self.state.session.disable_auto_more = false;
        self.state.session.more_requested = false;
        Ok(())
    }

    pub async fn filebase_more(&mut self, action: &Command) -> Res<bool> {
        let input = self
            .state
            .input_field(
                IceText::FilesMorePrompt,
                40,
                MASK_COMMAND,
                &action.help,
                display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
            )
            .await?;
        self.state.session.more_requested = false;
        self.state.session.num_lines_printed = 0;
        Ok(input.to_ascii_uppercase() != self.state.no_char.to_string())
    }
}
