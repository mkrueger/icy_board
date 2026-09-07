pub mod accounting_rates;
pub mod areas;
pub mod bullettins;
pub mod command;
pub mod dirs;
pub mod door;
pub mod languages;
pub mod protocols;
pub mod sec_editor;
pub mod surveys;

use std::path::Path;

use icy_board_tui::{
    config_menu::{ConfigMenu, ConfigMenuState, ListValue},
    get_text, get_text_args,
    tab_page::{InfoState, PageMessage},
};

/// Align a single-column editor to its longest translated label, measured in
/// terminal cells rather than UTF-8 bytes. Keep the existing minimum width.
fn align_editor_labels<T>(menu: ConfigMenu<T>) -> ConfigMenu<T> {
    menu.with_aligned_labels()
}

/// Reopening a nested form resets selection and modal state, not its board root.
fn reset_config_state(state: &mut ConfigMenuState) {
    let path_base = state.path_base.take();
    *state = ConfigMenuState::default();
    state.path_base = path_base;
}

/// Nested forms have no status line; advertise browsing on their own border.
fn path_browse_hint<T>(menu: &ConfigMenu<T>, state: &ConfigMenuState) -> String {
    if !state.is_path_browser_open()
        && menu
            .get_item(state.selected)
            .is_some_and(|item| item.editable() && matches!(item.value, ListValue::Path(_)))
    {
        get_text("path_browser_shortcut")
    } else {
        String::new()
    }
}

#[cfg(test)]
mod layout_tests;

pub fn save_file(path: &Path, save: impl FnOnce() -> icy_board_engine::Res<()>) -> PageMessage {
    let result = path
        .parent()
        .map(std::fs::create_dir_all)
        .transpose()
        .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)
        .and_then(|_| save());

    match result {
        Ok(()) => PageMessage::Close,
        Err(err) => PageMessage::InfoBox(
            InfoState::Error,
            get_text_args("icb_setup_save_failed", [("error".to_string(), err.to_string())].into()),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_file_reports_an_error_instead_of_panicking() {
        let path = std::env::temp_dir().join("icbsetup-save-error/file.toml");
        let message = save_file(&path, || Err(std::io::Error::other("disk full").into()));
        assert!(matches!(message, PageMessage::InfoBox(InfoState::Error, text) if text.contains("disk full")));
    }

    #[test]
    fn save_file_creates_the_parent_and_closes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new/directory/file.toml");
        let message = save_file(&path, || {
            std::fs::write(&path, b"saved")?;
            Ok(())
        });
        assert!(matches!(message, PageMessage::Close));
        assert_eq!(std::fs::read(path).unwrap(), b"saved");
    }
}
