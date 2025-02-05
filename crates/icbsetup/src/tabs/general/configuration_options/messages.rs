use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
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

use crate::{cfg_entry_bool, cfg_entry_u16};

pub struct Messages {
    pub state: ConfigMenuState,
    menu: ConfigMenu<Arc<Mutex<IcyBoard>>>,
}

impl Messages {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_width = 50;
            let entry: Vec<ConfigEntry<Arc<Mutex<IcyBoard>>>> = vec![
                cfg_entry_u16!("max_msg_lines", label_width, 17, 400, message, max_msg_lines, lock),
                cfg_entry_bool!("disable_message_scan_prompt", label_width, message, disable_message_scan_prompt, lock),
                cfg_entry_bool!("allow_esc_codes", label_width, message, allow_esc_codes, lock),
                cfg_entry_bool!("allow_carbon_copy", label_width, message, allow_carbon_copy, lock),
                cfg_entry_bool!("validate_to_name", label_width, message, validate_to_name, lock),
                cfg_entry_bool!("default_quick_personal_scan", label_width, message, default_quick_personal_scan, lock),
                cfg_entry_bool!(
                    "default_scan_all_selected_confs_at_login",
                    label_width,
                    message,
                    default_scan_all_selected_confs_at_login,
                    lock
                ),
                cfg_entry_bool!("prompt_to_read_mail", label_width, message, prompt_to_read_mail, lock),
            ];
            ConfigMenu { obj: icy_board.clone(), entry }
        };

        Self {
            state: ConfigMenuState::default(),
            menu,
        }
    }
}

impl Page for Messages {
    fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        let area = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(1),
        };

        let block: Block<'_> = Block::new()
            .style(get_tui_theme().background)
            .padding(Padding::new(2, 2, 1 + 4, 0))
            .borders(Borders::ALL)
            .border_set(BORDER_SET)
            .border_style(get_tui_theme().content_box);
        block.render(area, frame.buffer_mut());

        let val = get_text("configuration_options_messages");
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
