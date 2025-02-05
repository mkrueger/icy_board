use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState},
    tab_page::{Page, PageMessage},
    theme::get_tui_theme,
    BORDER_SET,
};
use ratatui::{
    layout::{Margin, Rect},
    text::Line,
    widgets::{Block, Borders, Padding, Widget},
};

pub struct DisplayFiles {
    pub state: ConfigMenuState,

    menu: ConfigMenu<u32>,
}

impl DisplayFiles {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let display_files_width = 30;
            let sysop_info = vec![
                ConfigEntry::Item(
                    ListItem::new("WELCOME File".to_string(), ListValue::Path(lock.config.paths.welcome.clone()))
                        .with_status("Name/Location of WELCOME File")
                        .with_label_width(display_files_width),
                ),
                ConfigEntry::Item(
                    ListItem::new("NEWUSER File".to_string(), ListValue::Path(lock.config.paths.newuser.clone()))
                        .with_status("Name/Location of NEWUSER File")
                        .with_label_width(display_files_width),
                ),
                ConfigEntry::Item(
                    ListItem::new("CLOSED File".to_string(), ListValue::Path(lock.config.paths.closed.clone()))
                        .with_status("Name/Location of CLOSED File")
                        .with_label_width(display_files_width),
                ),
                ConfigEntry::Item(
                    ListItem::new("WARNING File".to_string(), ListValue::Path(lock.config.paths.expire_warning.clone()))
                        .with_status("Name/Location of WARNING File")
                        .with_label_width(display_files_width),
                ),
                ConfigEntry::Item(
                    ListItem::new("EXPIRED File".to_string(), ListValue::Path(lock.config.paths.expired.clone()))
                        .with_status("Name/Location of EXPIRED File")
                        .with_label_width(display_files_width),
                ),
                ConfigEntry::Item(
                    ListItem::new("Conf. Join Menu".to_string(), ListValue::Path(lock.config.paths.conf_join_menu.clone()))
                        .with_status("Name/Location of Conference Join Menu File")
                        .with_label_width(display_files_width),
                ),
                ConfigEntry::Item(
                    ListItem::new("NOANSI Warning".to_string(), ListValue::Path(lock.config.paths.no_ansi.clone()))
                        .with_status("Name/Location of NOANSI Warning File")
                        .with_label_width(display_files_width),
                ),
            ];
            ConfigMenu { obj: 0, entry: sysop_info }
        };

        Self {
            state: ConfigMenuState::default(),
            menu,
        }
    }
}

impl Page for DisplayFiles {
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

        let val = "Configuration Files".to_string();
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
