use crate::icy_board::state::functions::MASK_COMMAND;
use crate::icy_board::state::user_commands::mods::filebrowser::FileList;
use crate::icy_board::state::IcyBoardState;
use crate::Res;
use crate::{
    icy_board::{
        icb_text::IceText,
        state::{functions::display_flags, NodeStatus},
    },
    vm::TerminalTarget,
};

impl IcyBoardState {
    pub async fn show_file_directories(&mut self, help: &str) -> Res<()> {
        self.set_activity(NodeStatus::Available).await;

        self.session.non_stop_off();
        self.session.more_requested = false;

        if self.session.current_conference.directories.is_empty() {
            self.display_text(IceText::NoDirectoriesAvailable, display_flags::NEWLINE | display_flags::LFBEFORE)
                .await?;
            self.press_enter().await?;
            return Ok(());
        }
        let directory_number = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            let mnu = self.session.current_conference.dir_menu.clone();
            let mnu = self.resolve_path(&mnu);
            self.display_menu(&mnu).await?;
            self.new_line().await?;

            self.input_field(
                if self.session.expert_mode {
                    IceText::FileListCommandExpert
                } else {
                    IceText::FileListCommand
                },
                40,
                MASK_COMMAND,
                help,
                None,
                display_flags::NEWLINE | display_flags::LFAFTER | display_flags::HIGHASCII,
            )
            .await?
        };

        if !directory_number.is_empty() {
            let mut joined = false;
            if let Ok(number) = directory_number.parse::<i32>() {
                if 1 <= number && (number as usize) <= self.session.current_conference.directories.len() {
                    let area = &self.session.current_conference.directories[number as usize - 1];
                    if area.list_security.user_can_access(&self.session) {
                        self.display_file_area(help, number as usize - 1).await?;
                    }
                    joined = true;
                }
            }

            if !joined {
                self.session.op_text = directory_number;
                self.display_text(IceText::InvalidEntry, display_flags::NEWLINE | display_flags::NOTBLANK)
                    .await?;
            }
        }
        self.new_line().await?;
        self.press_enter().await?;
        self.display_current_menu = true;
        Ok(())
    }

    async fn display_file_area(&mut self, help: &str, area: usize) -> Res<()> {
        self.session.disp_options.file_list_help = help.to_string();
        let area = &self.session.current_conference.directories[area];

        let colors = self.get_board().await.config.color_configuration.clone();
        let file_base_path = self.resolve_path(&area.path);
        let Ok(base) = self.get_filebase(&file_base_path).await else {
            return Ok(());
        };

        self.clear_screen(TerminalTarget::Both).await?;

        self.set_color(TerminalTarget::Both, colors.file_head).await?;
        self.println(TerminalTarget::Both, "Filename       Size      Date    Description of File Contents")
            .await?;
        self.println(
            TerminalTarget::Both,
            "============ ========  ========  ============================================",
        )
        .await?;

        let files = {
            let mut base = base.lock().await;
            base.file_headers
                .iter_mut()
                .map(|f| {
                    let _ = f.get_metadata();
                    f.clone()
                })
                .collect::<Vec<_>>()
        };
        let mut list = FileList::new(file_base_path.clone(), files);
        list.display_file_list(self).await?;

        if self.session.num_lines_printed > 0 {
            self.filebase_more().await?;
        }
        self.session.non_stop_off();
        self.session.more_requested = false;
        Ok(())
    }
}
