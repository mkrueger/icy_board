use crate::{icy_board::state::IcyBoardState, Res};
use chrono::{DateTime, Local};
use dizbase::file_base::FileBase;

use super::l_find_files::FileList;

impl IcyBoardState {
    pub async fn find_new_files(&mut self, help: &str, time_stamp: DateTime<Local>) -> Res<()> {
        for area in 0..self.session.current_conference.directories.len() {
            if self.session.current_conference.directories[area].list_security.user_can_access(&self.session) {
                self.find_newer_files(help, area, time_stamp).await?;
            }
            if self.session.cancel_batch {
                break;
            }
        }

        Ok(())
    }

    async fn find_newer_files(&mut self, help: &str, area: usize, time_stamp: DateTime<Local>) -> Res<()> {
        let file_base_path = self.resolve_path(&self.session.current_conference.directories[area].path);
        let Ok(mut base) = FileBase::open(&file_base_path) else {
            log::error!("Could not open file base: {}", file_base_path.display());
            return Ok(());
        };

        let files = base.find_newer_files(time_stamp)?;

        let mut list = FileList::new(files, help);
        list.display_file_list(self).await
    }
}
