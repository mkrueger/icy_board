use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    cfg_entry_path,
    config_menu::{ConfigEntry, ConfigMenu, ResultState},
    get_text,
    icbconfigmenu::ICBConfigMenuUI,
    tab_page::{Page, PageMessage},
};

use crate::editors::languages::edit_languages;

pub struct ConfigurationFiles {
    menu: ICBConfigMenuUI,
}

impl ConfigurationFiles {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_width = 33;
            let entry = vec![
                ConfigEntry::Separator,
                cfg_entry_path!(
                    "paths_pwrd_sec_level_file",
                    label_width,
                    paths,
                    pwrd_sec_level_file,
                    Box::new(crate::editors::sec_editor::edit_sec),
                    lock
                ),
                cfg_entry_path!(
                    "paths_protocol_data_file",
                    label_width,
                    paths,
                    protocol_data_file,
                    Box::new(crate::editors::protocols::edit_protocols),
                    lock
                ),
                cfg_entry_path!("paths_language_file", label_width, paths, language_file, Box::new(edit_languages), lock),
                cfg_entry_path!(
                    "paths_command_file",
                    label_width,
                    paths,
                    command_file,
                    Box::new(crate::editors::command::edit_commands),
                    lock
                ),
                ConfigEntry::Separator,
                cfg_entry_path!("paths_trashcan_upload_files", label_width, paths, trashcan_upload_files, lock),
                cfg_entry_path!("paths_trashcan_user", label_width, paths, trashcan_user, lock),
                cfg_entry_path!("paths_trashcan_passwords", label_width, paths, trashcan_passwords, lock),
                cfg_entry_path!("paths_trashcan_email", label_width, paths, trashcan_email, lock),
                cfg_entry_path!("paths_vip_users", label_width, paths, vip_users, lock),
            ];
            ConfigMenu { obj: icy_board.clone(), entry }
        };

        Self {
            menu: ICBConfigMenuUI::new(get_text("file_locations_config_files"), menu),
        }
    }
}

impl Page for ConfigurationFiles {
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
