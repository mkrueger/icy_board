use std::sync::{Arc, Mutex};

use icy_board_engine::icy_board::{icb_config::DisplayNewsBehavior, IcyBoard};
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState},
    get_text,
    tab_page::Page,
    theme::get_tui_theme,
    BORDER_SET,
};
use ratatui::{
    layout::{Margin, Rect},
    text::Line,
    widgets::{Block, Borders, Padding, Widget},
};

use crate::cfg_entry_bool;

pub struct ConfigSwitches {
    pub state: ConfigMenuState,
    menu: ConfigMenu<Arc<Mutex<IcyBoard>>>,
}

impl ConfigSwitches {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_width = 50;
            let entry = vec![
                cfg_entry_bool!("default_graphics_at_login", label_width, switches, default_graphics_at_login, lock),
                cfg_entry_bool!("non_graphics", label_width, switches, non_graphics, lock),
                cfg_entry_bool!("exclude_local_calls_stats", label_width, switches, exclude_local_calls_stats, lock),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("display_news_behavior"),
                        ListValue::Text(1, lock.config.switches.display_news_behavior.to_pcb_char().to_string()),
                    )
                    .with_status(&get_text("display_news_behavior-status"))
                    .with_label_width(label_width)
                    .with_update_text_value(&|board, value| {
                        if let Some(value) = value.chars().next() {
                            board.lock().unwrap().config.switches.display_news_behavior = DisplayNewsBehavior::from_pcb_char(value);
                        }
                    }),
                ),
                cfg_entry_bool!("display_userinfo_at_login", label_width, switches, display_userinfo_at_login, lock),
                cfg_entry_bool!("force_intro_on_join", label_width, switches, force_intro_on_join, lock),
                cfg_entry_bool!("scan_new_blt", label_width, switches, scan_new_blt, lock),
                cfg_entry_bool!("capture_grp_chat_session", label_width, switches, capture_grp_chat_session, lock),
                cfg_entry_bool!("allow_handle_in_grpchat", label_width, switches, allow_handle_in_grpchat, lock),
            ];
            ConfigMenu { obj: icy_board.clone(), entry }
        };

        Self {
            state: ConfigMenuState::default(),
            menu,
        }
    }
}

impl Page for ConfigSwitches {
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
        let val = get_text("configuration_options_config_switches");
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
}
