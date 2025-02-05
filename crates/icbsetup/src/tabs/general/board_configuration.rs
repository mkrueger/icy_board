use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState, Value},
    get_text,
    tab_page::{Page, PageMessage},
    theme::get_tui_theme,
    BORDER_SET,
};
use ratatui::{
    layout::{Margin, Rect},
    text::Line,
    widgets::{Block, Borders, Padding, Widget},
};

use crate::{cfg_entry_bool, cfg_entry_text, cfg_entry_u16};

pub struct BoardConfiguration {
    pub state: ConfigMenuState,

    menu: ConfigMenu<Arc<Mutex<IcyBoard>>>,
}

impl BoardConfiguration {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let icy_board2 = icy_board.clone();
            let lock: std::sync::MutexGuard<'_, IcyBoard> = icy_board.lock().unwrap();
            let board_info_width = 30;
            let sysop_info: Vec<ConfigEntry<Arc<Mutex<IcyBoard>>>> = vec![
                cfg_entry_text!("board_name", 25, board_info_width, board, name, lock),
                cfg_entry_bool!("allow_iemsi", board_info_width, board, allow_iemsi, lock),
                cfg_entry_text!("board_iemsi_location", 54, board_info_width, board, name, lock),
                cfg_entry_text!("board_iemsi_operator", 30, board_info_width, board, operator, lock),
                cfg_entry_text!("board_iemsi_notice", 30, board_info_width, board, notice, lock),
                cfg_entry_text!("board_iemsi_caps", 30, board_info_width, board, capabilities, lock),
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
                    .with_label_width(board_info_width)
                    .with_update_value(Box::new(|board: &Arc<Mutex<IcyBoard>>, value: &ListValue| {
                        let ListValue::ValueList(val, _) = value else {
                            return;
                        };
                        board.lock().unwrap().config.board.date_format = val.clone()
                    })),
                ),
                cfg_entry_u16!("board_node_num", 30, 1, 256, board, num_nodes, lock),
            ];
            ConfigMenu {
                obj: icy_board2,
                entry: sysop_info,
            }
        };

        Self {
            state: ConfigMenuState::default(),
            menu,
        }
    }
}

impl Page for BoardConfiguration {
    fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        let block: Block<'_> = Block::new()
            .style(get_tui_theme().background)
            .padding(Padding::new(2, 2, 1 + 4, 0))
            .borders(Borders::ALL)
            .border_set(BORDER_SET)
            .border_style(get_tui_theme().content_box);
        block.render(area, frame.buffer_mut());

        let val = get_text("board_config_title");
        let width = val.len() as u16;
        Line::raw(val).style(get_tui_theme().menu_title).render(
            Rect {
                x: area.x + 1 + area.width.saturating_sub(width) / 2,
                y: area.y + 1,
                width,
                height: 1,
            },
            frame.buffer_mut(),
        );

        frame.buffer_mut().set_string(
            area.x + 1,
            area.y + 2,
            "─".repeat((area.width as usize).saturating_sub(2)),
            get_tui_theme().content_box,
        );

        let area = area.inner(Margin { vertical: 4, horizontal: 1 });
        self.menu.render(area, frame, &mut self.state);
    }

    fn request_status(&self) -> ResultState {
        ResultState {
            edit_mode: icy_board_tui::config_menu::EditMode::None,
            status_line: self.menu.current_status_line(&self.state),
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        let res = self.menu.handle_key_press(key, &mut self.state);

        PageMessage::ResultState(res)
    }
}
