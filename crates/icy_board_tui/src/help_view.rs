use crossterm::event::{KeyCode, KeyEvent};

use md_tui::nodes::root::{Component, ComponentRoot};
use md_tui::parser;

use md_tui::util::colors::ColorConfig;
use ratatui::Frame;
use ratatui::layout::{Margin, Rect};
use ratatui::widgets::{Block, BorderType, Borders, ScrollbarState, Widget};

use crate::theme::get_tui_theme;

/// necessary as ScrollbarState fields are private
pub struct HelpViewState {
    scroll: u16,
    area: Rect,
    pub markdown: Option<ComponentRoot>,
}

impl HelpViewState {
    pub fn new() -> Self {
        let cfg: ColorConfig = ColorConfig {
            italic_color: get_tui_theme().help_box.fg.unwrap(),
            bold_color: get_tui_theme().help_box.fg.unwrap(),
            striketrough_color: get_tui_theme().help_box.fg.unwrap(),
            bold_italic_color: get_tui_theme().help_box.fg.unwrap(),

            code_fg_color: get_tui_theme().help_box.fg.unwrap(),
            code_bg_color: get_tui_theme().help_box.bg.unwrap(),

            link_color: get_tui_theme().help_box.fg.unwrap(),

            link_selected_fg_color: get_tui_theme().help_box.fg.unwrap(),
            link_selected_bg_color: get_tui_theme().help_box.bg.unwrap(),

            code_block_bg_color: get_tui_theme().help_box.bg.unwrap(),

            heading_fg_color: get_tui_theme().help_header.fg.unwrap(),
            heading_bg_color: get_tui_theme().help_header.bg.unwrap(),

            table_header_fg_color: get_tui_theme().help_box.fg.unwrap(),
            table_header_bg_color: get_tui_theme().help_box.bg.unwrap(),

            quote_bg_color: get_tui_theme().help_box.bg.unwrap(),

            file_tree_selected_fg_color: get_tui_theme().help_box.fg.unwrap(),
            file_tree_page_count_color: get_tui_theme().help_box.fg.unwrap(),
            file_tree_name_color: get_tui_theme().help_box.fg.unwrap(),
            file_tree_path_color: get_tui_theme().help_box.fg.unwrap(),

            quote_important: get_tui_theme().help_box.fg.unwrap(),
            quote_warning: get_tui_theme().help_box.fg.unwrap(),
            quote_tip: get_tui_theme().help_box.fg.unwrap(),
            quote_note: get_tui_theme().help_box.fg.unwrap(),
            quote_caution: get_tui_theme().help_box.fg.unwrap(),
            quote_default: get_tui_theme().help_box.fg.unwrap(),
        };

        unsafe {
            md_tui::util::colors::COLOR_CONFIG = cfg;
        }

        HelpViewState {
            markdown: None,
            area: Rect::default(),
            scroll: 0,
        }
    }

    fn scroll_down(&mut self) {
        if let Some(markdown) = &self.markdown {
            let len = markdown.height();
            if self.area.height > len {
                self.scroll = 0;
            } else {
                self.scroll = std::cmp::min(self.scroll.saturating_add(1), len.saturating_sub(self.area.height))
            }
        }
    }

    fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    fn scroll_page_down(&mut self) {
        if let Some(markdown) = &self.markdown {
            let len = markdown.height();
            self.scroll = std::cmp::min(self.scroll.saturating_add(self.area.height), len.saturating_sub(self.area.height))
        }
    }

    fn scroll_page_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(self.area.height);
    }

    fn scroll_top(&mut self) {
        self.scroll = 0;
    }

    fn scroll_bottom(&mut self) {
        if let Some(markdown) = &self.markdown {
            let len = markdown.height();
            self.scroll = len.saturating_sub(self.area.height);
        }
    }

    pub(crate) fn handle_key_press(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll_down();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll_up();
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.scroll_top();
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.scroll_bottom();
            }
            KeyCode::PageDown => {
                self.scroll_page_down();
            }
            KeyCode::PageUp => {
                self.scroll_page_up();
            }
            _ => {}
        }
    }

    pub fn set_content(&mut self, content: &str) {
        let area = self.area.inner(Margin { horizontal: 2, vertical: 2 });

        self.markdown = Some(parser::parse_markdown(None, content, area.width));
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        if let Some(markdown) = &mut self.markdown {
            markdown.set_scroll(self.scroll);
            let block = Block::new()
                .style(get_tui_theme().help_box)
                .borders(Borders::ALL)
                .border_type(BorderType::Double);
            block.render(self.area, frame.buffer_mut());

            let area = self.area.inner(Margin { horizontal: 2, vertical: 2 });
            for child in markdown.children() {
                if let Component::TextComponent(comp) = child {
                    let mut comp = comp.clone();
                    comp.set_y_offset(comp.y_offset() + area.y);
                    if comp.y_offset().saturating_sub(comp.scroll_offset()) >= area.height
                        || (comp.y_offset() + comp.height()).saturating_sub(comp.scroll_offset()) == 0
                    {
                        continue;
                    }

                    frame.render_widget(comp, area);
                }
            }
        }
    }

    pub(crate) fn set_area(&mut self, screen: Rect) {
        self.area = screen.inner(Margin { horizontal: 1, vertical: 2 });
        self.area.height += 1;
    }
}

impl From<&mut HelpViewState> for ScrollbarState {
    fn from(state: &mut HelpViewState) -> ScrollbarState {
        let max = state.markdown.as_ref().map_or(0, |m| m.height());
        ScrollbarState::new(max.saturating_sub(state.area.height) as usize).position(state.scroll as usize)
    }
}
