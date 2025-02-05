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

use crate::cfg_entry_color;

pub struct ColorOptions {
    pub state: ConfigMenuState,
    menu: ConfigMenu<Arc<Mutex<IcyBoard>>>,
}

impl ColorOptions {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_width = 50;
            let edge = vec![
                cfg_entry_color!("default_color", label_width, color_configuration, default, lock),
                cfg_entry_color!("msg_hdr_date", label_width, color_configuration, msg_hdr_date, lock),
                cfg_entry_color!("msg_hdr_to", label_width, color_configuration, msg_hdr_to, lock),
                cfg_entry_color!("msg_hdr_from", label_width, color_configuration, msg_hdr_from, lock),
                cfg_entry_color!("msg_hdr_subj", label_width, color_configuration, msg_hdr_subj, lock),
                cfg_entry_color!("msg_hdr_read", label_width, color_configuration, msg_hdr_read, lock),
                cfg_entry_color!("msg_hdr_conf", label_width, color_configuration, msg_hdr_conf, lock),
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
            state: ConfigMenuState::default(),
            menu,
        }
    }
}

impl Page for ColorOptions {
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

        let val = get_text("configuration_options_colors");
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
