use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    cfg_entry_bool, cfg_entry_text, cfg_entry_u16,
    config_menu::{ConfigEntry, ConfigMenu, ListItem, ListValue, ResultState, TextFlags, Value},
    get_text,
    icbconfigmenu::ICBConfigMenuUI,
    tab_page::{Page, PageMessage},
};

pub struct BoardConfiguration {
    menu: ICBConfigMenuUI,
}

impl BoardConfiguration {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let label_width = 16;
        let menu = {
            let icy_board2 = icy_board.clone();
            let lock: std::sync::MutexGuard<'_, IcyBoard> = icy_board.lock().unwrap();
            let sysop_info: Vec<ConfigEntry<Arc<Mutex<IcyBoard>>>> = vec![
                ConfigEntry::Separator,
                cfg_entry_text!("board_name", label_width, 45, board, name, lock),
                ConfigEntry::Separator,
                cfg_entry_bool!("allow_iemsi", label_width, board, allow_iemsi, lock),
                cfg_entry_text!("board_iemsi_location", label_width, 54, board, location, lock),
                cfg_entry_text!("board_iemsi_operator", label_width, 30, board, operator, lock),
                cfg_entry_text!("board_iemsi_notice", label_width, 30, board, notice, lock),
                cfg_entry_text!("board_iemsi_caps", label_width, 30, board, capabilities, lock),
                ConfigEntry::Separator,
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("date_format"),
                        ListValue::ValueList(
                            lock.config.board.date_format.clone(),
                            vec![
                                Value::new("1) MM/DD/YY", "%m/%d/%y"),
                                Value::new("2) DD/MM/YY", "%d/%m/%y"),
                                Value::new("3) YY/MM/DD", "%y/%m/%d"),
                                Value::new("4) MM.DD.YY", "%m.%d.%y"),
                                Value::new("5) DD.MM.YY", "%d.%m.%y"),
                                Value::new("6) YY.MM.DD", "%y.%m.%d"),
                                Value::new("7) MM-DD-YY", "%m-%d-%y"),
                                Value::new("8) DD-MM-YY", "%d-%m-%y"),
                                Value::new("9) YY-MM-DD", "%y-%m-%d"),
                            ],
                        ),
                    )
                    .with_status(get_text("date_format-status"))
                    .with_help(get_text("date_format-help"))
                    .with_label_width(label_width)
                    .with_update_value(Box::new(|board: &Arc<Mutex<IcyBoard>>, value: &ListValue| {
                        let ListValue::ValueList(val, _) = value else {
                            return;
                        };
                        board.lock().unwrap().config.board.date_format = val.clone()
                    })),
                ),
                cfg_entry_u16!("board_node_num", label_width, 1, 256, board, num_nodes, lock),
                ConfigEntry::Separator,
                cfg_entry_bool!("who_include_city", 33, board, who_include_city, lock),
                cfg_entry_bool!("who_show_alias", 33, board, who_show_alias, lock),
                ConfigEntry::Separator,
                ConfigEntry::Item(
                    ListItem::new(get_text("web_admin_enabled"), ListValue::Bool(lock.config.board.web_admin.enabled))
                        .with_status(get_text("web_admin_enabled-status"))
                        .with_help(get_text("web_admin_enabled-help"))
                        .with_label_width(33)
                        .with_update_bool_value(&|board: &Arc<Mutex<IcyBoard>>, value| {
                            board.lock().unwrap().config.board.web_admin.enabled = value;
                        }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("web_admin_address"),
                        ListValue::Text(45, TextFlags::None, lock.config.board.web_admin.address.clone()),
                    )
                    .with_status(get_text("web_admin_address-status"))
                    .with_help(get_text("web_admin_address-help"))
                    .with_label_width(33)
                    .with_update_text_value(&|board: &Arc<Mutex<IcyBoard>>, value| {
                        board.lock().unwrap().config.board.web_admin.address = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("web_admin_port"),
                        ListValue::U32(lock.config.board.web_admin.port as u32, 1, u16::MAX as u32),
                    )
                    .with_status(get_text("web_admin_port-status"))
                    .with_help(get_text("web_admin_port-help"))
                    .with_label_width(33)
                    .with_update_u32_value(&|board: &Arc<Mutex<IcyBoard>>, value| {
                        board.lock().unwrap().config.board.web_admin.port = value as u16;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(get_text("web_admin_allow_remote"), ListValue::Bool(lock.config.board.web_admin.allow_remote))
                        .with_status(get_text("web_admin_allow_remote-status"))
                        .with_help(get_text("web_admin_allow_remote-help"))
                        .with_label_width(33)
                        .with_update_bool_value(&|board: &Arc<Mutex<IcyBoard>>, value| {
                            board.lock().unwrap().config.board.web_admin.allow_remote = value;
                        }),
                ),
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn upper_labels_are_fully_visible_at_80_columns() {
        let mut page = BoardConfiguration::new(Arc::new(Mutex::new(IcyBoard::default())));
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| page.render(frame, frame.area())).unwrap();
        let buffer = terminal.backend().buffer();
        let rows: Vec<String> = (0..25).map(|y| (0..80).map(|x| buffer[(x, y)].symbol()).collect()).collect();
        for key in [
            "board_name",
            "allow_iemsi",
            "board_iemsi_location",
            "board_iemsi_operator",
            "board_iemsi_notice",
            "board_iemsi_caps",
            "date_format",
            "board_node_num",
        ] {
            let label = get_text(key);
            assert!(rows.iter().any(|row| row.contains(&label)), "clipped label: {label}");
        }
    }
}
