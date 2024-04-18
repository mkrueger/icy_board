use dizbase::file_base::{file_header::FileHeader, FileBase};
use icy_board_engine::{
    icy_board::{
        commands::Command,
        file_areas::FileAreaList,
        icb_text::IceText,
        state::{functions::display_flags, UserActivity},
        IcyBoardSerializer,
    },
    vm::TerminalTarget,
};
use icy_ppe::Res;

use super::{find_files::FileList, PcbBoardCommand, MASK_COMMAND};

impl PcbBoardCommand {
    pub fn show_file_directories(&mut self, action: &Command) -> Res<()> {
        self.state.node_state.lock().unwrap().user_activity = UserActivity::BrowseFiles;
        self.state.session.disable_auto_more = false;
        self.state.session.more_requested = false;

        let file_area_file = self.state.resolve_path(&self.state.session.current_conference.file_area_file);

        let file_areas = FileAreaList::load(&file_area_file)?;
        if file_areas.is_empty() {
            self.state
                .display_text(IceText::NoDirectoriesAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)?;
            self.state.press_enter()?;
            return Ok(());
        }
        let directory_number = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            let mnu = self.state.session.current_conference.file_area_menu.clone();
            let mnu = self.state.resolve_path(&mnu);
            self.state.display_menu(&mnu)?;
            self.state.new_line()?;

            self.state.input_field(
                if self.state.session.expert_mode {
                    IceText::FileListCommandExpert
                } else {
                    IceText::FileListCommand
                },
                40,
                MASK_COMMAND,
                &action.help,
                None,
                display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
            )?
        };

        if !directory_number.is_empty() {
            let mut joined = false;
            if let Ok(number) = directory_number.parse::<i32>() {
                if 1 <= number && (number as usize) <= file_areas.len() {
                    let area = &file_areas[number as usize - 1];

                    if area.list_security.user_can_access(&self.state.session) {
                        self.display_file_area(action, &area)?;
                    }

                    joined = true;
                }
            }

            if !joined {
                self.state.session.op_text = directory_number;
                self.state
                    .display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::NOTBLANK)?;
            }
        }
        self.state.new_line()?;
        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }

    fn display_file_area(&mut self, action: &Command, area: &&icy_board_engine::icy_board::file_areas::FileArea) -> Res<()> {
        let colors = self.state.board.lock().unwrap().config.color_configuration.clone();
        let file_base_path = self.state.resolve_path(&area.file_base);
        let Ok(base) = FileBase::open(&file_base_path) else {
            log::error!("Could not open file base: {}", file_base_path.display());
            self.state.session.op_text = area.file_base.to_str().unwrap().to_string();
            self.state
                .display_text(IceText::NotFoundOnDisk, display_flags::NEWLINE | display_flags::LFBEFORE)?;
            return Ok(());
        };

        self.state.clear_screen()?;

        self.state.set_color(TerminalTarget::Both, colors.file_head)?;
        self.state
            .print(TerminalTarget::Both, "Filename       Size      Date    Description of File Contents")?;
        self.state.new_line()?;
        self.state.print(
            TerminalTarget::Both,
            "============ ========  ========  ============================================",
        )?;
        self.state.new_line()?;

        let files: Vec<FileHeader> = base.iter().flatten().collect();
        let files: Vec<&FileHeader> = files.iter().collect();

        let mut list = FileList {
            base: &base,
            files,
            help: &action.help,
        };
        list.display_file_list(self)?;

        if self.state.session.num_lines_printed > 0 {
            list.filebase_more(self)?;
        }
        self.state.session.disable_auto_more = false;
        self.state.session.more_requested = false;
        Ok(())
    }
}
