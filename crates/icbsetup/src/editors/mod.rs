pub mod accounting_rates;
pub mod areas;
pub mod bullettins;
pub mod command;
mod common;
pub mod dirs;
pub mod door;
pub mod events;
pub mod languages;
mod list;
pub mod protocols;
pub mod sec_editor;
pub mod surveys;

pub(crate) use common::{EditorDialog, EditorSaveChanges, list_editor_frame, render_config_form, render_editor_footer, standalone_editor_frame};
pub(crate) use list::EditorList;

use std::path::Path;

use icy_board_tui::{
    chrome::frame_title,
    config_menu::{ConfigMenu, ConfigMenuState, ListValue},
    get_text_args,
    tab_page::{InfoState, PageMessage},
    theme::get_tui_theme,
};
use ratatui::{
    layout::Alignment,
    widgets::{Block, BorderType, Borders, Padding},
};

/// The frame every list editor shares, so all of them keep one look.
pub(crate) fn list_frame(title: String) -> Block<'static> {
    Block::new()
        .title_alignment(Alignment::Center)
        .title(frame_title(title, get_tui_theme().dialog_box_title))
        .style(get_tui_theme().dialog_box)
        .padding(Padding::new(2, 2, 1, 1))
        .borders(Borders::ALL)
        .border_set(icy_board_tui::BORDER_SET)
}

/// The nested form frame, matching the shared popup dialogs.
pub(crate) fn popup_frame(title: String) -> Block<'static> {
    Block::new()
        .title_alignment(Alignment::Center)
        .title(frame_title(title, get_tui_theme().dialog_box_title))
        .style(get_tui_theme().dialog_box)
        .padding(Padding::new(2, 2, 1, 1))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
}

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
fn path_browse_hint<T>(menu: &ConfigMenu<T>, state: &ConfigMenuState) -> bool {
    !state.is_path_browser_open()
        && menu
            .get_item(state.selected)
            .is_some_and(|item| item.editable() && matches!(item.value, ListValue::Path(_)))
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
    fn editor_frame_titles_have_space_between_text_and_border() {
        for block in [list_frame("Event Editor".into()), popup_frame("Edit Event".into())] {
            let mut buffer = ratatui::buffer::Buffer::empty(ratatui::layout::Rect::new(0, 0, 40, 3));
            ratatui::widgets::Widget::render(block, buffer.area, &mut buffer);
            let top: String = (0..40).map(|x| buffer[(x, 0)].symbol()).collect();
            assert!(top.contains(" Event Editor ") || top.contains(" Edit Event "), "unpadded title: {top}");
        }
    }

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

/// Footer IDs now render typed key hints; other IDs stay plain translations.
#[cfg(test)]
pub(crate) fn hint_text(id: &str) -> String {
    if icy_board_tui::hotkeys::presets::PRESET_IDS.contains(&id) {
        icy_board_tui::hotkeys::HotkeyBar::for_id(id).line().to_string()
    } else {
        icy_board_tui::get_text(id)
    }
}
