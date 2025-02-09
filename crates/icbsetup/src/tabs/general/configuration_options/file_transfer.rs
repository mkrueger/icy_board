use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    cfg_entry_bool, cfg_entry_u32,
    config_menu::{ConfigEntry, ConfigMenu, ResultState},
    get_text,
    icbconfigmenu::ICBConfigMenuUI,
    tab_page::{Page, PageMessage},
};

pub struct FileTransfers {
    menu: ICBConfigMenuUI,
}

impl FileTransfers {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_with = 31;
            let entry = vec![
                ConfigEntry::Separator,
                cfg_entry_bool!("disallow_batch_uploads", label_with, file_transfer, disallow_batch_uploads, lock),
                cfg_entry_bool!("promote_to_batch_transfers", label_with, file_transfer, promote_to_batch_transfers, lock),
                cfg_entry_u32!("upload_credit_time", label_with, 0, 10000, file_transfer, upload_credit_time, lock),
                cfg_entry_u32!("upload_credit_bytes", label_with, 0, 10000, file_transfer, upload_credit_bytes, lock),
                cfg_entry_bool!("display_uploader", label_with, file_transfer, display_uploader, lock),
                cfg_entry_bool!("verify_files_uploaded", label_with, file_transfer, verify_files_uploaded, lock),
                cfg_entry_bool!("disable_drive_size_check", label_with, file_transfer, disable_drive_size_check, lock),
                ConfigEntry::Separator,
                cfg_entry_u32!("stop_uploads_free_space", 41, 0, 1024 * 1024, file_transfer, stop_uploads_free_space, lock),
            ];
            ConfigMenu { obj: icy_board.clone(), entry }
        };

        Self {
            menu: ICBConfigMenuUI::new(get_text("configuration_options_file_transfer"), menu),
        }
    }
}

impl Page for FileTransfers {
    fn render(&mut self, frame: &mut ratatui::Frame, disp_area: ratatui::prelude::Rect) {
        self.menu.render(frame, disp_area)
    }
    fn request_status(&self) -> ResultState {
        self.menu.request_status()
    }
    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        self.menu.handle_key_press(key)
    }
}
