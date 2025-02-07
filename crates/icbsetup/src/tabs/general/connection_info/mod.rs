use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::ResultState,
    get_text,
    select_menu::{MenuItem, SelectMenu},
    tab_page::{Page, PageMessage},
};
use ratatui::{layout::Rect, Frame};
use secure_websockets::SecureWebsockets;
use websockets::Websockets;

use super::IcbSetupMenuUI;

mod secure_websockets;
mod ssh;
mod telnet;
mod websockets;

pub struct ConnectionInfo {
    pub page: IcbSetupMenuUI,
    icy_board: Arc<Mutex<IcyBoard>>,
}

impl ConnectionInfo {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        Self {
            page: IcbSetupMenuUI::new(SelectMenu::new(vec![
                MenuItem::new(0, 'A', get_text("connection_info_telnet")),
                MenuItem::new(1, 'B', get_text("connection_info_ssh")),
                MenuItem::new(2, 'C', get_text("connection_info_websockets")),
                MenuItem::new(3, 'D', get_text("connection_info_secure_websockets")),
            ]))
            .with_center_title(get_text("connection_info_title")),
            icy_board,
        }
    }
}

impl Page for ConnectionInfo {
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
                0 => PageMessage::OpenSubPage(Box::new(telnet::Telnet::new(self.icy_board.clone()))),
                1 => PageMessage::OpenSubPage(Box::new(ssh::SSH::new(self.icy_board.clone()))),
                2 => PageMessage::OpenSubPage(Box::new(Websockets::new(self.icy_board.clone()))),
                3 => PageMessage::OpenSubPage(Box::new(SecureWebsockets::new(self.icy_board.clone()))),
                _ => PageMessage::None,
            };
        }
        PageMessage::None
    }
}
