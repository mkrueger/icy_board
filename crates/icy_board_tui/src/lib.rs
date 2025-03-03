use std::{collections::HashMap, fmt::Display, io::stdout};

use crossterm::{
    execute,
    style::{Attribute, Color, Print, SetAttribute, SetForegroundColor},
};

pub mod app;
pub mod cfg_menu_macros;
pub mod colors;
pub mod config_menu;
pub mod help_view;
pub mod icbconfigmenu;
pub mod icbsetupmenu;
pub mod insert_table;
pub mod pcb_line;
pub mod position_editor;
pub mod save_changes_dialog;
pub mod select_menu;
pub mod tab_page;
pub mod term;
pub mod text_field;
pub mod theme;

use i18n_embed::{
    DesktopLanguageRequester,
    fluent::{FluentLanguageLoader, fluent_language_loader},
};
use i18n_embed_fl::fl;
use ratatui::{Terminal, backend::CrosstermBackend, symbols::border};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "i18n"] // path to the compiled localization resources
struct Localizations;

pub type TerminalType = Terminal<CrosstermBackend<std::io::Stdout>>;

use once_cell::sync::Lazy;
pub static LANGUAGE_LOADER: Lazy<FluentLanguageLoader> = Lazy::new(|| {
    let loader = fluent_language_loader!();
    let requested_languages = DesktopLanguageRequester::requested_languages();
    let _result = i18n_embed::select(&loader, &Localizations, &requested_languages);
    loader
});

pub fn get_text(message_id: &str) -> String {
    if !crate::LANGUAGE_LOADER.has(message_id) {
        log::error!("Missing translation for: {}", message_id);
    }
    crate::LANGUAGE_LOADER.get(message_id)
}

pub fn get_text_args(message_id: &str, args: HashMap<String, String>) -> String {
    if !crate::LANGUAGE_LOADER.has(message_id) {
        log::error!("Missing translation for: {}", message_id);
    }
    crate::LANGUAGE_LOADER.get_args(message_id, args)
}

pub fn print_error<A: Display>(error: A) {
    execute!(
        stdout(),
        SetAttribute(Attribute::Bold),
        SetForegroundColor(Color::Red),
        Print(fl!(crate::LANGUAGE_LOADER, "error_cmd_line_label")),
        Print(" "),
        SetAttribute(Attribute::Reset),
        SetAttribute(Attribute::Bold),
        Print(error),
        Print("\n"),
        SetAttribute(Attribute::Reset)
    )
    .unwrap();
}

pub static BORDER_SET: border::Set = border::Set {
    top_left: "╓",
    top_right: "╖",
    bottom_left: "╙",
    bottom_right: "╜",
    vertical_left: "║",
    vertical_right: "║",
    horizontal_top: "─",
    horizontal_bottom: "─",
};
