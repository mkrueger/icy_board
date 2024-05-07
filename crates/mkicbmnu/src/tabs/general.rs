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
}

impl GeneralTab {
    pub fn new(menu: Arc<Mutex<Menu>>) -> Self {
        let mnu = menu.lock().unwrap();
        let info_width = 16;

        let items = vec![
            ConfigEntry::Item(
                ListItem::new("title", "Title".to_string(), ListValue::Text(25, mnu.title.clone()))
                    .with_status("Enter the title of the menu.")
                    .with_label_width(info_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "display_file",
                    "Display File".to_string(),
                    ListValue::Text(25, mnu.display_file.to_string_lossy().to_string()),
                )
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
        ];

        Self {
            state: ConfigMenuState::default(),
            config: ConfigMenu {
                items: vec![ConfigEntry::Group(String::new(), items)],
            },
        }
    }
}

impl TabPage for GeneralTab {
    fn title(&self) -> String {
        "General".to_string()
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
        res
    }

    fn request_status(&self) -> ResultState {
        return ResultState {
            edit_mode: EditMode::None,
            status_line: if self.state.selected < self.config.items.len() {
                "".to_string()
            } else {
                "".to_string()
            },
        };
    }
}
