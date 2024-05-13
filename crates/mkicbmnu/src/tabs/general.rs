use core::panic;
use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::menu::Menu;
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, EditMode, ListItem, ListValue, ResultState},
    tab_page::TabPage,
    theme::THEME,
};
use ratatui::{
    layout::{Margin, Rect},
    widgets::{Block, BorderType, Borders, Clear, Padding, Widget},
    Frame,
};

pub struct GeneralTab {
    state: ConfigMenuState,
    config: ConfigMenu,
    menu: Arc<Mutex<Menu>>,
}

impl GeneralTab {
    pub fn new(menu: Arc<Mutex<Menu>>) -> Self {
        let info_width = 16;

        let items = if let Ok(mnu) = menu.lock() {
            vec![
                ConfigEntry::Item(
                    ListItem::new("title", "Title".to_string(), ListValue::Text(25, mnu.title.clone()))
                        .with_status("Enter the title of the menu.")
                        .with_label_width(info_width),
                ),
                ConfigEntry::Item(
                    ListItem::new("display_file", "Display File".to_string(), ListValue::Path(mnu.display_file.clone()))
                        .with_status("The menu background file to display.")
                        .with_label_width(info_width),
                ),
                ConfigEntry::Item(
                    ListItem::new("help_file", "Help File".to_string(), ListValue::Path(mnu.help_file.clone()))
                        .with_status("The help file to display.")
                        .with_label_width(info_width),
                ),
                ConfigEntry::Item(
                    ListItem::new("prompt", "Prompt".to_string(), ListValue::Text(25, mnu.prompt.clone()))
                        .with_status("The prompt for the menu.")
                        .with_label_width(info_width),
                ),
            ]
        } else {
            panic!();
        };

        Self {
            state: ConfigMenuState::default(),
            config: ConfigMenu {
                entry: vec![ConfigEntry::Group(String::new(), items)],
            },
            menu,
        }
    }

    fn write_back(&self) {
        let ConfigEntry::Group(_, items) = &self.config.entry[0] else {
            return;
        };
        for entry in items {
            match entry {
                ConfigEntry::Item(item) => match item.id.as_str() {
                    "title" => {
                        if let ListValue::Text(_, ref value) = item.value {
                            if let Ok(mut mnu) = self.menu.lock() {
                                mnu.title = value.to_string();
                            }
                        }
                    }
                    "display_file" => {
                        if let ListValue::Path(value) = &item.value {
                            if let Ok(mut mnu) = self.menu.lock() {
                                mnu.display_file = value.clone();
                            }
                        }
                    }
                    "help_file" => {
                        if let ListValue::Path(value) = &item.value {
                            if let Ok(mut mnu) = self.menu.lock() {
                                mnu.help_file = value.clone();
                            }
                        }
                    }
                    "prompt" => {
                        if let ListValue::Text(_, ref value) = item.value {
                            if let Ok(mut mnu) = self.menu.lock() {
                                mnu.prompt = value.to_string();
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

impl TabPage for GeneralTab {
    fn title(&self) -> String {
        "General".to_string()
    }

    fn has_control(&self) -> bool {
        self.state.in_edit
    }
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let width = (2 + 50 + 2).min(area.width) as u16;

        let lines = (6).min(area.height) as u16;
        let area = Rect::new(area.x + (area.width - width) / 2, (area.y + area.height - lines) / 2, width + 2, lines);

        Clear.render(area, frame.buffer_mut());

        let block = Block::new()
            .style(THEME.content_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        block.render(area, frame.buffer_mut());

        let area = area.inner(&Margin { vertical: 1, horizontal: 1 });
        self.config.render(area, frame, &mut self.state);
        if self.state.in_edit {
            self.config.get_item(self.state.selected).unwrap().text_field_state.set_cursor_position(frame);
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> ResultState {
        let res = self.config.handle_key_press(key, &mut self.state);

        /*
        if !self.state.in_edit {
            match key.code {
                KeyCode::Char('k') | KeyCode::Up => self.prev(),
                KeyCode::Char('j') | KeyCode::Down => self.next(),
                _ => {}
            }
            return ResultState {
                cursor: None,
                status_line: self.config.items[self.state.selected].status.clone(),
            };
        }

        let res = self.config.handle_key_press(key, &self.state);
        if res.cursor.is_none() {
            self.state.in_edit = false;
        }*/
        self.write_back();
        res
    }

    fn request_status(&self) -> ResultState {
        return ResultState {
            edit_mode: EditMode::None,
            status_line: if self.state.selected < self.config.entry.len() {
                "".to_string()
            } else {
                "".to_string()
            },
        };
    }
}
