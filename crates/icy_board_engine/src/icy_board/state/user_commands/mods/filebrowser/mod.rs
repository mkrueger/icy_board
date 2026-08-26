pub mod file_list;
pub use file_list::*;
use icy_engine::{EditableScreen, TextPane};
use regex::Regex;

use crate::icy_board::state::IcyBoardState;

pub mod more_prompt;

static RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| Regex::new("(\\S+)\\s+[\\.\\d]+\\s+(\\w+\\s+)?(\\d\\d/\\d\\d/\\d\\d)").unwrap());

impl IcyBoardState {
    /// Used for the ppe function SCRFILE
    pub fn scan_filename(&self, start_line: i32) -> Option<(i32, String)> {
        let mut y = start_line;
        let screen = self.display_screen();
        let height = screen.buffer.height();
        let width = screen.buffer.width();
        let top = screen.buffer.first_visible_line();
        while y < height {
            let mut str = String::new();
            for x in 0..width {
                let ch: icy_engine::AttributedChar = screen.buffer.char_at((x, top + y).into());
                str.push(ch.ch);
            }

            if let Some(cap) = RE.captures(&str) {
                let file_name = cap.get(1).unwrap().as_str();
                return Some((y, file_name.to_string()));
            }
            y += 1;
        }

        None
    }
}

#[cfg(test)]
mod test {
    use crate::icy_board::state::user_commands::mods::filebrowser::RE;

    #[test]
    fn test_regex() {
        let str = "3001-USM.ZIP  18.7 kB  07/27/95  █▀▀▀▀▀▀▀▀▀▀▀▀▀▀████████████████             ";
        assert!(RE.is_match(str));

        if let Some(cap) = RE.captures(str) {
            let file_name = cap.get(1).unwrap().as_str();
            assert_eq!(file_name, "3001-USM.ZIP");
            let date = cap.get(3).unwrap().as_str();
            assert_eq!(date, "07/27/95");
        }
    }
}
