use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::{user_base::Password, IcyBoard};
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState},
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

use crate::{cfg_entry_bool, cfg_entry_password, cfg_entry_text};

pub struct SysopInformation {
    pub state: ConfigMenuState,

    menu: ConfigMenu<Arc<Mutex<IcyBoard>>>,
}

impl SysopInformation {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu: ConfigMenu<Arc<Mutex<IcyBoard>>> = {
            let lock = icy_board.lock().unwrap();
            let label_width = 30;
            let sysop_info: Vec<ConfigEntry<Arc<Mutex<IcyBoard>>>> = vec![
                cfg_entry_text!("sysop_name", 45, label_width, sysop, name, lock),
                cfg_entry_password!("local_password", label_width, sysop, password, lock),
                cfg_entry_bool!("require_password_to_exit", label_width, sysop, require_password_to_exit, lock),
                cfg_entry_bool!("use_real_name", label_width, sysop, use_real_name, lock),
            ];
            ConfigMenu {
                obj: icy_board.clone(),
                entry: sysop_info,
            }
        };

        Self {
            state: ConfigMenuState::default(),
            menu,
        }
    }
}

impl Page for SysopInformation {
    fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        let block: Block<'_> = Block::new()
            .style(get_tui_theme().background)
            .padding(Padding::new(2, 2, 1 + 4, 0))
            .borders(Borders::ALL)
            .border_set(BORDER_SET)
            .border_style(get_tui_theme().content_box);
        block.render(area, frame.buffer_mut());

        let val = get_text("sysop_information_title");

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
