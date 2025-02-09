use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    cfg_entry_bool, cfg_entry_text, cfg_entry_u16,
    config_menu::{ConfigEntry, ConfigMenu, ListItem, ListValue, ResultState, Value},
    get_text,
    icbconfigmenu::ICBConfigMenuUI,
    tab_page::{Page, PageMessage},
};

pub struct BoardConfiguration {
    menu: ICBConfigMenuUI,
}

impl BoardConfiguration {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let icy_board2 = icy_board.clone();
            let lock: std::sync::MutexGuard<'_, IcyBoard> = icy_board.lock().unwrap();
            let sysop_info: Vec<ConfigEntry<Arc<Mutex<IcyBoard>>>> = vec![
                ConfigEntry::Separator,
                cfg_entry_text!("board_name", 13, 45, board, name, lock),
                ConfigEntry::Separator,
                cfg_entry_bool!("allow_iemsi", 13, board, allow_iemsi, lock),
                cfg_entry_text!("board_iemsi_location", 13, 54, board, name, lock),
                cfg_entry_text!("board_iemsi_operator", 13, 30, board, operator, lock),
                cfg_entry_text!("board_iemsi_notice", 13, 30, board, notice, lock),
                cfg_entry_text!("board_iemsi_caps", 13, 30, board, capabilities, lock),
                ConfigEntry::Separator,
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("date_format"),
                        ListValue::ValueList(
                            lock.config.board.date_format.clone(),
                            vec![
                                Value::new("MM/DD/YY", "%m/%d/%y"),
                                Value::new("DD/MM/YY", "%d/%m/%y"),
                                Value::new("YY/MM/DD", "%y/%m/%d"),
                                Value::new("MM.DD.YY", "%m.%d.%y"),
                                Value::new("DD.MM.YY", "%d.%m.%y"),
                                Value::new("YY.MM.DD", "%y.%m.%d"),
                                Value::new("MM-DD-YY", "%m-%d-%y"),
                                Value::new("DD-MM-YY", "%d-%m-%y"),
                                Value::new("YY-MM-DD", "%y-%m-%d"),
                            ],
                        ),
                    )
                    .with_status(&get_text("date_format-status"))
                    .with_help(&get_text("date_format-help"))
                    .with_label_width(14)
                    .with_update_value(Box::new(|board: &Arc<Mutex<IcyBoard>>, value: &ListValue| {
                        let ListValue::ValueList(val, _) = value else {
                            return;
                        };
                        board.lock().unwrap().config.board.date_format = val.clone()
                    })),
                ),
                cfg_entry_u16!("board_node_num", 14, 1, 256, board, num_nodes, lock),
                ConfigEntry::Separator,
                cfg_entry_bool!("who_include_city", 33, board, who_include_city, lock),
                cfg_entry_bool!("who_show_alias", 33, board, who_show_alias, lock),
            ];
            ConfigMenu {
                obj: icy_board2,
                entry: sysop_info,
            }
        };

        Self {
            menu: ICBConfigMenuUI::new(get_text("board_config_title"), menu),
        }
    }
}

impl Page for BoardConfiguration {
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
