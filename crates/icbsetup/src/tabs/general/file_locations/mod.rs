use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use display_files::DisplayFiles;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::ResultState,
    get_text,
    select_menu::{MenuItem, SelectMenu},
    tab_page::{Page, PageMessage},
};
use new_user_files::NewUserFiles;
use ratatui::{layout::Rect, Frame};
use system_files::SystemFiles;

use super::IcbSetupMenuUI;

mod configuration_files;
mod display_files;
mod new_user_files;
mod system_files;

pub struct FileLocations {
    pub page: IcbSetupMenuUI,
    icy_board: Arc<Mutex<IcyBoard>>,
}

impl FileLocations {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        Self {
            page: IcbSetupMenuUI::new(SelectMenu::new(vec![
                MenuItem::new(0, 'A', get_text("file_locations_files_dirs")),
                MenuItem::new(1, 'B', get_text("file_locations_config_files")),
                MenuItem::new(2, 'C', get_text("file_locations_display_files")),
                MenuItem::new(3, 'D', get_text("file_locations_surveys")),
            ]))
            .with_center_title(get_text("file_locations_title")),
            icy_board,
        }
    }
}

impl Page for FileLocations {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.page.render(frame, area);
    }

    fn request_status(&self) -> ResultState {
        self.page.request_status()
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if key.code == crossterm::event::KeyCode::Esc {
            return PageMessage::Close;
        }
        let (_state, opt) = self.page.handle_key_press(key);
        if let Some(selected) = opt {
            return match selected {
                0 => PageMessage::OpenSubPage(Box::new(SystemFiles::new(self.icy_board.clone()))),
                1 => PageMessage::OpenSubPage(Box::new(configuration_files::ConfigurationFiles::new(self.icy_board.clone()))),
                2 => PageMessage::OpenSubPage(Box::new(DisplayFiles::new(self.icy_board.clone()))),
                3 => PageMessage::OpenSubPage(Box::new(NewUserFiles::new(self.icy_board.clone()))),
                _ => PageMessage::None,
            };
        }
        PageMessage::None
    }
}
