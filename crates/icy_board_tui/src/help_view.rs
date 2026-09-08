use crossterm::event::{KeyCode, KeyEvent};

use md_tui::nodes::root::{Component, ComponentRoot};
use md_tui::parser;

use md_tui::util::colors::ColorConfig;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect};
use ratatui::widgets::{Block, BorderType, Borders, ScrollbarState, Widget};

use crate::theme::get_tui_theme;

/// necessary as ScrollbarState fields are private
pub struct HelpViewState {
    scroll: u16,
    area: Rect,
    pub markdown: Option<ComponentRoot>,
}

impl Default for HelpViewState {
    fn default() -> Self {
        Self::new()
    }
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

        md_tui::util::colors::set_color_config(cfg);

        HelpViewState {
            markdown: None,
            area: Rect::default(),
            scroll: 0,
        }
    }

    fn scroll_down(&mut self) {
        if let Some(markdown) = &self.markdown {
            let len = markdown.height();
            let height = self.content_area().height;
            if height > len {
                self.scroll = 0;
            } else {
                self.scroll = std::cmp::min(self.scroll.saturating_add(1), len.saturating_sub(height))
            }
        }
    }

    fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    fn scroll_page_down(&mut self) {
        if let Some(markdown) = &self.markdown {
            let len = markdown.height();
            let height = self.content_area().height;
            self.scroll = std::cmp::min(self.scroll.saturating_add(height), len.saturating_sub(height))
        }
    }

    fn scroll_page_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(self.content_area().height);
    }

    fn scroll_top(&mut self) {
        self.scroll = 0;
    }

    fn scroll_bottom(&mut self) {
        if let Some(markdown) = &self.markdown {
            let len = markdown.height();
            self.scroll = len.saturating_sub(self.content_area().height);
        }
    }

    pub fn handle_key_press(&mut self, key: KeyEvent) {
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
        let area = self.content_area();

        self.scroll = 0;
        self.markdown = Some(parser::parse_markdown(None, content, area.width));
    }

    fn content_area(&self) -> Rect {
        self.area.inner(Margin { horizontal: 2, vertical: 2 })
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let area = self.content_area();
        if let Some(markdown) = &mut self.markdown {
            markdown.set_scroll(self.scroll);
            let block = Block::new()
                .style(get_tui_theme().help_box)
                .borders(Borders::ALL)
                .border_type(BorderType::Double);
            block.render(self.area, frame.buffer_mut());

            if area.is_empty() {
                return;
            }

            // md-tui places every component at an absolute buffer row and clips against the
            // plain area height, so it only renders correctly in a buffer starting at row zero.
            let mut page = Buffer::empty(Rect::new(area.x, 0, area.width, area.height));
            page.set_style(page.area, get_tui_theme().help_box);
            for child in markdown.children() {
                if let Component::TextComponent(comp) = child {
                    if comp.y_offset().saturating_sub(comp.scroll_offset()) >= area.height || comp.y_offset() + comp.height() <= comp.scroll_offset() {
                        continue;
                    }

                    comp.clone().render(page.area, &mut page);
                }
            }

            let buffer = frame.buffer_mut();
            for y in 0..area.height {
                for x in area.x..area.right() {
                    buffer[(x, area.y + y)] = page[(x, y)].clone();
                }
            }
        }
    }

    pub fn set_area(&mut self, screen: Rect) {
        self.area = screen.inner(Margin { horizontal: 1, vertical: 2 });
        self.area.height += 1;
    }
}

impl From<&mut HelpViewState> for ScrollbarState {
    fn from(state: &mut HelpViewState) -> ScrollbarState {
        let max = state.markdown.as_ref().map_or(0, |m| m.height());
        let height = state.content_area().height;
        ScrollbarState::new(max.saturating_sub(height) as usize).position(state.scroll as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    const LIST_HELP: &str = "# Mode\n\
        \n\
        How callers are treated when the event is due.\n\
        \n\
        - Fixed: maintenance disconnects callers on time; online runs while they stay\n\
        - Slide: maintenance closes admission and waits for callers to leave;\n  online waits for an empty board without closing admission\n\
        - Idle: skips the occurrence while callers are online, in both execution types\n\
        \n\
        Enter opens the list, Up/Down selects, Enter confirms.\n";

    fn rendered(content: &str, width: u16, height: u16) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut state = HelpViewState::new();
        terminal
            .draw(|frame| {
                state.set_area(crate::app::get_screen_size(frame, false));
            })
            .unwrap();
        state.set_content(content);
        terminal.draw(|frame| state.draw(frame)).unwrap();

        let buffer = terminal.backend().buffer().clone();
        buffer
            .area
            .rows()
            .map(|row| row.columns().map(|cell| buffer[(cell.x, cell.y)].symbol().to_string()).collect::<String>())
            .collect()
    }

    #[test]
    fn every_bullet_of_a_list_is_shown() {
        for (width, height) in [(80, 25), (80, 40), (100, 30), (60, 20)] {
            let rows = rendered(LIST_HELP, width, height).join("\n");

            for bullet in ["Fixed:", "Slide:", "Idle:"] {
                assert!(rows.contains(bullet), "{width}x{height} lost the {bullet} bullet:\n{rows}");
            }
            assert!(rows.contains("Enter opens the list"), "{width}x{height} lost the closing paragraph:\n{rows}");
        }
    }

    #[test]
    fn scrolling_stays_inside_the_frame_and_reaches_the_last_line() {
        let mut content = String::from("# Long\n\n");
        for line in 0..60 {
            content.push_str(&format!("- entry {line}\n"));
        }

        let mut terminal = Terminal::new(TestBackend::new(80, 40)).unwrap();
        let mut state = HelpViewState::new();
        terminal
            .draw(|frame| {
                state.set_area(crate::app::get_screen_size(frame, false));
            })
            .unwrap();
        state.set_content(&content);
        state.handle_key_press(KeyEvent::from(KeyCode::End));
        terminal.draw(|frame| state.draw(frame)).unwrap();

        let buffer = terminal.backend().buffer().clone();
        let rows: Vec<String> = buffer
            .area
            .rows()
            .map(|row| row.columns().map(|cell| buffer[(cell.x, cell.y)].symbol().to_string()).collect::<String>())
            .collect();
        let screen = rows.join("\n");

        assert!(screen.contains("entry 59"), "the last entry is unreachable:\n{screen}");
        assert!(
            rows.iter().filter(|row| row.contains('╔') || row.contains('╚')).count() == 2,
            "the frame was overdrawn:\n{screen}"
        );
        for row in rows.iter().filter(|row| row.contains("entry")) {
            assert!(row.trim_start().starts_with('║'), "content escaped the frame:\n{screen}");
        }
    }
}
