use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::ResultState,
    select_menu::{MenuItem, SelectMenu, SelectMenuState},
    tab_page::{Page, PageMessage, TabPage},
    theme::get_tui_theme,
    BORDER_SET,
};
use ratatui::{
    layout::{Margin, Rect},
    text::{Line, Text},
    widgets::{Block, Borders, Padding, Widget},
    Frame,
};
mod sysop_information;
use crate::VERSION;

pub struct GeneralTab {
    pub state: SelectMenuState,
    menu: SelectMenu<i32>,
    icy_board: Arc<Mutex<IcyBoard>>,
    sub_pages: Vec<Box<dyn Page>>,
}

impl GeneralTab {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        Self {
            state: SelectMenuState::default(),
            menu: SelectMenu::new(vec![
                MenuItem::new(0, 'A', "Sysop Information".to_string()),
                MenuItem::new(1, 'B', "File Locations".to_string()),
                MenuItem::new(2, 'C', "Connection Information".to_string()),
                MenuItem::new(3, 'D', "Board Information".to_string()),
                MenuItem::new(4, 'E', "TODO".to_string()),
                MenuItem::new(5, 'F', "Subscription".to_string()),
                MenuItem::new(6, 'G', "Configuration Options".to_string()),
                MenuItem::new(7, 'H', "Security Levels".to_string()),
                MenuItem::new(8, 'I', "Accounting Configuration".to_string()),
                MenuItem::new(9, 'J', "TODO".to_string()),
                MenuItem::new(10, 'K', "TODO".to_string()),
                MenuItem::new(11, 'L', "Main Board Configuration".to_string()),
                MenuItem::new(12, 'M', "Conferences".to_string()),
            ]),
            icy_board,
            sub_pages: Vec::new(),
        }
    }
}

impl TabPage for GeneralTab {
    fn title(&self) -> String {
        "Main".to_string()
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let area = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(1),
        };

        if let Some(page) = self.sub_pages.last_mut() {
            page.render(frame, area);
            return;
        }

        let block: Block<'_> = Block::new()
            .style(get_tui_theme().background)
            .padding(Padding::new(2, 2, 1 + 4, 0))
            .borders(Borders::ALL)
            .border_set(BORDER_SET)
            .border_style(get_tui_theme().content_box);
        block.render(area, frame.buffer_mut());

        let val = "Main Menu".to_string();
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

        let val = self
            .icy_board
            .as_ref()
            .lock()
            .unwrap()
            .file_name
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let width = val.len() as u16;
        Line::raw(val).style(get_tui_theme().item).render(
            Rect {
                x: area.x + 1,
                y: area.y + 1,
                width,
                height: 1,
            },
            frame.buffer_mut(),
        );

        let val = format!("Use /w ICB {}", VERSION.to_string());
        let width = val.len() as u16;
        Line::raw(val).style(get_tui_theme().item).render(
            Rect {
                x: area.x + area.width.saturating_sub(width + 1),
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

        const MENU_WIDTH: u16 = 30;
        let mut menu_area = area.inner(Margin {
            vertical: 0,
            horizontal: (area.width.saturating_sub(MENU_WIDTH)) / 2,
        });
        menu_area.y += 4;
        menu_area.height = menu_area.height.saturating_sub(4);
        self.menu.render(menu_area, frame, &mut self.state);
    }

    fn has_control(&self) -> bool {
        !self.sub_pages.is_empty()
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> ResultState {
        if let Some(page) = self.sub_pages.last_mut() {
            let state = page.handle_key_press(key);
            match state {
                PageMessage::Close => {
                    self.sub_pages.pop();
                    return ResultState::default();
                }
                PageMessage::ResultState(state) => {
                    return state;
                }
                _ => {}
            }
        }

        if let Some(selected) = self.menu.handle_key_press(key, &mut self.state) {
            match selected {
                0 => {
                    let page = sysop_information::SysopInformation::new(self.icy_board.clone());
                    let initial_state = page.request_status();
                    self.sub_pages.push(Box::new(page));
                    return initial_state;
                }
                _ => {}
            }
        }

        ResultState::default()
    }

    fn get_help(&self) -> Text<'static> {
        String::new().into()
    }
}
