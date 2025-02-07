use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ResultState},
    get_text,
    tab_page::{Page, PageMessage},
};

use crate::{cfg_entry_path, tabs::ICBConfigMenuUI};

pub struct SystemFiles {
    menu: ICBConfigMenuUI,
}

fn edit_icbtext(_board: Arc<Mutex<IcyBoard>>, path: PathBuf) -> PageMessage {
    let mkicbtxt = std::env::current_exe().unwrap().with_file_name("mkicbtxt");
    match std::process::Command::new(mkicbtxt).arg(format!("{}", path.display())).spawn() {
        Ok(mut child) => match child.wait() {
            Ok(_) => {
                return PageMessage::ExternalProgramStarted;
            }
            Err(e) => {
                log::error!("Error opening mkicbtxt: {}", e);
                return PageMessage::ResultState(ResultState {
                    edit_mode: icy_board_tui::config_menu::EditMode::None,
                    status_line: format!("Error: {}", e),
                });
            }
        },
        Err(e) => {
            log::error!("Error opening mkicbtxt: {}", e);
            ratatui::init();
            return PageMessage::ResultState(ResultState {
                edit_mode: icy_board_tui::config_menu::EditMode::None,
                status_line: format!("Error: {}", e),
            });
        }
    }
}

impl SystemFiles {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_width = 33;
            let entry = vec![
                ConfigEntry::Separator,
                cfg_entry_path!("paths_conferences", label_width, paths, conferences, lock),
                cfg_entry_path!("paths_home_dir", label_width, paths, home_dir, lock),
                cfg_entry_path!("paths_caller_log", label_width, paths, caller_log, lock),
                cfg_entry_path!("paths_statistic_file", label_width, paths, statistics_file, lock),
                ConfigEntry::Separator,
                cfg_entry_path!("paths_icbtext", label_width, paths, icbtext, Box::new(edit_icbtext), lock),
                cfg_entry_path!("paths_tmp_files", label_width, paths, tmp_work_path, lock),
                cfg_entry_path!("paths_help_path", label_width, paths, help_path, lock),
                cfg_entry_path!("paths_security_file_path", label_width, paths, security_file_path, lock),
                cfg_entry_path!("paths_command_display_path", label_width, paths, command_display_path, lock),
            ];
            ConfigMenu { obj: icy_board.clone(), entry }
        };
        Self {
            menu: ICBConfigMenuUI::new(get_text("file_locations_files_dirs"), menu),
        }
    }
}

impl Page for SystemFiles {
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
