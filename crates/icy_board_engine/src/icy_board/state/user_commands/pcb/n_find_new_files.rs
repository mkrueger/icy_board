use crate::{
    icy_board::state::{user_commands::mods::filebrowser::FileList, IcyBoardState},
    Res,
};
use chrono::{DateTime, Local};

impl IcyBoardState {
    pub async fn find_new_files(&mut self, time_stamp: DateTime<Local>) -> Res<()> {
        for area in 0..self.session.current_conference.directories.len() {
            if self.session.current_conference.directories[area].list_security.user_can_access(&self.session) {
                self.find_newer_files(area, time_stamp).await?;
            }
            if self.session.cancel_batch {
                break;
            }
        }

        Ok(())
    }

    async fn find_newer_files(&mut self, area: usize, time_stamp: DateTime<Local>) -> Res<()> {
        let file_base_path = self.resolve_path(&self.session.current_conference.directories[area].path);
        let Ok(base) = self.get_filebase(&file_base_path).await else {
            return Ok(());
        };

        let mut b = base.lock().await;
        let files = b.find_newer_files(time_stamp)?;

        let mut list = FileList::new(file_base_path, files);
        list.display_file_list(self).await
    }
}
