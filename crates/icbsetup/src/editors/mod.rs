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
    get_text_args,
    tab_page::{InfoState, PageMessage},
};

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
