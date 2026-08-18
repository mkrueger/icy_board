use std::{collections::HashMap, fmt::Display, io::stderr, path::Path};

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
pub mod inactive_options;
pub mod insert_table;
pub mod message_box;
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
    let has_translation = crate::LANGUAGE_LOADER.has(message_id);
    if !has_translation {
        log::error!("Missing translation for: {}", message_id);
    }
    let text = crate::LANGUAGE_LOADER.get(message_id);
    if message_id.ends_with("-help") && (!has_translation || is_placeholder(&text)) {
        return fallback_help(message_id.trim_end_matches("-help"));
    }
    text
}

fn is_placeholder(text: &str) -> bool {
    text.trim().is_empty() || text.trim().eq_ignore_ascii_case("TODO")
}

fn fallback_help(message_id: &str) -> String {
    let label = crate::LANGUAGE_LOADER.get(message_id);
    let status = crate::LANGUAGE_LOADER.get(&format!("{message_id}-status"));
    let explanation = if !is_placeholder(&status) && status != format!("{message_id}-status") {
        status
    } else if message_id.starts_with("user_sec_") {
        format!("Set the minimum security level a caller needs to use {label}.")
    } else {
        match message_id {
            "min_pwd_length" => "Set the minimum number of characters accepted for a new password.".to_string(),
            "connection_info_enabled" => "Enable or disable this connection service.".to_string(),
            "connection_info_port" => "Set the TCP port on which this connection service listens.".to_string(),
            "connection_info_address" => "Set the local address on which this connection service listens.".to_string(),
            "connection_info_display_file" => "Select the screen shown when a caller connects through this service.".to_string(),
            _ => format!("Configure {label}."),
        }
    };
    format!("# {label}\n\n{explanation}")
}

pub fn get_text_args(message_id: &str, args: HashMap<String, String>) -> String {
    if !crate::LANGUAGE_LOADER.has(message_id) {
        log::error!("Missing translation for: {}", message_id);
    }
    crate::LANGUAGE_LOADER.get_args(message_id, args)
}

pub fn print_error<A: Display>(error: A) {
    execute!(
        stderr(),
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

pub fn print_board_config_not_found(program: &str, path: &Path) {
    print_error(get_text_args(
        "error_board_config_not_found",
        HashMap::from([("path".to_string(), path.display().to_string())]),
    ));
    eprintln!(
        "{}",
        get_text_args("error_board_config_help", HashMap::from([("program".to_string(), program.to_string())]),)
    );
}

pub fn print_input_file_not_found(program: &str, path: &Path) {
    print_error(get_text_args(
        "error_input_file_not_found",
        HashMap::from([("path".to_string(), path.display().to_string())]),
    ));
    eprintln!(
        "{}",
        get_text_args(
            "error_input_file_help",
            HashMap::from([("program".to_string(), program.to_string()), ("path".to_string(), path.display().to_string()),]),
        )
    );
}

pub fn print_parent_board_config_not_found(program: &str, path: &Path) {
    print_error(get_text_args(
        "error_parent_board_config_not_found",
        HashMap::from([("path".to_string(), path.display().to_string())]),
    ));
    eprintln!(
        "{}",
        get_text_args("error_parent_board_config_help", HashMap::from([("program".to_string(), program.to_string())]),)
    );
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

#[cfg(test)]
mod help_tests {
    use super::get_text;

    const ENGLISH_FTL: &str = include_str!("../i18n/en/icy_board_tui.ftl");

    #[test]
    fn every_declared_help_entry_resolves_without_a_placeholder() {
        let help_keys: Vec<_> = ENGLISH_FTL
            .lines()
            .filter_map(|line| line.split_once('='))
            .map(|(key, _)| key.trim())
            .filter(|key| key.ends_with("-help"))
            .collect();

        assert!(!help_keys.is_empty());
        for key in help_keys {
            let help = get_text(key);
            assert!(!help.trim().is_empty(), "{key} resolved to empty help");
            assert!(!help.to_ascii_uppercase().contains("TODO"), "{key} still contains a TODO placeholder");
        }
    }

    #[test]
    fn help_explains_the_option_rather_than_naming_it() {
        assert!(get_text("user_sec_cmd_d-help").contains("batch transfer level"));
        assert!(get_text("connection_info_port-help").contains("TCP port"));
        assert!(get_text("paths_trashcan_user-help").contains("may not be registered"));

        for key in ["user_sec_cmd_d-help", "connection_info_port-help", "paths_trashcan_user-help"] {
            assert!(get_text(key).lines().count() > 2, "{key} should carry a heading and an explanation");
        }
    }

    #[test]
    fn a_help_key_without_a_translation_still_says_something() {
        let help = get_text("future_setup_option-help");

        assert!(!help.contains("future_setup_option-help"));
        assert!(!help.trim().is_empty());
    }
}
