use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::ResultState,
    get_text,
    icbsetupmenu::IcbSetupMenuUI,
    select_menu::{MenuItem, SelectMenu},
    tab_page::{Page, PageMessage},
};
use ratatui::{Frame, layout::Rect};

mod addresses;
mod nodes;
mod options;

/// `PCBoard` reached the whole fidonet setup from one menu, and a sysop coming
/// from there looks for these names in this order.
pub struct FidoConfiguration {
    page: IcbSetupMenuUI,
    icy_board: Arc<Mutex<IcyBoard>>,
}

impl FidoConfiguration {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        Self {
            page: IcbSetupMenuUI::new(SelectMenu::new(vec![
                MenuItem::new(0, 'A', get_text("fido_menu_processing")),
                MenuItem::new(1, 'B', get_text("fido_menu_tosser")),
                MenuItem::new(2, 'C', get_text("fido_menu_nodes")),
                MenuItem::new(3, 'D', get_text("fido_menu_addresses")),
                MenuItem::new(4, 'E', get_text("fido_menu_directories")),
                MenuItem::new(5, 'F', get_text("fido_menu_origin")),
            ]))
            .with_center_title(get_text("fido_config_title")),
            icy_board,
        }
    }
}

impl Page for FidoConfiguration {
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
            let board = self.icy_board.clone();
            return match selected {
                0 => PageMessage::OpenSubPage(Box::new(options::processing(board))),
                1 => PageMessage::OpenSubPage(Box::new(options::tosser(board))),
                2 => PageMessage::OpenSubPage(Box::new(nodes::NodeConfiguration::new(board))),
                3 => PageMessage::OpenSubPage(Box::new(addresses::SystemAddresses::new(board))),
                4 => PageMessage::OpenSubPage(Box::new(options::directories(board))),
                5 => PageMessage::OpenSubPage(Box::new(options::origin(board))),
                _ => PageMessage::None,
            };
        }
        PageMessage::None
    }
}
