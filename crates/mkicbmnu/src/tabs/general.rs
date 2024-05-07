use std::sync::Arc;

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::menu::Menu;
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, EditMode, ListItem, ListValue, ResultState},
    theme::THEME,
};
use ratatui::{
    layout::{Margin, Rect},
    widgets::{Block, BorderType, Borders, Clear, Padding, Widget},
    Frame,
};

use crate::TabPage;

pub struct GeneralTab {
    state: ConfigMenuState,
    config: ConfigMenu,
}

impl GeneralTab {
    pub fn new(menu: Arc<Menu>) -> Self {
        let items = vec![
            ConfigEntry::Item(ListItem::new("title", "Title".to_string(), ListValue::Text(25, menu.title.clone()))),
            ConfigEntry::Item(ListItem::new(
                "display_file",
                "Display File".to_string(),
                ListValue::Text(25, menu.display_file.to_string_lossy().to_string()),
            )),
            ConfigEntry::Item(ListItem::new("help_file", "Help File".to_string(), ListValue::Path(menu.help_file.clone()))),
            ConfigEntry::Item(ListItem::new("prompt", "Prompt".to_string(), ListValue::Text(25, menu.prompt.clone()))),
        ];

        Self {
            state: ConfigMenuState::default(),
            config: ConfigMenu {
                items: vec![ConfigEntry::Group("Switches".to_string(), items)],
            },
        }
    }
    fn prev(&mut self) {
        let selected = self.state.selected;
        self.state.selected = (selected + 3) % 4;
    }

    fn next(&mut self) {
        let selected = self.state.selected;
        self.state.selected = (selected + 1) % 4;
    }
}

impl TabPage for GeneralTab {
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
