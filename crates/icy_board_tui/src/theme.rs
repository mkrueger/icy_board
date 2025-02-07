use ratatui::style::{Color, Modifier, Style};

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
    pub edit_value: Style,

    pub content_box: Style,
    pub content_box_title: Style,
    pub config_title: Style,

    pub filter_text: Style,
    pub description_text: Style,

    pub text_field_text: Style,
    pub text_field_background: Style,
    pub text_field_filler_char: char,

    pub table: Style,
    pub table_inactive: Style,
    pub table_header: Style,
    pub help_box: Style,
    pub swatch: bool,
}

pub fn get_tui_theme() -> &'static Theme {
    &CLASSIC_THEME
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

    content_box: Style::new().bg(DOS_BLACK).fg(DOS_RED),

    key_binding: Style::new().bg(DOS_BROWN).fg(DOS_BLACK),
    key_binding_description: Style::new().bg(DOS_BROWN).fg(DOS_BLACK),

    status_line: Style::new().bg(DOS_BLACK).fg(DOS_CYAN),

    item: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_GREEN),
    selected_item: Style::new().bg(DOS_CYAN).fg(DOS_YELLOW),
    item_separator: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_GRAY),

    value: Style::new().bg(DOS_BLACK).fg(DOS_CYAN),
    edit_value: Style::new().bg(DOS_RED).fg(DOS_WHITE),
    table_header: Style::new().bg(DOS_BLACK).fg(DOS_RED),

    status_line_text: Style::new().bg(DOS_BLACK).fg(DOS_WHITE),
    menu_title: Style::new().bg(DOS_BLACK).fg(DOS_YELLOW),
    menu_label: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_GRAY),
    content_box_title: Style::new().bg(DOS_BLACK).fg(DOS_YELLOW),
    config_title: Style::new().bg(DOS_BLACK).fg(DOS_RED),
    filter_text: Style::new().bg(DOS_BLACK).fg(DOS_YELLOW),
    description_text: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_GRAY),
    table: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_GREEN),
    table_inactive: Style::new().bg(DOS_BLACK).fg(DOS_DARK_GRAY),
    help_box: Style::new().bg(DOS_BLUE).fg(DOS_LIGHT_GRAY),

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

    content_box: Style::new().bg(DOS_BLACK).fg(DOS_DARK_GRAY),
    content_box_title: Style::new().bg(DOS_BLACK).fg(DOS_WHITE),

    key_binding: Style::new().bg(DOS_DARK_GRAY).fg(DOS_LIGHT_GRAY),
    key_binding_description: Style::new().bg(DOS_BLACK).fg(DOS_DARK_GRAY),

    status_line: Style::new().bg(DOS_BLACK).fg(DOS_CYAN),
    status_line_text: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_CYAN),
    item: Style::new().bg(DOS_BLACK).fg(DOS_WHITE),
    selected_item: Style::new().bg(DOS_BLUE).fg(DOS_LIGHT_CYAN),
    item_separator: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_GRAY),
    config_title: Style::new().bg(DOS_BLACK).fg(DOS_LIGHT_CYAN),
    value: Style::new().bg(DOS_BLACK).fg(LIGHT_GRAY),
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
    swatch: true,
};

// const DARK_BLUE: Color = Color::Rgb(16, 24, 48);
const LIGHT_GRAY: Color = Color::Rgb(188, 188, 188);
const WHITE: Color = Color::Rgb(238, 238, 238); // not really white, often #eeeeee

pub const DOS_BLACK: Color = Color::Rgb(0, 0, 0);
pub const DOS_BLUE: Color = Color::Rgb(0, 0, 0xAA);
pub const DOS_GREEN: Color = Color::Rgb(0, 0xAA, 0);
pub const DOS_CYAN: Color = Color::Rgb(0, 0xAA, 0xAA);
pub const DOS_RED: Color = Color::Rgb(0xAA, 0, 0);
pub const DOS_MAGENTA: Color = Color::Rgb(0xAA, 0, 0xAA);
pub const DOS_BROWN: Color = Color::Rgb(0xAA, 0x55, 0);
pub const DOS_LIGHT_GRAY: Color = Color::Rgb(0xAA, 0xAA, 0xAA);

pub const DOS_DARK_GRAY: Color = Color::Rgb(0x55, 0x55, 0x55);
pub const DOS_LIGHT_BLUE: Color = Color::Rgb(0x55, 0x55, 0xFF);
pub const DOS_LIGHT_GREEN: Color = Color::Rgb(0x55, 0xFF, 0x55);
pub const DOS_LIGHT_CYAN: Color = Color::Rgb(0x55, 0xFF, 0xFF);
pub const DOS_LIGHT_RED: Color = Color::Rgb(0xFF, 0x55, 0x55);
pub const DOS_LIGHT_MAGENTA: Color = Color::Rgb(0xFF, 0x55, 0xFF);
pub const DOS_YELLOW: Color = Color::Rgb(0xFF, 0xFF, 0x55);
pub const DOS_WHITE: Color = Color::Rgb(0xFF, 0xFF, 0xFF);
