use crate::{menu_runner::PcbBoardCommand, Res};
use dizbase::file_base::FileBase;
use icy_board_engine::icy_board::commands::Command;

use super::l_find_files::FileList;

impl PcbBoardCommand {
    pub async fn find_new_files(&mut self, action: &Command, time_stamp: u64) -> Res<()> {
        for area in 0..self.state.session.current_conference.directories.len() {
            if self.state.session.current_conference.directories[area]
                .list_security
                .user_can_access(&self.state.session)
            {
                self.find_newer_files(action, area, time_stamp).await?;
            }
            if self.state.session.cancel_batch {
                break;
            }
        }

        Ok(())
    }

    async fn find_newer_files(&mut self, action: &Command, area: usize, time_stamp: u64) -> Res<()> {
        let file_base_path = self.state.resolve_path(&self.state.session.current_conference.directories[area].file_base);
        let Ok(mut base) = FileBase::open(&file_base_path) else {
            log::error!("Could not open file base: {}", file_base_path.display());
            return Ok(());
        };
        base.load_headers()?;

        let files = base.find_newer_files(time_stamp)?;

        let mut list = FileList {
            base: &base,
            files,
            help: &action.help,
        };
        list.display_file_list(self).await
    }
}
