use crossterm::event::{KeyCode, KeyEvent};

use md_tui::nodes::root::{Component, ComponentRoot};
use md_tui::parser;

use md_tui::util::colors::ColorConfig;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect};
use ratatui::style::Modifier;
use ratatui::widgets::{Block, BorderType, Borders, ScrollbarState, Widget};

use crate::get_text;
use crate::hotkeys::{Hotkey, HotkeyBar};
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
        // PCBoard colours the help window's body with "Help Text", not with
        // the frame's "Help Box" attribute.
        let cfg: ColorConfig = ColorConfig {
            italic_color: get_tui_theme().help_text.fg.unwrap(),
            bold_color: get_tui_theme().help_text.fg.unwrap(),
            striketrough_color: get_tui_theme().help_text.fg.unwrap(),
            bold_italic_color: get_tui_theme().help_text.fg.unwrap(),

            code_fg_color: get_tui_theme().help_text.fg.unwrap(),
            code_bg_color: get_tui_theme().help_text.bg.unwrap(),

            link_color: get_tui_theme().help_text.fg.unwrap(),

            link_selected_fg_color: get_tui_theme().help_text.fg.unwrap(),
            link_selected_bg_color: get_tui_theme().help_text.bg.unwrap(),

            code_block_bg_color: get_tui_theme().help_text.bg.unwrap(),

            heading_fg_color: get_tui_theme().help_header.fg.unwrap(),
            heading_bg_color: get_tui_theme().help_header.bg.unwrap(),

            table_header_fg_color: get_tui_theme().help_text.fg.unwrap(),
            table_header_bg_color: get_tui_theme().help_text.bg.unwrap(),

            quote_bg_color: get_tui_theme().help_text.bg.unwrap(),

            file_tree_selected_fg_color: get_tui_theme().help_text.fg.unwrap(),
            file_tree_page_count_color: get_tui_theme().help_text.fg.unwrap(),
            file_tree_name_color: get_tui_theme().help_text.fg.unwrap(),
            file_tree_path_color: get_tui_theme().help_text.fg.unwrap(),

            quote_important: get_tui_theme().help_text.fg.unwrap(),
            quote_warning: get_tui_theme().help_text.fg.unwrap(),
            quote_tip: get_tui_theme().help_text.fg.unwrap(),
            quote_note: get_tui_theme().help_text.fg.unwrap(),
            quote_caution: get_tui_theme().help_text.fg.unwrap(),
            quote_default: get_tui_theme().help_text.fg.unwrap(),
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
        let inner = self.area.inner(Margin { horizontal: 2, vertical: 2 });
        // The bar sits above the frame; the first row fits in the bottom margin.
        let reserved = Self::hint_height(self.hint_width()).saturating_sub(1);
        Rect {
            height: inner.height.saturating_sub(reserved),
            ..inner
        }
    }

    /// The bar spans the frame's interior, wider than the text column.
    fn hint_width(&self) -> u16 {
        self.area.width.saturating_sub(2)
    }

    /// The window's own keys, in PCBoard's instruction-bar position.
    fn hints() -> HotkeyBar {
        let style = get_tui_theme().help_description;
        HotkeyBar::new([
            Hotkey::alternatives([KeyCode::Up, KeyCode::Down], get_text("hotkey_scroll")),
            Hotkey::alternatives([KeyCode::PageUp, KeyCode::PageDown], get_text("hotkey_page")),
            Hotkey::new(KeyCode::Home, get_text("hotkey_first")),
            Hotkey::new(KeyCode::End, get_text("hotkey_last")),
            Hotkey::new(KeyCode::Esc, get_text("hotkey_close")),
        ])
        .with_styles(style.add_modifier(Modifier::BOLD), style)
    }

    fn hint_height(width: u16) -> u16 {
        Self::hints().rows(width).len().min(2) as u16
    }

    fn render_hints(&self, buf: &mut Buffer) {
        if self.area.width < 4 || self.area.height < 3 {
            return;
        }
        let width = self.hint_width();
        let height = Self::hint_height(width);
        if height == 0 {
            return;
        }
        let area = Rect::new(self.area.x + 1, self.area.bottom() - 1 - height, width, height);
        Block::new().style(get_tui_theme().help_description).render(area, buf);
        for (line, row) in Self::hints().rows(width).into_iter().zip(area.rows()) {
            line.render(row, buf);
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let area = self.content_area();
        if let Some(markdown) = &mut self.markdown {
            markdown.set_scroll(self.scroll);
            let block = Block::new()
                .style(get_tui_theme().help_box)
                .border_style(get_tui_theme().help_text)
                .borders(Borders::ALL)
                .border_type(BorderType::Double);
            block.render(self.area, frame.buffer_mut());

            if area.is_empty() {
                return;
            }

            // md-tui places every component at an absolute buffer row and clips against the
            // plain area height, so it only renders correctly in a buffer starting at row zero.
            let mut page = Buffer::empty(Rect::new(area.x, 0, area.width, area.height));
            page.set_style(page.area, get_tui_theme().help_text);
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
            self.render_hints(frame.buffer_mut());
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

    #[test]
    fn frame_body_and_instruction_bar_use_their_own_pcboard_colours() {
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        let mut state = HelpViewState::new();
        terminal
            .draw(|frame| {
                state.set_area(crate::app::get_screen_size(frame, false));
            })
            .unwrap();
        state.set_content(LIST_HELP);
        terminal.draw(|frame| state.draw(frame)).unwrap();

        let buffer = terminal.backend().buffer().clone();
        let theme = get_tui_theme();
        let cell = |x: u16, y: u16| buffer[(x, y)].clone();
        let at = |needle: &str| {
            buffer
                .area
                .rows()
                .find_map(|row| {
                    let text: String = row.columns().map(|c| buffer[(c.x, c.y)].symbol().to_string()).collect();
                    text.contains(needle).then(|| (text.find(needle).unwrap() as u16, row.y))
                })
                .unwrap_or_else(|| panic!("{needle} not rendered"))
        };

        let (x, y) = at("╔");
        assert_eq!(cell(x, y).style().fg, theme.help_text.fg, "the frame follows the text colour");
        assert_eq!(cell(x, y).style().bg, theme.help_box.bg);

        let (x, y) = at("How callers");
        assert_eq!(cell(x, y).style().fg, theme.help_text.fg, "body text is Help Text, not Help Box");

        let (x, y) = at("Mode");
        assert_eq!(cell(x, y).style().fg, theme.help_header.fg);

        let (x, y) = at(&get_text("hotkey_close"));
        assert_eq!(cell(x, y).style().bg, theme.help_description.bg, "the bar uses Help Description");
        assert!(state.area.contains(ratatui::layout::Position::new(x, y)), "the bar stays inside the frame");
        assert!(y < state.area.bottom() - 1, "the bar sits above the bottom border");
        assert!(y >= state.content_area().bottom(), "the bar never covers help text");
    }
}
