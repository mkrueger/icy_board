use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::icy_board::{
    IcyBoard,
    icb_config::{ColorConfiguration, IcbColor},
};
use icy_board_tui::{
    config_menu::ResultState,
    tab_page::{Page, PageMessage},
    theme::{dos_attribute_style, get_tui_theme},
};
use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget},
};

use super::preferences::{centered, move_dos_picker, render_dos_picker};

const ROLE_NAMES: [&str; 10] = [
    "Filename",
    "File Size",
    "File Date",
    "Description",
    "Header and Column Titles",
    "Text Lines",
    "Duplicate Files",
    "Deleted Files",
    "Offline Files",
    "New File Marker",
];

const ROLE_POINTS: [(u16, u16); 10] = [(2, 5), (15, 5), (25, 5), (35, 5), (2, 1), (2, 17), (2, 6), (25, 22), (25, 21), (32, 5)];

const SAMPLE_LINES: [(u16, &str); 18] = [
    (1, "Sample DIR File illustrating color selections"),
    (3, "Filename       Size      Date    Description of File Contents"),
    (4, "============ ========  ========  ============================================="),
    (5, "145HELP.ZIP     34802  06-19-90* PCBoard v14.5 standard help files - colorized"),
    (6, "                                 using the @X macros."),
    (7, "                                 Uploaded by: Terry West"),
    (8, "ANSI145.ZIP      7350  03-20-90* Displays and edits your PCBoard 14.5 text"),
    (9, "                                 files.  Converts @X variables to the correct"),
    (10, "                                 ANSI statements."),
    (11, "                                 Uploaded by: Mark Herring"),
    (12, "145MENUS.ZIP    12110  08-03-90* Menu enhancements for 14.5, now includes to"),
    (13, "                                 expert menus for the new expert prompts"),
    (14, "                                 Uploaded by: Dean Gangstee"),
    (17, "A text line in the middle of the DIR file (possibly used for sub-titles)"),
    (18, "------------------------------------------------------------------------"),
    (20, "FIRSTONE.ZIP     2176  08-31-90* First test file"),
    (21, "TESTFILE.ZIP    14536  OFF-LINE  This file has been moved offline"),
    (22, "TROJAN.ZIP       2583  DELETED   This file has been deleted"),
];

pub struct DirColorEditor {
    icy_board: Arc<Mutex<IcyBoard>>,
    selected: usize,
    show_instructions: bool,
    picker: Option<u8>,
}

impl DirColorEditor {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        Self {
            icy_board,
            selected: 4,
            show_instructions: true,
            picker: None,
        }
    }

    fn colors(&self) -> [u8; 10] {
        colors_from_config(&self.icy_board.lock().unwrap().config.color_configuration)
    }

    fn apply_color(&self, color: u8) {
        set_config_color(&mut self.icy_board.lock().unwrap().config.color_configuration, self.selected, color);
    }

    fn reset_colors(&self) {
        reset_dir_colors(&mut self.icy_board.lock().unwrap().config.color_configuration);
    }

    fn preview_area(area: Rect) -> Rect {
        let width = area.width.min(80);
        let height = area.height.min(23);
        Rect::new(area.x + (area.width - width) / 2, area.y + (area.height - height) / 2, width, height)
    }

    fn render_preview(&self, frame: &mut Frame, area: Rect, colors: &[u8; 10]) {
        Clear.render(area, frame.buffer_mut());
        Block::new()
            .style(Style::new().bg(ratatui::style::Color::Indexed(0)))
            .render(area, frame.buffer_mut());

        for (y, text) in SAMPLE_LINES {
            if y < area.height {
                frame.buffer_mut().set_string(area.x + 1, area.y + y, text, Style::default());
            }
        }

        paint_rect(frame, area, 1, 1, 78, 4, colors[4]);
        paint_rect(frame, area, 1, 5, 12, 22, colors[0]);
        paint_rect(frame, area, 14, 5, 21, 22, colors[1]);
        paint_rect(frame, area, 24, 5, 31, 20, colors[2]);
        paint_rect(frame, area, 32, 5, 32, 22, colors[9]);
        paint_rect(frame, area, 34, 5, 78, 22, colors[3]);
        paint_rect(frame, area, 24, 21, 31, 21, colors[8]);
        paint_rect(frame, area, 24, 22, 31, 22, colors[7]);
        paint_rect(frame, area, 1, 6, 78, 7, colors[6]);
        paint_rect(frame, area, 1, 9, 78, 11, colors[6]);
        paint_rect(frame, area, 1, 13, 78, 14, colors[6]);
        paint_rect(frame, area, 1, 15, 78, 19, colors[5]);

        let (x, y) = ROLE_POINTS[self.selected];
        if x < area.width
            && y < area.height
            && let Some(cell) = frame.buffer_mut().cell_mut((area.x + x, area.y + y))
        {
            cell.set_style(cell.style().add_modifier(Modifier::REVERSED));
        }
    }

    fn render_instructions(&self, frame: &mut Frame, area: Rect) {
        let theme = get_tui_theme();
        let popup = centered(area, 58.min(area.width), 15.min(area.height));
        Clear.render(popup, frame.buffer_mut());
        Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .style(theme.help_box)
            .render(popup, frame.buffer_mut());
        let inner = popup.inner(Margin { horizontal: 2, vertical: 1 });
        Paragraph::new(vec![
            Line::styled("INSTRUCTIONS", theme.help_header),
            Line::raw(""),
            Line::styled("1) Use the arrow keys to select an element.", theme.help_box),
            Line::styled("   Press ENTER to change its color.", theme.help_box),
            Line::raw(""),
            Line::styled("2) Choose a color from the matrix", theme.help_box),
            Line::styled("   and press ENTER.", theme.help_box),
            Line::raw(""),
            Line::styled("3) When finished, press ESC to exit.", theme.help_box),
            Line::styled("   Press F5 to restore default colors.", theme.help_box),
            Line::raw(""),
            Line::styled(" press any key to continue ", theme.key_binding),
        ])
        .alignment(Alignment::Center)
        .render(inner, frame.buffer_mut());
    }
}

impl Page for DirColorEditor {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let preview = Self::preview_area(area);
        let colors = self.colors();
        self.render_preview(frame, preview, &colors);
        if self.show_instructions {
            self.render_instructions(frame, preview);
        } else if let Some(value) = self.picker {
            render_dos_picker(frame, preview, &format!("{} ({value:02X})", ROLE_NAMES[self.selected]), value, 16);
        }
    }

    fn request_status(&self) -> ResultState {
        let color = self.picker.unwrap_or_else(|| self.colors()[self.selected]);
        ResultState::status_line(format!(
            "F5: defaults  |  {} ({color:02X})  |  Arrows: select  Enter: change  Esc: exit",
            ROLE_NAMES[self.selected]
        ))
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if key.code == KeyCode::F(5) {
            self.reset_colors();
            self.show_instructions = false;
            self.picker = None;
            return PageMessage::ResultState(self.request_status());
        }
        if self.show_instructions {
            self.show_instructions = false;
            return PageMessage::ResultState(self.request_status());
        }
        if let Some(mut value) = self.picker {
            match key.code {
                KeyCode::Esc => self.picker = None,
                KeyCode::Enter => {
                    self.apply_color(value);
                    self.picker = None;
                }
                _ => value = move_dos_picker(value, key.code, 16),
            }
            if self.picker.is_some() {
                self.picker = Some(value);
            }
            return PageMessage::ResultState(self.request_status());
        }
        match key.code {
            KeyCode::Esc => return PageMessage::Close,
            KeyCode::Enter => self.picker = Some(self.colors()[self.selected]),
            KeyCode::Left | KeyCode::Up | KeyCode::BackTab => {
                self.selected = (self.selected + ROLE_NAMES.len() - 1) % ROLE_NAMES.len();
            }
            KeyCode::Right | KeyCode::Down | KeyCode::Tab => {
                self.selected = (self.selected + 1) % ROLE_NAMES.len();
            }
            KeyCode::Home => self.selected = 0,
            KeyCode::End => self.selected = ROLE_NAMES.len() - 1,
            _ => {}
        }
        PageMessage::ResultState(self.request_status())
    }
}

fn dos(color: &IcbColor, fallback: u8) -> u8 {
    match color {
        IcbColor::Dos(value) => *value,
        IcbColor::None | IcbColor::IcyEngine(_) => fallback,
    }
}

fn colors_from_config(colors: &ColorConfiguration) -> [u8; 10] {
    [
        dos(&colors.file_name, 0x0E),
        dos(&colors.file_size, 0x02),
        dos(&colors.file_date, 0x04),
        dos(&colors.file_description, 0x0B),
        dos(&colors.file_head, 0x06),
        dos(&colors.file_text, 0x06),
        dos(&colors.file_duplicate, 0x03),
        dos(&colors.file_deleted, 0x0F),
        dos(&colors.file_offline, 0x05),
        dos(&colors.file_new_file, 0x8F),
    ]
}

fn set_config_color(colors: &mut ColorConfiguration, role: usize, value: u8) {
    let color = IcbColor::Dos(value);
    match role {
        0 => colors.file_name = color,
        1 => colors.file_size = color,
        2 => colors.file_date = color,
        3 => colors.file_description = color,
        4 => colors.file_head = color,
        5 => colors.file_text = color,
        6 => colors.file_duplicate = color,
        7 => colors.file_deleted = color,
        8 => colors.file_offline = color,
        9 => colors.file_new_file = color,
        _ => {}
    }
}

fn reset_dir_colors(colors: &mut ColorConfiguration) {
    let defaults = colors_from_config(&ColorConfiguration::default());
    for (role, value) in defaults.into_iter().enumerate() {
        set_config_color(colors, role, value);
    }
}

fn paint_rect(frame: &mut Frame, area: Rect, x1: u16, y1: u16, x2: u16, y2: u16, attribute: u8) {
    let style = dos_attribute_style(attribute);
    let right = x2.min(area.width.saturating_sub(1));
    let bottom = y2.min(area.height.saturating_sub(1));
    for y in y1..=bottom {
        for x in x1..=right {
            if let Some(cell) = frame.buffer_mut().cell_mut((area.x + x, area.y + y)) {
                cell.set_style(style);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use icy_board_engine::icy_board::icb_config::{ColorConfiguration, IcbColor};

    use super::{colors_from_config, reset_dir_colors, set_config_color};

    #[test]
    fn pcboard_role_order_matches_the_persisted_configuration() {
        let mut config = ColorConfiguration::default();
        assert_eq!(colors_from_config(&config), [0x0E, 0x02, 0x04, 0x0B, 0x06, 0x06, 0x03, 0x0F, 0x05, 0x8F]);

        set_config_color(&mut config, 8, 0x35);
        assert!(matches!(config.file_offline, IcbColor::Dos(0x35)));
    }

    #[test]
    fn resetting_dir_colors_does_not_touch_other_color_roles() {
        let mut config = ColorConfiguration::default();
        config.default = IcbColor::Dos(0x31);
        set_config_color(&mut config, 0, 0x44);
        set_config_color(&mut config, 9, 0x22);

        reset_dir_colors(&mut config);

        assert_eq!(colors_from_config(&config), [0x0E, 0x02, 0x04, 0x0B, 0x06, 0x06, 0x03, 0x0F, 0x05, 0x8F]);
        assert!(matches!(config.default, IcbColor::Dos(0x31)));
    }
}
