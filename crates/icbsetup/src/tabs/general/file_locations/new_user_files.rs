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

pub struct NewUserFiles {
    menu: ICBConfigMenuUI,
}

impl NewUserFiles {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_width = 26;
            let entry = vec![
                ConfigEntry::Separator,
                cfg_entry_path!("paths_newask_survey", label_width, paths, newask_survey, lock),
                cfg_entry_path!("paths_newask_answer", label_width, paths, newask_answer, lock),
                ConfigEntry::Separator,
                cfg_entry_path!("paths_logon_survey", label_width, paths, logon_survey, lock),
                cfg_entry_path!("paths_logon_answer", label_width, paths, logon_answer, lock),
                ConfigEntry::Separator,
                cfg_entry_path!("paths_logoff_survey", label_width, paths, logoff_survey, lock),
                cfg_entry_path!("paths_logoff_answer", label_width, paths, logoff_answer, lock),
            ];
            ConfigMenu { obj: icy_board.clone(), entry }
        };

        Self {
            menu: ICBConfigMenuUI::new(get_text("file_locations_surveys"), menu),
        }
    }
}

impl Page for NewUserFiles {
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
