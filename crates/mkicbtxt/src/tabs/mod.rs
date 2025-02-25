pub mod about;

pub use about::*;

pub mod record;
use crossterm::event::KeyEvent;
pub use record::*;

use icy_board_tui::TerminalType;
use ratatui::{Frame, layout::Rect};

use crate::app::ResultState;

pub trait TabPage {
    fn render(&mut self, frame: &mut Frame, area: Rect);

    fn grab_focus(&self) -> bool {
        false
    }

    fn handle_key_press(&mut self, _key: KeyEvent) -> ResultState {
        ResultState::default()
    }

    fn _request_edit_mode(&mut self, _terminal: &mut TerminalType, _full_screen: bool) -> ResultState {
        ResultState::default()
    }

    fn request_status(&self) -> ResultState {
        ResultState::default()
    }
}
