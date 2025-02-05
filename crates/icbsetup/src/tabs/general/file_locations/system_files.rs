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

pub struct SystemFiles {
    pub state: ConfigMenuState,

    menu: ConfigMenu<u32>,
}

impl SystemFiles {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let system_files_width = 30;
            let sysop_info = vec![
                ConfigEntry::Item(
                    ListItem::new("Conference Data".to_string(), ListValue::Path(lock.config.paths.conferences.clone()))
                        .with_status("Name/Loc of Conference Data")
                        .with_label_width(system_files_width),
                ),
                ConfigEntry::Item(
                    ListItem::new("Home Directory".to_string(), ListValue::Path(lock.config.paths.home_dir.clone()))
                        .with_status("User Home Directory")
                        .with_label_width(system_files_width),
                ),
                ConfigEntry::Item(
                    ListItem::new("Statistics File".to_string(), ListValue::Path(lock.config.paths.statistics_file.clone()))
                        .with_status("Name/Loc of Statistics file")
                        .with_label_width(system_files_width),
                ),
                ConfigEntry::Item(
                    ListItem::new("ICBTEXT File".to_string(), ListValue::Path(lock.config.paths.icbtext.clone()))
                        .with_status("Name/Loc of ICBTEXT file")
                        .with_label_width(system_files_width),
                ),
                ConfigEntry::Item(
                    ListItem::new("Temp. Work Files".to_string(), ListValue::Path(lock.config.paths.tmp_work_path.clone()))
                        .with_status("Location of Temporary Work Files")
                        .with_label_width(system_files_width),
                ),
                ConfigEntry::Item(
                    ListItem::new("Help Files".to_string(), ListValue::Path(lock.config.paths.help_path.clone()))
                        .with_status("Location of Help Files")
                        .with_label_width(system_files_width),
                ),
                ConfigEntry::Item(
                    ListItem::new("Login Sec. Files".to_string(), ListValue::Path(lock.config.paths.security_file_path.clone()))
                        .with_status("Location of Login Security Files")
                        .with_label_width(system_files_width),
                ),
                ConfigEntry::Item(
                    ListItem::new("CMD Display Files".to_string(), ListValue::Path(lock.config.paths.command_display_path.clone()))
                        .with_status("Location of Command Display Files")
                        .with_label_width(system_files_width),
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

impl Page for SystemFiles {
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

        let val = "System Files".to_string();
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
