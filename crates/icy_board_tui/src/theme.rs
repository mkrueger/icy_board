use ratatui::style::{Color, Modifier, Style};
use std::sync::RwLock;

use icy_board_engine::icy_board::icb_config::PcbScreenColors;

#[derive(Clone, Copy)]
pub struct Theme {
    pub background: Style,
    pub title_bar: Style,
    pub app_title: Style,
    pub tabs: Style,
    pub tabs_selected: Style,

    pub key_binding: Style,
    pub key_binding_description: Style,

    pub status_line: Style,
    pub status_line_text: Style,

    pub menu_title: Style,
    pub menu_label: Style,

    pub item_separator: Style,
    pub item: Style,
    pub selected_item: Style,

    pub value: Style,
    pub true_value: Style,
    pub false_value: Style,

    pub edit_value: Style,

    pub dialog_box: Style,
    pub dialog_box_title: Style,
    pub dialog_box_scrollbar: Style,

    pub menu_box: Style,
    pub menu_box_title: Style,

    pub config_title: Style,
    pub group_title: Style,

    pub filter_text: Style,
    pub description_text: Style,

    pub text_field_text: Style,
    pub text_field_background: Style,
    pub text_field_filler_char: char,

    pub table: Style,
    pub table_inactive: Style,
    pub table_header: Style,
    pub help_box: Style,
    pub help_header: Style,

    pub swatch: bool,
}

lazy_static::lazy_static! {
    static ref TUI_THEME: RwLock<Theme> = RwLock::new(CLASSIC_THEME);
}

pub fn get_tui_theme() -> Theme {
    *TUI_THEME.read().unwrap()
}

pub fn set_tui_theme(colors: &PcbScreenColors) {
    *TUI_THEME.write().unwrap() = Theme::from_pcboard(colors);
}

pub(crate) const DOS_ANSI_INDEX: [u8; 16] = [0, 4, 2, 6, 1, 5, 3, 7, 8, 12, 10, 14, 9, 13, 11, 15];

fn dos_color(index: u8) -> Color {
    Color::Indexed(DOS_ANSI_INDEX[usize::from(index & 0x0F)])
}

pub fn dos_attribute_style(attribute: u8) -> Style {
    let style = Style::new().fg(dos_color(attribute)).bg(dos_color((attribute >> 4) & 0x07));
    if attribute & 0x80 != 0 {
        style.add_modifier(Modifier::SLOW_BLINK)
    } else {
        style
    }
}

impl Theme {
    pub fn from_pcboard(palette: &PcbScreenColors) -> Self {
        let colors = &palette.colors;
        Self {
            background: dos_attribute_style(colors[0]),
            title_bar: dos_attribute_style(colors[0]),
            app_title: dos_attribute_style(colors[2]).add_modifier(Modifier::BOLD),
            tabs: dos_attribute_style(colors[1]),
            tabs_selected: dos_attribute_style(colors[6]).add_modifier(Modifier::BOLD),
            key_binding: dos_attribute_style(colors[7]),
            key_binding_description: dos_attribute_style(colors[14]),
            status_line: dos_attribute_style(colors[1]),
            status_line_text: dos_attribute_style(colors[20]),
            menu_title: dos_attribute_style(colors[4]),
            menu_label: dos_attribute_style(colors[13]),
            item_separator: dos_attribute_style(colors[13]),
            item: dos_attribute_style(colors[5]),
            selected_item: dos_attribute_style(colors[6]),
            value: dos_attribute_style(colors[11]),
            true_value: dos_attribute_style(colors[11]),
            false_value: dos_attribute_style(colors[8]),
            edit_value: dos_attribute_style(colors[12]),
            dialog_box: dos_attribute_style(colors[0]),
            dialog_box_title: dos_attribute_style(colors[2]),
            dialog_box_scrollbar: dos_attribute_style(colors[21]),
            menu_box: dos_attribute_style(colors[3]),
            menu_box_title: dos_attribute_style(colors[4]),
            config_title: dos_attribute_style(colors[2]),
            group_title: dos_attribute_style(colors[2]).add_modifier(Modifier::UNDERLINED),
            filter_text: dos_attribute_style(colors[10]),
            description_text: dos_attribute_style(colors[14]),
            text_field_text: dos_attribute_style(colors[12]),
            text_field_background: dos_attribute_style(colors[12]),
            text_field_filler_char: ' ',
            table: dos_attribute_style(colors[5]),
            table_inactive: dos_attribute_style(colors[8]),
            table_header: dos_attribute_style(colors[2]),
            help_box: dos_attribute_style(colors[15]),
            help_header: dos_attribute_style(colors[16]),
            swatch: false,
        }
    }
}

pub static CLASSIC_THEME: Theme = Theme {
    background: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_GRAY),
    title_bar: Style::new().bg(DOS_RED),
    app_title: Style::new().fg(DOS_YELLOW).bg(DOS_RED).add_modifier(Modifier::BOLD),
    tabs: Style::new().fg(DOS_YELLOW).bg(DOS_RED),
    tabs_selected: Style::new()
        .bg(DOS_RED)
        .fg(DOS_YELLOW)
        .add_modifier(Modifier::BOLD)
        .add_modifier(Modifier::REVERSED),

    dialog_box: Style::new().bg(DOS_BLACK).fg(DOS_BLUE),
    dialog_box_title: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_RED),
    dialog_box_scrollbar: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_GRAY),

    menu_box: Style::new().bg(DOS_BLACK).fg(DOS_RED),
    menu_box_title: Style::new().bg(DOS_BLACK).fg(DOS_YELLOW),

    key_binding: Style::new().bg(DOS_BROWN).fg(DOS_BLACK),
    key_binding_description: Style::new().bg(DOS_BROWN).fg(DOS_BLACK),

    status_line: Style::new().bg(DOS_BLACK).fg(DOS_CYAN),

    item: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_GREEN),
    selected_item: Style::new().bg(DOS_CYAN).fg(DOS_YELLOW),
    item_separator: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_GRAY),

    value: Style::new().bg(DOS_BLACK).fg(DOS_CYAN),
    true_value: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_GREEN),
    false_value: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_RED),
    edit_value: Style::new().bg(DOS_RED).fg(DOS_WHITE),

    status_line_text: Style::new().bg(DOS_BLACK).fg(DOS_WHITE),
    menu_title: Style::new().bg(DOS_BLACK).fg(DOS_YELLOW),
    menu_label: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_GRAY),
    config_title: Style::new().bg(DOS_BLACK).fg(DOS_RED),
    group_title: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_GRAY).add_modifier(Modifier::UNDERLINED),
    filter_text: Style::new().bg(DOS_BLACK).fg(DOS_YELLOW),
    description_text: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_GRAY),

    table: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_GREEN),
    table_inactive: Style::new().bg(DOS_BLACK).fg(DOS_DARK_GRAY),
    table_header: Style::new().bg(DOS_BLACK).fg(DOS_WHITE),

    help_box: Style::new().bg(DOS_GREEN).fg(DOS_BLACK),
    help_header: Style::new().bg(DOS_GREEN).fg(DOS_YELLOW),

    text_field_text: Style::new().bg(DOS_RED).fg(DOS_WHITE),
    text_field_filler_char: ' ',
    text_field_background: Style::new().bg(DOS_RED).fg(DOS_WHITE),

    swatch: false,
};

pub static DEFAULT_THEME: Theme = Theme {
    background: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_GRAY),
    title_bar: Style::new().bg(DOS_BLUE),
    app_title: Style::new().fg(WHITE).bg(DOS_BLUE).add_modifier(Modifier::BOLD),
    tabs: Style::new().fg(DOS_WHITE).bg(DOS_BLUE),
    tabs_selected: Style::new()
        .fg(DOS_CYAN)
        .bg(DOS_LIGHT_CYAN)
        .add_modifier(Modifier::BOLD)
        .add_modifier(Modifier::REVERSED),

    menu_title: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_CYAN),
    menu_label: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_GRAY),

    dialog_box: Style::new().bg(DOS_BLACK).fg(DOS_DARK_GRAY),
    dialog_box_title: Style::new().bg(DOS_BLACK).fg(DOS_WHITE),
    dialog_box_scrollbar: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_GRAY),

    menu_box: Style::new().bg(DOS_BLACK).fg(DOS_DARK_GRAY),
    menu_box_title: Style::new().bg(DOS_BLACK).fg(DOS_WHITE),

    key_binding: Style::new().bg(DOS_DARK_GRAY).fg(DOS_LIGHT_GRAY),
    key_binding_description: Style::new().bg(DOS_BLACK).fg(DOS_DARK_GRAY),

    status_line: Style::new().bg(DOS_BLACK).fg(DOS_CYAN),
    status_line_text: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_CYAN),
    item: Style::new().bg(DOS_BLACK).fg(DOS_WHITE),
    selected_item: Style::new().bg(DOS_BLUE).fg(DOS_LIGHT_CYAN),
    item_separator: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_GRAY),
    config_title: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_CYAN),
    group_title: Style::new().bg(DOS_BLACK).fg(LIGHT_GRAY).add_modifier(Modifier::UNDERLINED),
    value: Style::new().bg(DOS_BLACK).fg(LIGHT_GRAY),
    true_value: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_GREEN),
    false_value: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_RED),

    edit_value: Style::new().bg(DOS_BLUE).fg(DOS_LIGHT_CYAN),
    table: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_GRAY),
    table_inactive: Style::new().bg(DOS_BLACK).fg(DOS_DARK_GRAY),
    table_header: Style::new().bg(DOS_BLACK).fg(DOS_CYAN),

    text_field_text: Style::new().bg(DOS_BLUE).fg(DOS_LIGHT_CYAN),
    text_field_background: Style::new().bg(DOS_BLUE).fg(DOS_LIGHT_GRAY),
    text_field_filler_char: '▒',

    filter_text: Style::new().bg(DOS_BLUE).fg(DOS_YELLOW),
    description_text: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_GRAY),
    help_box: Style::new().bg(DOS_BLUE).fg(DOS_LIGHT_GRAY),
    help_header: Style::new().bg(DOS_BLUE).fg(DOS_YELLOW),

    swatch: true,
};

const LIGHT_GRAY: Color = Color::Indexed(7);
const WHITE: Color = Color::Indexed(15);

pub const DOS_BLACK: Color = Color::Indexed(0);
pub const DOS_BLUE: Color = Color::Indexed(4);
pub const DOS_GREEN: Color = Color::Indexed(2);
pub const DOS_CYAN: Color = Color::Indexed(6);
pub const DOS_RED: Color = Color::Indexed(1);
pub const DOS_MAGENTA: Color = Color::Indexed(5);
pub const DOS_BROWN: Color = Color::Indexed(3);
pub const DOS_LIGHT_GRAY: Color = Color::Indexed(7);

pub const DOS_DARK_GRAY: Color = Color::Indexed(8);
pub const DOS_LIGHT_BLUE: Color = Color::Indexed(12);
pub const DOS_LIGHT_GREEN: Color = Color::Indexed(10);
pub const DOS_LIGHT_CYAN: Color = Color::Indexed(14);
pub const DOS_LIGHT_RED: Color = Color::Indexed(9);
pub const DOS_LIGHT_MAGENTA: Color = Color::Indexed(13);
pub const DOS_YELLOW: Color = Color::Indexed(11);
pub const DOS_WHITE: Color = Color::Indexed(15);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dos_palette_uses_screen_compatible_ansi_colors() {
        let colors = [
            DOS_BLACK,
            DOS_BLUE,
            DOS_GREEN,
            DOS_CYAN,
            DOS_RED,
            DOS_MAGENTA,
            DOS_BROWN,
            DOS_LIGHT_GRAY,
            DOS_DARK_GRAY,
            DOS_LIGHT_BLUE,
            DOS_LIGHT_GREEN,
            DOS_LIGHT_CYAN,
            DOS_LIGHT_RED,
            DOS_LIGHT_MAGENTA,
            DOS_YELLOW,
            DOS_WHITE,
        ];

        assert!(colors.into_iter().all(|color| matches!(color, Color::Indexed(0..=15))));
    }
}
