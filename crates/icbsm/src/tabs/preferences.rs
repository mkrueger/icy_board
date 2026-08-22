use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::icy_board::{
    IcyBoard,
    icb_config::{PCB_SCREEN_COLOR_NAMES, PcbScreenColors},
};
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ListItem, ListValue, ResultState, TextFlags},
    get_text,
    icbconfigmenu::ICBConfigMenuUI,
    icbsetupmenu::IcbSetupMenuUI,
    select_menu::{MenuItem, SelectMenu},
    tab_page::{Page, PageMessage},
    theme::{dos_attribute_style, set_tui_theme},
};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Modifier,
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget},
};

pub struct EditorConfiguration {
    menu: ICBConfigMenuUI,
}

impl EditorConfiguration {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let lock = icy_board.lock().unwrap();
        let entries = vec![
            ConfigEntry::Separator,
            ConfigEntry::Item(
                ListItem::new(
                    get_text("icbsm_text_editor"),
                    ListValue::Text(128, TextFlags::None, lock.config.sysop.external_editor.clone()),
                )
                .with_label_width(20)
                .with_update_text_value(&|board: &Arc<Mutex<IcyBoard>>, value: String| {
                    board.lock().unwrap().config.sysop.external_editor = value;
                }),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    get_text("icbsm_graphics_editor"),
                    ListValue::Text(128, TextFlags::None, lock.config.sysop.graphics_editor.clone()),
                )
                .with_label_width(20)
                .with_update_text_value(&|board: &Arc<Mutex<IcyBoard>>, value: String| {
                    board.lock().unwrap().config.sysop.graphics_editor = value;
                }),
            ),
        ];
        drop(lock);
        Self {
            menu: ICBConfigMenuUI::new(
                get_text("icbsm_define_editors"),
                ConfigMenu {
                    obj: icy_board,
                    entry: entries,
                },
            ),
        }
    }
}

impl Page for EditorConfiguration {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.menu.render(frame, area);
    }

    fn request_status(&self) -> ResultState {
        self.menu.request_status()
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        self.menu.handle_key_press(key)
    }
}

pub struct ColorCustomization {
    page: IcbSetupMenuUI,
    icy_board: Arc<Mutex<IcyBoard>>,
}

impl ColorCustomization {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        Self {
            page: IcbSetupMenuUI::new(SelectMenu::new(vec![
                MenuItem::new(0, 'A', get_text("icbsm_color_default_1")),
                MenuItem::new(1, 'B', get_text("icbsm_color_default_2")),
                MenuItem::new(2, 'C', get_text("icbsm_color_bw")),
                MenuItem::new(3, 'D', get_text("icbsm_color_customize")),
            ]))
            .with_center_title(get_text("icbsm_color_title")),
            icy_board,
        }
    }

    fn apply_preset(&self, name: &str, palette: PcbScreenColors) {
        let mut board = self.icy_board.lock().unwrap();
        board.config.sysop.config_color_theme = name.to_string();
        board.config.sysop.config_color_configuration = palette.clone();
        set_tui_theme(&palette);
    }
}

impl Page for ColorCustomization {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.page.render(frame, area);
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if self.page.sub_pages.is_empty() && key.code == KeyCode::Esc {
            return PageMessage::Close;
        }
        let (state, selected) = self.page.handle_key_press(key);
        match selected {
            Some(0) => self.apply_preset("DEFAULT1", PcbScreenColors::default()),
            Some(1) => self.apply_preset("DEFAULT2", PcbScreenColors::default_2()),
            Some(2) => self.apply_preset("BLACK_AND_WHITE", PcbScreenColors::black_and_white()),
            Some(3) => return PageMessage::OpenSubPage(Box::new(CustomColorEditor::new(self.icy_board.clone()))),
            _ => {}
        }
        PageMessage::ResultState(state)
    }
}

struct CustomColorEditor {
    icy_board: Arc<Mutex<IcyBoard>>,
    selected: usize,
    show_instructions: bool,
    picker: Option<u8>,
}

impl CustomColorEditor {
    fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        Self {
            icy_board,
            selected: 2,
            show_instructions: true,
            picker: None,
        }
    }

    fn palette(&self) -> PcbScreenColors {
        self.icy_board.lock().unwrap().config.sysop.config_color_configuration.clone()
    }

    fn apply_color(&self, color: u8) {
        let mut board = self.icy_board.lock().unwrap();
        board.config.sysop.config_color_theme = "CUSTOM".to_string();
        board.config.sysop.config_color_configuration.colors[self.selected] = color;
        set_tui_theme(&board.config.sysop.config_color_configuration);
    }

    fn reset_colors(&self) {
        let mut board = self.icy_board.lock().unwrap();
        board.config.sysop.config_color_theme = "DEFAULT1".to_string();
        board.config.sysop.config_color_configuration = PcbScreenColors::default();
        set_tui_theme(&board.config.sysop.config_color_configuration);
    }

    fn preview_area(area: Rect) -> Rect {
        let width = area.width.min(80);
        let height = area.height.min(23);
        Rect::new(area.x + (area.width - width) / 2, area.y + (area.height - height) / 2, width, height)
    }

    fn render_preview(&self, frame: &mut Frame, area: Rect, colors: &[u8; 23]) {
        Clear.render(area, frame.buffer_mut());
        Block::new().style(dos_attribute_style(colors[0])).render(area, frame.buffer_mut());
        Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(dos_attribute_style(colors[0]))
            .render(area, frame.buffer_mut());

        let put = |frame: &mut Frame, x: u16, y: u16, text: &str, role: usize| {
            if x < area.width && y < area.height {
                frame.buffer_mut().set_string(area.x + x, area.y + y, text, dos_attribute_style(colors[role]));
            }
        };

        put(frame, 26, 1, "Headings and Screen Titles", 2);

        let menu = local_rect(area, 5, 3, 30, 11);
        Block::new()
            .borders(Borders::ALL)
            .border_style(dos_attribute_style(colors[3]))
            .render(menu, frame.buffer_mut());
        put(frame, 15, 4, "Main Menu", 4);
        put(frame, 7, 6, "A  Selected Menu Item", 5);
        put(frame, 7, 7, "B  Other Menu Items ...", 5);
        put(frame, 7, 8, "C  Other Menu Items ...", 5);
        put(frame, 7, 9, "D  Unavailable Menu Item", 8);
        put(frame, 7, 10, "E  Highlighted unavailable", 9);
        fill_text(frame, local_rect(area, 9, 6, 24, 1), "Selected Menu Item", colors[6]);
        fill_text(frame, local_rect(area, 9, 10, 24, 1), "Highlighted unavailable", colors[9]);
        fill_text(frame, local_rect(area, 6, 12, 27, 1), "Use arrow keys to move", colors[7]);

        let help = local_rect(area, 37, 3, 38, 11);
        Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .style(dos_attribute_style(colors[15]))
            .render(help, frame.buffer_mut());
        put(frame, 50, 5, "HELP TITLE", 16);
        put(frame, 50, 6, "----------", 16);
        put(frame, 65, 7, "page: 01", 13);
        put(frame, 48, 8, "Help Subtitle", 17);
        put(frame, 42, 10, "Text for help screen 12345", 18);
        fill_text(frame, local_rect(area, 39, 12, 34, 1), "press  PGDN  to go forward", colors[19]);

        put(frame, 2, 14, &"·".repeat(75), 0);
        put(frame, 3, 16, "Question #1 :", 10);
        put(frame, 3, 17, "Question #2 ?", 10);
        put(frame, 3, 18, "Question #3 :", 10);
        put(frame, 3, 19, "Question #4 :", 10);
        put(frame, 17, 16, "Unhighlighted Input Field", 11);
        fill_text(frame, local_rect(area, 17, 17, 25, 1), "Current Input Field", colors[12]);
        put(frame, 17, 18, "Unhighlighted Input Field", 11);
        put(frame, 17, 19, "Unhighlighted Input Field", 11);
        put(frame, 48, 16, "Descriptive on screen text", 13);
        put(frame, 48, 17, "--------------------------", 13);
        put(frame, 48, 18, "ABCDEFGHIJKLMNOPQRSTUVWXYZ", 13);
        fill_text(frame, local_rect(area, 19, 20, 42, 1), "Special Instructions or Descriptive Text", colors[14]);
        put(frame, 3, 22, "12:00:00  ---  01/01/88", 1);
        put(frame, 32, 22, "F1 = Help", 20);
        put(frame, 47, 22, "caps: OFF  num: OFF  ins: OFF", 1);
        for y in 3..20 {
            put(frame, 78, y, "│", 21);
        }
        put(frame, 78, 11, "█", 22);

        let (x, y) = ROLE_POINTS[self.selected];
        if x < area.width && y < area.height {
            let cell = frame.buffer_mut().cell_mut((area.x + x, area.y + y)).unwrap();
            cell.set_style(cell.style().add_modifier(Modifier::REVERSED));
        }
    }

    fn render_instructions(&self, frame: &mut Frame, area: Rect, colors: &[u8; 23]) {
        let popup = centered(area, 58.min(area.width), 15.min(area.height));
        Clear.render(popup, frame.buffer_mut());
        Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .style(dos_attribute_style(colors[15]))
            .render(popup, frame.buffer_mut());
        let inner = popup.inner(ratatui::layout::Margin { horizontal: 2, vertical: 1 });
        Paragraph::new(vec![
            Line::styled("INSTRUCTIONS", dos_attribute_style(colors[16])),
            Line::raw(""),
            Line::styled("1) Use the arrow keys to select an element.", dos_attribute_style(colors[18])),
            Line::styled("   Press ENTER to change its color.", dos_attribute_style(colors[18])),
            Line::raw(""),
            Line::styled("2) Choose a foreground/background combination", dos_attribute_style(colors[18])),
            Line::styled("   from the color matrix and press ENTER.", dos_attribute_style(colors[18])),
            Line::raw(""),
            Line::styled("3) When finished, press ESC to exit.", dos_attribute_style(colors[18])),
            Line::styled("   Press F5 to restore default colors.", dos_attribute_style(colors[18])),
            Line::raw(""),
            Line::styled(" press any key to continue ", dos_attribute_style(colors[19])),
        ])
        .alignment(Alignment::Center)
        .render(inner, frame.buffer_mut());
    }

    fn render_picker(&self, frame: &mut Frame, area: Rect, value: u8) {
        render_dos_picker(frame, area, &format!("{} ({:02X})", PCB_SCREEN_COLOR_NAMES[self.selected], value), value, 8);
    }
}

impl Page for CustomColorEditor {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let preview = Self::preview_area(area);
        let palette = self.palette();
        self.render_preview(frame, preview, &palette.colors);
        if self.show_instructions {
            self.render_instructions(frame, preview, &palette.colors);
        } else if let Some(value) = self.picker {
            self.render_picker(frame, preview, value);
        }
    }

    fn request_status(&self) -> ResultState {
        let color = self.picker.unwrap_or_else(|| self.palette().colors[self.selected]);
        ResultState::status_line(format!(
            "F5: defaults  |  {} ({:02X})  |  Arrows: select  Enter: change  Esc: exit",
            PCB_SCREEN_COLOR_NAMES[self.selected], color
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
                _ => value = move_dos_picker(value, key.code, 8),
            }
            if self.picker.is_some() {
                self.picker = Some(value);
            }
            return PageMessage::ResultState(self.request_status());
        }
        match key.code {
            KeyCode::Esc => return PageMessage::Close,
            KeyCode::Enter => self.picker = Some(self.palette().colors[self.selected]),
            KeyCode::Left | KeyCode::Up | KeyCode::BackTab => self.selected = previous_selection(self.selected),
            KeyCode::Right | KeyCode::Down | KeyCode::Tab => self.selected = next_selection(self.selected),
            KeyCode::Home => self.selected = PREVIEW_ORDER[0],
            KeyCode::End => self.selected = *PREVIEW_ORDER.last().unwrap(),
            _ => {}
        }
        PageMessage::ResultState(self.request_status())
    }
}

const ROLE_POINTS: [(u16, u16); 23] = [
    (1, 1),
    (8, 22),
    (38, 1),
    (5, 4),
    (18, 4),
    (8, 7),
    (10, 6),
    (8, 12),
    (8, 9),
    (10, 10),
    (5, 17),
    (20, 16),
    (20, 17),
    (52, 16),
    (35, 20),
    (37, 4),
    (53, 5),
    (52, 8),
    (50, 10),
    (52, 12),
    (34, 22),
    (78, 5),
    (78, 11),
];

const PREVIEW_ORDER: [usize; 23] = [0, 2, 3, 4, 15, 16, 21, 6, 5, 17, 8, 9, 18, 22, 7, 19, 10, 11, 13, 12, 14, 1, 20];

fn next_selection(selected: usize) -> usize {
    let position = PREVIEW_ORDER.iter().position(|role| *role == selected).unwrap_or(0);
    PREVIEW_ORDER[(position + 1) % PREVIEW_ORDER.len()]
}

fn previous_selection(selected: usize) -> usize {
    let position = PREVIEW_ORDER.iter().position(|role| *role == selected).unwrap_or(0);
    PREVIEW_ORDER[(position + PREVIEW_ORDER.len() - 1) % PREVIEW_ORDER.len()]
}

fn local_rect(area: Rect, x: u16, y: u16, width: u16, height: u16) -> Rect {
    Rect::new(
        area.x + x,
        area.y + y,
        width.min(area.width.saturating_sub(x)),
        height.min(area.height.saturating_sub(y)),
    )
}

pub(crate) fn centered(area: Rect, width: u16, height: u16) -> Rect {
    Rect::new(area.x + (area.width - width) / 2, area.y + (area.height - height) / 2, width, height)
}

pub(crate) fn render_dos_picker(frame: &mut Frame, area: Rect, title: &str, value: u8, backgrounds: u8) {
    let width = (title.chars().count() as u16 + 4).max(22).min(area.width);
    let height = (u16::from(backgrounds) + 4).min(area.height);
    let popup = centered(area, width, height);
    Clear.render(popup, frame.buffer_mut());
    Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .title(format!(" {title} "))
        .render(popup, frame.buffer_mut());
    let start_x = popup.x + (popup.width.saturating_sub(16)) / 2;
    let start_y = popup.y + 2;
    for background in 0..backgrounds {
        for foreground in 0..16u8 {
            let attribute = background << 4 | foreground;
            let mut style = dos_attribute_style(attribute);
            if attribute == value {
                style = style.add_modifier(Modifier::REVERSED);
            }
            frame
                .buffer_mut()
                .set_string(start_x + u16::from(foreground), start_y + u16::from(background), "X", style);
        }
    }
}

pub(crate) fn move_dos_picker(value: u8, key: KeyCode, backgrounds: u8) -> u8 {
    let background_mask = backgrounds.saturating_sub(1) << 4;
    match key {
        KeyCode::Left => value & 0xF0 | value.wrapping_sub(1) & 0x0F,
        KeyCode::Right => value & 0xF0 | value.wrapping_add(1) & 0x0F,
        KeyCode::Up => value & 0x0F | value.wrapping_sub(0x10) & background_mask,
        KeyCode::Down => value & 0x0F | value.wrapping_add(0x10) & background_mask,
        KeyCode::Home => value & 0xF0,
        KeyCode::End => value & 0xF0 | 0x0F,
        _ => value,
    }
}

fn fill_text(frame: &mut Frame, area: Rect, text: &str, attribute: u8) {
    let style = dos_attribute_style(attribute);
    Block::new().style(style).render(area, frame.buffer_mut());
    frame.buffer_mut().set_string(area.x, area.y, text, style);
}

#[cfg(test)]
mod color_editor_tests {
    use crossterm::event::KeyCode;

    use super::{move_dos_picker, next_selection, previous_selection};

    #[test]
    fn navigation_follows_a_stable_screen_order() {
        assert_eq!(next_selection(2), 3);
        assert_eq!(next_selection(4), 15);
        assert_eq!(previous_selection(15), 4);
        assert_eq!(previous_selection(0), 20);
    }

    #[test]
    fn shared_picker_honors_the_available_background_rows() {
        assert_eq!(move_dos_picker(0x70, KeyCode::Down, 8), 0x00);
        assert_eq!(move_dos_picker(0xF0, KeyCode::Down, 16), 0x00);
        assert_eq!(move_dos_picker(0x00, KeyCode::Up, 16), 0xF0);
    }
}
