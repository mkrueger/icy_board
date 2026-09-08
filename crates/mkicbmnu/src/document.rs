//! Menu persistence and bounded, whole-document undo history.
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use color_eyre::{Result, eyre::eyre};
use icy_board_engine::icy_board::{menu::Menu, write_atomic};
use icy_board_tui::get_text;

pub struct Document {
    pub path: PathBuf,
    saved: Menu,
    disk: Option<Vec<u8>>,
    undo: Vec<Menu>,
    redo: Vec<Menu>,
}

impl Document {
    pub fn new(path: &Path, menu: &Menu, create: bool) -> Result<Self> {
        let disk = if create {
            if path.try_exists()? {
                return Err(eyre!(get_text("mnu_app_exists")));
            }
            None
        } else {
            Some(fs::read(path)?)
        };
        Ok(Self {
            path: path.to_path_buf(),
            saved: menu.clone(),
            disk,
            undo: Vec::new(),
            redo: Vec::new(),
        })
    }

    pub fn is_dirty(&self, menu: &Menu) -> bool {
        self.disk.is_none() || menu != &self.saved
    }

    pub fn record(&mut self, before: Menu, after: &Menu) {
        if &before == after {
            return;
        }
        if self.undo.len() == 100 {
            self.undo.remove(0);
        }
        self.undo.push(before);
        self.redo.clear();
    }

    pub fn undo(&mut self, menu: &mut Menu) {
        if let Some(previous) = self.undo.pop() {
            self.redo.push(std::mem::replace(menu, previous));
        }
    }

    pub fn redo(&mut self, menu: &mut Menu) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(menu, next));
        }
    }

    pub fn save(&mut self, menu: &Menu) -> Result<()> {
        let current = match fs::read(&self.path) {
            Ok(bytes) => Some(bytes),
            Err(err) if err.kind() == io::ErrorKind::NotFound => None,
            Err(err) => return Err(err.into()),
        };
        if current != self.disk {
            return Err(eyre!(get_text("mnu_app_external_change")));
        }
        let bytes = toml::to_string(menu)?.into_bytes();
        if self.disk.is_none() {
            // A new destination must never replace a file created after our check.
            let dir = self.path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
            let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
            tmp.write_all(&bytes)?;
            tmp.as_file().sync_all()?;
            tmp.persist_noclobber(&self.path).map_err(|err| err.error)?;
            if let Ok(dir) = fs::File::open(dir) {
                let _ = dir.sync_all();
            }
        } else {
            write_atomic(&self.path, &bytes)?;
        }
        self.disk = Some(bytes);
        self.saved = menu.clone();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_is_deferred_and_never_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new.mnu");
        let menu = Menu::default();
        let mut doc = Document::new(&path, &menu, true).unwrap();
        assert!(!path.exists());
        assert!(doc.is_dirty(&menu));
        fs::write(&path, "external").unwrap();
        assert!(doc.save(&menu).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "external");
        assert!(Document::new(&path, &menu, true).is_err());
    }

    #[test]
    fn save_roundtrips_hidden_fields_and_clears_dirty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("menu.mnu");
        let menu = Menu {
            force_display: true,
            pass_through: true,
            prompts: vec![(".DE".into(), "Wahl?".into())],
            ..Menu::default()
        };
        let mut doc = Document::new(&path, &menu, true).unwrap();
        doc.save(&menu).unwrap();
        assert!(!doc.is_dirty(&menu));
        assert!(toml::from_str::<Menu>(&fs::read_to_string(&path).unwrap()).unwrap() == menu);
        fs::write(&path, "external").unwrap();
        assert!(doc.save(&menu).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "external");
    }

    #[test]
    fn failed_save_preserves_dirty_and_retry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing/menu.mnu");
        let menu = Menu::default();
        let mut doc = Document::new(&path, &menu, true).unwrap();
        assert!(doc.save(&menu).is_err());
        assert!(doc.is_dirty(&menu));
        fs::create_dir(path.parent().unwrap()).unwrap();
        doc.save(&menu).unwrap();
        assert!(!doc.is_dirty(&menu));
    }

    #[test]
    fn undo_redo_and_new_branch() {
        let dir = tempfile::tempdir().unwrap();
        let mut menu = Menu::default();
        let mut doc = Document::new(&dir.path().join("menu.mnu"), &menu, true).unwrap();
        let before = menu.clone();
        menu.title = "A".into();
        doc.record(before, &menu);
        doc.save(&menu).unwrap();
        doc.undo(&mut menu);
        assert_eq!(menu.title, "");
        assert!(doc.is_dirty(&menu));
        doc.redo(&mut menu);
        assert_eq!(menu.title, "A");
        assert!(!doc.is_dirty(&menu));
        doc.undo(&mut menu);
        let before = menu.clone();
        menu.title = "B".into();
        doc.record(before, &menu);
        doc.redo(&mut menu);
        assert_eq!(menu.title, "B");
    }
}
