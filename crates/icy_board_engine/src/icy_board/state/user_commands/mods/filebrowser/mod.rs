pub mod file_list;
pub use file_list::*;
use icy_engine::{Screen, TextPane};
use regex::Regex;

use crate::icy_board::state::IcyBoardState;

pub mod more_prompt;

lazy_static::lazy_static! {
    static ref re:Regex = Regex::new("(\\S+)\\s+[\\.\\d]+\\s+(\\w+\\s+)?(\\d\\d/\\d\\d/\\d\\d)").unwrap();
}

impl IcyBoardState {
    /// Used for the ppe function SCRFILE
    pub fn scan_filename(&self, start_line: i32) -> Option<(i32, String)> {
        let mut y = start_line;
        let height = self.user_screen.buffer.get_height();
        let width = self.user_screen.buffer.get_width();
        let top = self.user_screen.buffer.get_first_visible_line();
        while y < height {
            let mut str = String::new();
            for x in 0..width {
                let ch: icy_engine::AttributedChar = self.user_screen.buffer.get_char((x, top + y).into());
                str.push(ch.ch);
            }

            if let Some(cap) = re.captures(&str) {
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
    use crate::icy_board::state::user_commands::mods::filebrowser::re;

    #[test]
    fn test_regex() {
        let str = "3001-USM.ZIP  18.7 kB  07/27/95  █▀▀▀▀▀▀▀▀▀▀▀▀▀▀████████████████             ";
        assert!(re.is_match(&str));

        if let Some(cap) = re.captures(&str) {
            let file_name = cap.get(1).unwrap().as_str();
            assert_eq!(file_name, "3001-USM.ZIP");
            let date = cap.get(3).unwrap().as_str();
            assert_eq!(date, "07/27/95");
        }
    }
}
