use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Margin, Rect},
    text::Line,
    widgets::{Block, Borders, Clear, Padding, Widget},
};

use crate::{
    BORDER_SET,
    config_menu::{EditMessage, ResultState},
    hotkeys::HotkeyBar,
    message_box::MessageBox,
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
    footer_id: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{select_menu::MenuItem, tab_page::InfoState};
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    fn menu() -> IcbSetupMenuUI {
        IcbSetupMenuUI::new(SelectMenu::new(vec![
            MenuItem::new(1, 'A', "First".into()).with_help("First help".into()),
            MenuItem::new(2, 'B', "Second".into()).with_help("Second help".into()),
        ]))
        .with_left_title("Left".into())
        .with_center_title("Settings".into())
        .with_right_title("Right".into())
    }

    fn row_text(buffer: &Buffer, y: u16) -> String {
        (0..buffer.area.width).map(|x| buffer[(x, y)].symbol()).collect()
    }

    #[test]
    fn classic_menu_keeps_geometry_focus_symbols_help_and_modal_footer_policy() {
        let mut menu = menu();
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| menu.render(frame, frame.area())).unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(25, 5)].symbol(), "A");
        assert_eq!(buffer[(25, 6)].symbol(), "B");
        assert_eq!(
            buffer[(28, 5)].style(),
            get_tui_theme()
                .background
                .patch(get_tui_theme().selected_item)
                .underline_color(ratatui::style::Color::Reset)
        );
        assert!(row_text(buffer, 2).contains("Settings"));
        assert!(row_text(buffer, 24).contains(&HotkeyBar::for_id("icb_setup_key_menu_help").line().to_string()));
        assert_eq!(row_text(buffer, 3).chars().filter(|ch| *ch == '─').count(), 76);
        menu.handle_key_press(KeyCode::Down.into());
        let (state, _) = menu.handle_key_press(KeyCode::F(1).into());
        assert!(matches!(state.edit_msg, EditMessage::DisplayHelp(ref text) if text == "Second help"));
        assert_eq!(menu.handle_key_press(KeyCode::Enter.into()).1, Some(2));
        menu.open_sup_page(Box::new(MessageBox::new(InfoState::Info, "Modal".into())));
        terminal.draw(|frame| menu.render(frame, frame.area())).unwrap();
        assert!(!row_text(terminal.backend().buffer(), 24).contains("F1"));
        assert_eq!(terminal.backend().buffer()[(25, 5)].symbol(), "A");
        menu.handle_key_press(KeyCode::Esc.into());
        terminal.draw(|frame| menu.render(frame, frame.area())).unwrap();
        assert!(row_text(terminal.backend().buffer(), 24).contains(&HotkeyBar::for_id("icb_setup_key_menu_help").line().to_string()));
        assert_eq!(menu.state.selected, 1);
    }

    #[test]
    fn menu_titles_and_footers_are_safe_on_tiny_screens() {
        for (width, height) in [(0, 0), (1, 1), (2, 2), (4, 10), (8, 3), (20, 5), (80, 25)] {
            let mut menu = menu().with_center_title("界 e\u{301} Settings".into());
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| menu.render(frame, frame.area())).unwrap();
            menu.open_sup_page(Box::new(MessageBox::new(InfoState::Warning, "Modal".into())));
            terminal.draw(|frame| menu.render(frame, frame.area())).unwrap();
        }
    }
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
            footer_id: "icb_setup_key_menu_help",
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let disp_area = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(1),
        };

        // A page paints only the cells it uses, so what the frame before left
        // there has to go first.
        Clear.render(area, frame.buffer_mut());
        Block::new().style(get_tui_theme().background).render(area, frame.buffer_mut());

        // Everything under the topmost full page is hidden by it, a modal only
        // covers a box of its own.
        if let Some(topmost) = self.sub_pages.iter().rposition(|page| !page.is_modal()) {
            for page in self.sub_pages[topmost..].iter_mut() {
                page.render(frame, area);
            }
            return;
        }

        let mut block: Block<'_> = Block::new()
            .style(get_tui_theme().background)
            .padding(Padding::new(2, 2, 1 + 4, 0))
            .borders(Borders::ALL)
            .border_set(BORDER_SET)
            .border_style(get_tui_theme().menu_box)
            .title_alignment(ratatui::layout::Alignment::Center);
        if self.sub_pages.is_empty() {
            block = block.title_bottom(HotkeyBar::for_id(self.footer_id).line());
        }
        block.render(disp_area, frame.buffer_mut());

        let title_area = disp_area.inner(Margin { horizontal: 1, vertical: 1 });
        if let Some(val) = &self.center_title {
            let width = Line::raw(val).width().min(title_area.width as usize) as u16;
            Line::raw(val).style(get_tui_theme().menu_title).render(
                Rect {
                    x: (disp_area.x + 1 + disp_area.width.saturating_sub(width) / 2).min(title_area.right().saturating_sub(width)),
                    y: title_area.y,
                    width,
                    height: title_area.height.min(1),
                },
                frame.buffer_mut(),
            );
        }

        if let Some(val) = &self.left_title {
            let width = Line::raw(val).width().min(title_area.width as usize) as u16;
            Line::raw(val).style(get_tui_theme().item).render(
                Rect {
                    x: title_area.x,
                    y: title_area.y,
                    width,
                    height: title_area.height.min(1),
                },
                frame.buffer_mut(),
            );
        }

        if let Some(val) = &self.right_title {
            let width = Line::raw(val).width().min(title_area.width as usize) as u16;
            Line::raw(val).style(get_tui_theme().item).render(
                Rect {
                    x: title_area.right().saturating_sub(width),
                    y: title_area.y,
                    width,
                    height: title_area.height.min(1),
                },
                frame.buffer_mut(),
            );
        }

        if disp_area.height > 3 && disp_area.width > 2 {
            frame.buffer_mut().set_string(
                disp_area.x + 1,
                disp_area.y + 2,
                "─".repeat((disp_area.width as usize).saturating_sub(2)),
                get_tui_theme().menu_box,
            );
        }

        let menu_width = self.menu.preferred_width();
        let mut menu_area = disp_area.inner(Margin {
            vertical: 0,
            horizontal: (disp_area.width.saturating_sub(menu_width)) / 2,
        });
        menu_area.y += 4;
        menu_area.height = menu_area.height.saturating_sub(4);
        if menu_area.width >= 3 && menu_area.height > 0 {
            self.menu.render(menu_area, frame, &mut self.state);
        }

        for page in self.sub_pages.iter_mut() {
            page.render(frame, area);
        }
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
    pub fn with_footer(mut self, footer_id: &'static str) -> Self {
        self.footer_id = footer_id;
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
                            edit_msg: EditMessage::ExternalProgramStarted,
                            ..Default::default()
                        },
                        None,
                    );
                }
                PageMessage::InfoBox(state, message) => {
                    return (self.open_sup_page(Box::new(MessageBox::new(state, message))), None);
                }
                _ => {
                    return (ResultState::default(), None);
                }
            }
        }
        if let KeyCode::F(1) = key.code
            && let Some(str) = self.menu.help(&mut self.state)
        {
            return (
                ResultState {
                    edit_msg: EditMessage::DisplayHelp(str.to_string()),
                    ..Default::default()
                },
                None,
            );
        }

        (ResultState::default(), self.menu.handle_key_press(key, &mut self.state).cloned())
    }

    pub fn request_status(&self) -> ResultState {
        ResultState {
            edit_msg: EditMessage::None,
            status_line: String::new(),
        }
    }

    pub fn open_sup_page(&mut self, page: Box<dyn Page>) -> ResultState {
        let initial_state = page.request_status();
        self.sub_pages.push(page);
        initial_state
    }
}
