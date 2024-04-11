use ratatui::prelude::*;

pub struct Theme {
    pub content: Style,

    pub title_bar: Style,
    pub app_title: Style,
    pub tabs: Style,
    pub tabs_selected: Style,

    pub key_binding: Style,
    pub key_binding_description: Style,

    pub status_line: Style,

    pub item: Style,
    pub selected_item: Style,

    pub value: Style,
    pub edit_value: Style,

    pub content_box: Style,
}

pub const THEME: Theme = Theme {
    content: Style::new().bg(DARK_BLUE).fg(LIGHT_GRAY),

    title_bar: Style::new().bg(DOS_BLUE),
    app_title: Style::new().fg(WHITE).bg(DOS_BLUE).add_modifier(Modifier::BOLD),
    tabs: Style::new().fg(DOS_WHITE).bg(DOS_BLUE),
    tabs_selected: Style::new()
        .fg(DOS_CYAN)
        .bg(DOS_LIGHT_CYAN)
        .add_modifier(Modifier::BOLD)
        .add_modifier(Modifier::REVERSED),

    content_box: Style::new().bg(DOS_BLACK).fg(DOS_DARKGRAY),

    key_binding: Style::new().bg(DOS_DARKGRAY).fg(DOS_LIGHTGRAY),
    key_binding_description: Style::new().bg(DOS_BLACK).fg(DOS_DARKGRAY),

    status_line: Style::new().bg(DOS_BLACK).fg(DOS_CYAN),

    item: Style::new().bg(DOS_BLACK).fg(DOS_WHITE),
    selected_item: Style::new().bg(DOS_BLUE).fg(DOS_LIGHT_CYAN),

    value: Style::new().bg(DOS_BLACK).fg(LIGHT_GRAY),
    edit_value: Style::new().bg(DOS_BLUE).fg(DOS_LIGHT_CYAN),
    
};

const DARK_BLUE: Color = Color::Rgb(16, 24, 48);
const LIGHT_BLUE: Color = Color::Rgb(64, 96, 192);
const LIGHT_YELLOW: Color = Color::Rgb(192, 192, 96);
const LIGHT_GREEN: Color = Color::Rgb(64, 192, 96);
const LIGHT_RED: Color = Color::Rgb(192, 96, 96);
const RED: Color = Color::Rgb(215, 0, 0);
const BLACK: Color = Color::Rgb(8, 8, 8); // not really black, often #080808
const DARK_GRAY: Color = Color::Rgb(68, 68, 68);
const MID_GRAY: Color = Color::Rgb(128, 128, 128);
const LIGHT_GRAY: Color = Color::Rgb(188, 188, 188);
const WHITE: Color = Color::Rgb(238, 238, 238); // not really white, often #eeeeee

pub const DOS_BLACK: Color = Color::Rgb(0, 0, 0);
pub const DOS_BLUE: Color = Color::Rgb(0, 0, 0xAA);
pub const DOS_GREEN: Color = Color::Rgb(0, 0xAA, 0);
pub const DOS_CYAN: Color = Color::Rgb(0, 0xAA, 0xAA);
pub const DOS_RED: Color = Color::Rgb(0xAA, 0, 0);
pub const DOS_MAGENTA: Color = Color::Rgb(0xAA, 0, 0xAA);
pub const DOS_BROWN: Color = Color::Rgb(0xAA, 0x55, 0);
pub const DOS_LIGHTGRAY: Color = Color::Rgb(0xAA, 0xAA, 0xAA);

pub const DOS_DARKGRAY: Color = Color::Rgb(0x55, 0x55, 0x55);
pub const DOS_LIGHT_BLUE: Color = Color::Rgb(0x55, 0x55, 0xFF);
pub const DOS_LIGHT_GREEN: Color = Color::Rgb(0x55, 0xFF, 0x55);
pub const DOS_LIGHT_CYAN: Color = Color::Rgb(0x55, 0xFF, 0xFF);
pub const DOS_LIGHT_RED: Color = Color::Rgb(0xFF, 0x55, 0x55);
pub const DOS_LIGHT_MAGENTA: Color = Color::Rgb(0xFF, 0x55, 0xFF);
pub const DOS_YELLOW: Color = Color::Rgb(0xFF, 0xFF, 0x55);
pub const DOS_WHITE: Color = Color::Rgb(0xFF, 0xFF, 0xFF);
