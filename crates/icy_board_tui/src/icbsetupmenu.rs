use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Margin, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Widget},
};

use crate::{
    BORDER_SET,
    config_menu::{EditMode, ResultState},
    get_text,
    select_menu::{SelectMenu, SelectMenuState},
    tab_page::{Page, PageMessage},
    theme::get_tui_theme,
};

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

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
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
            .border_style(get_tui_theme().menu_box)
            .title_alignment(ratatui::layout::Alignment::Center)
            .title_bottom(Span::styled(get_text("icb_setup_key_menu_help"), get_tui_theme().key_binding));
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
            let width = val.chars().count() as u16 - 2;
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
            get_tui_theme().menu_box,
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

    pub fn with_left_title(mut self, left_title: String) -> Self {
        self.left_title = Some(left_title);
        self
    }
    pub fn with_center_title(mut self, center_title: String) -> Self {
        self.center_title = Some(center_title);
        self
    }
    pub fn with_right_title(mut self, right_title: String) -> Self {
        self.right_title = Some(right_title);
        self
    }

    pub fn handle_key_press(&mut self, key: KeyEvent) -> (ResultState, Option<i32>) {
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
                PageMessage::ExternalProgramStarted => {
                    return (
                        ResultState {
                            edit_mode: EditMode::ExternalProgramStarted,
                            ..Default::default()
                        },
                        None,
                    );
                }
                _ => {
                    return (ResultState::default(), None);
                }
            }
        }

        (ResultState::default(), self.menu.handle_key_press(key, &mut self.state).cloned())
    }

    pub fn request_status(&self) -> ResultState {
        ResultState {
            edit_mode: EditMode::None,
            status_line: String::new(),
        }
    }

    pub fn open_sup_page(&mut self, page: Box<dyn Page>) -> ResultState {
        let initial_state = page.request_status();
        self.sub_pages.push(page);
        initial_state
    }
}
