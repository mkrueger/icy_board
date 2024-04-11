use std::{collections::HashMap, fmt::Display, io::stdout};

use crossterm::{
    execute,
    style::{Attribute, Color, Print, SetAttribute, SetForegroundColor},
};

pub mod colors;
pub mod term;
pub mod theme;

use i18n_embed::{
    fluent::{fluent_language_loader, FluentLanguageLoader},
    DesktopLanguageRequester,
};
use i18n_embed_fl::fl;
use ratatui::{backend::CrosstermBackend, Terminal};
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

pub fn get_text(txt: &str) -> String {
    crate::LANGUAGE_LOADER.get(txt)
}

pub fn get_text_args(txt: &str, args: HashMap<String, String>) -> String {
    crate::LANGUAGE_LOADER.get_args(txt, args)
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
