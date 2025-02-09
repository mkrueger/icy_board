use std::sync::{Arc, Mutex};

use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    cfg_entry_color,
    config_menu::{ConfigEntry, ConfigMenu, ResultState},
    get_text,
    icbconfigmenu::ICBConfigMenuUI,
    tab_page::Page,
};

pub struct ColorOptions {
    menu: ICBConfigMenuUI,
}

impl ColorOptions {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_width = 18;
            let edge = vec![
                ConfigEntry::Separator,
                cfg_entry_color!("default_color", label_width, color_configuration, default, lock),
                cfg_entry_color!("msg_hdr_date", label_width, color_configuration, msg_hdr_date, lock),
                cfg_entry_color!("msg_hdr_to", label_width, color_configuration, msg_hdr_to, lock),
                cfg_entry_color!("msg_hdr_from", label_width, color_configuration, msg_hdr_from, lock),
                cfg_entry_color!("msg_hdr_subj", label_width, color_configuration, msg_hdr_subj, lock),
                cfg_entry_color!("msg_hdr_read", label_width, color_configuration, msg_hdr_read, lock),
                cfg_entry_color!("msg_hdr_conf", label_width, color_configuration, msg_hdr_conf, lock),
                ConfigEntry::Separator,
                cfg_entry_color!("file_head", label_width, color_configuration, file_head, lock),
                cfg_entry_color!("file_name", label_width, color_configuration, file_name, lock),
                cfg_entry_color!("file_size", label_width, color_configuration, file_size, lock),
                cfg_entry_color!("file_date", label_width, color_configuration, file_date, lock),
                cfg_entry_color!("file_description", label_width, color_configuration, file_description, lock),
                cfg_entry_color!("file_description_low", label_width, color_configuration, file_description_low, lock),
                cfg_entry_color!("file_text", label_width, color_configuration, file_text, lock),
                cfg_entry_color!("file_deleted", label_width, color_configuration, file_deleted, lock),
            ];
            ConfigMenu {
                obj: icy_board.clone(),
                entry: edge,
            }
        };

        Self {
            menu: ICBConfigMenuUI::new(get_text("configuration_options_colors"), menu),
        }
    }
}

impl Page for ColorOptions {
    fn render(&mut self, frame: &mut ratatui::Frame, disp_area: ratatui::prelude::Rect) {
        self.menu.render(frame, disp_area)
    }
    fn request_status(&self) -> ResultState {
        self.menu.request_status()
    }
    fn handle_key_press(&mut self, key: crossterm::event::KeyEvent) -> icy_board_tui::tab_page::PageMessage {
        self.menu.handle_key_press(key)
    }
}
