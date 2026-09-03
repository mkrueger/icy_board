use crate::Res;
use crate::icy_board::lookup_case_insensitive;
use crate::vm::expressions::to_base_36;
use jamjam::jam::JamMessageBase;
use std::path::{Path, PathBuf};

use super::{TerminalTarget, VirtualMachine};

impl VirtualMachine<'_> {
    pub async fn resolve_file<P: AsRef<Path>>(&self, file: &P) -> PathBuf {
        // A PPE that built this name with MID or a fixed width field hands us the padding
        // as well, and PCBoard opens the file regardless. Verified against PCBoard 15.4.
        let mut file = file.as_ref().to_string_lossy().trim_end().to_string();
        if std::path::MAIN_SEPARATOR == '/' {
            file = file.replace('\\', "/");
        } else if std::path::MAIN_SEPARATOR == '\\' {
            file = file.replace('/', "\\");
        }

        let board_root = self.icy_board_state.get_board().await.root_path.clone();
        let resolved = if let Some(stripped) = file.strip_prefix("C:/") {
            log::warn!("Absolute path detected: {file}, change the src file.");
            self.icy_board_state.get_board().await.resolve_file(&PathBuf::from(stripped))
        } else {
            self.icy_board_state.get_board().await.resolve_file(&file)
        };
        if resolved.exists() {
            return resolved;
        }
        // A brand-new absolute path whose parent directory is real - such as one built from
        // `TempPath()` - is a modern path that simply doesn't have its target file yet, not a
        // stale DOS import to go hunting for below the PPE.
        if resolved.is_absolute() && resolved.parent().is_some_and(Path::exists) {
            return resolved;
        }
        // A bare name is the board's, the way PCBoard read it from its own directory - only a
        // path that leads somewhere else is worth looking for below the PPE.
        if !file.contains(std::path::MAIN_SEPARATOR) {
            return resolved;
        }
        self.resolve_below_ppe(&file, &board_root).unwrap_or(resolved)
    }

    /// An imported PPE still names its files the way they lay on the sysop's DOS drive.
    /// Whatever is left of such a path below the PPE's own directory - or below the board,
    /// which is where the PPE directories went - is where the file lives now.
    fn resolve_below_ppe(&self, file: &str, board_root: &Path) -> Option<PathBuf> {
        let dir = self.file_name.parent()?;
        let from_dos_drive = file.starts_with('/') || file.chars().nth(1) == Some(':');
        let mut rel = PathBuf::from(file);
        loop {
            if rel.is_relative() && !rel.as_os_str().is_empty() {
                for base in [dir, board_root] {
                    let candidate = lookup_case_insensitive(&base.join(&rel));
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
            }
            // Only a path off a drive holds directories that are gone; a relative one
            // says where the file is and is taken as it stands.
            if !from_dos_drive {
                return None;
            }
            let mut components = rel.components();
            components.next()?;
            let rest = components.as_path().to_path_buf();
            if rest.as_os_str().is_empty() {
                // Nothing of the path is left, so a file still to be written goes next to the PPE.
                let name = PathBuf::from(file).file_name()?.to_os_string();
                return Some(dir.join(name));
            }
            rel = rest;
        }
    }

    /// The message base a conference/area pair addresses, or `None` when the
    /// PPE named one that does not exist.
    pub async fn message_base_path(&self, conference: i32, area: i32) -> Option<PathBuf> {
        let board = self.icy_board_state.get_board().await;
        let conf = board.conferences.get(conference as usize)?;
        Some(conf.areas.as_ref()?.get(area as usize)?.path.clone())
    }

    /// Runs `read` against the open message base for `path`, opening it when the last
    /// call was for another area. The handle is what a walk saves: reading a message
    /// costs a seek rather than opening the base again.
    pub fn with_message_base<R>(&mut self, path: &Path, read: impl FnOnce(&mut JamMessageBase) -> jamjam::Result<R>) -> jamjam::Result<R> {
        if self.message_base.as_ref().is_none_or(|(cached, _)| cached != path) {
            self.message_base = Some((path.to_path_buf(), JamMessageBase::open(path)?));
        }
        let Some((_, base)) = self.message_base.as_mut() else {
            unreachable!("the base was just cached")
        };
        read(base)
    }

    /// Forgets the open message base, so the next read opens it again. A write goes
    /// through its own handle, which leaves what this one knows behind.
    pub fn invalidate_message_base(&mut self) {
        self.message_base = None;
        self.cached_msg_header = None;
    }

    pub(super) async fn set_rip_mouseregion(
        &mut self,
        num: i32,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        font_x: i32,
        font_y: i32,
        invert: bool,
        clear: bool,
        text: String,
    ) -> Res<()> {
        let rip_cmd = format!(
            "|M{}{}{}{}{}{}{}{}{}",
            to_base_36(2, num),
            to_base_36(2, (x1 - 1) * font_x),
            to_base_36(2, (y1 - 1) * font_y),
            to_base_36(2, (x2 - 1) * font_x),
            to_base_36(2, (y2 - 1) * font_y),
            i32::from(invert),
            i32::from(clear),
            "00000", // unused
            text
        );
        self.icy_board_state
            .write_raw(TerminalTarget::Both, rip_cmd.chars().collect::<Vec<char>>().as_slice())
            .await
    }
}
