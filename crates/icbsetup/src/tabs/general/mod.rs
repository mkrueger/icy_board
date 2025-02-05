use std::sync::{Arc, Mutex};

use board_configuration::BoardConfiguration;
use configuration_options::ConfigurationOptions;
use crossterm::event::KeyEvent;
use file_locations::FileLocations;
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
use subscription_information::SubscriptionInformation;
mod board_configuration;
mod conferences;
mod configuration_options;
mod file_locations;
mod new_user_options;
mod security_levels;
mod subscription_information;
mod sysop_information;

use crate::VERSION;

pub struct IcbSetupMenuUI {
    pub state: SelectMenuState,
    menu: SelectMenu<i32>,
    pub sub_pages: Vec<Box<dyn Page>>,
    left_title: Option<String>,
    center_title: Option<String>,
    right_title: Option<String>,
}

impl IcbSetupMenuUI {
    pub fn new(menu: SelectMenu<i32>) -> Self {
        Self {
            state: SelectMenuState::default(),
            menu,
            sub_pages: Vec::new(),
            left_title: None,
            center_title: None,
            right_title: None,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let disp_area = Rect {
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
        block.render(disp_area, frame.buffer_mut());

        if let Some(val) = &self.center_title {
            let width = val.len() as u16;
            Line::raw(val).style(get_tui_theme().menu_title).render(
                Rect {
                    x: disp_area.x + 1 + disp_area.width.saturating_sub(width) / 2,
                    y: disp_area.y + 1,
                    width,
                    height: 1,
                },
                frame.buffer_mut(),
            );
        }

        if let Some(val) = &self.left_title {
            let width = val.len() as u16;
            Line::raw(val).style(get_tui_theme().item).render(
                Rect {
                    x: disp_area.x + 1,
                    y: disp_area.y + 1,
                    width,
                    height: 1,
                },
                frame.buffer_mut(),
            );
        }

        if let Some(val) = &self.right_title {
            let width = val.len() as u16;
            Line::raw(val).style(get_tui_theme().item).render(
                Rect {
                    x: disp_area.x + disp_area.width.saturating_sub(width + 1),
                    y: disp_area.y + 1,
                    width,
                    height: 1,
                },
                frame.buffer_mut(),
            );
        }

        frame.buffer_mut().set_string(
            disp_area.x + 1,
            disp_area.y + 2,
            "─".repeat((disp_area.width as usize).saturating_sub(2)),
            get_tui_theme().content_box,
        );

        const MENU_WIDTH: u16 = 30;
        let mut menu_area = disp_area.inner(Margin {
            vertical: 0,
            horizontal: (disp_area.width.saturating_sub(MENU_WIDTH)) / 2,
        });
        menu_area.y += 4;
        menu_area.height = menu_area.height.saturating_sub(4);
        self.menu.render(menu_area, frame, &mut self.state);
    }

    fn with_left_title(mut self, left_title: String) -> Self {
        self.left_title = Some(left_title);
        self
    }
    fn with_center_title(mut self, center_title: String) -> Self {
        self.center_title = Some(center_title);
        self
    }
    fn with_right_title(mut self, right_title: String) -> Self {
        self.right_title = Some(right_title);
        self
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> (ResultState, Option<i32>) {
        if key.code == crossterm::event::KeyCode::Esc {
            if !self.sub_pages.is_empty() {
                self.sub_pages.pop();
            }
            return (ResultState::default(), None);
        }

        if let Some(page) = self.sub_pages.last_mut() {
            let state = page.handle_key_press(key);
            match state {
                PageMessage::OpenSubPage(page) => {
                    return (self.open_sup_page(page), None);
                }
                PageMessage::ResultState(state) => {
                    return (state, None);
                }
                PageMessage::Close => {
                    self.sub_pages.pop();
                    return (ResultState::default(), None);
                }
                _ => {
                    return (ResultState::default(), None);
                }
            }
        }

        (ResultState::default(), self.menu.handle_key_press(key, &mut self.state).cloned())
    }

    fn request_status(&self) -> ResultState {
        ResultState {
            edit_mode: icy_board_tui::config_menu::EditMode::None,
            status_line: String::new(),
        }
    }

    pub fn open_sup_page(&mut self, page: Box<dyn Page>) -> ResultState {
        let initial_state = page.request_status();
        self.sub_pages.push(page);
        initial_state
    }
}

pub struct GeneralTab {
    pub page: IcbSetupMenuUI,
    icy_board: Arc<Mutex<IcyBoard>>,
}

impl GeneralTab {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let left_title = icy_board.as_ref().lock().unwrap().file_name.file_name().unwrap().to_string_lossy().to_string();
        let center_title = "Main Menu".to_string();
        let right_title = format!("Use /w ICB {}", VERSION.to_string());
        Self {
            page: IcbSetupMenuUI::new(SelectMenu::new(vec![
                MenuItem::new(0, 'A', "Sysop Information".to_string()),
                MenuItem::new(1, 'B', "File Locations".to_string()),
                MenuItem::new(2, 'C', "Connection Information".to_string()),
                MenuItem::new(3, 'D', "Board Configuration".to_string()),
                MenuItem::new(4, 'E', "TODO".to_string()),
                MenuItem::new(5, 'F', "Subscription".to_string()),
                MenuItem::new(6, 'G', "Configuration Options".to_string()),
                MenuItem::new(7, 'H', "Security Levels".to_string()),
                MenuItem::new(8, 'I', "Accounting Configuration".to_string()),
                MenuItem::new(9, 'J', "New User Options".to_string()),
                MenuItem::new(10, 'K', "TODO".to_string()),
                MenuItem::new(11, 'L', "Main Board Configuration".to_string()),
                MenuItem::new(12, 'M', "Conferences".to_string()),
            ]))
            .with_left_title(left_title)
            .with_center_title(center_title)
            .with_right_title(right_title),
            icy_board,
        }
    }
}

impl TabPage for GeneralTab {
    fn title(&self) -> String {
        "Main".to_string()
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.page.render(frame, area);
    }

    fn has_control(&self) -> bool {
        !self.page.sub_pages.is_empty()
    }

    fn request_status(&self) -> ResultState {
        self.page.request_status()
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> ResultState {
        let (state, opt) = self.page.handle_key_press(key);
        if let Some(selected) = opt {
            match selected {
                0 => {
                    let page = sysop_information::SysopInformation::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }
                1 => {
                    let page = FileLocations::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }
                2 => { // Connection Information
                }
                3 => {
                    let page = BoardConfiguration::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }

                4 => { // Events - todo
                }

                5 => {
                    let page = SubscriptionInformation::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }
                6 => {
                    let page = ConfigurationOptions::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }

                7 => {
                    let page = security_levels::SecurityLevelOptions::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }

                8 => { // Accounting Configuration TODO
                }

                9 => {
                    let page = new_user_options::NewUserOptions::new(self.icy_board.clone());
                    return self.page.open_sup_page(Box::new(page));
                }

                11 => {
                    // Main Board Configuration
                    let page = conferences::ConferenceEditor::new(self.icy_board.clone(), 0);
                    return self.page.open_sup_page(Box::new(page));
                }
                _ => {}
            }
        }
        state
    }

    fn get_help(&self) -> Text<'static> {
        String::new().into()
    }
}
