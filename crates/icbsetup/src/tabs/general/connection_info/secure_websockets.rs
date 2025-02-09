use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ListItem, ListValue, ResultState},
    get_text,
    icbconfigmenu::ICBConfigMenuUI,
    tab_page::{Page, PageMessage},
};

pub struct SecureWebsockets {
    menu: ICBConfigMenuUI,
}

impl SecureWebsockets {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_width = 12;
            let entry = vec![
                ConfigEntry::Separator,
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("connection_info_enabled"),
                        ListValue::Bool(lock.config.login_server.secure_websocket.is_enabled),
                    )
                    .with_status(&get_text("connection_info_enabled-status"))
                    .with_label_width(label_width)
                    .with_update_bool_value(&|board: &Arc<Mutex<IcyBoard>>, value: bool| {
                        board.lock().unwrap().config.login_server.secure_websocket.is_enabled = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("connection_info_port"),
                        ListValue::U32(lock.config.login_server.secure_websocket.port as u32, 0, u16::MAX as u32),
                    )
                    .with_status(&get_text("connection_info_port-status"))
                    .with_label_width(label_width)
                    .with_update_u32_value(&|board: &Arc<Mutex<IcyBoard>>, value: u32| {
                        board.lock().unwrap().config.login_server.secure_websocket.port = value as u16;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("connection_info_address"),
                        ListValue::Text(60, lock.config.login_server.secure_websocket.address.clone()),
                    )
                    .with_status(&get_text("connection_info_address-status"))
                    .with_label_width(label_width)
                    .with_update_text_value(&|board: &Arc<Mutex<IcyBoard>>, value: String| {
                        board.lock().unwrap().config.login_server.secure_websocket.address = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("connection_info_display_file"),
                        ListValue::Path(lock.config.login_server.secure_websocket.display_file.clone()),
                    )
                    .with_status(&get_text("connection_info_display_file-status"))
                    .with_label_width(label_width)
                    .with_update_path_value(&|board: &Arc<Mutex<IcyBoard>>, value: PathBuf| {
                        board.lock().unwrap().config.login_server.secure_websocket.display_file = value;
                    }),
                ),
            ];
            ConfigMenu { obj: icy_board.clone(), entry }
        };

        Self {
            menu: ICBConfigMenuUI::new(get_text("connection_info_secure_websockets"), menu),
        }
    }
}

impl Page for SecureWebsockets {
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
