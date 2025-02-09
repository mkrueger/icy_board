use std::sync::{Arc, Mutex};

use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ListItem, ListValue, ResultState},
    get_text,
    icbconfigmenu::ICBConfigMenuUI,
    tab_page::Page,
};

pub struct FunctionKeys {
    menu: ICBConfigMenuUI,
}

impl FunctionKeys {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();

            let function_keys_width = 9;
            let mut entry = Vec::new();
            entry.push(ConfigEntry::Separator);

            for i in 0..10 {
                entry.push(ConfigEntry::Item(
                    ListItem::new(format!("F-Key #{}", i + 1), ListValue::Text(50, lock.config.func_keys[i].to_string()))
                        .with_label_width(function_keys_width)
                        .with_update_value(Box::new(move |board: &Arc<Mutex<IcyBoard>>, value: &ListValue| {
                            let ListValue::Text(_, text) = value else {
                                return;
                            };
                            board.lock().unwrap().config.func_keys[i] = text.clone();
                        })),
                ));
            }
            ConfigMenu { obj: icy_board.clone(), entry }
        };

        Self {
            menu: ICBConfigMenuUI::new(get_text("configuration_options_func_keys"), menu),
        }
    }
}

impl Page for FunctionKeys {
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
